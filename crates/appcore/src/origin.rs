//! Where a piece of the library came from (LH-BIB-17). Computed from the path, never stored:
//! moving a file between places changes its origin, and progress follows the song id, so it
//! stays.
//!
//! ```text
//! bundled exercises ─────────────────► Exercise
//! data folder, exercises/ ───────────► Exercise   (generated exercises)
//! data folder, pieces/ ──────────────► Generated  (pieces in a form, from a style or a plan)
//! inside a linked folder ────────────► Folder     (the longest linked folder wins)
//! download folder ───────────────────► Download
//! anything else ─────────────────────► Imported
//! ```

use crate::bundled::{EXERCISES_DIR, PIECES_DIR};
use crate::Core;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Origin {
    Exercise,
    Generated,
    Download,
    Imported,
    Folder,
}

impl Origin {
    /// In the order the tree and the group headers show them.
    pub(crate) const ALL: [Origin; 5] = [
        Origin::Exercise,
        Origin::Generated,
        Origin::Download,
        Origin::Imported,
        Origin::Folder,
    ];

    pub(crate) fn key(self) -> &'static str {
        match self {
            Origin::Exercise => "exercise",
            Origin::Generated => "generated",
            Origin::Download => "download",
            Origin::Imported => "imported",
            Origin::Folder => "folder",
        }
    }

    pub(crate) fn from_key(key: &str) -> Option<Origin> {
        Origin::ALL.into_iter().find(|o| o.key() == key)
    }
}

/// The places an origin is read from.
#[derive(Debug, Clone, Default)]
pub(crate) struct Places {
    /// Bundled exercises and the bundled demo piece.
    pub(crate) bundled: Vec<PathBuf>,
    pub(crate) exercises: PathBuf,
    pub(crate) pieces: PathBuf,
    pub(crate) downloads: Option<PathBuf>,
    pub(crate) folders: Vec<PathBuf>,
}

/// What `Places::classify` found: the origin and, for a linked folder, which one.
pub(crate) struct Found<'a> {
    pub(crate) origin: Origin,
    pub(crate) root: Option<&'a Path>,
}

impl Places {
    pub(crate) fn classify(&self, path: &Path) -> Found<'_> {
        let plain = |origin| Found { origin, root: None };
        if self.bundled.iter().any(|b| path.starts_with(b)) || path.starts_with(&self.exercises) {
            return plain(Origin::Exercise);
        }
        if path.starts_with(&self.pieces) {
            return plain(Origin::Generated);
        }

        // Nested links (`~/tabs` and `~/tabs/bass`): the inner one names the folder.
        let root = self
            .folders
            .iter()
            .filter(|f| path.starts_with(f))
            .max_by_key(|f| f.components().count());
        if let Some(root) = root {
            return Found {
                origin: Origin::Folder,
                root: Some(root),
            };
        }
        if self.downloads.as_ref().is_some_and(|d| path.starts_with(d)) {
            return plain(Origin::Download);
        }
        plain(Origin::Imported)
    }
}

impl Core {
    pub(crate) fn places(&self) -> Places {
        Places {
            bundled: vec![Self::exercises_dir(), Self::demo_path()],
            exercises: self.data_dir.join(EXERCISES_DIR),
            pieces: self.data_dir.join(PIECES_DIR),
            downloads: self.downloads.clone(),
            folders: self.library.linked_folders.clone(),
        }
    }
}
