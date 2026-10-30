//! What the fretboard quiz remembers (LH-LRN-18): the frets it asks and, per place on the neck,
//! how often the note was found or missed and when last. Weak and long unpractised places get a
//! higher weight, so they come more often.
//!
//! ```text
//! weight = floor + weak × (wrong + 1) / (right + wrong + 2) + stale × min(age / week, 1)
//!   never asked:            0.2 + 3 × 1/2 + 1 = 2.7
//!   missed 5×, today:       0.2 + 3 × 6/7 + 0 ≈ 2.8
//!   found 5×, today:        0.2 + 3 × 1/7 + 0 ≈ 0.6
//! ```

use serde::{Deserialize, Serialize};
use std::ops::RangeInclusive;

/// File in the data directory.
pub const FILE: &str = "quiz.json";
/// Highest fret a range may reach: the quiz asks up to the octave.
const TOP_FRET: u8 = 12;
/// Default range: the first position.
const DEFAULT_HIGH: u8 = 4;
/// A place not practised for a week counts as fully stale.
const STALE_S: f64 = 7.0 * 24.0 * 3600.0;
/// Every place keeps a floor, so a well-known one still comes now and then.
const WEIGHT_FLOOR: f64 = 0.2;
const WEAK_FACTOR: f64 = 3.0;
const STALE_FACTOR: f64 = 1.0;

/// Frets the quiz asks, both included.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FretRange {
    pub low: u8,
    pub high: u8,
}

impl FretRange {
    /// Clamped to the neck, `low` ≤ `high` ≤ `TOP_FRET` (5–2 becomes 5–5).
    pub fn new(low: i32, high: i32) -> FretRange {
        let top = i32::from(TOP_FRET);
        let low = low.clamp(0, top);
        FretRange {
            low: low as u8,
            high: high.clamp(low, top) as u8,
        }
    }

    pub(crate) fn frets(self) -> RangeInclusive<u8> {
        self.low..=self.high.max(self.low)
    }
}

impl Default for FretRange {
    fn default() -> Self {
        FretRange {
            low: 0,
            high: DEFAULT_HIGH,
        }
    }
}

/// How an answer went. A question left without a note (time up, skipped) records nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// Found without a wrong note before.
    Right,
    /// A wrong note was played (once per question).
    Wrong,
}

/// One place: the open string (MIDI) and the fret, so a 5-string and a 4-string bass share
/// their common strings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Place {
    open: u8,
    fret: u8,
    right: u32,
    wrong: u32,
    last_s: u64,
}

/// Stored in `FILE`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct QuizStats {
    pub range: FretRange,
    places: Vec<Place>,
}

impl QuizStats {
    pub fn new(range: FretRange) -> QuizStats {
        QuizStats {
            range,
            places: Vec::new(),
        }
    }

    pub(crate) fn record(&mut self, open: u8, fret: u8, outcome: Outcome, now_s: u64) {
        let index = match self.find(open, fret) {
            Some(i) => i,
            None => {
                self.places.push(Place {
                    open,
                    fret,
                    right: 0,
                    wrong: 0,
                    last_s: now_s,
                });
                self.places.len() - 1
            }
        };
        let place = &mut self.places[index];
        match outcome {
            Outcome::Right => place.right += 1,
            Outcome::Wrong => place.wrong += 1,
        }
        place.last_s = now_s;
    }

    /// (right, wrong) of a place.
    #[cfg(test)]
    pub(crate) fn counts(&self, open: u8, fret: u8) -> (u32, u32) {
        self.find(open, fret)
            .map_or((0, 0), |i| (self.places[i].right, self.places[i].wrong))
    }

    /// Joins the answers of another machine: per place the newer record counts, so importing
    /// the same file twice changes nothing; the own range stays.
    pub(crate) fn merged(mut self, from: QuizStats) -> QuizStats {
        for place in from.places {
            match self.find(place.open, place.fret) {
                Some(i) if self.places[i].last_s >= place.last_s => {}
                Some(i) => self.places[i] = place,
                None => self.places.push(place),
            }
        }
        self
    }

    /// How likely the place is asked next, relative to the others (see the module comment).
    pub(crate) fn weight(&self, open: u8, fret: u8, now_s: u64) -> f64 {
        let Some(place) = self.find(open, fret).map(|i| &self.places[i]) else {
            return WEIGHT_FLOOR + WEAK_FACTOR * 0.5 + STALE_FACTOR;
        };
        let (right, wrong) = (f64::from(place.right), f64::from(place.wrong));
        let weak = (wrong + 1.0) / (right + wrong + 2.0);
        let age_s = now_s.saturating_sub(place.last_s) as f64;
        let stale = (age_s / STALE_S).min(1.0);
        WEIGHT_FLOOR + WEAK_FACTOR * weak + STALE_FACTOR * stale
    }

    fn find(&self, open: u8, fret: u8) -> Option<usize> {
        self.places
            .iter()
            .position(|p| p.open == open && p.fret == fret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_000_000_000;
    const E1: u8 = 28;

    #[test]
    fn ranges_stay_on_the_neck() {
        assert_eq!(FretRange::default().frets(), 0..=4);
        assert_eq!(FretRange::new(5, 2).frets(), 5..=5);
        assert_eq!(FretRange::new(-3, 40).frets(), 0..=12);
    }

    #[test]
    fn weak_and_stale_places_weigh_more() {
        let mut stats = QuizStats::default();
        for _ in 0..5 {
            stats.record(E1, 1, Outcome::Wrong, NOW);
            stats.record(E1, 2, Outcome::Right, NOW);
        }
        stats.record(E1, 3, Outcome::Right, NOW - 30 * 24 * 3600);
        let missed = stats.weight(E1, 1, NOW);
        let found = stats.weight(E1, 2, NOW);
        assert!(missed > 4.0 * found, "{missed} vs {found}");
        assert!(
            stats.weight(E1, 3, NOW) > stats.weight(E1, 2, NOW),
            "a month ago"
        );
        assert!(stats.weight(E1, 4, NOW) > found, "never asked");
        assert_eq!(stats.counts(E1, 1), (0, 5));
    }

    #[test]
    fn stats_survive_a_save() {
        let mut stats = QuizStats::new(FretRange::new(0, 7));
        stats.record(E1, 5, Outcome::Wrong, NOW);
        let text = serde_json::to_string(&stats).unwrap();
        assert_eq!(serde_json::from_str::<QuizStats>(&text).unwrap(), stats);
        assert_eq!(
            serde_json::from_str::<QuizStats>("{}").unwrap(),
            QuizStats::default()
        );
    }
}
