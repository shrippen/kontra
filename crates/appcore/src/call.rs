//! Call and response in the app (LH-LRN-19): a switch on the practice page turns a piece with
//! chords into calls and answers (`kontra_session::call_and_response`). The tab stays hidden,
//! the player answers by ear; after `REVEAL_AFTER` failed tries it shows.
//!
//! ```text
//! try 1  ✗ 60 %   tab hidden ("listen and answer")
//! try 2  ✗ 70 %   tab shows
//! ```

use crate::Core;
use kontra_model::Arrangement;
use kontra_session::{call_and_response, Density};
use std::path::PathBuf;

/// Failed tries before the tab shows.
const REVEAL_AFTER: u32 = 2;
/// A try with fewer hits among the answer's notes failed.
const TRY_SITS: f64 = 0.8;

/// Whether call and response is on, and the failed tries on the piece it was counted for.
#[derive(Debug, Default)]
pub(crate) struct CallState {
    on: bool,
    failed: u32,
    path: Option<PathBuf>,
}

/// The notes to play: calls and answers when on and possible, else the density's share.
pub(crate) fn shape(full: &Arrangement, density: Density, call: bool) -> Arrangement {
    call.then(|| call_and_response(full))
        .flatten()
        .unwrap_or_else(|| density.thin(full))
}

impl Core {
    /// Call and response on or off (LH-LRN-19); true if the notes changed (the run stops).
    pub fn set_call(&mut self, on: bool) -> bool {
        self.call = CallState {
            on,
            failed: 0,
            path: self.loaded_path().map(PathBuf::from),
        };
        self.rethin()
    }

    /// Whether call and response is switched on (the piece may have no chords for it).
    pub(crate) fn call_wanted(&self) -> bool {
        self.call.on
    }

    /// Whether the loaded piece plays as call and response.
    pub fn call_on(&self) -> bool {
        self.call.on && self.arrangement().is_some_and(|a| !a.calls.is_empty())
    }

    /// The tab is hidden: call and response on and fewer than `REVEAL_AFTER` failed tries.
    pub fn call_hidden(&self) -> bool {
        self.call_on() && self.call_failures() < REVEAL_AFTER
    }

    /// Counts a finished try (a pass or a run) with `hit_rate`: below `TRY_SITS` it failed.
    pub(crate) fn call_try(&mut self, hit_rate: f64) {
        if !self.call_on() || hit_rate >= TRY_SITS {
            return;
        }
        let failures = self.call_failures();
        self.call.path = self.loaded_path().map(PathBuf::from);
        self.call.failed = failures + 1;
    }

    /// Failed tries on the loaded piece; another piece starts at none.
    fn call_failures(&self) -> u32 {
        if self.call.path.as_deref() == self.loaded_path() {
            self.call.failed
        } else {
            0
        }
    }
}
