//! A struck note's settled pitch (decision round 11): estimates 90–160 ms after the onset,
//! one a semitone or an octave above the emitted pitch becomes a `PitchRevision`.

use kontra_model::InputSource;

/// Follow estimates after the onset that tell a note's settled pitch. FiloBass and IDMT
/// (testdata/korpus/QUELLEN.md): 90–160 ms rounds most flat starts right; later windows reach
/// the next note.
pub(super) const SETTLE_FROM_S: f64 = 0.09;
pub(super) const SETTLE_TO_S: f64 = 0.16;
/// Estimates the window holds at most (one every `FOLLOW_STEP_S`).
const SETTLE_SLOTS: usize = 8;
/// Fewer estimates tell nothing (a short or fading note).
const SETTLE_MIN: usize = 3;
/// The latest estimates that give the settled pitch (odd: their median is one of them).
const SETTLE_TAIL: usize = 3;
/// The settled pitch must lie this far above the emitted one …
const SETTLE_MIN_CENTS: f64 = 50.0;
/// … and at least half the estimates this close to their median.
const SETTLE_SPREAD_CENTS: f64 = 50.0;
/// Corrected steps: a flat start a semitone up, a sub-octave attack an octave up. Downward
/// steps fixed 20 notes but broke 4 right ones (a pull-off, a pop; QUELLEN.md).
const SEMITONE: f64 = 1.0;
const OCTAVE: f64 = 12.0;

/// Which steps up a settled pitch may correct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Steps {
    SemitoneOrOctave,
    /// At a microphone the low cut weakens the fundamental, and YIN on the ringing note jumps an
    /// octave up (synthetic `room-double`: 6 of 156 C2 told as C3).
    Semitone,
}

impl Steps {
    pub(super) fn for_source(source: InputSource) -> Steps {
        match source {
            InputSource::Microphone => Steps::Semitone,
            _ => Steps::SemitoneOrOctave,
        }
    }

    fn allow(self, step: f64) -> bool {
        step == SEMITONE || (step == OCTAVE && self == Steps::SemitoneOrOctave)
    }
}

/// Follow estimates of a struck note in its settle window.
#[derive(Debug, Clone, Copy)]
pub(super) struct Settle {
    /// The note's reported onset (sample) and emitted pitch.
    pub(super) onset: u64,
    pub(super) midi: f64,
    /// A dead note: its pitch wanders and does not count.
    pub(super) muted: bool,
    steps: Steps,
    /// (end of window, MIDI) of each estimate.
    heard: [(u64, f64); SETTLE_SLOTS],
    count: usize,
}

impl Settle {
    pub(super) fn new(onset: u64, midi: f64, muted: bool, steps: Steps) -> Settle {
        Settle {
            onset,
            midi,
            muted,
            steps,
            heard: [(0, 0.0); SETTLE_SLOTS],
            count: 0,
        }
    }

    pub(super) fn hear(&mut self, until: u64, midi: f64) {
        if self.count < SETTLE_SLOTS {
            self.heard[self.count] = (until, midi);
            self.count += 1;
        }
    }

    /// The settled pitch, from estimates ending before `before` (a new attack), if it corrects
    /// the emitted one: a semitone or an octave higher, by at least `SETTLE_MIN_CENTS`.
    pub(super) fn settled(&self, before: u64) -> Option<f64> {
        if self.muted {
            return None;
        }
        let mut sorted = [0.0f64; SETTLE_SLOTS];
        let mut count = 0;
        for &(until, midi) in &self.heard[..self.count] {
            if until <= before {
                sorted[count] = midi;
                count += 1;
            }
        }
        if count < SETTLE_MIN {
            return None;
        }
        // The pitch it settled on: the latest estimates, whose windows lie after the rise. All
        // of them read a tone rising for 120 ms 39 cents flat, a miss on fretless (QA 27.10.).
        let mut latest = [0.0f64; SETTLE_TAIL];
        latest.copy_from_slice(&sorted[count - SETTLE_TAIL..count]);
        latest.sort_by(f64::total_cmp);
        let settled = latest[SETTLE_TAIL / 2];
        let sorted = &mut sorted[..count];
        sorted.sort_by(f64::total_cmp);
        let median = match count % 2 {
            0 => (sorted[count / 2 - 1] + sorted[count / 2]) / 2.0,
            _ => sorted[count / 2],
        };

        // Stable: at least half the estimates near the median.
        let near = sorted
            .iter()
            .filter(|m| (*m - median).abs() * 100.0 < SETTLE_SPREAD_CENTS)
            .count();
        if near * 2 < count {
            return None;
        }

        // Higher, on the next semitone or octave: e.g. 37.3 emitted, 38.0 settled.
        let step = median.round() - self.midi.round();
        if !self.steps.allow(step) {
            return None;
        }
        // The semitone from all estimates (FiloBass: the latest alone reach the next note more
        // often), the cents from the latest when they agree on it.
        let pitch = if settled.round() == median.round() {
            settled
        } else {
            median
        };
        ((median - self.midi) * 100.0 >= SETTLE_MIN_CENTS).then_some(pitch)
    }
}
