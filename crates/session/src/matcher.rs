//! Matches detected notes to expected notes of one loop pass.

#[cfg(test)]
use crate::expected::ChordTones;
use crate::expected::ExpectedNote;
use crate::settings::ScoreSettings;
use kontra_model::Technique;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Outcome {
    /// Right pitch inside the timing window.
    Hit,
    /// Something was played inside the window, but the wrong pitch.
    WrongPitch { played_midi: f64 },
    /// The right note, but further off in cents than the pitch limit allows ("unsauber",
    /// intonation on fretless or double bass). The deviation is in `NoteResult::cents`.
    OffPitch,
    /// Nothing was played for this note.
    Missed,
    /// Played where no note was expected.
    Extra { played_midi: f64 },
}

/// Result for one expected note (or one extra note) in one pass.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NoteResult {
    pub pass: u32,
    /// Index into `Arrangement::notes`; `None` for extras.
    pub note: Option<usize>,
    pub bar: Option<usize>,
    pub outcome: Outcome,
    /// Real milliseconds, negative = early.
    pub offset_ms: Option<f64>,
    /// Pitch deviation from the expected note.
    pub cents: Option<f64>,
    /// Arrangement position of the expected note (or of the extra note).
    pub position_s: f64,
}

impl NoteResult {
    pub fn is_hit(&self) -> bool {
        matches!(self.outcome, Outcome::Hit)
    }
}

/// Whether a scored note was free or taken from an earlier, worse detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Claim {
    /// No other detection had it (or none at all: an extra note).
    Free,
    /// An earlier detection with a worse pitch had it; its result is this one now and the
    /// earlier detection an extra note.
    TakenOver,
}

/// Rank of a resolution no later detection may take over (a missed note, Wait mode).
const FINAL_RANK: u8 = 0;

#[derive(Debug, Clone)]
pub(crate) struct Matcher {
    expected: Vec<ExpectedNote>,
    /// Resolved (pass, slot) with the pitch rank of the detection that resolved it.
    resolved: HashMap<(u32, usize), u8>,
}

impl Matcher {
    pub(crate) fn new(expected: Vec<ExpectedNote>) -> Matcher {
        Matcher {
            expected,
            resolved: HashMap::new(),
        }
    }

    pub(crate) fn expected(&self) -> &[ExpectedNote] {
        &self.expected
    }

    /// The same attacks with other pitches (transposed while playing); what was judged stays.
    pub(crate) fn retarget(&mut self, expected: Vec<ExpectedNote>) {
        self.expected = expected;
    }

    /// Whether an unresolved note of `pass` within `window` of `position_s` has the pitch
    /// `midi` (the right semitone).
    pub(crate) fn has_pitch(
        &self,
        pass: u32,
        position_s: f64,
        window: f64,
        range: (f64, f64),
        midi: f64,
    ) -> bool {
        self.expected.iter().enumerate().any(|(slot, e)| {
            (e.position_s - position_s).abs() <= window
                && e.position_s >= range.0
                && e.position_s < range.1
                && !self.resolved.contains_key(&(pass, slot))
                && pitch_rank(e.cents_from(midi), SEMITONE_CENTS) == 0
        })
    }

    pub(crate) fn is_resolved(&self, pass: u32, slot: usize) -> bool {
        self.resolved.contains_key(&(pass, slot))
    }

    pub(crate) fn resolve(&mut self, pass: u32, slot: usize) {
        self.resolved.insert((pass, slot), FINAL_RANK);
    }

    /// A resolved note judged again (a pitch told later): a later detection takes it over only
    /// with a better pitch than this.
    pub(crate) fn rejudge(&mut self, pass: u32, slot: usize, outcome: Outcome) {
        if let Some(rank) = self.resolved.get_mut(&(pass, slot)) {
            *rank = outcome_rank(outcome);
        }
    }

