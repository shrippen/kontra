//! Deterministic bass-like test signals and input-path simulations.
//!
//! Used by tests, by the synthetic corpus (`kontra korpus`) and as the reference bass the player
//! can listen to (LH-WIE-03). Additive synthesis of a stiff string: partial n sits at
//! `n·f0·√(1 + B·n²)`, decays faster the higher it is, and the attack carries a short noise burst
//! like a finger or pick hitting the string.

use crate::biquad::{Biquad, FilterKind};
use std::f64::consts::PI;

/// Inharmonicity of a bass string (B in the stiff-string formula).
const INHARMONICITY: f64 = 0.000_08;
const ATTACK_NOISE_S: f64 = 0.004;

/// Timbre of the synthetic bass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToneShape {
    /// Round-wound strings, fingerstyle: many partials, strong 2nd harmonic.
    Rich,
    /// Flat-wound or tone knob down: few partials.
    Dull,
}

/// One note for `render`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SynthNote {
    pub onset_s: f64,
    pub hz: f64,
    pub duration_s: f64,
    /// 0..=1.
    pub velocity: f32,
}

/// Small deterministic noise source (xorshift64*).
#[derive(Debug, Clone)]
pub struct Noise(u64);

impl Noise {
    pub fn new(seed: u64) -> Noise {
        Noise(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    /// Uniform in [-1, 1).
    pub fn sample(&mut self) -> f32 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        let v = self.0.wrapping_mul(0x2545_F491_4F6C_DD1D);
        ((v >> 40) as f32 / (1u64 << 23) as f32) - 1.0
    }
}

/// A single tone of `duration_s` starting at sample 0.
pub fn bass_tone(
    hz: f64,
    sample_rate: f64,
    duration_s: f64,
    shape: ToneShape,
    seed: u64,
) -> Vec<f32> {
    let note = SynthNote {
        onset_s: 0.0,
        hz,
        duration_s,
        velocity: 0.8,
    };
    render(&[note], sample_rate, duration_s, shape, seed)
}

/// Renders notes into one buffer of `total_s`. A new note cuts the ringing of the previous one
/// (a bassist stops the old string), with a 15 ms fade.
pub fn render(
    notes: &[SynthNote],
    sample_rate: f64,
    total_s: f64,
    shape: ToneShape,
    seed: u64,
) -> Vec<f32> {
    let len = (total_s * sample_rate).ceil() as usize;
    let mut out = vec![0.0f32; len];
    let mut noise = Noise::new(seed);
    let mut sorted: Vec<&SynthNote> = notes.iter().collect();
    sorted.sort_by(|a, b| a.onset_s.total_cmp(&b.onset_s));
    for (i, note) in sorted.iter().enumerate() {
        let next = sorted.get(i + 1).map_or(f64::INFINITY, |n| n.onset_s);
        let end_s = (note.onset_s + note.duration_s).min(next);
        add_note(&mut out, note, end_s, sample_rate, shape, &mut noise);
    }
    out
}

fn add_note(
    out: &mut [f32],
    note: &SynthNote,
    end_s: f64,
    sr: f64,
    shape: ToneShape,
    noise: &mut Noise,
) {
    let start = (note.onset_s * sr).round() as usize;
    let end = ((end_s * sr).round() as usize).min(out.len());
    if start >= end {
        return;
    }
    let partials = partials(note.hz, sr, shape);
    // Muting a string takes a moment; a hard cut would click like a new attack.
    let fade = (0.015 * sr) as usize;
    let attack = (ATTACK_NOISE_S * sr) as usize;
    let mut burst = Biquad::new(FilterKind::HighPass, sr, 800.0);
    let gain = f64::from(note.velocity) * 0.5;
    for (i, slot) in out.iter_mut().enumerate().take(end).skip(start) {
        let t = (i - start) as f64 / sr;
        let mut v = 0.0;
        for p in &partials {
            v += p.amp * (-t / p.decay_s).exp() * (2.0 * PI * p.hz * t + p.phase).sin();
        }
        let mut sample = (v * gain) as f32;
        if i - start < attack {
            let env = 1.0 - (i - start) as f32 / attack as f32;
            sample += burst.process(noise.sample()) * env * note.velocity * 0.25;
        }
        let remaining = end - i;
        if remaining < fade {
            let x = remaining as f32 / fade as f32;
            sample *= 0.5 - 0.5 * (std::f32::consts::PI * x).cos();
        }
        *slot += sample;
    }
}

struct Partial {
    hz: f64,
    amp: f64,
    decay_s: f64,
    phase: f64,
}

fn partials(f0: f64, sr: f64, shape: ToneShape) -> Vec<Partial> {
    let (count, tilt) = match shape {
        ToneShape::Rich => (24, 1.1),
        ToneShape::Dull => (6, 1.8),
    };
    let mut list = Vec::new();
    for n in 1..=count {
        let n_f = f64::from(n);
        let hz = n_f * f0 * (1.0 + INHARMONICITY * n_f * n_f).sqrt();
        if hz > sr * 0.45 {
            break;
        }
        // Fingerstyle: the 2nd harmonic is often as strong as the fundamental.
        let boost = if n == 2 { 1.4 } else { 1.0 };
        list.push(Partial {
            hz,
            amp: boost / n_f.powf(tilt),
            decay_s: 2.5 / (1.0 + 0.35 * n_f),
            phase: 0.3 * n_f,
        });
    }
    list
}

/// Simulates a bass plugged into a laptop's combo jack: wrong impedance (treble loss),
/// low level, mains hum and noise.
pub fn laptop_jack(signal: &[f32], sample_rate: f64, seed: u64) -> Vec<f32> {
    let mut lowpass = Biquad::new(FilterKind::LowPass, sample_rate, 2500.0);
    let mut noise = Noise::new(seed);
    let gain = 0.18; // about -15 dB
    signal
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let t = i as f64 / sample_rate;
            let hum = (0.001 * (2.0 * PI * 50.0 * t).sin() + 0.0005 * (2.0 * PI * 150.0 * t).sin())
                as f32;
            lowpass.process(*s) * gain + hum + noise.sample() * 0.0005
        })
        .collect()
}

/// Adds white noise at `level_db` dBFS (peak).
pub fn add_noise(signal: &mut [f32], level_db: f32, seed: u64) {
    let amp = 10f32.powf(level_db / 20.0);
    let mut noise = Noise::new(seed);
    for s in signal.iter_mut() {
        *s += noise.sample() * amp;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::{peak_db, rms_db};

    #[test]
    fn deterministic() {
        let a = bass_tone(41.2, 48_000.0, 0.2, ToneShape::Rich, 1);
        let b = bass_tone(41.2, 48_000.0, 0.2, ToneShape::Rich, 1);
        assert_eq!(a, b);
        assert!(peak_db(&a) < 0.0);
    }

    #[test]
    fn laptop_is_quieter() {
        let a = bass_tone(55.0, 48_000.0, 0.5, ToneShape::Rich, 1);
        let b = laptop_jack(&a, 48_000.0, 2);
        assert!(rms_db(&b) < rms_db(&a) - 10.0);
    }

    #[test]
    fn next_note_cuts_previous() {
        let notes = [
            SynthNote {
                onset_s: 0.0,
                hz: 41.2,
                duration_s: 2.0,
                velocity: 0.8,
            },
            SynthNote {
                onset_s: 0.5,
                hz: 55.0,
                duration_s: 0.1,
                velocity: 0.8,
            },
        ];
        let out = render(&notes, 48_000.0, 1.0, ToneShape::Rich, 1);
        assert!(rms_db(&out[36_000..]) < -100.0);
    }
}
