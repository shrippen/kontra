//! Double stops the tab announces (R8): which of the two tones sound, told as a
//! `DoubleStopVerdict` once 160 ms of the attack are there.
//!
//! ```text
//! note emitted near an expected strike with two tones ─► octave? ─► Unknown at once
//!   └─► wait for attack + 20 ms + 140 ms (a new attack cuts the frame short) ─► DyadCheck
//! ```
//!
//! The note itself goes out as before (one pitch); the verdict follows like a technique.

use super::{NoteTracker, Pending, Waiting};
use crate::dyad::{DyadCheck, Heard, FRAME_S, SKIP_S};
use kontra_model::{midi_to_hz, DoubleStopHeard, DoubleStopVerdict};

/// Double stops waiting for their audio: attacks closer than 160 ms overlap.
const WAITING: usize = 4;
/// A frame cut short by the next attack tells nothing below this (two periods of B0 and more).
const MIN_FRAME_S: f64 = 0.07;

/// A double stop waiting for its audio.
#[derive(Debug, Clone, Copy)]
struct Due {
    /// The emitted note's onset (sample), which the verdict names.
    report: u64,
    /// Where the analysed audio starts (the attack plus `SKIP_S`) and ends.
    from: u64,
    to: u64,
    lower: u8,
    upper: u8,
}

/// The checker and the double stops it still has to judge.
#[derive(Debug)]
pub(super) struct DoubleStops {
    check: DyadCheck,
    waiting: [Option<Due>; WAITING],
    told: Vec<DoubleStopVerdict>,
}

impl DoubleStops {
    pub(super) fn new(sample_rate: f64) -> DoubleStops {
        DoubleStops {
            check: DyadCheck::new(sample_rate),
            waiting: [None; WAITING],
            told: Vec::with_capacity(2 * WAITING),
        }
    }

    pub(super) fn told(&self) -> &[DoubleStopVerdict] {
        &self.told
    }

    pub(super) fn clear_told(&mut self) {
        self.told.clear();
    }
}

impl NoteTracker {
    /// Waits for the audio of `p` when the tab expects two tones there (call on emit).
    pub(super) fn announce_double(&mut self, p: &Pending) {
        let Some(strike) = self.expected.near(p.report) else {
            return;
        };
        let [a, b] = strike.tones() else {
            return;
        };
        let (lower, upper) = (a.midi.min(b.midi), a.midi.max(b.midi));
        if !DyadCheck::can_tell(lower, upper) {
            self.tell_double(p.report, DoubleStopHeard::Unknown);
            return;
        }
        let from = p.onset + (SKIP_S * self.sample_rate) as u64;
        let due = Due {
            report: p.report,
            from,
            to: from + (FRAME_S * self.sample_rate) as u64,
            lower,
            upper,
        };
        let doubles = &mut self.double_stops;
        let slot = doubles.waiting.iter().position(Option::is_none);
        let earliest = || {
            let first = doubles.waiting.iter().flatten().map(|d| d.from).min();
            doubles
                .waiting
                .iter()
                .position(|d| d.map(|d| d.from) == first)
                .unwrap_or(0)
        };
        let at = slot.unwrap_or_else(earliest);
        doubles.waiting[at] = Some(due);
    }

    /// A new attack at `onset` ends the frames of the double stops before it.
    pub(super) fn cut_doubles(&mut self, onset: u64) {
        for due in self.double_stops.waiting.iter_mut().flatten() {
            due.to = due.to.min(onset);
        }
    }

    /// Forgets the double stop of the note told at `report`: it was taken back (R1).
    pub(super) fn forget_double(&mut self, report: u64) {
        let doubles = &mut self.double_stops;
        for slot in &mut doubles.waiting {
            if slot.is_some_and(|d| d.report == report) {
                *slot = None;
            }
        }
        let onset_s = report as f64 / self.sample_rate;
        doubles.told.retain(|v| v.onset_s != onset_s);
    }

    /// Judges the double stops whose audio is there (`ForAudio`), or all on what exists.
    pub(super) fn judge_doubles(&mut self, waiting: Waiting) {
        for i in 0..WAITING {
            let Some(mut due) = self.double_stops.waiting[i] else {
                continue;
            };
            if waiting == Waiting::ForAudio && self.pushed < due.to {
                continue;
            }
            self.double_stops.waiting[i] = None;
            due.to = due.to.min(self.pushed);
            let heard = self.hear_double(&due);
            self.tell_double(due.report, heard);
        }
    }

    /// Which tones of `due` sound in its frame.
    fn hear_double(&mut self, due: &Due) -> DoubleStopHeard {
        let min = (MIN_FRAME_S * self.sample_rate) as u64;
        let oldest = self.pushed.saturating_sub(self.ring.len() as u64);
        if due.to < due.from + min || due.from < oldest {
            return DoubleStopHeard::Unknown;
        }
        let len = self.ring.len() as u64;
        let have = due.to - due.from;
        for (i, x) in self.double_stops.check.frame_mut().iter_mut().enumerate() {
            let at = due.from + i as u64;
            *x = if (i as u64) < have {
                self.ring[(at % len) as usize]
            } else {
                0.0
            };
        }
        let hz = |midi: u8| midi_to_hz(f64::from(midi), self.config.a4_hz);
        let (lower_hz, upper_hz) = (hz(due.lower), hz(due.upper));
        match self.double_stops.check.check(lower_hz, upper_hz) {
            Heard::Both => DoubleStopHeard::Both,
            Heard::Lower => DoubleStopHeard::Lower,
            Heard::Upper => DoubleStopHeard::Upper,
            Heard::Unknown => DoubleStopHeard::Unknown,
        }
    }

    fn tell_double(&mut self, report: u64, heard: DoubleStopHeard) {
        self.double_stops.told.push(DoubleStopVerdict {
            onset_s: report as f64 / self.sample_rate,
            heard,
        });
    }
}
