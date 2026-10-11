//! RBJ biquad filters (Audio EQ Cookbook), direct form I.

use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterKind {
    LowPass,
    HighPass,
}

/// Equaliser bands (RBJ cookbook, shelf slope 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EqKind {
    LowShelf,
    Peak,
    HighShelf,
}

/// Bandwidth of the peaking band: a broad bell, about two octaves.
const PEAK_Q: f64 = 0.7;

#[derive(Debug, Clone)]
pub struct Biquad {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl Biquad {
    /// Butterworth-like filter (Q = 0.707) at `cutoff_hz`.
    pub fn new(kind: FilterKind, sample_rate: f64, cutoff_hz: f64) -> Biquad {
        let q = std::f64::consts::FRAC_1_SQRT_2;
        let w0 = 2.0 * PI * (cutoff_hz / sample_rate).clamp(1e-6, 0.49);
        let alpha = w0.sin() / (2.0 * q);
        let cos = w0.cos();
        let (b0, b1, b2) = match kind {
            FilterKind::LowPass => ((1.0 - cos) / 2.0, 1.0 - cos, (1.0 - cos) / 2.0),
            FilterKind::HighPass => ((1.0 + cos) / 2.0, -(1.0 + cos), (1.0 + cos) / 2.0),
        };
        let a0 = 1.0 + alpha;
        Biquad {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: -2.0 * cos / a0,
            a2: (1.0 - alpha) / a0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    /// Equaliser band at `hz` boosting (or cutting, if negative) by `gain_db`.
    /// Example: `eq(EqKind::LowShelf, 48_000.0, 100.0, 6.0)` lifts everything below 100 Hz by 6 dB.
    pub fn eq(kind: EqKind, sample_rate: f64, hz: f64, gain_db: f64) -> Biquad {
        let mut filter = Biquad::new(FilterKind::LowPass, sample_rate, hz);
        filter.retune(kind, sample_rate, hz, gain_db);
        filter
    }

    /// New equaliser coefficients; the state stays, so a change does not click.
    pub fn retune(&mut self, kind: EqKind, sample_rate: f64, hz: f64, gain_db: f64) {
        let a = 10f64.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * (hz / sample_rate).clamp(1e-6, 0.49);
        let (sin, cos) = w0.sin_cos();
        let q = match kind {
            EqKind::Peak => PEAK_Q,
            EqKind::LowShelf | EqKind::HighShelf => std::f64::consts::FRAC_1_SQRT_2,
        };
        let alpha = sin / (2.0 * q);
        let root = 2.0 * a.sqrt() * alpha;
        let (b0, b1, b2, a0, a1, a2) = match kind {
            EqKind::Peak => (
                1.0 + alpha * a,
                -2.0 * cos,
                1.0 - alpha * a,
                1.0 + alpha / a,
                -2.0 * cos,
                1.0 - alpha / a,
            ),
            EqKind::LowShelf => (
                a * ((a + 1.0) - (a - 1.0) * cos + root),
                2.0 * a * ((a - 1.0) - (a + 1.0) * cos),
                a * ((a + 1.0) - (a - 1.0) * cos - root),
                (a + 1.0) + (a - 1.0) * cos + root,
                -2.0 * ((a - 1.0) + (a + 1.0) * cos),
                (a + 1.0) + (a - 1.0) * cos - root,
            ),
            EqKind::HighShelf => (
                a * ((a + 1.0) + (a - 1.0) * cos + root),
                -2.0 * a * ((a - 1.0) + (a + 1.0) * cos),
                a * ((a + 1.0) + (a - 1.0) * cos - root),
                (a + 1.0) - (a - 1.0) * cos + root,
                2.0 * ((a - 1.0) - (a + 1.0) * cos),
                (a + 1.0) - (a - 1.0) * cos - root,
            ),
        };
        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = a1 / a0;
        self.a2 = a2 / a0;
    }

    /// Gain in dB at `hz`, from the coefficients (for tests and displays).
    pub fn response_db(&self, hz: f64, sample_rate: f64) -> f64 {
        let w = 2.0 * PI * hz / sample_rate;
        let (s1, c1) = w.sin_cos();
        let (s2, c2) = (2.0 * w).sin_cos();
        let num = (self.b0 + self.b1 * c1 + self.b2 * c2).hypot(self.b1 * s1 + self.b2 * s2);
        let den = (1.0 + self.a1 * c1 + self.a2 * c2).hypot(self.a1 * s1 + self.a2 * s2);
        20.0 * (num / den).log10()
    }

    pub fn process(&mut self, x: f32) -> f32 {
        let x = f64::from(x);
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y as f32
    }

    pub fn process_block(&mut self, samples: &mut [f32]) {
        for s in samples.iter_mut() {
            *s = self.process(*s);
        }
    }

    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::rms_db;

    fn sine(hz: f64, sr: f64, n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| (2.0 * PI * hz * i as f64 / sr).sin() as f32)
            .collect()
    }

    #[test]
    fn highpass_attenuates_low() {
        let mut f = Biquad::new(FilterKind::HighPass, 48_000.0, 1000.0);
        let mut low = sine(50.0, 48_000.0, 48_000);
        f.process_block(&mut low);
        assert!(rms_db(&low[24_000..]) < -40.0);
    }

    #[test]
    fn eq_bands_hit_their_gain() {
        let sr = 48_000.0;
        let low = Biquad::eq(EqKind::LowShelf, sr, 100.0, 6.0);
        assert!((low.response_db(20.0, sr) - 6.0).abs() < 0.3);
        assert!(low.response_db(5000.0, sr).abs() < 0.1);

        let peak = Biquad::eq(EqKind::Peak, sr, 800.0, -4.0);
        assert!((peak.response_db(800.0, sr) + 4.0).abs() < 0.01);
        assert!(peak.response_db(40.0, sr).abs() < 0.3);

        let high = Biquad::eq(EqKind::HighShelf, sr, 4000.0, 3.0);
        assert!((high.response_db(18_000.0, sr) - 3.0).abs() < 0.3);
        assert!(high.response_db(60.0, sr).abs() < 0.1);
    }

    #[test]
    fn flat_eq_passes_unchanged() {
        let mut f = Biquad::eq(EqKind::Peak, 48_000.0, 800.0, 0.0);
        let input = sine(220.0, 48_000.0, 4800);
        let mut out = input.clone();
        f.process_block(&mut out);
        let max_diff = input
            .iter()
            .zip(&out)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f32::max);
        assert!(max_diff < 1e-5);
    }

    #[test]
    fn lowpass_passes_low() {
        let mut f = Biquad::new(FilterKind::LowPass, 48_000.0, 1000.0);
        let mut low = sine(50.0, 48_000.0, 48_000);
        f.process_block(&mut low);
        assert!((rms_db(&low[24_000..]) + 3.01).abs() < 0.2);
    }
}
