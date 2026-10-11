//! Tone shaping for input monitoring (LH-EIN-08): three equaliser bands and a compressor.
//!
//! ```text
//! bass ─► low shelf 100 Hz ─► peak 800 Hz ─► high shelf 4 kHz ─► compressor ─► monitor
//! ```
//!
//! Runs in the audio callback: no allocation, one sample at a time. Changed settings glide
//! towards their target in small steps every 32 samples, so moving a slider does not click.

use crate::biquad::{Biquad, EqKind};

/// Largest boost or cut of a band, in dB.
pub const SHAPING_RANGE_DB: f32 = 12.0;

const LOW_HZ: f64 = 100.0;
const MID_HZ: f64 = 800.0;
const HIGH_HZ: f64 = 4000.0;

/// Samples between two steps towards the target settings.
const RAMP_FRAMES: u32 = 32;
/// Largest band change per step: 12 dB take 48 steps, 32 ms at 48 kHz.
const RAMP_DB: f32 = 0.25;
/// Largest compressor change per step.
const RAMP_COMPRESS: f32 = 0.02;

const ATTACK_S: f64 = 0.010;
const RELEASE_S: f64 = 0.150;
/// Threshold at the lightest setting and how far it drops at the strongest (−6 to −30 dBFS).
const THRESHOLD_DB: f32 = -6.0;
const THRESHOLD_RANGE_DB: f32 = 24.0;
/// Ratio rises from 1:1 to 6:1.
const RATIO_RANGE: f32 = 5.0;

/// Settings of the tone shaper; `FLAT` changes nothing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shaping {
    /// Bands in dB, within ±`SHAPING_RANGE_DB`.
    pub low_db: f32,
    pub mid_db: f32,
    pub high_db: f32,
    /// Compressor amount, 0 (off) to 1 (strong).
    pub compress: f32,
}

impl Shaping {
    pub const FLAT: Shaping = Shaping {
        low_db: 0.0,
        mid_db: 0.0,
        high_db: 0.0,
        compress: 0.0,
    };

    fn clamped(self) -> Shaping {
        let band = |db: f32| db.clamp(-SHAPING_RANGE_DB, SHAPING_RANGE_DB);
        Shaping {
            low_db: band(self.low_db),
            mid_db: band(self.mid_db),
            high_db: band(self.high_db),
            compress: self.compress.clamp(0.0, 1.0),
        }
    }
}

impl Default for Shaping {
    fn default() -> Self {
        Shaping::FLAT
    }
}

/// Applies a `Shaping` to a mono signal.
#[derive(Debug, Clone)]
pub struct Shaper {
    low: Biquad,
    mid: Biquad,
    high: Biquad,
    current: Shaping,
    target: Shaping,
    sample_rate: f64,
    countdown: u32,
    /// Peak follower of the compressor, linear.
    envelope: f32,
    attack: f32,
    release: f32,
}

impl Shaper {
    pub fn new(sample_rate: f64) -> Shaper {
        let flat = Shaping::FLAT;
        let mut shaper = Shaper {
            low: Biquad::eq(EqKind::LowShelf, sample_rate, LOW_HZ, 0.0),
            mid: Biquad::eq(EqKind::Peak, sample_rate, MID_HZ, 0.0),
            high: Biquad::eq(EqKind::HighShelf, sample_rate, HIGH_HZ, 0.0),
            current: flat,
            target: flat,
            sample_rate,
            countdown: 0,
            envelope: 0.0,
            attack: 0.0,
            release: 0.0,
        };
        shaper.retime(sample_rate);
        shaper
    }

    /// New target; the shaper glides there.
    pub fn set(&mut self, shaping: Shaping) {
        self.target = shaping.clamped();
    }

    /// One sample at `sample_rate` (the output device's rate may change between streams).
    pub fn process(&mut self, x: f32, sample_rate: f64) -> f32 {
        if sample_rate != self.sample_rate {
            self.retime(sample_rate);
            self.retune();
        }
        if self.countdown == 0 {
            self.countdown = RAMP_FRAMES;
            self.step();
        }
        self.countdown -= 1;

        // Flat bands still run, so their state is current when a band moves, but pass the
        // input exactly (no rounding residue).
        let shaped = self.high.process(self.mid.process(self.low.process(x)));
        let c = self.current;
        let flat = c.low_db == 0.0 && c.mid_db == 0.0 && c.high_db == 0.0;
        let y = if flat { x } else { shaped };
        self.compress(y)
    }

    /// Moves `current` one step towards `target` and recomputes the filters if it moved.
    fn step(&mut self) {
        if self.current == self.target {
            return;
        }
        let toward = |from: f32, to: f32, max: f32| from + (to - from).clamp(-max, max);
        let (c, t) = (self.current, self.target);
        self.current = Shaping {
            low_db: toward(c.low_db, t.low_db, RAMP_DB),
            mid_db: toward(c.mid_db, t.mid_db, RAMP_DB),
            high_db: toward(c.high_db, t.high_db, RAMP_DB),
            compress: toward(c.compress, t.compress, RAMP_COMPRESS),
        };
        self.retune();
    }

