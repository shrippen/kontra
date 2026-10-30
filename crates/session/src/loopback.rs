//! Latency by cable (LH-EIN-05): the output plays tones into the input, the median delay is the
//! system latency. Unlike "play on the click" it holds no lean of the player, so a steady lean
//! in a run is the player's and no reason to recalibrate (LH-EIN-14, timing report 29.10.).
//!
//! ```text
//! output ─► tone ─► cable ─► input ─► detected onset
//!           │◄──────── round trip (median) ────────►│ = latency
//! ```

use crate::check::{loopback_length_s, loopback_plan, round_trip, LiveTake, RoundTrip, TakeSound};
use crate::live::LiveSetup;
use kontra_engine::EngineError;
use kontra_model::Instrument;

/// At least this many of the tones must come back: fewer is crosstalk or noise, not a cable.
const MIN_HEARD: usize = 6;
/// Above this spread the tones came back too unevenly (dropouts, a busy machine).
const MAX_SPREAD_MS: f64 = 10.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoopbackResult {
    /// Median round trip and its largest deviation.
    Measured { latency_ms: f64, spread_ms: f64 },
    /// Too few tones came back: no cable, or the output too quiet.
    Silent { heard: usize },
    /// The tones came back too unevenly to trust.
    Uneven { spread_ms: f64 },
}

impl LoopbackResult {
    /// Judges a measured round trip; `None` (nothing came back) is silence.
    pub fn from_trip(trip: Option<RoundTrip>) -> LoopbackResult {
        let Some(trip) = trip else {
            return LoopbackResult::Silent { heard: 0 };
        };
        if trip.heard < MIN_HEARD {
            return LoopbackResult::Silent { heard: trip.heard };
        }
        if trip.spread_ms > MAX_SPREAD_MS {
            return LoopbackResult::Uneven {
                spread_ms: trip.spread_ms,
            };
        }
        LoopbackResult::Measured {
            latency_ms: trip.median_ms,
            spread_ms: trip.spread_ms,
        }
    }
}

/// A running loopback measurement.
#[derive(Debug)]
pub struct LiveLoopback {
    take: Option<LiveTake>,
    plan: Vec<(f64, TakeSound)>,
}

impl LiveLoopback {
    /// Starts input and output; the tones begin one second after the input's first sample.
    pub fn start(setup: &LiveSetup, instrument: &Instrument) -> Result<LiveLoopback, EngineError> {
        let plan = loopback_plan();
        let take = LiveTake::start(setup, instrument, plan.clone(), loopback_length_s())?;
        Ok(LiveLoopback {
            take: Some(take),
            plan,
        })
    }

    /// Polls; returns the result once, when the last tone can have come back.
    pub fn poll(&mut self) -> Option<LoopbackResult> {
        if !self.take.as_mut()?.poll() {
            return None;
        }
        let take = self.take.take()?.finish();
        Some(LoopbackResult::from_trip(round_trip(&self.plan, &take)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trip(median_ms: f64, spread_ms: f64, heard: usize) -> Option<RoundTrip> {
        Some(RoundTrip {
            median_ms,
            spread_ms,
            heard,
            sent: 12,
        })
    }

    #[test]
    fn judges_the_round_trip() {
        assert_eq!(
            LoopbackResult::from_trip(trip(21.0, 1.0, 12)),
            LoopbackResult::Measured {
                latency_ms: 21.0,
                spread_ms: 1.0
            }
        );
        assert_eq!(
            LoopbackResult::from_trip(None),
            LoopbackResult::Silent { heard: 0 },
            "no cable"
        );
        assert_eq!(
            LoopbackResult::from_trip(trip(21.0, 1.0, 3)),
            LoopbackResult::Silent { heard: 3 },
            "crosstalk"
        );
        assert_eq!(
            LoopbackResult::from_trip(trip(21.0, 40.0, 12)),
            LoopbackResult::Uneven { spread_ms: 40.0 }
        );
    }
}
