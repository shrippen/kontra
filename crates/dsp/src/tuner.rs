//! Chromatic tuner (LH-TUN-01): a reading every 30 ms from the last ~150 ms of audio.

use crate::level::rms_db;
use crate::yin::Yin;
use kontra_model::{Pitch, A4_HZ};

const INTERVAL_S: f64 = 0.03;
/// Below this level the tuner shows nothing.
const GATE_DB: f32 = -55.0;
const MIN_CONFIDENCE: f32 = 0.8;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TunerReading {
    pub hz: f64,
    pub pitch: Pitch,
    /// Deviation from `pitch` in cents (positive = sharp).
    pub cents: f64,
    pub confidence: f32,
}

#[derive(Debug)]
pub struct Tuner {
    yin: Yin,
    a4_hz: f64,
    buffer: Vec<f32>,
    filled: usize,
    since: usize,
    interval: usize,
}

impl Tuner {
    pub fn new(sample_rate: f64, min_hz: f64, max_hz: f64) -> Tuner {
        let yin = Yin::new(sample_rate, min_hz, max_hz, 1.0 / min_hz);
        let len = 2 * yin.needed();
        Tuner {
            yin,
            a4_hz: A4_HZ,
            buffer: vec![0.0; len],
            filled: 0,
            since: 0,
            interval: (INTERVAL_S * sample_rate) as usize,
        }
    }

    pub fn set_reference(&mut self, a4_hz: f64) {
        self.a4_hz = a4_hz;
    }

    /// Feeds samples; returns a reading when one is due and the signal is clear.
    pub fn push(&mut self, samples: &[f32]) -> Option<TunerReading> {
        let take = samples.len().min(self.buffer.len());
        let keep = self.buffer.len() - take;
        self.buffer.copy_within(take.., 0);
        self.buffer[keep..].copy_from_slice(&samples[samples.len() - take..]);
        self.filled = (self.filled + take).min(self.buffer.len());
        self.since += samples.len();
        if self.since < self.interval || self.filled < self.buffer.len() {
            return None;
        }
        self.since = 0;
        if rms_db(&self.buffer[self.buffer.len() / 2..]) < GATE_DB {
            return None;
        }
        let estimate = self.yin.estimate_fundamental(&self.buffer)?;
        if estimate.confidence < MIN_CONFIDENCE {
            return None;
        }
        let (pitch, cents) = Pitch::nearest(estimate.hz, self.a4_hz)?;
        Some(TunerReading {
            hz: estimate.hz,
            pitch,
            cents,
            confidence: estimate.confidence,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::{bass_tone, ToneShape};

    #[test]
    fn reads_detuned_low_b_within_one_cent() {
        let sr = 48_000.0;
        let target = Pitch::parse("B0").unwrap().hz(A4_HZ) * 2f64.powf(-30.0 / 1200.0);
        let tone = bass_tone(target, sr, 1.0, ToneShape::Rich, 9);
        let mut tuner = Tuner::new(sr, 27.0, 420.0);
        let mut last = None;
        for block in tone.chunks(512) {
            if let Some(r) = tuner.push(block) {
                last = Some(r);
            }
        }
        let r = last.unwrap();
        assert_eq!(r.pitch.to_string(), "B0");
        assert!((r.cents + 30.0).abs() < 1.0, "{}", r.cents);
    }
}
