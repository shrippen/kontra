//! Early report on a growing window (R1): every attack is tried from 8 ms after the attack
//! skip on, on all audio since then. The lags searched grow with it, just past one period of
//! the pitch found, the rest is the window: a period shows once about two of it are there, E2
//! after 25 ms of audio instead of the 55 a full window for D1 needs.
//!
//! ```text
//! attack ─ skip ─ every 5 ms: audio since the skip ├─ window ─┼─ lags (1.06×) ─┤
//!                 a dip below the threshold inside the lags ─► twice in a row? ─► emit
//!          … full window there ─► the usual search has the last word: another pitch (up or
//!                                 down, an octave too), or the same a few cents off ─► revise
//! ```
//!
//! The short window cannot see the octave below (prototype pitch-fast, 29.10.2026: first
//! reports 1–2.5 points more often wrong, final pitches as before); lags over twice the
//! period would see it but gave back most of the gain (`line-5` p95 97 → 89 instead of 80 ms).
//!
//! A note told early can prove to be no note of its own: a much louder attack (+12 dB) comes
//! before the full window would have told it, so it was a precursor, e.g. the fretting finger
//! tapping the string. It is taken back (`NoteRetraction`), as a precursor still waiting for
//! its pitch is dropped (`close_pending`).

use super::{expect::Confirm, NoteTracker, Pending, PRECURSOR_FACTOR, RISE_AFTER_S};
use crate::intonation::refine_hz;
use crate::yin::PitchEstimate;
use kontra_model::NoteRetraction;

/// Lags searched, in periods of the pitch found: just past one. The window holds one more.
const LAG_PERIODS: f64 = 1.06;

impl NoteTracker {
    /// The pitch on all audio from the attack skip up to `p.next_try`, if its period is
    /// unambiguous there (see the module comment).
    pub(super) fn hear_free(&mut self, p: &Pending) -> Option<PitchEstimate> {
        let skip = (self.config.attack_skip_s * self.sample_rate) as u64;
        let avail = p.next_try.saturating_sub(p.onset + skip) as usize;
        let len = avail.min(self.scratch.len()).min(self.yin.needed());
        let lag = (len as f64 * LAG_PERIODS / (1.0 + LAG_PERIODS)) as usize;
        self.copy_out(p.next_try - len as u64, len);
        let samples = &self.scratch[..len];
        let heard = self.yin.estimate_short(samples, lag)?;
        let unambiguous = heard.clear
            && heard.period * LAG_PERIODS <= lag as f64
            && heard.estimate.confidence >= self.config.min_confidence;
        if !unambiguous {
            return None;
        }
        let mut estimate = heard.estimate;
        if let Some(hz) = refine_hz(samples, self.sample_rate, estimate.hz) {
            estimate.hz = hz;
        }
        Some(estimate)
    }

    /// Takes the note told early back when the attack at `onset` shows it was a precursor: much
    /// louder, and before the full window would have told the note (or still without a pitch
    /// for it). Returns whether it did.
    pub(super) fn retracts_early(&mut self, onset: u64) -> bool {
        let Some(c) = self.confirm else {
            return false;
        };
        // The search sees the new attack only once it runs (`RISE_AFTER_S` later); what counts
        // is whether its first full window ends before the attack. Without a pitch it still
        // ends at the timeout (150 ms, `search`): a later attack finds no `confirm` here.
        if onset >= self.full_try(c.search.onset) && c.search.best.is_some() {
            return false;
        }
        let before = self.peak(c.report, onset);
        let after = self.peak(onset, onset + (RISE_AFTER_S * self.sample_rate) as u64);
        if after < before * PRECURSOR_FACTOR {
            return false;
        }
        self.confirm = None;
        self.retract(&c);
        true
    }

    /// Tells that the early note of `c` is no note; nothing more comes for it.
    fn retract(&mut self, c: &Confirm) {
        self.retracted.push(NoteRetraction {
            onset_s: c.report as f64 / self.sample_rate,
        });
        let ringing = self
            .sounding
            .and_then(|s| s.settle)
            .is_some_and(|t| t.onset == c.report);
        if ringing {
            self.sounding = None;
        }
        for slot in &mut self.strikes {
            if *slot == Some(c.report) {
                *slot = None;
            }
        }
    }
}
