//! Effects behind the tone shaping of the monitored bass: overdrive, a mono-to-stereo widener
//! and a limiter.
//!
//! ```text
//! mono ─► Overdrive ─► Widener ─┬─ L ─► Limiter (linked) ─► L
//!                               └─ R ─►                 ─► R
//!
//! Widener: L = x + s, R = x − s, s = width · highpass(x, 12 ms ago)
//!          the low end stays in the middle, L + R is the dry signal (mono-safe)
//! ```
//!
//! Each runs in the audio callback: no allocation after `new`, one frame at a time.

use crate::biquad::{Biquad, FilterKind};

/// Highest device rate the widener's delay line is sized for.
const MAX_RATE_HZ: f64 = 192_000.0;
/// Delay of the widener's side signal: long enough to sound apart, short enough to stay one note.
const WIDEN_DELAY_S: f64 = 0.012;
/// Below this the widener leaves the bass in the middle (fundamentals stay centred).
const WIDEN_LOW_HZ: f64 = 250.0;
/// Side level at full width relative to the middle.
const WIDEN_SIDE: f32 = 0.5;

/// Overdrive gain at full drive (+26 dB) before the soft clip.
const DRIVE_GAIN: f32 = 20.0;
/// A peak at this level comes out of the overdrive at the same level, whatever the drive.
const DRIVE_REFERENCE: f32 = 0.25;
/// Tames the fizz of the clipped signal.
const DRIVE_TONE_HZ: f64 = 5000.0;
/// Below this drive the overdriven path fades in from the dry signal (no click when switched on).
const DRIVE_FADE: f32 = 0.1;

/// The limiter's ceiling, −1 dBFS: the monitored bass never reaches full scale.
pub const LIMIT_CEILING: f32 = 0.891;
/// How fast the limiter lets go after a peak.
const LIMIT_RELEASE_S: f64 = 0.080;

/// Overdrive: amplified, soft-clipped, then filtered. 0 passes the input unchanged.
#[derive(Debug, Clone)]
pub(crate) struct Overdrive {
    tone: Biquad,
}

impl Overdrive {
    pub(crate) fn new(sample_rate: f64) -> Overdrive {
        Overdrive {
            tone: Biquad::new(FilterKind::LowPass, sample_rate, DRIVE_TONE_HZ),
        }
    }

    /// One sample at `drive` 0..=1. The tone filter always runs, so its state is current when
    /// drive comes on.
    pub(crate) fn process(&mut self, x: f32, drive: f32) -> f32 {
        let gain = 1.0 + (DRIVE_GAIN - 1.0) * drive;
        let norm = DRIVE_REFERENCE / (gain * DRIVE_REFERENCE).tanh();
        let wet = self.tone.process((gain * x).tanh() * norm);
        if drive <= 0.0 {
            return x;
        }
        let mix = (drive / DRIVE_FADE).min(1.0);
        x + (wet - x) * mix
    }
}

/// Mono to stereo: a delayed, high-passed copy added to one side and taken from the other.
#[derive(Debug, Clone)]
pub(crate) struct Widener {
    line: Vec<f32>,
    at: usize,
    delay: usize,
    highpass: Biquad,
}

impl Widener {
    pub(crate) fn new(sample_rate: f64) -> Widener {
        let size = (MAX_RATE_HZ * WIDEN_DELAY_S) as usize + 1;
        let mut widener = Widener {
            line: vec![0.0; size],
            at: 0,
            delay: 0,
            highpass: Biquad::new(FilterKind::HighPass, sample_rate, WIDEN_LOW_HZ),
        };
        widener.retime(sample_rate);
        widener
    }

    /// A new device rate; keeps the delay line (no allocation).
    pub(crate) fn retime(&mut self, sample_rate: f64) {
        self.delay = ((sample_rate * WIDEN_DELAY_S) as usize).clamp(1, self.line.len() - 1);
        self.highpass = Biquad::new(FilterKind::HighPass, sample_rate, WIDEN_LOW_HZ);
    }

