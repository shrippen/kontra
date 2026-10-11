//! Tone shaping for input monitoring (LH-EIN-08): three equaliser bands, a compressor and
//! warmth (light saturation, like an amp driven a little harder).
//!
//! ```text
//! input ─► low shelf ─► mid bell ─► high shelf ─► compressor ─► warmth ─► monitor
//!                                                                  │
//!                         2× rate: up ─► tanh ─► anti-alias ─► down
//!
//! The band frequencies come from the instrument profile (`ToneBands`; bass: 100 Hz,
//! 800 Hz, 4 kHz), so another instrument needs no change here.
//! ```
//!
//! Runs in the audio callback: no allocation, one sample at a time. Changed settings glide
//! towards their target in small steps every 32 samples, so moving a slider does not click.

use crate::biquad::{Biquad, EqKind, FilterKind};
use kontra_model::ToneBands;

/// Largest boost or cut of a band, in dB.
pub const SHAPING_RANGE_DB: f32 = 12.0;

/// Samples between two steps towards the target settings.
const RAMP_FRAMES: u32 = 32;
/// Largest band change per step: 12 dB take 48 steps, 32 ms at 48 kHz.
const RAMP_DB: f32 = 0.25;
/// Largest compressor change per step.
const RAMP_COMPRESS: f32 = 0.02;

/// Drive of the saturation at full warmth: small signals pass at unity, peaks round off.
/// Example: a 0.5 peak comes out at 0.24 (−6 dB), a 0.05 peak almost unchanged.
const WARMTH_DRIVE: f32 = 3.0;
/// Below this warmth the saturated path fades in from the dry signal (no click when switched on).
const WARMTH_FADE: f32 = 0.1;
/// Anti-alias cutoff relative to the base rate (20 kHz at 48 kHz); four Butterworth sections.
const ALIAS_CUTOFF: f64 = 0.42;
const ALIAS_SECTIONS: usize = 4;

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
    /// Saturation, 0 (off) to 1 (warm).
    pub warmth: f32,
    /// Where the bands sit, from the instrument profile; changes at once, not gliding.
    pub bands: ToneBands,
}

impl Shaping {
    pub const FLAT: Shaping = Shaping {
        low_db: 0.0,
        mid_db: 0.0,
        high_db: 0.0,
        compress: 0.0,
        warmth: 0.0,
        bands: ToneBands::BASS,
    };

