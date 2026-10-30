//! Noise floor of the onset band, measured at the calibration's count-in (LH-EIN-16), and the
//! onset gate set from it.
//!
//! ```text
//! band level per frame (dB)   ▁▁▂▁▁█▁▁▁▂▁▁▁█▁▁▁   count-in: silence, the clicks heard in the room
//!                              └─ 10th percentile ─► floor ─► gate = floor + 6 dB, −62..−40
//! ```
//!
//! A low percentile, not the mean: clicks in a microphone and a pluck along the count-in are
//! loud for a few frames only.

use std::ops::Range;

/// Lowest band level counted (dB, as `OnsetConfig::gate_db`); quieter frames count here.
const LOWEST_DB: f32 = -120.0;
/// One count per dB from `LOWEST_DB` to 0 dB.
const LEVELS: usize = 121;
/// Share of the frames at or below the floor.
const PERCENTILE: f32 = 0.1;
/// An attack must rise this far above the floor. Steady noise alone gives no onsets (the flux
/// is compared with its own mean), so the gate only needs to sit above the noise, not far.
const GATE_OVER_FLOOR_DB: f32 = 6.0;
/// The gate at a quiet input: the line default (`OnsetConfig::default`).
const GATE_MIN_DB: f32 = -62.0;
/// The gate at a loud floor: above it soft plucks at a microphone (about −30 dBFS) are lost.
const GATE_MAX_DB: f32 = -40.0;

/// The onset gate for a measured band floor: `GATE_OVER_FLOOR_DB` above it, within
/// `GATE_MIN_DB..=GATE_MAX_DB`. E.g. white noise at −50 dBFS reads about −58 dB in the band
/// and gives −52 dB.
pub fn gate_over_floor(floor_db: f32) -> f32 {
    (floor_db + GATE_OVER_FLOOR_DB).clamp(GATE_MIN_DB, GATE_MAX_DB)
}

/// Collects band levels of frames within a span of samples; allocation-free.
#[derive(Debug, Clone)]
pub(crate) struct FloorMeter {
    span: Range<u64>,
    counts: [u32; LEVELS],
    frames: u32,
    /// A frame after the span was seen: the floor is final.
    done: bool,
}

impl Default for FloorMeter {
    fn default() -> Self {
        FloorMeter {
            span: 0..0,
            counts: [0; LEVELS],
            frames: 0,
            done: false,
        }
    }
}

impl FloorMeter {
    /// Measures frames that lie wholly in `span` (samples); forgets an earlier measurement.
    pub(crate) fn start(&mut self, span: Range<u64>) {
        *self = FloorMeter {
            span,
            ..FloorMeter::default()
        };
    }

    /// The frame covering `frame` (samples) had `level_db` in the band.
    pub(crate) fn add(&mut self, frame: Range<u64>, level_db: f32) {
        if self.span.is_empty() || self.done {
            return;
        }
        if frame.end > self.span.end {
            self.done = true;
            return;
        }
        if frame.start < self.span.start {
            return;
        }
        let index = (level_db - LOWEST_DB)
            .floor()
            .clamp(0.0, (LEVELS - 1) as f32) as usize;
        self.counts[index] += 1;
        self.frames += 1;
    }

    /// The floor (dB, upper edge of its 1-dB step), once the span has passed and was heard.
    pub(crate) fn floor_db(&self) -> Option<f32> {
        if !self.done || self.frames == 0 {
            return None;
        }
        let wanted = ((self.frames as f32 * PERCENTILE).ceil() as u32).max(1);
        let mut seen = 0;
        for (i, count) in self.counts.iter().enumerate() {
            seen += count;
            if seen >= wanted {
                return Some(LOWEST_DB + i as f32 + 1.0);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_percentile_ignores_short_loud_frames() {
        let mut meter = FloorMeter::default();
        meter.start(100..1100);
        for i in 0..10u64 {
            // One click frame in ten.
            let level = if i == 3 { -10.0 } else { -58.4 };
            meter.add(100 + i * 100..200 + i * 100, level);
        }
        assert_eq!(meter.floor_db(), None, "span not passed yet");
        meter.add(1050..1150, -58.0);
        assert_eq!(meter.floor_db(), Some(-58.0));
    }

    #[test]
    fn frames_outside_the_span_do_not_count() {
        let mut meter = FloorMeter::default();
        meter.start(1000..2000);
        meter.add(900..1100, -10.0);
        meter.add(1100..1300, -70.0);
        meter.add(1900..2100, -10.0);
        assert_eq!(meter.floor_db(), Some(-69.0));
    }

    #[test]
    fn gate_stays_within_sensible_values() {
        assert_eq!(gate_over_floor(-58.0), -52.0);
        assert_eq!(gate_over_floor(-120.0), GATE_MIN_DB);
        assert_eq!(gate_over_floor(-20.0), GATE_MAX_DB);
    }
}