    /// One sample at `width` 0..=1 as (left, right); 0 is the input on both sides.
    pub(crate) fn process(&mut self, x: f32, width: f32) -> (f32, f32) {
        let len = self.line.len();
        self.line[self.at] = x;
        let delayed = self.line[(self.at + len - self.delay) % len];
        self.at = (self.at + 1) % len;
        let side = self.highpass.process(delayed) * WIDEN_SIDE * width;
        (x + side, x - side)
    }
}

/// Peak limiter, both sides linked: catches a peak at once, lets go over `LIMIT_RELEASE_S`.
/// Example: a slap peaking at 1.6 comes out at the ceiling, quiet notes pass unchanged.
#[derive(Debug, Clone)]
pub(crate) struct Limiter {
    gain: f32,
    release: f32,
}

impl Limiter {
    pub(crate) fn new(sample_rate: f64) -> Limiter {
        let mut limiter = Limiter {
            gain: 1.0,
            release: 0.0,
        };
        limiter.retime(sample_rate);
        limiter
    }

    pub(crate) fn retime(&mut self, sample_rate: f64) {
        self.release = (-1.0 / (LIMIT_RELEASE_S * sample_rate)).exp() as f32;
    }

    /// One frame; `amount` 0 (off) to 1 (on) fades between dry and limited.
    pub(crate) fn process(&mut self, (l, r): (f32, f32), amount: f32) -> (f32, f32) {
        let peak = l.abs().max(r.abs());
        let wanted = if peak > LIMIT_CEILING {
            LIMIT_CEILING / peak
        } else {
            1.0
        };
        self.gain = if wanted < self.gain {
            wanted
        } else {
            wanted + self.release * (self.gain - wanted)
        };
        let gain = 1.0 + (self.gain - 1.0) * amount;
        (l * gain, r * gain)
    }
}

/// The limiter on everything that leaves Kontra (LH-WIE-06): always on, follows the device's
/// rate. Example: backing track, click and bass together reach 1.4; the sum comes out at the
/// ceiling and the loud spot gets quieter for a moment instead of crackling.
#[derive(Debug, Clone)]
pub struct OutputLimiter {
    limiter: Limiter,
    sample_rate: f64,
}

impl OutputLimiter {
    pub fn new(sample_rate: f64) -> OutputLimiter {
        OutputLimiter {
            limiter: Limiter::new(sample_rate),
            sample_rate,
        }
    }

    /// One stereo frame at `sample_rate`; the output device's rate may change between streams.
    pub fn process(&mut self, frame: (f32, f32), sample_rate: f64) -> (f32, f32) {
        if sample_rate != self.sample_rate {
            self.sample_rate = sample_rate;
            self.limiter.retime(sample_rate);
        }
        self.limiter.process(frame, 1.0)
    }

    /// The gain of the last frame, 1 while nothing is limited: what else follows the output
    /// (the echo reference) scales with it.
    pub fn gain(&self) -> f32 {
        self.limiter.gain
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::rms_db;
    use std::f64::consts::PI;

    const SR: f64 = 48_000.0;

    fn sine(hz: f64, amplitude: f32, n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| amplitude * (2.0 * PI * hz * i as f64 / SR).sin() as f32)
            .collect()
    }

    #[test]
    fn limiter_holds_the_ceiling() {
        // A slap far over full scale: no sample above −1 dBFS.
        let mut limiter = Limiter::new(SR);
        let out: Vec<f32> = sine(55.0, 1.6, 9600)
            .iter()
            .map(|&x| limiter.process((x, x), 1.0).0)
            .collect();
        let peak = out.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(peak <= LIMIT_CEILING + 1e-6, "peak {peak}");
    }

    #[test]
    fn limiter_passes_quiet_notes() {
        let mut limiter = Limiter::new(SR);
        let input = sine(55.0, 0.5, 9600);
        let max_diff = input
            .iter()
            .map(|&x| (limiter.process((x, x), 1.0).0 - x).abs())
            .fold(0.0, f32::max);
        assert!(max_diff < 1e-6, "{max_diff}");
    }

