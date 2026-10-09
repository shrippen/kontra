//! Beat tracking for backing tracks (LH-BIB-06): where the beats of a recording fall, so an
//! arrangement's bars can follow the band instead of a constant tempo.
//!
//! ```text
//! samples ─► spectral flux (onset strength) ─► tempo (autocorrelation, prior ~120 BPM)
//!                                         └──► beats (dynamic programming, Ellis 2007)
//! ```
//!
//! Offline only: the whole file is analysed once when it is coupled to a piece.

use rustfft::num_complex::Complex32;
use rustfft::FftPlanner;
use std::f32::consts::PI;

const WINDOW: usize = 1024;
const HOP: usize = 256;
const MIN_BPM: f64 = 50.0;
const MAX_BPM: f64 = 220.0;
/// Tempo prior: log-normal around this, one octave wide, to pick 100 rather than 200 BPM.
const PRIOR_BPM: f64 = 120.0;
const PRIOR_OCTAVES: f64 = 1.0;
/// How strongly the tracker keeps the found tempo between beats.
const TIGHTNESS: f64 = 100.0;

/// Tempo and beat times (seconds) of a recording.
#[derive(Debug, Clone, PartialEq)]
pub struct BeatGrid {
    pub bpm: f64,
    pub beats: Vec<f64>,
}

/// Finds the beats of `samples` (mono) at `rate` Hz. Silence or very short input gives no beats.
pub fn track_beats(samples: &[f32], rate: u32) -> BeatGrid {
    let strength = onset_strength(samples);
    let frame_s = HOP as f64 / f64::from(rate.max(1));
    let Some(period) = tempo_period(&strength, frame_s) else {
        return BeatGrid {
            bpm: 0.0,
            beats: Vec::new(),
        };
    };
    let frames = dynamic_beats(&strength, period);
    // Flux at frame i compares windows starting at i-1 and i: the attack sits near the start of
    // window i plus a little.
    let offset_s = (WINDOW as f64 / 4.0) / f64::from(rate.max(1));
    let beats = frames
        .iter()
        .map(|f| *f as f64 * frame_s + offset_s)
        .collect();
    BeatGrid {
        bpm: 60.0 / (period * frame_s),
        beats,
    }
}

/// Half-wave rectified spectral flux of log magnitudes, minus a one-second moving mean.
fn onset_strength(samples: &[f32]) -> Vec<f64> {
    if samples.len() < WINDOW * 2 {
        return Vec::new();
    }
    let fft = FftPlanner::<f32>::new().plan_fft_forward(WINDOW);
    let window: Vec<f32> = (0..WINDOW)
        .map(|i| 0.5 - 0.5 * (2.0 * PI * i as f32 / WINDOW as f32).cos())
        .collect();
    let bins = WINDOW / 2;
    let mut previous = vec![0.0f32; bins];
    let mut buffer = vec![Complex32::new(0.0, 0.0); WINDOW];
    let frames = (samples.len() - WINDOW) / HOP;
    let mut flux = Vec::with_capacity(frames);
    for f in 0..frames {
        let start = f * HOP;
        for (i, b) in buffer.iter_mut().enumerate() {
            *b = Complex32::new(samples[start + i] * window[i], 0.0);
        }
        fft.process(&mut buffer);
        let mut sum = 0.0f64;
        for (bin, prev) in previous.iter_mut().enumerate() {
            let magnitude = (1.0 + 100.0 * buffer[bin + 1].norm()).ln();
            if f > 0 {
                sum += f64::from((magnitude - *prev).max(0.0));
            }
            *prev = magnitude;
        }
        flux.push(sum);
    }
    // Remove the slowly changing part (crescendos), keep the attacks.
    let half = 40; // about half a second at 44.1 kHz
    let mut out = Vec::with_capacity(flux.len());
    for i in 0..flux.len() {
        let from = i.saturating_sub(half);
        let to = (i + half + 1).min(flux.len());
        let mean = flux[from..to].iter().sum::<f64>() / (to - from) as f64;
        out.push((flux[i] - mean).max(0.0));
    }
    let deviation = (out.iter().map(|v| v * v).sum::<f64>() / out.len().max(1) as f64).sqrt();
    if deviation > 0.0 {
        out.iter_mut().for_each(|v| *v /= deviation);
    }
    out
}

/// Beat period in frames (fractional) from the autocorrelation, weighted by the tempo prior.
fn tempo_period(strength: &[f64], frame_s: f64) -> Option<f64> {
    let min_lag = (60.0 / MAX_BPM / frame_s).floor().max(1.0) as usize;
    let max_lag = (60.0 / MIN_BPM / frame_s).ceil() as usize;
    if strength.len() < max_lag * 4 || strength.iter().all(|v| *v == 0.0) {
        return None;
    }
    let correlation = |lag: usize| -> f64 {
        strength
            .iter()
            .zip(&strength[lag..])
            .map(|(a, b)| a * b)
            .sum::<f64>()
            / (strength.len() - lag) as f64
    };
    let raw: Vec<f64> = (0..=max_lag + 1)
        .map(|lag| if lag < min_lag { 0.0 } else { correlation(lag) })
        .collect();
    let weighted = |lag: usize| {
        let bpm = 60.0 / (lag as f64 * frame_s);
        let octaves = (bpm / PRIOR_BPM).log2() / PRIOR_OCTAVES;
        raw[lag] * (-0.5 * octaves * octaves).exp()
    };
    let best = (min_lag..=max_lag).max_by(|a, b| weighted(*a).total_cmp(&weighted(*b)))?;
    if raw[best] <= 0.0 {
        return None;
    }
    // Parabolic interpolation for a period between frames.
    let (l, c, r) = (raw[best - 1], raw[best], raw[best + 1]);
    let denominator = l - 2.0 * c + r;
    let shift = if denominator.abs() > 1e-12 {
        (0.5 * (l - r) / denominator).clamp(-0.5, 0.5)
    } else {
        0.0
    };
    Some(best as f64 + shift)
}

