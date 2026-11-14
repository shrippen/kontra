//! Title, artist, tempo and song id of files the library lists but never opened (QA 13.11.: Q5,
//! Q6). Files of linked folders and old entries without a tempo are read by a thread after
//! each walk; a cache in the cache folder keeps what was read, by path, size and modification
//! time, so the next start parses only what changed.
//!
//! ```text
//!   walk ends ──► paths ──► thread: stat, parse what changed ──► chunks ──► rows fill in
//!                                                                  ├─► old entries get a tempo
//!                                                                  └─► a moved entry follows its file
//! ```

use crate::{piece_name, store, Core};
use kontra_session::{import_file, ProgressStore};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::time::UNIX_EPOCH;

const CACHE_FILE: &str = "library-meta.json";
/// Files read before the page hears of them: a large folder fills in step by step.
const CHUNK: usize = 32;

/// What a score says about itself.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct FileMeta {
    pub(crate) title: String,
    pub(crate) artist: String,
    pub(crate) bpm: Option<f64>,
    pub(crate) song_id: String,
}

/// A file's size and modification time: when both are unchanged, it is not read again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct Stamp {
    size: u64,
    modified_ms: u64,
}

/// One file read; `meta` is `None` for a file that did not parse (tried once per change).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Known {
    stamp: Stamp,
    meta: Option<FileMeta>,
}

type Chunk = Vec<(PathBuf, Known)>;

#[derive(Debug, Default)]
pub(crate) struct Metas {
    known: BTreeMap<PathBuf, Known>,
    loaded: bool,
    job: Option<Receiver<Chunk>>,
    /// Asked to read while a read ran: read once more when it ends.
    again: bool,
    /// Read since the cache was last saved.
    dirty: bool,
    /// Read since the page last asked.
    changed: bool,
    /// The paths the last read looked at: the cache keeps only these, so files deleted or
    /// unlinked leave it.
    wanted: Vec<PathBuf>,
}

impl Metas {
    pub(crate) fn get(&self, path: &Path) -> Option<&FileMeta> {
        self.known.get(path)?.meta.as_ref()
    }
}

impl Core {
    /// True while files are read in the background.
    #[cfg(test)]
    pub(crate) fn meta_busy(&self) -> bool {
        self.folders.metas.job.is_some()
    }

    /// Reads, in the background, the linked folders' files and the library's entries that have
    /// no tempo yet. Called when a walk ended.
    pub(crate) fn read_meta(&mut self) {
        if self.folders.metas.job.is_some() {
            self.folders.metas.again = true;
            return;
        }
        if !self.folders.metas.loaded {
            self.folders.metas.loaded = true;
            self.folders.metas.known = load_cache(&self.meta_cache_path());
        }

        let mut paths: Vec<PathBuf> = self.walked_files().into_iter().cloned().collect();
        paths.extend(
            self.library
                .recent
                .iter()
                .filter(|e| e.tempo_bpm.is_none())
                .map(|e| e.path.clone()),
        );
        let known: BTreeMap<PathBuf, Stamp> = paths
            .iter()
            .filter_map(|p| Some((p.clone(), self.folders.metas.known.get(p)?.stamp)))
            .collect();

        self.folders.metas.wanted = paths.clone();
        let (tell, result) = channel();
        std::thread::spawn(move || {
            let mut chunk = Chunk::new();
            for path in paths {
                let Some(stamp) = stamp_of(&path) else {
                    continue;
                };
                if known.get(&path) == Some(&stamp) {
                    continue;
                }
                let meta = read(&path);
                chunk.push((path, Known { stamp, meta }));
                if chunk.len() >= CHUNK && tell.send(std::mem::take(&mut chunk)).is_err() {
                    return;
                }
            }
            // The core may be gone; nobody waits then.
            let _ = tell.send(chunk);
        });
        self.folders.metas.job = Some(result);
    }

