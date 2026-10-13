//! Mains hum removal: narrow notches on the 50 and 60 Hz grids and their strongest harmonics.
//! A laptop's combo jack picks up hum from the charger (J-A2); it beats with low notes and pulls
//! pitch estimates onto 50 Hz.

use std::f64::consts::TAU;

/// Mains hum (50 and 60 Hz grids) and its strongest harmonics.
const HUM_HZ: [f64; 6] = [50.0, 60.0, 100.0, 120.0, 150.0, 180.0];

/// All hum notches in series, each `width_hz` wide (−3 dB). The width trades how close a note
/// may lie to the hum against how long an attack rings in the notch (`1 / (π · width_hz)`).
#[derive(Debug, Clone, Copy)]
pub(crate) struct HumFilter {
    notches: [Notch; HUM_HZ.len()],
}

impl HumFilter {
    pub(crate) fn new(sample_rate: f64, width_hz: f64) -> HumFilter {
        HumFilter {
            notches: HUM_HZ.map(|hz| Notch::new(sample_rate, hz, width_hz)),
        }
    }

    pub(crate) fn process(&mut self, x: f32) -> f32 {
        self.notches.iter_mut().fold(x, |y, notch| notch.process(y))
    }
}

/// RBJ notch biquad (Audio EQ Cookbook), direct form I.
///
/// A linear filter does not change the frequency of a (decaying) tone; only the onset leaves a
/// short ringing at the notch frequency.
#[derive(Debug, Clone, Copy)]
struct Notch {
    b1: f64,
    a0: f64,
    a2: f64,
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl Notch {
    fn new(sample_rate: f64, hz: f64, width_hz: f64) -> Notch {
        let w0 = TAU * hz / sample_rate;
        let alpha = w0.sin() * width_hz / (2.0 * hz);
        let a0 = 1.0 + alpha;
        Notch {
            b1: -2.0 * w0.cos() / a0,
            a0,
            a2: (1.0 - alpha) / a0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    /// y = b0·x + b1·x1 + b2·x2 − a1·y1 − a2·y2; for a notch b0 = b2 = 1/a0 and a1 = b1.
    fn process(&mut self, x: f32) -> f32 {
        let x = f64::from(x);
        let y = (x + self.x2) / self.a0 + self.b1 * (self.x1 - self.y1) - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y as f32
    }
}
