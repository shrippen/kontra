//! Folders on disk the user linked (LH-BIB-17): read in place, never copied. A thread walks
//! them, so a large or slow folder (a network share) never holds up a frame; the page asks
//! for a rescan when it opens and reads the result when `library_changed` says so.
//!
//!   link, unlink, page opens ──► thread: walk each folder ──► tick ──► found, changed
//!   a folder gone or unreadable ──► marked missing, no error
//!
//! Only file names are read here; title, artist and tempo are read behind the walk by a second
//! thread (`meta`), since parsing every file of a large folder costs seconds.

mod meta;

use crate::Core;
use kontra_session::FileKind;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, TryRecvError};

/// Folders below the linked one that are looked into.
pub(crate) const DEPTH_MAX: usize = 8;
/// Files kept per linked folder; more are left out and the folder is marked as cut off.
pub(crate) const FILES_MAX: usize = 5000;
/// Plain `.xml` is MusicXML, but also every other XML file: not taken from a folder walk.
const PLAIN_XML: &str = "xml";

/// What walking one linked folder found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Walked {
    pub(crate) files: Vec<PathBuf>,
    /// Not there or not readable.
    pub(crate) missing: bool,
    /// More than `FILES_MAX` files: the rest is not listed.
    pub(crate) cut_off: bool,
}

/// Walks one linked folder; tests put in a slow or fixed one.
pub(crate) type Walk = fn(&Path) -> Walked;

#[derive(Debug)]
pub(crate) struct Folders {
    walk: Walk,
    found: BTreeMap<PathBuf, Walked>,
    job: Option<Receiver<BTreeMap<PathBuf, Walked>>>,
    /// Title, artist and tempo read from the files behind the walk.
    pub(crate) metas: meta::Metas,
    /// Something changed while a walk ran (a link): walk again when it ends.
    again: bool,
    changed: bool,
}

impl Default for Folders {
    fn default() -> Self {
        Self {
            walk: walk_folder,
            found: BTreeMap::new(),
            metas: meta::Metas::default(),
            job: None,
            again: false,
            changed: false,
        }
    }
}

impl Folders {
    pub(crate) fn walked(&self, root: &Path) -> Option<&Walked> {
        self.found.get(root)
    }
}

impl Core {
    /// Links `dir` and starts reading it. An error for what is no folder, or a name the library
    /// cannot keep (`keepable`); linking a folder again does nothing.
    pub fn link_folder(&mut self, dir: &Path) -> Result<(), String> {
        self.keepable(dir)?;
        if !dir.is_dir() {
            let name = dir.to_string_lossy().into_owned();
            return Err(self.tr("library.folder.none", &[("name", name)]));
        }
        if !self.library.linked_folders.iter().any(|f| f == dir) {
            self.library.linked_folders.push(dir.to_path_buf());
            self.save_library();
        }
        self.library_rescan();
        Ok(())
    }

    /// Takes the link away; the files stay where they are, and so does their progress.
    pub fn unlink_folder(&mut self, dir: &Path) {
        self.library.linked_folders.retain(|f| f != dir);
        self.folders.found.remove(dir);
        self.folders.changed = true;
        self.save_library();
    }

    /// Reads the linked folders again in the background. One walk at a time: asked during a
    /// walk, it walks once more after that.
    pub fn library_rescan(&mut self) {
        // Without folders only the library's own entries are read (`read_meta`).
        if self.library.linked_folders.is_empty() {
            self.read_meta();
            return;
        }
        if self.folders.job.is_some() {
            self.folders.again = true;
            return;
        }
        let (tell, result) = channel();
        let walk = self.folders.walk;
        let roots = self.library.linked_folders.clone();
        std::thread::spawn(move || {
            let walked = roots
                .into_iter()
                .map(|root| {
                    let found = walk(&root);
                    (root, found)
                })
                .collect();
            // The core may be gone by now; nobody waits for the result then.
            let _ = tell.send(walked);
        });
        self.folders.job = Some(result);
    }

    /// True while a walk runs.
    #[cfg(test)]
    pub(crate) fn walk_busy(&self) -> bool {
        self.folders.job.is_some()
    }

    /// True once a walk ended, a link went or files were read; the page then reads
    /// `library_json` again.
    pub fn library_changed(&mut self) -> bool {
        self.take_walk();
        let read = self.meta_changed();
        std::mem::take(&mut self.folders.changed) || read
    }

    /// Keeps the result of a finished walk and what was read of the files; called at every
    /// tick.
    pub(crate) fn take_walk(&mut self) {
        self.take_meta();
        let Some(job) = self.folders.job.as_ref() else {
            return;
        };
        let found = match job.try_recv() {
            Ok(found) => found,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => BTreeMap::new(),
        };
        self.folders.job = None;

        // A folder unlinked during the walk stays gone.
        let linked = &self.library.linked_folders;
        self.folders.found = found
            .into_iter()
            .filter(|(root, _)| linked.contains(root))
            .collect();
        self.folders.changed = true;
        if std::mem::take(&mut self.folders.again) {
            self.library_rescan();
            return;
        }
        // Titles, artists and tempos of what was found, behind the walk (`meta`).
        self.read_meta();
    }

    #[cfg(test)]
    pub(crate) fn walk_with(&mut self, walk: Walk) {
        self.folders.walk = walk;
    }
}

/// The scores below `root`: no hidden folders or files, no links to folders (a link back up
/// would loop), at most `DEPTH_MAX` levels and `FILES_MAX` files, in path order.
pub(crate) fn walk_folder(root: &Path) -> Walked {
    let mut walked = Walked::default();
    let mut open = vec![(root.to_path_buf(), 0)];
    if std::fs::read_dir(root).is_err() {
        walked.missing = true;
        return walked;
    }

    while let Some((dir, depth)) = open.pop() {
        // An unreadable subfolder is left out, not a failure of the whole.
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if is_hidden(&path) {
                continue;
            }
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                if depth < DEPTH_MAX {
                    open.push((path, depth + 1));
                }
                continue;
            }
            // A link to a file counts; a link to a folder is neither.
            let file = kind.is_file() || (kind.is_symlink() && path.is_file());
            if !file || !is_score(&path) {
                continue;
            }
            if walked.files.len() >= FILES_MAX {
                walked.cut_off = true;
                walked.files.sort();
                return walked;
            }
            walked.files.push(path);
        }
    }
    walked.files.sort();
    walked
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with('.'))
}

fn is_score(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .filter(|e| !e.eq_ignore_ascii_case(PLAIN_XML))
        .is_some_and(|e| FileKind::from_extension(e).is_some())
}
