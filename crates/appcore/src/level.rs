//! Level advice (LH-EIN-03/04): judges the input level over the last seconds, not the instant
//! peak, so pauses between notes do not flip to "too quiet" (QA P1 #8). Tells "no signal" apart
//! from "too quiet", and after a few quiet seconds on the line input suggests the laptop-jack
//! profile (QA P1 #14).
//!
//! ```text
//!  −∞ … −60 dB   silent (after 1.5 s listening; before: nothing yet)
//! −60 … −36 dB   quiet  ──4 s on the line input──► suggest the laptop jack
//! −36 …  −6 dB   good
//!  −6 …   0 dB   loud;  any clipping in the last 3 s: clip
//! ```

use std::collections::VecDeque;

/// Below this the input carries no signal.
pub const SILENT_DB: f32 = -60.0;
/// Below this the bass is too quiet for safe detection.
pub const QUIET_DB: f32 = -36.0;
/// Above this the input is close to clipping.
pub const LOUD_DB: f32 = -6.0;
/// The loudest peak of this window counts.
const HOLD_S: f64 = 3.0;
/// "Silent" only after listening this long (the first notes take a moment).
const SILENT_AFTER_S: f64 = 1.5;
/// Quiet this long on the line input: perhaps the bass is in the laptop jack.
const LAPTOP_AFTER_S: f64 = 4.0;

/// What the level says, for the advice text ("advice.<key>").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelState {
    /// Listening started a moment ago.
    Waiting,
    Silent,
    Quiet,
    Good,
    Loud,
    Clip,
}

impl LevelState {
    pub fn key(self) -> &'static str {
        match self {
            LevelState::Waiting => "",
            LevelState::Silent => "silent",
            LevelState::Quiet => "quiet",
            LevelState::Good => "good",
            LevelState::Loud => "loud",
            LevelState::Clip => "clip",
        }
    }
}

/// Collects level readings and judges them.
#[derive(Debug, Clone, Default)]
pub struct LevelAdvisor {
    /// (time in s, peak in dBFS, clipped) of the last `HOLD_S`.
    history: VecDeque<(f64, f32, bool)>,
    first_s: Option<f64>,
    quiet_since_s: Option<f64>,
}

impl LevelAdvisor {
    /// Starts over (a new input).
    pub fn reset(&mut self) {
        *self = LevelAdvisor::default();
    }

    /// One reading at `at_s` (any clock that only moves forward).
    pub fn push(&mut self, at_s: f64, peak_db: f32, clipped: bool) {
        self.first_s.get_or_insert(at_s);
        self.history.push_back((at_s, peak_db, clipped));
        while self
            .history
            .front()
            .is_some_and(|(t, _, _)| at_s - t > HOLD_S)
        {
            self.history.pop_front();
        }
        match self.state_at(at_s) {
            LevelState::Quiet => {
                self.quiet_since_s.get_or_insert(at_s);
            }
            _ => self.quiet_since_s = None,
        }
    }

    /// The state after the last reading.
    pub fn state(&self) -> LevelState {
        let now = self.history.back().map_or(0.0, |(t, _, _)| *t);
        self.state_at(now)
    }

    fn state_at(&self, now: f64) -> LevelState {
        let held = self
            .history
            .iter()
            .map(|(_, db, _)| *db)
            .fold(f32::NEG_INFINITY, f32::max);
        let listened = self.first_s.map_or(0.0, |first| now - first);
        match held {
            _ if self.history.iter().any(|(_, _, clip)| *clip) => LevelState::Clip,
            db if db < SILENT_DB && listened > SILENT_AFTER_S => LevelState::Silent,
            db if db < SILENT_DB => LevelState::Waiting,
            db if db < QUIET_DB => LevelState::Quiet,
            db if db > LOUD_DB => LevelState::Loud,
            _ => LevelState::Good,
        }
    }

    /// Quiet for a while: suggest the laptop-jack profile (only useful on the line input).
    pub fn suggests_laptop(&self) -> bool {
        let now = self.history.back().map_or(0.0, |(t, _, _)| *t);
        self.quiet_since_s
            .is_some_and(|since| now - since > LAPTOP_AFTER_S)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Readings every 50 ms from `from_s` to `to_s` at `db`.
    fn feed(a: &mut LevelAdvisor, from_s: f64, to_s: f64, db: f32) {
        let mut t = from_s;
        while t < to_s {
            a.push(t, db, false);
            t += 0.05;
        }
    }

    #[test]
    fn levels_map_to_states() {
        for (db, state) in [
            (-70.0, LevelState::Silent),
            (-45.0, LevelState::Quiet),
            (-20.0, LevelState::Good),
            (-3.0, LevelState::Loud),
        ] {
            let mut a = LevelAdvisor::default();
            feed(&mut a, 0.0, 2.0, db);
            assert_eq!(a.state(), state, "{db} dB");
        }
    }

    #[test]
    fn silence_waits_a_moment_and_pauses_do_not_count() {
        let mut a = LevelAdvisor::default();
        feed(&mut a, 0.0, 1.0, -90.0);
        assert_eq!(a.state(), LevelState::Waiting);
        // A good note, then two seconds of pause: still good (the last 3 s count).
        a.push(1.0, -18.0, false);
        feed(&mut a, 1.05, 3.0, -90.0);
        assert_eq!(a.state(), LevelState::Good);
        feed(&mut a, 3.0, 4.2, -90.0);
        assert_eq!(a.state(), LevelState::Silent);
    }

    #[test]
    fn clipping_holds_for_the_window() {
        let mut a = LevelAdvisor::default();
        a.push(0.0, -0.1, true);
        feed(&mut a, 0.05, 2.5, -20.0);
        assert_eq!(a.state(), LevelState::Clip);
        feed(&mut a, 2.5, 3.5, -20.0);
        assert_eq!(a.state(), LevelState::Good);
    }

    #[test]
    fn long_quiet_suggests_the_laptop_jack() {
        let mut a = LevelAdvisor::default();
        feed(&mut a, 0.0, 3.0, -45.0);
        assert!(!a.suggests_laptop());
        feed(&mut a, 3.0, 5.0, -45.0);
        assert!(a.suggests_laptop());
        a.push(5.0, -20.0, false);
        assert!(!a.suggests_laptop(), "a good note ends it");
    }
}
