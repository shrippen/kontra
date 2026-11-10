//! The engine clock shared by input and output.

use std::sync::Arc;
use std::time::Instant;

/// Seconds since creation; cheap to clone and read on any thread.
#[derive(Debug, Clone)]
pub struct Clock {
    origin: Arc<Instant>,
}

impl Default for Clock {
    fn default() -> Self {
        Clock::new()
    }
}

impl Clock {
    pub fn new() -> Clock {
        Clock {
            origin: Arc::new(Instant::now()),
        }
    }

    pub fn now_s(&self) -> f64 {
        self.origin.elapsed().as_secs_f64()
    }
}

/// A stream's start settles over this much of its own time, then holds (R3).
pub(crate) const SETTLE_S: f64 = 0.5;

/// The start of a stream on the engine clock, from one more guess `elapsed_s` into the stream.
/// A late callback only makes a guess later, so the earliest wins; after `SETTLE_S` it holds.
/// Holding matters: input and output stamp with the same nominal rate, so a device clock that
/// runs fast moves both guesses alike; following it on one side only would shift the notes
/// against the beat (review #249: 20 ppm over 20 min ≈ 24 ms).
///
/// ```text
/// guesses  ·˙ ·  ˙·  ·      │ ·  ·  ·  (lower, ignored)
/// start    ─────────────────┴──────────
///          0 s           0.5 s
/// ```
pub(crate) fn settle(start_s: f64, guess_s: f64, elapsed_s: f64) -> f64 {
    if start_s.is_nan() {
        return guess_s;
    }
    if elapsed_s < SETTLE_S && guess_s < start_s {
        return guess_s;
    }
    start_s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_start_takes_the_earliest_guess_then_holds() {
        let mut start = f64::NAN;
        for (guess, elapsed) in [(10.004, 0.0), (10.001, 0.1), (10.003, 0.2), (10.0005, 0.6)] {
            start = settle(start, guess, elapsed);
        }
        assert_eq!(start, 10.001);
    }
}