/// Ellis' dynamic programming: each beat follows the best previous one about a period earlier.
fn dynamic_beats(strength: &[f64], period: f64) -> Vec<usize> {
    let n = strength.len();
    let mut score = vec![0.0f64; n];
    let mut from = vec![usize::MAX; n];
    let lo = (period / 2.0).round().max(1.0) as usize;
    let hi = (period * 2.0).round() as usize;
    for t in 0..n {
        let mut best = 0.0;
        let mut best_p = usize::MAX;
        if t >= lo {
            for p in t.saturating_sub(hi)..=t - lo {
                let gap = (t - p) as f64 / period;
                let s = score[p] - TIGHTNESS * gap.ln().powi(2);
                if best_p == usize::MAX || s > best {
                    best = s;
                    best_p = p;
                }
            }
        }
        score[t] = strength[t]
            + if best_p == usize::MAX {
                0.0
            } else {
                best.max(0.0)
            };
        from[t] = if best > 0.0 { best_p } else { usize::MAX };
    }
    // Start from the best-scoring frame in the last period, then walk back.
    let tail = n.saturating_sub(period.ceil() as usize);
    let Some(mut t) = (tail..n).max_by(|a, b| score[*a].total_cmp(&score[*b])) else {
        return Vec::new();
    };
    let mut beats = vec![t];
    while from[t] != usize::MAX {
        t = from[t];
        beats.push(t);
    }
    beats.reverse();
    beats
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 44_100;

    /// A drum-like click: a short noise burst with a fast decay, plus a low thump.
    fn click_track(beats: &[f64], seconds: f64) -> Vec<f32> {
        let mut out = vec![0.0f32; (seconds * f64::from(RATE)) as usize];
        let mut noise = 12345u32;
        for &b in beats {
            let start = (b * f64::from(RATE)) as usize;
            for i in 0..2000 {
                let Some(s) = out.get_mut(start + i) else {
                    break;
                };
                noise = noise.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let n = (noise >> 9) as f32 / (1u32 << 23) as f32 - 0.5;
                let t = i as f32 / RATE as f32;
                *s += (n * 0.8 + (2.0 * PI * 60.0 * t).sin()) * (-t / 0.02).exp();
            }
        }
        out
    }

    fn nearest_error(found: &[f64], truth: &[f64]) -> f64 {
        truth
            .iter()
            .filter(|t| **t > 2.0 && **t < truth.last().unwrap() - 1.0)
            .map(|t| found.iter().map(|f| (f - t).abs()).fold(f64::MAX, f64::min))
            .fold(0.0, f64::max)
    }

    #[test]
    fn finds_a_steady_tempo_and_its_beats() {
        let truth: Vec<f64> = (0..40).map(|i| 0.5 + f64::from(i) * 0.6).collect(); // 100 BPM
        let grid = track_beats(&click_track(&truth, 25.0), RATE);
        assert!((grid.bpm - 100.0).abs() < 1.5, "{}", grid.bpm);
        let error = nearest_error(&grid.beats, &truth);
        assert!(error < 0.015, "worst beat {error:.3} s off");
    }

    #[test]
    fn eighth_note_hats_do_not_double_the_tempo() {
        let truth: Vec<f64> = (0..36).map(|i| 0.4 + f64::from(i) * 0.6).collect();
        let mut hats: Vec<f64> = truth.iter().map(|b| b + 0.3).collect();
        hats.extend(&truth);
        let mut audio = click_track(&truth, 23.0);
        let quiet = click_track(&hats, 23.0);
        for (a, q) in audio.iter_mut().zip(quiet) {
            *a += 0.3 * q;
        }
        let grid = track_beats(&audio, RATE);
        assert!((grid.bpm - 100.0).abs() < 2.0, "{}", grid.bpm);
    }

    #[test]
    fn follows_a_tempo_change() {
        // 90 BPM for 16 beats, then 110 BPM.
        let mut truth = Vec::new();
        let mut t = 0.5;
        for i in 0..40 {
            truth.push(t);
            t += if i < 16 { 60.0 / 90.0 } else { 60.0 / 110.0 };
        }
        let grid = track_beats(&click_track(&truth, t + 0.5), RATE);
        let error = nearest_error(&grid.beats, &truth);
        assert!(error < 0.03, "worst beat {error:.3} s off");
    }

    #[test]
    fn silence_has_no_beats() {
        let grid = track_beats(&vec![0.0; RATE as usize * 5], RATE);
        assert!(grid.beats.is_empty());
    }
}
