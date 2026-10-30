//! Double stops judged by both tones (R8, LH-BEW-11).
//!
//! The tracker hears one pitch per attack; a double stop's note is first judged like a chord
//! (any tone counts for all). About 160 ms later the input tells which tones sounded, and both
//! tones are judged again:
//!
//! ```text
//! verdict   lower tone              upper tone
//! Both      hit                     hit            (a wrong pitch from the mix is not a mistake)
//! Lower     hit                     missed         ("nur ein Ton")
//! Upper     missed                  hit
//! Unknown   as judged (an octave, or neither tone clear)
//! ```
//!
//! A tone the tracker heard keeps its cents (in tune or off pitch); the other is a hit without
//! cents. A verdict naming one tone while the tracker heard the other changes nothing.

use super::{Judgement, Session, SessionEvent, REVISION_MATCH_S};
use crate::matcher::{pitch_outcome, NoteResult, Outcome};
use kontra_model::{ChordHearing, DoubleStopHeard, DoubleStopVerdict};

/// Nearer than this (semitones), the tracker heard that tone.
const HALF_SEMITONE: f64 = 0.5;
const CENTS_PER_SEMITONE: f64 = 100.0;
const SEMITONES_PER_OCTAVE: u8 = 12;

/// The tones of a double stop, lower first: (note index, MIDI).
type Tones = [(usize, u8); 2];

impl Session {
    /// Which tones of an announced double stop sounded (R8): its two tones are judged again.
    pub fn on_double_stop(&mut self, verdict: &DoubleStopVerdict) {
        if self.settings.source.chord_hearing() != ChordHearing::AnnouncedDoubleStops {
            return;
        }
        let Some(entry) = self
            .scored
            .iter()
            .rposition(|(onset, ..)| (onset - verdict.onset_s).abs() < REVISION_MATCH_S)
        else {
            return;
        };
        let (_, at, _, midi, judgement) = self.scored[entry];
        if judgement == Judgement::BothTones {
            return;
        }
        let first = self.results[at];
        let Some(slot) = first.note.and_then(|n| self.slot_of(n, first.position_s)) else {
            return;
        };
        let e = self.matcher.expected()[slot];
        if e.ghost || e.harmonic || e.any_pitch {
            return;
        }
        let mut others = e.chord.iter();
        let (Some(upper), None) = (others.next(), others.next()) else {
            return;
        };
        let tones: Tones = [(e.index, e.midi), upper];
        let Some(outcomes) = self.judge_tones(&tones, midi, verdict.heard) else {
            return;
        };

        // The earlier results are replaced, not followed: a pass counts every result it holds.
        for ((index, _), (outcome, cents)) in tones.iter().zip(outcomes) {
            let Some(r) = self.results.iter().rposition(|r| {
                r.pass == first.pass && r.position_s == first.position_s && r.note == Some(*index)
            }) else {
                continue;
            };
            let missed = outcome == Outcome::Missed;
            let judged = NoteResult {
                outcome,
                cents,
                offset_ms: if missed {
                    None
                } else {
                    self.results[r].offset_ms
                },
                ..self.results[r]
            };
            self.results[r] = judged;
            self.events.push(SessionEvent::Result(judged));
        }
        let best = outcomes
            .iter()
            .map(|(o, _)| *o)
            .find(|o| *o != Outcome::Missed)
            .unwrap_or(Outcome::Missed);
        self.matcher.rejudge(first.pass, slot, best);

        // Final: a pitch told later does not judge the tones again; a retraction still finds it.
        self.scored[entry].4 = Judgement::BothTones;
    }

    /// Two-tone double stops in the score this input does not judge by both tones: all but
    /// octaves (no input tells them) at a microphone (Runde 17), none elsewhere.
    pub(super) fn untold_double_stops(&self) -> u32 {
        if self.settings.source.chord_hearing() != ChordHearing::OneTone {
            return 0;
        }
        let count = self
            .matcher
            .expected()
            .iter()
            .filter(|e| !(e.ghost || e.harmonic || e.any_pitch))
            .filter(|e| {
                let mut others = e.chord.iter();
                match (others.next(), others.next()) {
                    (Some((_, upper)), None) => upper.abs_diff(e.midi) % SEMITONES_PER_OCTAVE != 0,
                    _ => false,
                }
            })
            .count();
        u32::try_from(count).unwrap_or(u32::MAX)
    }

    /// Outcome and cents of each tone for `heard`, with `midi` what the tracker heard; `None`:
    /// leave the judgement as it is.
    fn judge_tones(
        &self,
        tones: &Tones,
        midi: f64,
        heard: DoubleStopHeard,
    ) -> Option<[(Outcome, Option<f64>); 2]> {
        let limit = self.settings.pitch_cents;
        let tracked = |tone: u8| (midi - f64::from(tone)).abs() < HALF_SEMITONE;

        // A sounding tone: as the tracker heard it, else a hit without cents.
        let sounding = |tone: u8| {
            if !tracked(tone) {
                return (Outcome::Hit, None);
            }
            let cents = (midi - f64::from(tone)) * CENTS_PER_SEMITONE;
            (pitch_outcome(cents, limit, midi), Some(cents))
        };
        let missing = (Outcome::Missed, None);
        let (lower, upper) = (tones[0].1, tones[1].1);
        match heard {
            DoubleStopHeard::Unknown => None,
            DoubleStopHeard::Both => Some([sounding(lower), sounding(upper)]),
            DoubleStopHeard::Lower if !tracked(upper) => Some([sounding(lower), missing]),
            DoubleStopHeard::Upper if !tracked(lower) => Some([missing, sounding(upper)]),
            DoubleStopHeard::Lower | DoubleStopHeard::Upper => None,
        }
    }
}
