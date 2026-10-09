//! Matches detected notes to expected notes of one loop pass.

#[cfg(test)]
use crate::expected::ChordTones;
use crate::expected::ExpectedNote;
use crate::settings::ScoreSettings;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Outcome {
    /// Right pitch inside the timing window.
    Hit,
    /// Something was played inside the window, but the wrong pitch.
    WrongPitch { played_midi: f64 },
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

#[derive(Debug, Clone)]
pub(crate) struct Matcher {
    expected: Vec<ExpectedNote>,
    resolved: HashSet<(u32, usize)>,
}

impl Matcher {
    pub(crate) fn new(expected: Vec<ExpectedNote>) -> Matcher {
        Matcher {
            expected,
            resolved: HashSet::new(),
        }
    }

    pub(crate) fn expected(&self) -> &[ExpectedNote] {
        &self.expected
    }

    pub(crate) fn is_resolved(&self, pass: u32, slot: usize) -> bool {
        self.resolved.contains(&(pass, slot))
    }

    pub(crate) fn resolve(&mut self, pass: u32, slot: usize) {
        self.resolved.insert((pass, slot));
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
                && !self.resolved.contains(&(pass, slot))
        })
    }

    /// Scores a note played at `position_s` in `pass`. `speed` converts the real timing window
    /// into arrangement seconds; `range` limits candidates to the loop.
    pub(crate) fn score(
        &mut self,
        pass: u32,
        position_s: f64,
        midi: f64,
        speed: f64,
        settings: &ScoreSettings,
        range: (f64, f64),
    ) -> NoteResult {
        let window = settings.strictness.window_ms() / 1000.0 * speed;
        let candidates = self.expected.iter().enumerate().filter(|(slot, e)| {
            (e.position_s - position_s).abs() <= window
                && e.position_s >= range.0
                && e.position_s < range.1
                && !self.resolved.contains(&(pass, *slot))
        });
        let cents_of = |e: &ExpectedNote| e.cents_from(midi);
        let mut best: Option<(usize, bool, f64)> = None;
        for (slot, e) in candidates {
            let in_tune = cents_of(e).abs() <= settings.pitch_cents;
            let distance = (e.position_s - position_s).abs();
            let better = match best {
                None => true,
                Some((_, best_tune, best_distance)) => {
                    (in_tune && !best_tune) || (in_tune == best_tune && distance < best_distance)
                }
            };
            if better {
                best = Some((slot, in_tune, distance));
            }
        }
        let Some((slot, in_tune, _)) = best else {
            return NoteResult {
                pass,
                note: None,
                bar: None,
                outcome: Outcome::Extra { played_midi: midi },
                offset_ms: None,
                cents: None,
                position_s,
            };
        };
        self.resolved.insert((pass, slot));
        let e = self.expected[slot];
        let outcome = if in_tune {
            Outcome::Hit
        } else {
            Outcome::WrongPitch { played_midi: midi }
        };
        NoteResult {
            pass,
            note: Some(e.index),
            bar: e.bar,
            outcome,
            offset_ms: Some((position_s - e.position_s) / speed * 1000.0),
            cents: Some(cents_of(&e)),
            position_s: e.position_s,
        }
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
            },
            ExpectedNote {
                index: 1,
                position_s: 1.05,
                midi: 33,
                bar: Some(0),
                chord: ChordTones::default(),
            },
            ExpectedNote {
                index: 2,
                position_s: 2.0,
                midi: 28,
                bar: Some(1),
                chord: ChordTones::default(),
            },
        ])
    }

    const ALL: (f64, f64) = (0.0, f64::INFINITY);

    #[test]
    fn prefers_in_tune_candidate() {
        let mut m = matcher();
        let r = m.score(0, 1.02, 33.1, 1.0, &ScoreSettings::default(), ALL);
        assert_eq!(r.note, Some(1));
        assert!(r.is_hit());
        assert!((r.offset_ms.unwrap() + 30.0).abs() < 1e-6);
    }

    #[test]
    fn wrong_pitch_and_extra() {
        let mut m = matcher();
        let r = m.score(0, 2.01, 30.0, 1.0, &ScoreSettings::default(), ALL);
        assert!(matches!(r.outcome, Outcome::WrongPitch { .. }));
        let r = m.score(0, 3.0, 30.0, 1.0, &ScoreSettings::default(), ALL);
        assert!(matches!(r.outcome, Outcome::Extra { .. }));
    }

    #[test]
    fn window_scales_with_speed() {
        let mut m = matcher();
        // 50 ms real at half speed is 25 ms of arrangement time.
        let r = m.score(0, 2.025, 28.0, 0.5, &ScoreSettings::default(), ALL);
        assert!(r.is_hit());
        assert!((r.offset_ms.unwrap() - 50.0).abs() < 1e-6);
    }
}
