//! Offline sample-rate conversion with a Blackman-windowed sinc (e.g. 44.1 → 48 kHz backing tracks).

use std::f64::consts::PI;

/// Zero crossings on each side of the kernel.
const HALF_WIDTH: f64 = 16.0;
/// Cutoff relative to the lower Nyquist frequency, leaves room for the transition band.
const CUTOFF: f64 = 0.94;

/// Resamples `input` from `from_hz` to `to_hz`; returns `input` unchanged if the rates match.
pub fn resample(input: &[f32], from_hz: u32, to_hz: u32) -> Vec<f32> {
    if from_hz == to_hz || from_hz == 0 || to_hz == 0 || input.is_empty() {
        return input.to_vec();
    }
    let ratio = f64::from(to_hz) / f64::from(from_hz);
    let cutoff = CUTOFF * ratio.min(1.0);
    let reach = HALF_WIDTH / cutoff;
    let out_len = (input.len() as f64 * ratio).round() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let center = i as f64 / ratio;
        let first = (center - reach).ceil().max(0.0) as usize;
        let last = ((center + reach).floor() as usize).min(input.len() - 1);
        let mut sum = 0.0;
        let mut weight = 0.0;
        for (j, sample) in input.iter().enumerate().take(last + 1).skip(first) {
            let k = kernel(j as f64 - center, cutoff, reach);
            sum += f64::from(*sample) * k;
            weight += k;
        }
        // Normalizing by the kernel sum keeps unity gain at every fractional position.
        out.push(if weight.abs() > 1e-9 {
            (sum / weight) as f32
        } else {
            0.0
        });
    }
    out
}

fn kernel(x: f64, cutoff: f64, reach: f64) -> f64 {
    let sinc = if x.abs() < 1e-12 {
        1.0
    } else {
        (PI * cutoff * x).sin() / (PI * x)
    };
    let t = (x / reach + 1.0) / 2.0;
    let window = 0.42 - 0.5 * (2.0 * PI * t).cos() + 0.08 * (4.0 * PI * t).cos();
    sinc * window
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::yin::Yin;

    #[test]
    fn keeps_pitch_and_length() {
        let sine: Vec<f32> = (0..44_100)
            .map(|i| (2.0 * PI * 110.0 * i as f64 / 44_100.0).sin() as f32 * 0.5)
            .collect();
        let out = resample(&sine, 44_100, 48_000);
        assert_eq!(out.len(), 48_000);
        let mut yin = Yin::new(48_000.0, 50.0, 400.0, 0.04);
        let hz = yin.estimate(&out[10_000..]).unwrap().hz;
        assert!((1200.0 * (hz / 110.0).log2()).abs() < 1.0);
        let peak = out[1000..47_000].iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!((peak - 0.5).abs() < 0.01, "peak {peak}");
        let down = resample(&out, 48_000, 44_100);
        assert_eq!(down.len(), 44_100);
        let error = down[2000..42_000]
            .iter()
            .zip(&sine[2000..42_000])
            .fold(0.0f32, |m, (a, b)| m.max((a - b).abs()));
        assert!(error < 0.01, "round trip error {error}");
    }
}
