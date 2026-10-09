//! Fine frequency of the fundamental from the phase advance between two windowed single-bin
//! DFTs (phase-vocoder estimate). Stiff strings have sharp upper partials that pull a period
//! estimate up by several cents; a Hann window of two periods puts the 2nd partial on its first
//! zero, so only the fundamental is measured. Needed for fretless intonation (LH-BEW-05).

use std::f64::consts::{PI, TAU};

/// Refines `hz` (a coarse estimate, e.g. from YIN) using `samples`; `None` if too short.
pub fn refine_hz(samples: &[f32], sample_rate: f64, hz: f64) -> Option<f64> {
    if hz <= 0.0 || !hz.is_finite() {
        return None;
    }
    let period = sample_rate / hz;
    let window = (2.0 * period).round() as usize;
    let shift = samples.len().checked_sub(window)?;
    if (shift as f64) < period * 0.25 {
        return None;
    }
    let first = bin(&samples[..window], sample_rate, hz);
    let second = bin(&samples[shift..shift + window], sample_rate, hz);
    let expected = TAU * hz * shift as f64 / sample_rate;
    let deviation = wrap(second.1.atan2(second.0) - first.1.atan2(first.0) - expected);
    let refined = hz + deviation * sample_rate / (TAU * shift as f64);
    // Accept only small corrections; a large one means the coarse value was wrong.
    ((refined / hz).log2().abs() < 0.04).then_some(refined)
}

/// Hann-windowed DFT of `samples` at `hz`: (re, im).
fn bin(samples: &[f32], sample_rate: f64, hz: f64) -> (f64, f64) {
    let n = samples.len() as f64;
    let step = TAU * hz / sample_rate;
    let mut re = 0.0;
    let mut im = 0.0;
    for (i, s) in samples.iter().enumerate() {
        let w = 0.5 - 0.5 * (TAU * i as f64 / n).cos();
        let x = f64::from(*s) * w;
        let angle = step * i as f64;
        re += x * angle.cos();
        im -= x * angle.sin();
    }
    (re, im)
}

fn wrap(angle: f64) -> f64 {
    (angle + PI).rem_euclid(TAU) - PI
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::{bass_tone, ToneShape};
    use kontra_model::{cents_between, Pitch, A4_HZ};

    #[test]
    fn removes_stiff_string_bias() {
        let sr = 48_000.0;
        for name in ["B0", "E1", "A1", "D2", "G2", "C3"] {
            let hz = Pitch::parse(name).unwrap().hz(A4_HZ) * 2f64.powf(17.0 / 1200.0);
            let tone = bass_tone(hz, sr, 0.3, ToneShape::Rich, 4);
            let span = (2.6 * sr / hz) as usize;
            let coarse = hz * 2f64.powf(8.0 / 1200.0);
            let refined = refine_hz(&tone[600..600 + span], sr, coarse).unwrap();
            assert!(
                cents_between(hz, refined).abs() < 2.0,
                "{name}: {}",
                cents_between(hz, refined)
            );
        }
    }
}
