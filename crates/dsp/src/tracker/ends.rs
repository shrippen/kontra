//! Where a note ends (LH-BEW-10, E5): the emitted note's level is followed until the hand
//! mutes it, it rings out or the next note replaces it.
//!
//! ```text
//! emit ─ 60 ms ─┬─ every 2 ms: level now (peak over one period of the lowest note)
//!               │    ≥ 10 dB under the level 60 ms before? ─► Muted, ends where it is 10 dB down
//!               │    30 dB under the note's start?          ─► Decayed
//!               └─ next attack or legato note first          ─► Replaced at its onset
//! ```
//!
//! Measured (`kontra-cli korpus ends`): see `planung/FORSCHUNG-AUDIO.md`, „Notenende (E5)“.

use super::NoteTracker;
use kontra_model::{Ending, NoteEnd};

/// The attack and its first decay are no end: looked for from this long after the onset.
const FROM_S: f64 = 0.06;
const STEP_S: f64 = 0.002;
/// A mute: the level falls this much within `FALL_S` (IDMT lines: muted notes fall 10 dB in
/// 30–60 ms, held notes lose 1–3 dB in that time).
const FALL_DB: f32 = 10.0;
const FALL_S: f64 = 0.06;
/// The end of a muted note: where the level first lies this much under the level before the
/// fall, back by the level's span. IDMT offsets lie closest to it (median −8 ms; the fall's
/// 3 dB point −28 ms, 6 dB −18 ms).
const END_DB: f32 = 10.0;
/// Rung out: this far under the level at the note's start.
const DECAY_DB: f32 = 30.0;
/// Ends told in one `push` before it allocates (one per note: a block of 16 notes).
const TOLD: usize = 16;
/// Block peaks kept (2 ms each): the fall (30 blocks) and the span (up to 20 for a B0) fit.
const BLOCKS: usize = 64;

/// The note whose end is waited for.
#[derive(Debug, Clone, Copy)]
struct Follow {
    /// The note's reported onset (sample).
    report: u64,
    /// The level at its start (peak magnitude).
    start: f32,
    next_check: u64,
}

/// The end follower of a tracker; allocation-free after `new`.
#[derive(Debug)]
pub(super) struct Ends {
    follow: Option<Follow>,
    /// Samples per block of `STEP_S`.
    step: u64,
    /// Level span in blocks: one period of the lowest note, so the peak does not ripple.
    span: u64,
    /// The fall in blocks.
    fall: u64,
    /// Peak of each block, by block number + 1 (0: empty); each block is read once.
    peaks: [(u64, f32); BLOCKS],
    told: Vec<NoteEnd>,
}

impl Ends {
    pub(super) fn new(sample_rate: f64, min_hz: f64) -> Ends {
        let step = ((STEP_S * sample_rate) as u64).max(1);
        let fall = (FALL_S / STEP_S).round() as u64;
        let span = ((sample_rate / min_hz) / step as f64).ceil() as u64;
        Ends {
            follow: None,
            step,
            span: span.clamp(1, BLOCKS as u64 - fall - 1),
            fall,
            peaks: [(0, 0.0); BLOCKS],
            told: Vec::with_capacity(TOLD),
        }
    }

    pub(super) fn told(&self) -> &[NoteEnd] {
        &self.told
    }

    pub(super) fn clear_told(&mut self) {
        self.told.clear();
    }
}

/// dB as a factor on magnitudes.
fn factor(db: f32) -> f32 {
    10f32.powf(-db / 20.0)
}

impl NoteTracker {
    /// Follows the note emitted at `report` for its end; the note before ends here.
    pub(super) fn follow_end(&mut self, report: u64) {
        self.end_at(report);
        let e = &self.ends;
        let from = report + (FROM_S * self.sample_rate) as u64;
        let start = self.peak(report, report + e.span * e.step);
        // Checked on block boundaries, the first with a whole fall after `from`.
        let first = from.div_ceil(e.step) + e.fall;
        self.ends.follow = Some(Follow {
            report,
            start,
            next_check: first,
        });
    }

    /// A new attack or legato note at `onset` replaces the followed note.
    pub(super) fn end_at(&mut self, onset: u64) {
        let Some(f) = self.ends.follow.take() else {
            return;
        };
        if onset <= f.report {
            self.ends.follow = Some(f);
            return;
        }
        self.tell_end(f.report, onset, Ending::Replaced);
    }

    /// Drops the followed note at `report` (taken back, a dead note): it has no length.
    pub(super) fn forget_end(&mut self, report: u64) {
        if self.ends.follow.is_some_and(|f| f.report == report) {
            self.ends.follow = None;
        }
    }

    /// Checks the followed note's level up to the audio there (`next_check` counts blocks).
    pub(super) fn watch_end(&mut self) {
        let Some(mut f) = self.ends.follow else {
            return;
        };
        let (step, fall) = (self.ends.step, self.ends.fall);
        while f.next_check * step <= self.pushed {
            let t = f.next_check;
            f.next_check += 1;
            let before = self.level_at(t - fall);
            let now = self.level_at(t);
            if now < before * factor(FALL_DB) {
                let end = self.fall_start(t - fall, t, before * factor(END_DB));
                self.ends.follow = None;
                self.tell_end(f.report, end * step, Ending::Muted);
                return;
            }
            if now < f.start * factor(DECAY_DB) {
                self.ends.follow = None;
                let end = t.saturating_sub(self.ends.span);
                self.tell_end(f.report, end * step, Ending::Decayed);
                return;
            }
        }
        self.ends.follow = Some(f);
    }

    /// Peak over the span ending at block `t`.
    fn level_at(&mut self, t: u64) -> f32 {
        let from = t.saturating_sub(self.ends.span);
        (from..t).map(|b| self.block_peak(b)).fold(0.0, f32::max)
    }

    /// Peak of block `b`, read from the ring once.
    fn block_peak(&mut self, b: u64) -> f32 {
        let slot = (b % BLOCKS as u64) as usize;
        let (key, peak) = self.ends.peaks[slot];
        if key == b + 1 {
            return peak;
        }
        let step = self.ends.step;
        let peak = self.peak(b * step, (b + 1) * step);
        self.ends.peaks[slot] = (b + 1, peak);
        peak
    }

    /// The first block in `from..to` where the level lies under `under`, back by the span (the
    /// peak lags the signal by up to one span).
    fn fall_start(&mut self, from: u64, to: u64, under: f32) -> u64 {
        let mut t = from;
        while t < to && self.level_at(t) >= under {
            t += 1;
        }
        t.saturating_sub(self.ends.span)
    }

    fn tell_end(&mut self, report: u64, end: u64, ending: Ending) {
        let sr = self.sample_rate;
        self.ends.told.push(NoteEnd {
            onset_s: report as f64 / sr,
            end_s: end.max(report) as f64 / sr,
            ending,
        });
    }
}