    #[test]
    fn limiter_lets_go_after_a_peak() {
        // A peak, then a quiet note: back to unity within a few release times.
        let mut limiter = Limiter::new(SR);
        for &x in &sine(55.0, 1.6, 4800) {
            limiter.process((x, x), 1.0);
        }
        let quiet = sine(55.0, 0.3, 48_000);
        let out: Vec<f32> = quiet
            .iter()
            .map(|&x| limiter.process((x, x), 1.0).0)
            .collect();
        let change = rms_db(&out[24_000..]) - rms_db(&quiet[24_000..]);
        assert!(change.abs() < 0.1, "{change} dB");
    }

    #[test]
    fn output_limiter_follows_the_rate_and_tells_its_gain() {
        let mut limiter = OutputLimiter::new(SR);
        assert_eq!(limiter.process((0.5, -0.5), SR), (0.5, -0.5));
        assert_eq!(limiter.gain(), 1.0);
        // Twice the ceiling on one side: both sides halve, at another rate too.
        let (l, r) = limiter.process((2.0 * LIMIT_CEILING, 0.4), 96_000.0);
        assert!(
            (l - LIMIT_CEILING).abs() < 1e-6 && (r - 0.2).abs() < 1e-6,
            "{l} {r}"
        );
        assert!((limiter.gain() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn widener_keeps_the_sum_and_the_low_end() {
        let mut widener = Widener::new(SR);
        // 1 kHz spreads: the sides differ, their sum is the input.
        let input = sine(1000.0, 0.3, 9600);
        let out: Vec<(f32, f32)> = input.iter().map(|&x| widener.process(x, 1.0)).collect();
        let sum_error = input
            .iter()
            .zip(&out)
            .map(|(x, (l, r))| (l + r - 2.0 * x).abs())
            .fold(0.0, f32::max);
        assert!(sum_error < 1e-6, "{sum_error}");
        let side: Vec<f32> = out[4800..].iter().map(|(l, r)| l - r).collect();
        assert!(rms_db(&side) - rms_db(&input[4800..]) > -12.0);

        // 41 Hz (low E) stays in the middle.
        let mut widener = Widener::new(SR);
        let low = sine(41.2, 0.3, 48_000);
        let out: Vec<f32> = low
            .iter()
            .map(|&x| {
                let (l, r) = widener.process(x, 1.0);
                l - r
            })
            .collect();
        assert!(rms_db(&out[24_000..]) - rms_db(&low[24_000..]) < -20.0);
    }

    #[test]
    fn widener_at_zero_is_mono() {
        let mut widener = Widener::new(SR);
        for &x in &sine(1000.0, 0.3, 4800) {
            let (l, r) = widener.process(x, 0.0);
            assert_eq!((l, r), (x, x));
        }
    }

    #[test]
    fn widener_survives_a_rate_change() {
        let mut widener = Widener::new(SR);
        widener.retime(MAX_RATE_HZ);
        widener.retime(8000.0);
        for &x in &sine(1000.0, 0.3, 4800) {
            let (l, r) = widener.process(x, 1.0);
            assert!(l.is_finite() && r.is_finite());
        }
    }

    #[test]
    fn overdrive_clips_loud_notes_and_keeps_the_level() {
        let mut drive = Overdrive::new(SR);
        // A loud note is squashed (louder input, about the same output) ...
        let loud = sine(110.0, 0.8, 48_000);
        let soft = sine(110.0, DRIVE_REFERENCE, 48_000);
        let out_loud: Vec<f32> = loud.iter().map(|&x| drive.process(x, 1.0)).collect();
        let out_soft: Vec<f32> = soft.iter().map(|&x| drive.process(x, 1.0)).collect();
        let range_in = rms_db(&loud) - rms_db(&soft);
        let range_out = rms_db(&out_loud[24_000..]) - rms_db(&out_soft[24_000..]);
        assert!(range_out < range_in - 6.0, "{range_in} → {range_out}");
        // ... and never exceeds full scale.
        let peak = out_loud.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(peak < 1.0, "peak {peak}");
    }

    #[test]
    fn overdrive_at_zero_passes() {
        let mut drive = Overdrive::new(SR);
        for &x in &sine(110.0, 0.5, 4800) {
            assert_eq!(drive.process(x, 0.0), x);
        }
    }
}