    /// Keeps what the thread read so far; `library_changed` then says so. Called at every tick.
    pub(crate) fn take_meta(&mut self) {
        loop {
            let Some(job) = self.folders.metas.job.as_ref() else {
                return;
            };
            match job.try_recv() {
                Ok(chunk) => {
                    self.folders.metas.dirty |= !chunk.is_empty();
                    self.folders.metas.known.extend(chunk);
                    self.apply_meta();
                    self.folders.metas.changed = true;
                }
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => {
                    self.folders.metas.job = None;
                    let metas = &mut self.folders.metas;
                    let before = metas.known.len();
                    let wanted: HashSet<&PathBuf> = metas.wanted.iter().collect();
                    metas.known.retain(|path, _| wanted.contains(path));
                    if std::mem::take(&mut metas.dirty) || metas.known.len() != before {
                        save_cache(&self.meta_cache_path(), &self.folders.metas.known);
                    }
                    if std::mem::take(&mut self.folders.metas.again) {
                        self.read_meta();
                    }
                    return;
                }
            }
        }
    }

    /// True once files were read since the last call.
    pub(crate) fn meta_changed(&mut self) -> bool {
        self.take_meta();
        std::mem::take(&mut self.folders.metas.changed)
    }

    /// The files of all linked folders as the last walk found them.
    fn walked_files(&self) -> Vec<&PathBuf> {
        self.library
            .linked_folders
            .iter()
            .filter_map(|root| self.folders.walked(root))
            .flat_map(|walked| walked.files.iter())
            .collect()
    }

    /// Gives old entries their tempo and lets an entry whose file is gone follow the file of a
    /// linked folder with the same song id (moved there), when exactly one has it.
    fn apply_meta(&mut self) {
        let mut save = false;
        for entry in &mut self.library.recent {
            if entry.tempo_bpm.is_some() {
                continue;
            }
            if let Some(bpm) = self.folders.metas.get(&entry.path).and_then(|m| m.bpm) {
                entry.tempo_bpm = Some(bpm);
                save = true;
            }
        }

        let files: Vec<&PathBuf> = self
            .walked_files()
            .into_iter()
            .filter(|p| self.library.entry(p).is_none())
            .collect();
        let mut moves: Vec<(PathBuf, PathBuf)> = Vec::new();
        for entry in &self.library.recent {
            if entry.song_id.is_empty() || entry.path.exists() {
                continue;
            }
            let mut same = files.iter().filter(|p| {
                self.folders
                    .metas
                    .get(p)
                    .is_some_and(|m| m.song_id == entry.song_id)
            });
            let (Some(found), None) = (same.next(), same.next()) else {
                continue;
            };
            // One entry per file (review #326: a piece downloaded twice): the most recent one
            // follows, the other stays as missing.
            if moves.iter().all(|(_, to)| to != *found) {
                moves.push((entry.path.clone(), (*found).clone()));
            }
        }
        for (from, to) in &moves {
            self.library.rename_path(from, to);
        }

        if save || !moves.is_empty() {
            self.save_library();
        }
    }

    fn meta_cache_path(&self) -> PathBuf {
        store::cache_dir(&self.data_dir).join(CACHE_FILE)
    }
}

fn stamp_of(path: &Path) -> Option<Stamp> {
    let data = std::fs::metadata(path).ok()?;
    let modified = data.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
    Some(Stamp {
        size: data.len(),
        modified_ms: u64::try_from(modified.as_millis()).unwrap_or(u64::MAX),
    })
}

/// Parses `path` as the import does; `None` when it is no score Kontra reads.
fn read(path: &Path) -> Option<FileMeta> {
    let import = import_file(path).ok()?;
    let arrangement = import.arrangement(import.suggested().unwrap_or(0)).ok()?;
    Some(FileMeta {
        title: piece_name(&arrangement.title, path),
        artist: arrangement.artist.clone(),
        bpm: Some(arrangement.tempo.bpm_at(0)),
        song_id: ProgressStore::song_id(&arrangement),
    })
}

/// The cache, empty when it is missing or unreadable (it is made again).
fn load_cache(path: &Path) -> BTreeMap<PathBuf, Known> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Saves the cache; a failure costs only a second read on the next start.
fn save_cache(path: &Path, known: &BTreeMap<PathBuf, Known>) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(text) = serde_json::to_string(known) {
        let _ = std::fs::write(path, text);
    }
}
