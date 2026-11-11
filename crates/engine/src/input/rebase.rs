//! A mode attaching to an input that stays open (Runde 20): the analysis starts over for its
//! instrument, and a device settles its start again, since its clock drifted meanwhile. The
//! session waits for `Event::Rebased` with its epoch; what came before is the last mode's.
//!
//! ```text
//!  Input::rebase ─ new Analysis ─► swaps ring ─► worker: takes it, keeps the count ─► Rebased
//!               └─ settle flag ──► callback: Anchor::rebase ─► settled += 1 ─► worker: shift
//! ```

use super::Analysis;
use crate::Event;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::mpsc::Sender;

/// New analyses waiting for the worker; one per rebase, so a few are plenty.
const SWAPS: usize = 4;

/// A new analysis for the worker, with the epoch it announces.
pub(super) struct Swap {
    pub(super) analysis: Analysis,
    pub(super) epoch: u64,
}

/// What `Input::rebase` tells the device threads.
#[derive(Debug, Default)]
pub(super) struct Rebasing {
    /// For the callback: settle the start again.
    pub(super) settle: AtomicBool,
    /// Starts settled again so far; the worker resets its stall shift when this moves.
    pub(super) settled: AtomicU64,
}

impl std::fmt::Debug for Swap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Swap")
            .field("epoch", &self.epoch)
            .finish_non_exhaustive()
    }
}

pub(super) type Swaps = (rtrb::Producer<Box<Swap>>, rtrb::Consumer<Box<Swap>>);

pub(super) fn swap_ring() -> Swaps {
    rtrb::RingBuffer::new(SWAPS)
}

/// Takes the analyses `Input::rebase` sent: the count of samples and the start go on, so the
/// stamps continue; tells the session each epoch.
pub(super) fn take_swaps(
    swaps: &mut rtrb::Consumer<Box<Swap>>,
    analysis: &mut Analysis,
    events: &Sender<Event>,
) {
    while let Ok(swap) = swaps.pop() {
        let Swap {
            analysis: mut next,
            epoch,
        } = *swap;
        next.start_s = analysis.start_s;
        next.analysed = analysis.analysed;
        // Its tracker counts from here.
        next.origin_s = analysis.analysed as f64 / analysis.sample_rate;
        *analysis = next;
        let _ = events.send(Event::Rebased { epoch });
    }
}