    /// Whether an unresolved expected note of `pass` lies within `window` of `position_s`.
    pub(crate) fn has_candidate(
        &self,
        pass: u32,
        position_s: f64,
        window: f64,
        range: (f64, f64),
    ) -> bool {
        self.expected.iter().enumerate().any(|(slot, e)| {
            (e.position_s - position_s).abs() <= window
                && e.position_s >= range.0
                && e.position_s < range.1
                && !self.resolved.contains_key(&(pass, slot))
        })
    }

    /// Scores a note played at `position_s` in `pass`. `speed` converts the real timing window
    /// into arrangement seconds; `range` limits candidates to the loop. A ghost note played as
    /// one hits a ghost note of the score whatever its pitch: a muted string has none.
    ///
    /// A note already taken by a detection with a worse pitch is taken over (H5): a real bass
    /// sounds the open string or a thump just before the attack, inside the next note's window.
    ///
    /// ```text
    /// window of C2   ├──────────┼──────────┤
    ///                   E1 (−30 dB)  C2 (attack)
    ///                   wrong ──────► hit; E1 becomes an extra note
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn score(
        &mut self,
        pass: u32,
        position_s: f64,
        midi: f64,
        technique: Option<Technique>,
        speed: f64,
        settings: &ScoreSettings,
        range: (f64, f64),
    ) -> (NoteResult, Claim) {
        let window = settings.strictness.window_ms() / 1000.0 * speed;
        let muted = |e: &ExpectedNote| e.ghost && technique == Some(Technique::Muted);
        let rank_of = |e: &ExpectedNote| {
            if muted(e) {
                0
            } else {
                pitch_rank(e.cents_from(midi), settings.pitch_cents)
            }
        };

        // Rank candidates: in tune before off pitch before wrong, then by distance; a taken
        // note only when this pitch is better than the one that took it.
        let mut best: Option<(usize, u8, f64, Claim)> = None;
        for (slot, e) in self.expected.iter().enumerate() {
            let distance = (e.position_s - position_s).abs();
            if distance > window || e.position_s < range.0 || e.position_s >= range.1 {
                continue;
            }
            let rank = rank_of(e);
            let claim = match self.resolved.get(&(pass, slot)) {
                None => Claim::Free,
                Some(&taken) if rank < taken => Claim::TakenOver,
                Some(_) => continue,
            };
            let better = match best {
                None => true,
                Some((_, best_rank, best_distance, _)) => {
                    rank < best_rank || (rank == best_rank && distance < best_distance)
                }
            };
            if better {
                best = Some((slot, rank, distance, claim));
            }
        }
        let Some((slot, rank, _, claim)) = best else {
            let extra = NoteResult {
                pass,
                note: None,
                bar: None,
                outcome: Outcome::Extra { played_midi: midi },
                offset_ms: None,
                cents: None,
                position_s,
            };
            return (extra, Claim::Free);
        };
        self.resolved.insert((pass, slot), rank);
        let e = self.expected[slot];
        let (outcome, cents) = if muted(&e) {
            (Outcome::Hit, None)
        } else {
            let cents = e.cents_from(midi);
            (
                pitch_outcome(cents, settings.pitch_cents, midi),
                Some(cents),
            )
        };
        let result = NoteResult {
            pass,
            note: Some(e.index),
            bar: e.bar,
            outcome,
            offset_ms: Some((position_s - e.position_s) / speed * 1000.0),
            cents,
            position_s: e.position_s,
        };
        (result, claim)
    }
}

/// Hit within `limit_cents`; the right semitone but beyond the limit is off pitch; anything
/// nearer to another semitone is a wrong note.
pub(crate) fn pitch_outcome(cents: f64, limit_cents: f64, played_midi: f64) -> Outcome {
    match pitch_rank(cents, limit_cents) {
        0 => Outcome::Hit,
        1 => Outcome::OffPitch,
        _ => Outcome::WrongPitch { played_midi },
    }
}

