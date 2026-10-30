//! Latency calibration "play on the click" (LH-EIN-05): eight clicks, the player plucks along,
//! the median offset between click and detected onset is the round-trip latency (output +
//! input + detection) that scoring subtracts.

use serde::{Deserialize, Serialize};

pub const CLICKS: usize = 8;
pub const INTERVAL_S: f64 = 0.6;
/// Clicks before the measured ones: musicians wait for a count-in.
pub const COUNT_IN: usize = 4;
/// A detection belongs to the nearest click within this distance.
const MATCH_S: f64 = 0.25;
/// At least this many plucks must be matched.
const MIN_MATCHED: usize = 5;
/// Above this spread the player was not steady enough to measure.
const MAX_SPREAD_MS: f64 = 30.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CalibrationResult {
    /// Median offset of the plucks after the clicks in ms (latency plus the player's lean, below
    /// zero when the lean is ahead by more than the latency) and the spread (median absolute
    /// deviation) of the offsets.
    Measured { latency_ms: f64, spread_ms: f64 },
    /// Too few plucks heard (`matched` of `CLICKS`).
    TooFew { matched: usize },
    /// Plucks too uneven to measure.
    Unsteady { spread_ms: f64 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Calibration {
    clicks: Vec<f64>,
    onsets: Vec<f64>,
}

impl Calibration {
    /// Clicks every `INTERVAL_S` from `first_click_s` on (engine clock).
    pub fn new(first_click_s: f64) -> Calibration {
        let clicks = (0..CLICKS)
            .map(|i| first_click_s + i as f64 * INTERVAL_S)
            .collect();
        Calibration {
            clicks,
            onsets: Vec::new(),
        }
    }

    pub fn clicks(&self) -> &[f64] {
        &self.clicks
    }

    /// The count-in, one bar before the first measured click; plucks there are not measured.
    pub fn count_in(&self) -> Vec<f64> {
        let first = self.clicks.first().copied().unwrap_or(0.0);
        (0..COUNT_IN)
            .map(|i| first - (COUNT_IN - i) as f64 * INTERVAL_S)
            .collect()
    }

    /// The click last passed at `now_s`: −4..−1 the count-in, 0..7 the measured ones; `None`
    /// before the first (for a display that counts along, QA 14.10.: P1 heard clicks only).
    pub fn beat_at(&self, now_s: f64) -> Option<i32> {
        let passed = self
            .count_in()
            .iter()
            .chain(&self.clicks)
            .filter(|t| **t <= now_s)
            .count();
        let passed = i32::try_from(passed).ok()?;
        (passed > 0).then(|| passed - 1 - COUNT_IN as i32)
    }

    /// When the last pluck can have arrived.
    pub fn ends_at(&self) -> f64 {
        self.clicks.last().copied().unwrap_or(0.0) + MATCH_S + 0.5
    }

    pub fn on_onset(&mut self, onset_s: f64) {
        self.onsets.push(onset_s);
    }

    pub fn result(&self) -> CalibrationResult {
        let mut offsets = Vec::new();
        for click in &self.clicks {
            let nearest = self
                .onsets
                .iter()
                .map(|o| o - click)
                .filter(|d| d.abs() <= MATCH_S)
                .min_by(|a, b| a.abs().total_cmp(&b.abs()));
            if let Some(offset) = nearest {
                offsets.push(offset * 1000.0);
            }
        }
        if offsets.len() < MIN_MATCHED {
            return CalibrationResult::TooFew {
                matched: offsets.len(),
            };
        }
        let latency = median(&mut offsets.clone());
        let mut deviations: Vec<f64> = offsets.iter().map(|o| (o - latency).abs()).collect();
        let spread = median(&mut deviations);
        if spread > MAX_SPREAD_MS {
            return CalibrationResult::Unsteady { spread_ms: spread };
        }
        CalibrationResult::Measured {
            latency_ms: latency,
            spread_ms: spread,
        }
    }
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    let n = values.len();
    if n == 0 {
        return 0.0;
    }
    if n % 2 == 1 {
        values[n / 2]
    } else {
        (values[n / 2 - 1] + values[n / 2]) / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measures_median_latency_ignoring_one_outlier() {
        let mut c = Calibration::new(1.0);
        for (i, click) in c.clicks().to_vec().iter().enumerate() {
            let jitter = [0.0, 3.0, -2.0, 1.0, -1.0, 2.0, 0.0, 120.0][i] / 1000.0;
            c.on_onset(click + 0.040 + jitter);
        }
        let CalibrationResult::Measured {
            latency_ms,
            spread_ms,
        } = c.result()
        else {
            panic!("{:?}", c.result());
        };
        assert!((latency_ms - 40.5).abs() < 1.0, "{latency_ms}");
        assert!(spread_ms < 3.0);
    }

    // Hardware check 13.10.: without a count-in the player waited four clicks and missed half.
    #[test]
    fn count_in_comes_before_the_measured_clicks() {
        let c = Calibration::new(3.0);
        let count_in = c.count_in();
        assert_eq!(count_in.len(), COUNT_IN);
        assert!((count_in[0] - (3.0 - COUNT_IN as f64 * INTERVAL_S)).abs() < 1e-9);
        assert!((c.clicks()[0] - count_in[COUNT_IN - 1] - INTERVAL_S).abs() < 1e-9);
    }

    #[test]
    fn beats_count_in_then_measure() {
        let c = Calibration::new(3.0);
        assert_eq!(c.beat_at(0.0), None);
        assert_eq!(c.beat_at(0.61), Some(-4), "first count-in click at 0.6 s");
        assert_eq!(c.beat_at(2.99), Some(-1));
        assert_eq!(c.beat_at(3.0), Some(0));
        assert_eq!(c.beat_at(9.0), Some(7));
    }

    // Timing report 29.10. (R4): a player 30 ms ahead at 20 ms latency is heard 10 ms before the
    // click; the lean (click − loopback) needs that −10, not 0.
    #[test]
    fn keeps_a_median_before_the_click() {
        let mut c = Calibration::new(1.0);
        for click in c.clicks().to_vec() {
            c.on_onset(click - 0.010);
        }
        let CalibrationResult::Measured { latency_ms, .. } = c.result() else {
            panic!("{:?}", c.result());
        };
        assert!((latency_ms + 10.0).abs() < 1e-6, "{latency_ms}");
    }

    #[test]
    fn too_few_and_unsteady() {
        let mut c = Calibration::new(0.0);
        c.on_onset(0.05);
        assert_eq!(c.result(), CalibrationResult::TooFew { matched: 1 });
        let mut c = Calibration::new(0.0);
        for (i, click) in c.clicks().to_vec().iter().enumerate() {
            c.on_onset(click + if i % 2 == 0 { 0.0 } else { 0.2 });
        }
        assert!(matches!(c.result(), CalibrationResult::Unsteady { .. }));
    }
}
