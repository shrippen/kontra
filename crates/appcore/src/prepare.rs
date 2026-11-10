//! Backing track prepared in the background (QA 27.10., J2): the first play after opening a
//! piece decoded and resampled its MP3 in the UI thread (a second, 20 s in a debug build).
//!
//! ```text
//! tick ─ no run, songs changed ─► BackingCache::prepare (thread) ─► UiEvent::BackingLoading(true)
//! tick ─ thread done ───────────────────────────────────────────► UiEvent::BackingLoading(false)
//! play ─ while preparing ─► a hint, no run; afterwards Live reads the prepared file
//! ```

use crate::{store, Core, InputKind, UiEvent};
use kontra_session::{BackingCache, Preparing};
use std::path::PathBuf;

const CACHE_DIR: &str = "backing";

/// The songs and devices last prepared for (or being prepared) and the running job.
#[derive(Debug, Default)]
pub(crate) struct Prepare {
    songs: Vec<PathBuf>,
    device: Option<String>,
    /// The saved input as the player set it: under Windows the output runs at the input's
    /// rate, so another input may need the songs at another rate (review #247).
    input: Option<InputKey>,
    job: Option<Preparing>,
}

/// What names the input without asking the system: saved device, connection, stand-in.
type InputKey = (Option<String>, InputKind, Option<String>);

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
        // The saved devices as they are: comparing them asks the system nothing. Another
        // output or input may play at another rate, so a change prepares again (review #162).
        let device = self.settings.output_device.clone();
        let input = Some((
            self.settings.input_device.clone(),
            self.settings.input_kind,
            self.fallback_input.clone(),
        ));
        let same_devices = device == self.prepare.device && input == self.prepare.input;
        if songs == self.prepare.songs && same_devices {
            return None;
        }
        self.prepare.songs = songs.clone();
        self.prepare.device = device;
        self.prepare.input = input;
        if songs.is_empty() {
            return None;
        }
        let cache = BackingCache::new(&self.backing_cache_dir());
        // The devices as the run will open them (this lists devices, once per change): a
        // saved output that is gone plays on the default one.
        let (output, input) = (self.output_device(), self.input_choice());
        self.prepare.job = Some(cache.prepare(songs, output, input));
        Some(UiEvent::BackingLoading(true))
    }

    /// Forgets what was prepared (the cache was emptied): the next tick prepares again.
    pub(crate) fn prepare_again(&mut self) {
        self.prepare.songs.clear();
        self.prepare.device = None;
        self.prepare.input = None;
    }

    /// Files the next run plays: the backing track and, without bass, the original.
    fn backing_songs(&self) -> Vec<PathBuf> {
        let played = self.backing().map(|(path, _)| path).filter(|p| p.exists());
        played.into_iter().chain(self.backing_original()).collect()
    }
}
