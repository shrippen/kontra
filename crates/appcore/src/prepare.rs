//! Backing track prepared in the background (QA 27.10., J2): the first play after opening a
//! piece decoded and resampled its MP3 in the UI thread (a second, 20 s in a debug build).
//!
//! ```text
//! tick ─ no run, songs changed ─► BackingCache::prepare (thread) ─► UiEvent::BackingLoading(true)
//! tick ─ thread done ───────────────────────────────────────────► UiEvent::BackingLoading(false)
//! play ─ while preparing ─► a hint, no run; afterwards Live reads the prepared file
//! ```

use crate::{store, Core, UiEvent};
use kontra_session::{BackingCache, Preparing};
use std::path::PathBuf;

const CACHE_DIR: &str = "backing";

/// The songs last prepared (or being prepared) and the running job.
#[derive(Debug, Default)]
pub(crate) struct Prepare {
    songs: Vec<PathBuf>,
    job: Option<Preparing>,
}

impl Core {
    /// Folder of the prepared backing tracks, beside the separated songs.
    pub(crate) fn backing_cache_dir(&self) -> PathBuf {
        store::cache_dir(&self.data_dir).join(CACHE_DIR)
    }

    /// True while the loaded piece's backing track is prepared; play waits for it.
    pub fn backing_loading(&self) -> bool {
        self.prepare.job.is_some()
    }

    /// Starts preparing when the backing songs changed and nothing plays; tells the UI when
    /// preparing starts or ends.
    pub(crate) fn prepare_backing(&mut self) -> Option<UiEvent> {
        if let Some(job) = self.prepare.job.as_mut() {
            if job.running() {
                return None;
            }
            self.prepare.job = None;
            return Some(UiEvent::BackingLoading(false));
        }
        if self.live.is_some() {
            return None;
        }
        let songs = self.backing_songs();
        if songs == self.prepare.songs {
            return None;
        }
        self.prepare.songs = songs.clone();
        if songs.is_empty() {
            return None;
        }
        let cache = BackingCache::new(&self.backing_cache_dir());
        // The saved device as it is: listing devices is left to the thread.
        let device = self.settings.output_device.clone();
        self.prepare.job = Some(cache.prepare(songs, device));
        Some(UiEvent::BackingLoading(true))
    }

    /// Files the next run plays: the backing track and, without bass, the original.
    fn backing_songs(&self) -> Vec<PathBuf> {
        let played = self.backing().map(|(path, _)| path).filter(|p| p.exists());
        played.into_iter().chain(self.backing_original()).collect()
    }
}