    fn retune(&mut self) {
        let (sr, c) = (self.sample_rate, self.current);
        self.low
            .retune(EqKind::LowShelf, sr, LOW_HZ, f64::from(c.low_db));
        self.mid
            .retune(EqKind::Peak, sr, MID_HZ, f64::from(c.mid_db));
        self.high
            .retune(EqKind::HighShelf, sr, HIGH_HZ, f64::from(c.high_db));
    }

    fn retime(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate;
        self.attack = (-1.0 / (ATTACK_S * sample_rate)).exp() as f32;
        self.release = (-1.0 / (RELEASE_S * sample_rate)).exp() as f32;
    }

    /// Peak compressor with automatic make-up of half the reduction at full scale.
    /// Example at strength 1: threshold −30 dBFS, 6:1; a −6 dBFS peak comes out at −14 dBFS.
    fn compress(&mut self, y: f32) -> f32 {
        let amount = self.current.compress;
        if amount <= 0.0 {
            return y;
        }
        let level = y.abs();
        let coeff = if level > self.envelope {
            self.attack
        } else {
            self.release
        };
        self.envelope = level + coeff * (self.envelope - level);

        let threshold_db = THRESHOLD_DB - THRESHOLD_RANGE_DB * amount;
        let slope = 1.0 - 1.0 / (1.0 + RATIO_RANGE * amount);
        let makeup_db = -threshold_db * slope / 2.0;
        let over_db = crate::amplitude_to_db(self.envelope) - threshold_db;
        let reduction_db = over_db.max(0.0) * slope;
        y * 10f32.powf((makeup_db - reduction_db) / 20.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::{peak_db, rms_db};
    use std::f64::consts::PI;

    const SR: f64 = 48_000.0;

    fn sine(hz: f64, amplitude: f32, n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| amplitude * (2.0 * PI * hz * i as f64 / SR).sin() as f32)
            .collect()
    }

    fn run(shaper: &mut Shaper, input: &[f32]) -> Vec<f32> {
        input.iter().map(|&x| shaper.process(x, SR)).collect()
    }

    #[test]
    fn flat_changes_nothing() {
        let mut shaper = Shaper::new(SR);
        let input = sine(110.0, 0.5, 4800);
        let out = run(&mut shaper, &input);
        let max_diff = input
            .iter()
            .zip(&out)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f32::max);
        assert!(max_diff < 1e-5);
    }

    #[test]
    fn low_band_lifts_the_low_end() {
        let mut shaper = Shaper::new(SR);
        shaper.set(Shaping {
            low_db: 6.0,
            ..Shaping::FLAT
        });
        let input = sine(41.2, 0.25, 48_000);
        let out = run(&mut shaper, &input);
        let gain = rms_db(&out[24_000..]) - rms_db(&input[24_000..]);
        assert!((gain - 6.0).abs() < 0.5, "gain {gain}");
    }

    #[test]
    fn compressor_narrows_dynamics() {
        let mut shaper = Shaper::new(SR);
        shaper.set(Shaping {
            compress: 1.0,
            ..Shaping::FLAT
        });
        let loud = sine(110.0, 0.5, 24_000);
        let quiet = sine(110.0, 0.05, 24_000);
        // Warm up so the ramp has ended.
        run(&mut shaper, &quiet);
        let loud_out = run(&mut shaper, &loud);
        let quiet_out = run(&mut shaper, &quiet);
        let range_in = peak_db(&loud) - peak_db(&quiet);
        let range_out = peak_db(&loud_out[12_000..]) - peak_db(&quiet_out[12_000..]);
        assert!(range_out < range_in - 8.0, "{range_in} → {range_out}");
    }

    #[test]
    fn changes_glide_without_jumps() {
        // A full 12 dB boost while a low note sounds: no step larger than the
        // steepest slope of the boosted note itself.
        let mut shaper = Shaper::new(SR);
        let input = sine(41.2, 0.25, 9600);
        let mut out = run(&mut shaper, &input[..4800]);
        shaper.set(Shaping {
            low_db: SHAPING_RANGE_DB,
            ..Shaping::FLAT
        });
        out.extend(run(&mut shaper, &input[4800..]));
        let max_step = out
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .fold(0.0, f32::max);
        let boosted_slope = 0.25 * 4.0 * (2.0 * PI * 41.2 / SR) as f32;
        assert!(max_step < boosted_slope * 1.1, "step {max_step}");
    }

    #[test]
    fn settings_are_clamped() {
        let mut shaper = Shaper::new(SR);
        shaper.set(Shaping {
            low_db: 40.0,
            mid_db: -40.0,
            high_db: 0.0,
            compress: 3.0,
        });
        assert_eq!(shaper.target.low_db, SHAPING_RANGE_DB);
        assert_eq!(shaper.target.mid_db, -SHAPING_RANGE_DB);
        assert_eq!(shaper.target.compress, 1.0);
    }
}