    fn clamped(self) -> Shaping {
        let band = |db: f32| db.clamp(-SHAPING_RANGE_DB, SHAPING_RANGE_DB);
        Shaping {
            low_db: band(self.low_db),
            mid_db: band(self.mid_db),
            high_db: band(self.high_db),
            compress: self.compress.clamp(0.0, 1.0),
            warmth: self.warmth.clamp(0.0, 1.0),
            bands: self.bands,
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
    warmth: Saturator,
}

impl Shaper {
    pub fn new(sample_rate: f64) -> Shaper {
        let flat = Shaping::FLAT;
        let mut shaper = Shaper {
            low: Biquad::eq(EqKind::LowShelf, sample_rate, flat.bands.low_hz, 0.0),
            mid: Biquad::eq(EqKind::Peak, sample_rate, flat.bands.mid_hz, 0.0),
            high: Biquad::eq(EqKind::HighShelf, sample_rate, flat.bands.high_hz, 0.0),
            current: flat,
            target: flat,
            sample_rate,
            countdown: 0,
            envelope: 0.0,
            attack: 0.0,
            release: 0.0,
            warmth: Saturator::new(sample_rate),
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
            self.warmth = Saturator::new(sample_rate);
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
        let y = self.compress(y);
        self.warmth.process(y, self.current.warmth)
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
            warmth: toward(c.warmth, t.warmth, RAMP_COMPRESS),
            bands: t.bands,
        };
        self.retune();
    }

    fn retune(&mut self) {
        let (sr, c) = (self.sample_rate, self.current);
        let b = c.bands;
        self.low
            .retune(EqKind::LowShelf, sr, b.low_hz, f64::from(c.low_db));
        self.mid
            .retune(EqKind::Peak, sr, b.mid_hz, f64::from(c.mid_db));
        self.high
            .retune(EqKind::HighShelf, sr, b.high_hz, f64::from(c.high_db));
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

/// Soft clipping at twice the sample rate, so the harmonics it adds above the base Nyquist
/// are filtered out instead of folding back as inharmonic tones.
#[derive(Debug, Clone)]
struct Saturator {
    up: [Biquad; ALIAS_SECTIONS],
    down: [Biquad; ALIAS_SECTIONS],
}

impl Saturator {
    fn new(sample_rate: f64) -> Saturator {
        let section = || {
            Biquad::new(
                FilterKind::LowPass,
                2.0 * sample_rate,
                ALIAS_CUTOFF * sample_rate,
            )
        };
        Saturator {
            up: std::array::from_fn(|_| section()),
            down: std::array::from_fn(|_| section()),
        }
    }

    /// One base-rate sample; `warmth` 0 returns `x` unchanged. The filters always run, so
    /// their state is current when warmth comes on.
    fn process(&mut self, x: f32, warmth: f32) -> f32 {
        let drive = 1.0 + (WARMTH_DRIVE - 1.0) * warmth;
        let saturate = |v: f32| (drive * v).tanh() / drive;

        // Zero-stuffing doubles the rate; the factor 2 keeps the level.
        let mut wet = 0.0;
        for input in [2.0 * x, 0.0] {
            let up = self.up.iter_mut().fold(input, |v, f| f.process(v));
            wet = self.down.iter_mut().fold(saturate(up), |v, f| f.process(v));
        }

        if warmth <= 0.0 {
            return x;
        }
        let mix = (warmth / WARMTH_FADE).min(1.0);
        x + (wet - x) * mix
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
    fn bands_follow_the_profile() {
        // Another instrument with its low shelf at 400 Hz: a 6 dB lift reaches 200 Hz,
        // which the bass shelf at 100 Hz leaves almost untouched.
        let lift = |low_hz: f64| {
            let mut shaper = Shaper::new(SR);
            shaper.set(Shaping {
                low_db: 6.0,
                bands: ToneBands {
                    low_hz,
                    ..ToneBands::BASS
                },
                ..Shaping::FLAT
            });
            let input = sine(200.0, 0.25, 48_000);
            let out = run(&mut shaper, &input);
            rms_db(&out[24_000..]) - rms_db(&input[24_000..])
        };
        assert!(lift(400.0) > 4.5);
        assert!(lift(ToneBands::BASS.low_hz) < 1.5);
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
            warmth: -1.0,
            ..Shaping::FLAT
        });
        assert_eq!(shaper.target.low_db, SHAPING_RANGE_DB);
        assert_eq!(shaper.target.mid_db, -SHAPING_RANGE_DB);
        assert_eq!(shaper.target.compress, 1.0);
        assert_eq!(shaper.target.warmth, 0.0);
    }

    /// Level of `hz` in `signal`, in dB relative to a full-scale sine (Goertzel).
    fn tone_db(signal: &[f32], hz: f64) -> f32 {
        let w = 2.0 * PI * hz / SR;
        let (mut s1, mut s2) = (0.0f64, 0.0f64);
        for &x in signal {
            let s0 = f64::from(x) + 2.0 * w.cos() * s1 - s2;
            s2 = s1;
            s1 = s0;
        }
        let power = s1 * s1 + s2 * s2 - 2.0 * w.cos() * s1 * s2;
        let amplitude = 2.0 * power.sqrt() / signal.len() as f64;
        crate::amplitude_to_db(amplitude as f32)
    }

    fn warm(input: &[f32]) -> Vec<f32> {
        let mut shaper = Shaper::new(SR);
        shaper.set(Shaping {
            warmth: 1.0,
            ..Shaping::FLAT
        });
        run(&mut shaper, input)
    }

    #[test]
    fn warmth_adds_harmonics() {
        // 110 Hz (A string): the third harmonic at 330 Hz appears.
        let input = sine(110.0, 0.5, 48_000);
        let out = warm(&input);
        let tail = &out[24_000..];
        assert!(tone_db(&input[24_000..], 330.0) < -80.0);
        assert!(tone_db(tail, 330.0) - tone_db(tail, 110.0) > -30.0);
    }

    #[test]
    fn warmth_does_not_alias() {
        // 7 kHz: the fifth harmonic (35 kHz) would fold back to 13 kHz at the base rate.
        let input = sine(7000.0, 0.5, 48_000);
        let out = warm(&input);
        let tail = &out[24_000..];
        let alias = tone_db(tail, 13_000.0) - tone_db(tail, 7000.0);
        assert!(alias < -50.0, "alias {alias} dB");
    }

    #[test]
    fn warmth_keeps_quiet_notes() {
        // A soft note passes almost unchanged, so the bass does not get quieter overall.
        let input = sine(55.0, 0.05, 48_000);
        let out = warm(&input);
        let change = rms_db(&out[24_000..]) - rms_db(&input[24_000..]);
        assert!(change.abs() < 0.5, "{change} dB");
    }
}
