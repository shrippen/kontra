//! RBJ biquad filters (Audio EQ Cookbook), direct form I.

use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterKind {
    LowPass,
    HighPass,
}

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
    fn lowpass_passes_low() {
        let mut f = Biquad::new(FilterKind::LowPass, 48_000.0, 1000.0);
        let mut low = sine(50.0, 48_000.0, 48_000);
        f.process_block(&mut low);
        assert!((rms_db(&low[24_000..]) + 3.01).abs() < 0.2);
    }
}
