//! Chromatic tuner (LH-TUN-01): a reading every 30 ms from the last ~0.25 s of audio.

use crate::hum::HumFilter;
use crate::level::rms_db;
use crate::yin::Yin;
use kontra_model::{Pitch, A4_HZ};

const INTERVAL_S: f64 = 0.03;
/// Below this level the tuner shows nothing.
const GATE_DB: f32 = -55.0;
const MIN_CONFIDENCE: f32 = 0.8;
/// Width of each hum notch. Narrow keeps notes next to it (G2 at 98 Hz keeps 80 % of its level);
/// wide lets the ringing a new note leaves in a notch die out fast (0.1 s). On the open strings
/// B0–C3, ±30 cent, 3 Hz kept every reading 0.5 s after the attack within 1 cent; 1 Hz did not.
const HUM_WIDTH_HZ: f64 = 3.0;
/// YIN integration window. Pitch noise falls with window^1.5: at 54 ms (two periods of a low E)
/// a fading note through a laptop jack (SNR ≈ 17 dB) jittered by ±3 cents, at 100 ms by < 1.
const WINDOW_S: f64 = 0.1;

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
    hum: HumFilter,
    a4_hz: f64,
    buffer: Vec<f32>,
    filled: usize,
    since: usize,
    interval: usize,
}

impl Tuner {
    pub fn new(sample_rate: f64, min_hz: f64, max_hz: f64) -> Tuner {
        let yin = Yin::new(sample_rate, min_hz, max_hz, WINDOW_S.max(1.0 / min_hz));
        let len = 2 * yin.needed();
        Tuner {
            yin,
            hum: HumFilter::new(sample_rate, HUM_WIDTH_HZ),
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

        // Hum removed before estimation: 50 Hz hum beats with a 41 Hz low E and swings the
        // reading by ±40 cents (laptop jack, J-A2). Every sample passes the notches so their
        // state stays continuous; the newest `take` are kept.
        let skip = samples.len() - take;
        for (i, x) in samples.iter().enumerate() {
            let y = self.hum.process(*x);
            if i >= skip {
                self.buffer[keep + i - skip] = y;
            }
        }
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
    use crate::synth::{bass_tone, laptop_jack, render, SynthNote, ToneShape};

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

    #[test]
    fn laptop_jack_low_e_is_steady() {
        // J-A2 (P1): E1 −30 ct through the laptop jack. Mains hum (50 Hz) beats with the
        // 40.5 Hz fundamental; the reading must stay on E1 ≈ −30 ct (tune up), without jumps.
        let sr = 48_000.0;
        let onset_s = 1.0;
        let settle_s = 0.5;
        let target = Pitch::parse("E1").unwrap().hz(A4_HZ) * 2f64.powf(-30.0 / 1200.0);
        for seed in [1, 2, 3] {
            let note = SynthNote {
                onset_s,
                hz: target,
                duration_s: 6.0,
                velocity: 0.75,
            };
            let clean = render(&[note], sr, onset_s + 6.0, ToneShape::Rich, seed);
            let signal = laptop_jack(&clean, sr, seed);
            let mut tuner = Tuner::new(sr, 36.7, 420.0);
            let mut readings = Vec::new();
            for (i, block) in signal.chunks(512).enumerate() {
                let Some(r) = tuner.push(block) else {
                    continue;
                };
                let t = ((i + 1) * 512) as f64 / sr;
                if t >= onset_s + settle_s {
                    readings.push((t, r));
                }
            }
            let last = readings.last().map_or(0.0, |(t, _)| *t);
            assert!(
                last > onset_s + 4.0,
                "seed {seed}: readings end at {last:.2} s"
            );
            let mut previous: Option<f64> = None;
            for (t, r) in &readings {
                assert_eq!(r.pitch.to_string(), "E1", "seed {seed}, {t:.2} s: {r:?}");
                assert!(r.cents < 0.0, "seed {seed}, {t:.2} s: tune up, {}", r.cents);
                assert!(
                    (r.cents + 30.0).abs() <= 3.0,
                    "seed {seed}, {t:.2} s: {}",
                    r.cents
                );
                if let Some(p) = previous {
                    assert!(
                        (r.cents - p).abs() <= 5.0,
                        "seed {seed}, {t:.2} s: jump {p:.1} -> {:.1}",
                        r.cents
                    );
                }
                previous = Some(r.cents);
            }
        }
    }
}