fn outcome_rank(outcome: Outcome) -> u8 {
    match outcome {
        Outcome::Hit => 0,
        Outcome::OffPitch => 1,
        _ => 2,
    }
}

/// Half a semitone: nearer than this, a pitch is the note's semitone.
const SEMITONE_CENTS: f64 = 50.0;

fn pitch_rank(cents: f64, limit_cents: f64) -> u8 {
    let off = cents.abs();
    if off <= limit_cents {
        0
    } else if off < 50.0 {
        1
    } else {
        2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matcher() -> Matcher {
        Matcher::new(vec![
            ExpectedNote {
                index: 0,
                position_s: 1.0,
                midi: 28,
                bar: Some(0),
                chord: ChordTones::default(),
                ghost: false,
                harmonic: false,
            },
            ExpectedNote {
                index: 1,
                position_s: 1.05,
                midi: 33,
                bar: Some(0),
                chord: ChordTones::default(),
                ghost: false,
                harmonic: false,
            },
            ExpectedNote {
                index: 2,
                position_s: 2.0,
                midi: 28,
                bar: Some(1),
                chord: ChordTones::default(),
                ghost: false,
                harmonic: false,
            },
        ])
    }

    const ALL: (f64, f64) = (0.0, f64::INFINITY);

    #[test]
    fn prefers_in_tune_candidate() {
        let mut m = matcher();
        let r = m
            .score(0, 1.02, 33.1, None, 1.0, &ScoreSettings::default(), ALL)
            .0;
        assert_eq!(r.note, Some(1));
        assert!(r.is_hit());
        assert!((r.offset_ms.unwrap() + 30.0).abs() < 1e-6);
    }

    #[test]
    fn right_note_beyond_the_limit_is_off_pitch() {
        let mut m = matcher();
        let tight = ScoreSettings {
            pitch_cents: 30.0,
            ..ScoreSettings::default()
        };
        let r = m.score(0, 2.0, 28.4, None, 1.0, &tight, ALL).0;
        assert_eq!(r.outcome, Outcome::OffPitch);
        assert!((r.cents.unwrap() - 40.0).abs() < 1e-6);
        let r = m.score(0, 1.0, 28.2, None, 1.0, &tight, ALL).0;
        assert!(r.is_hit());
        let r = m.score(0, 1.05, 33.6, None, 1.0, &tight, ALL).0;
        assert!(matches!(r.outcome, Outcome::WrongPitch { .. }));
    }

    #[test]
    fn wrong_pitch_and_extra() {
        let mut m = matcher();
        let r = m
            .score(0, 2.01, 30.0, None, 1.0, &ScoreSettings::default(), ALL)
            .0;
        assert!(matches!(r.outcome, Outcome::WrongPitch { .. }));
        let r = m
            .score(0, 3.0, 30.0, None, 1.0, &ScoreSettings::default(), ALL)
            .0;
        assert!(matches!(r.outcome, Outcome::Extra { .. }));
    }

    #[test]
    fn ghost_note_is_not_judged_by_pitch() {
        let mut m = Matcher::new(vec![ExpectedNote {
            index: 0,
            position_s: 1.0,
            midi: 28,
            bar: Some(0),
            chord: ChordTones::default(),
            ghost: true,
            harmonic: false,
        }]);
        let settings = ScoreSettings::default();
        let r = m
            .score(0, 1.0, 0.0, Some(Technique::Muted), 1.0, &settings, ALL)
            .0;
        assert!(r.is_hit());
        assert_eq!(r.cents, None);
    }

    #[test]
    fn window_scales_with_speed() {
        let mut m = matcher();
        // 50 ms real at half speed is 25 ms of arrangement time.
        let r = m
            .score(0, 2.025, 28.0, None, 0.5, &ScoreSettings::default(), ALL)
            .0;
        assert!(r.is_hit());
        assert!((r.offset_ms.unwrap() - 50.0).abs() < 1e-6);
    }
}
