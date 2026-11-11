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
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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
    /// Starts settled again so far; the worker follows when this moves.
    pub(super) settled: AtomicU64,
    /// The device's sample count where the new start of the last rebase holds from.
    pub(super) from_sample: AtomicU64,
    /// The start before it (`f64` bits): the samples before `from_sample` keep it.
    pub(super) old_start: AtomicU64,
}

/// A rebase the worker has seen but not reached in the data yet.
#[derive(Debug, Clone, Copy)]
struct Pending {
    from_sample: u64,
    old_start_s: f64,
}

/// The worker's side of a settled-again start. The callback settles it when the next data
/// arrives, but the worker still reads what came before (a backlog, a stall that just ended):
/// that keeps the old start. Past `from_sample` the analysis is lined up with the device's own
/// count, which already holds every stretch the device skipped (review #286: setting the shift
/// at once counted a stall's 0.8 s twice).
///
/// ```text
/// data     ……old start……│ from_sample ……new start, shift = (device count - analysed)……
/// ```
#[derive(Debug, Default)]
pub(super) struct Follow {
    seen: u64,
    pending: Option<Pending>,
}

impl Follow {
    /// The start for the next block: the callback's, or the old one while a rebase waits.
    pub(super) fn start_s(&mut self, rebasing: &Rebasing, started_s: Option<f64>) -> Option<f64> {
        // Acquire: the callback stored where and from what before it counted the rebase.
        let settled = rebasing.settled.load(Ordering::Acquire);
        if settled != self.seen {
            self.seen = settled;
            let old_start_s = f64::from_bits(rebasing.old_start.load(Ordering::Relaxed));
            let from_sample = rebasing.from_sample.load(Ordering::Relaxed);
            // A second rebase seen before the first was reached: the data before both keeps
            // the start from before the first. Not covered: two rebases the worker sees as one
            // (both within one of its rounds), where the callback already replaced the first
            // one's start; the backlog before the first then runs on the second's start. Modes
            // attach seconds apart, so this needs two switches within a few ms.
            let old_start_s = self.pending.map_or(old_start_s, |p| p.old_start_s);
            self.pending = Some(Pending {
                from_sample,
                old_start_s,
            });
        }
        match self.pending {
            Some(pending) if !pending.old_start_s.is_nan() => Some(pending.old_start_s),
            _ => started_s,
        }
    }

    /// After a block, the device having delivered `position` samples so far: the stall shift
    /// once the new start holds.
    pub(super) fn shift_s(&mut self, position: u64, analysed: u64, rate: f64) -> Option<f64> {
        let pending = self.pending?;
        if position < pending.from_sample {
            return None;
        }
        self.pending = None;
        // The analysis counts the silence that bridged stalls too, the device does not.
        Some((position as f64 - analysed as f64) / rate)
    }
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
