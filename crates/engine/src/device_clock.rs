//! A device's clock as told by its block stamps, smoothed by a delay-locked loop (DLL; F.
//! Adriaensen, "Using a DLL to filter time", 2005). The stamps jitter (cpal: "avail_delay()
//! varies between cycles"); the loop follows the device's real rate and drops the jitter, so a
//! gap shows as a clean step.
//!
//! ```text
//! stamps   ·  ˙ ·  ·˙  ·  ˙·                      ·˙  · ˙ ·
//! loop     ─────────────────── step: skipped 0.8 s ──────────
//!          sample →
//! ```
//!
//! Runs on the analysis thread: the audio callback only writes stamps into the input ring.

use std::f64::consts::{PI, SQRT_2};

/// Loop bandwidth while locking: follows the start quickly.
const LOCK_HZ: f64 = 5.0;
/// Loop bandwidth once locked: stamp jitter (about ±0.5 ms) drops to a few µs, drift of
/// ±50 ppm stays followed (simulation 29.10.2026, `sim3.py`: ≤ 0.12 ms over 5 minutes).
const TRACK_HZ: f64 = 0.5;
/// How long the loop locks with `LOCK_HZ`, in device time.
const LOCK_S: f64 = 2.0;
/// Loop gain per update at most (2π · bandwidth · update interval). Larger gains overshoot and
/// ring: at a quantum of 2048 frames (43 ms) `LOCK_HZ` alone would give 1.34.
const MAX_GAIN: f64 = 0.5;
/// A stamp this far off the loop is a step, not jitter: the device skipped (or repeated) time.
/// Smaller losses count as none; they stay below what a player hears as off the beat.
const STEP_MIN_S: f64 = 0.004;

/// Where the loop stands: the smoothed device time of one sample.
#[derive(Debug, Clone, Copy)]
struct Lock {
    sample: u64,
    time_s: f64,
}

/// A device clock filtered from block stamps.
#[derive(Debug, Clone)]
pub(crate) struct DeviceClock {
    /// Nominal sample rate: the loop's period starts there.
    rate: f64,
    /// Seconds per sample as the loop measures them.
    period_s: f64,
    lock: Option<Lock>,
    /// Device time of the first stamp, for `LOCK_S`.
    first_s: f64,
    /// Device time the stamps stepped ahead of the samples so far.
    skipped_s: f64,
}

impl DeviceClock {
    pub(crate) fn new(rate: f64) -> DeviceClock {
        DeviceClock {
            rate,
            period_s: 1.0 / rate,
            lock: None,
            first_s: 0.0,
            skipped_s: 0.0,
        }
    }

    /// Takes a stamp: the device time of `sample`. Stamps come in sample order.
    pub(crate) fn stamp(&mut self, sample: u64, stamp_s: f64) {
        let Some(lock) = self.lock else {
            self.first_s = stamp_s;
            self.lock = Some(Lock {
                sample,
                time_s: stamp_s,
            });
            return;
        };
        let Some(frames) = sample.checked_sub(lock.sample).filter(|f| *f > 0) else {
            return;
        };

        let predicted_s = lock.time_s + frames as f64 * self.period_s;
        let error_s = stamp_s - predicted_s;

        // A step: the loop locks again from the stamp, at the rate and bandwidth it had.
        if error_s.abs() >= STEP_MIN_S {
            self.skipped_s += error_s;
            self.lock = Some(Lock {
                sample,
                time_s: stamp_s,
            });
            return;
        }

        // Second-order loop: the time moves by √2·ω of the error, the period by ω² of it.
        // E.g. ω 0.04 (128 frames, 0.5 Hz): 5.7 % of each error, the rate by 0.16 % of it.
        let interval_s = frames as f64 / self.rate;
        let gain = (2.0 * PI * self.bandwidth_hz(stamp_s) * interval_s).min(MAX_GAIN);
        self.lock = Some(Lock {
            sample,
            time_s: predicted_s + SQRT_2 * gain * error_s,
        });
        self.period_s += gain * gain * error_s / frames as f64;
    }

    /// Smoothed device time of `sample`, once a stamp came. Only the tests read it until the
    /// input and the player are anchored on it (R3, second part).
    #[cfg(test)]
    fn time_s(&self, sample: u64) -> Option<f64> {
        let lock = self.lock?;
        let frames = sample as f64 - lock.sample as f64;
        Some(lock.time_s + frames * self.period_s)
    }

    /// Device time skipped so far: the sum of the steps (negative ones included).
    pub(crate) fn skipped_s(&self) -> f64 {
        self.skipped_s
    }

    fn bandwidth_hz(&self, stamp_s: f64) -> f64 {
        if stamp_s - self.first_s < LOCK_S {
            return LOCK_HZ;
        }
        TRACK_HZ
    }
}

#[cfg(test)]
mod tests;
