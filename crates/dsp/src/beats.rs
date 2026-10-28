//! Beat tracking for backing tracks (LH-BIB-06): where the beats of a recording fall, so an
//! arrangement's bars can follow the band instead of a constant tempo.
//!
//! ```text
//! samples ─► spectral flux (onset strength) ─► tempo (autocorrelation, prior ~120 BPM)
//!       │                                 └──► beats (dynamic programming, Ellis 2007)
//!       └──► low-band flux (kick, bass) ─────────► phase: on the beat or half a beat off?
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
/// Kick drum and bass live below this; hi-hats and most of a snare above.
const LOW_HZ: f64 = 150.0;
/// Frames either side of a beat searched for its low-band onset (about ±12 ms).
const PHASE_REACH: usize = 2;
/// Low-band rise (linear FFT magnitude) below which there is no kick or bass: about 48 dB under
/// a full-scale sine (256 in a Hann window of 1024).
const LOW_FLOOR: f64 = 1.0;

/// Tempo and beat times (seconds) of a recording.
#[derive(Debug, Clone, PartialEq)]
pub struct BeatGrid {
    pub bpm: f64,
    pub beats: Vec<f64>,
    /// Share (0–1) of the kick and bass onsets on the beats rather than half a beat off. Near 0.5
    /// (or 0 without any): the grid may sit on the offbeat, the player should check by ear.
    pub on_beat_share: f64,
}

/// Finds the beats of `samples` (mono) at `rate` Hz. Silence or very short input gives no beats.
pub fn track_beats(samples: &[f32], rate: u32) -> BeatGrid {
    let (strength, low) = onset_strength(samples, rate);
    let frame_s = HOP as f64 / f64::from(rate.max(1));
    let Some(period) = tempo_period(&strength, frame_s) else {
        return BeatGrid {
            bpm: 0.0,
            beats: Vec::new(),
            on_beat_share: 0.0,
        };
    };
    let mut frames = dynamic_beats(&strength, period);
    let on_beat_share = fix_phase(&mut frames, &low, period);
    // Flux at frame i compares windows starting at i-1 and i: it jumps in the first window that
    // holds the attack, with the attack in its last hop.
    let offset_s = (WINDOW - HOP) as f64 / f64::from(rate.max(1));
    let beats = frames
        .iter()
        .map(|f| *f as f64 * frame_s + offset_s)
        .collect();
    BeatGrid {
        bpm: 60.0 / (period * frame_s),
        beats,
        on_beat_share,
    }
}

/// Moves the beats half a period when kick and bass sit between them rather than on them (a
/// loud hi-hat on the offbeat pulls the tracker there); returns the share of the low-band onset
/// that then falls on the beats (0.5: no telling which).
fn fix_phase(beats: &mut Vec<usize>, low: &[f64], period: f64) -> f64 {
    let half = (period / 2.0).round() as usize;
    let peak = |frame: usize| -> f64 {
        let from = frame.saturating_sub(PHASE_REACH);
        let to = (frame + PHASE_REACH + 1).min(low.len());
        low.get(from..to)
            .map_or(0.0, |w| w.iter().copied().fold(0.0, f64::max))
            .max(LOW_FLOOR)
            - LOW_FLOOR
    };
    let on: f64 = beats.iter().map(|b| peak(*b)).sum();
    let off: f64 = beats.iter().map(|b| peak(b + half)).sum();
    // Half a beat earlier, so a downbeat before the first offbeat stays; the last beat moves
    // half a beat later instead of being lost.
    if off > on {
        let last = beats.last().map(|b| b + half).filter(|b| *b < low.len());
        beats.retain(|b| *b >= half);
        beats.iter_mut().for_each(|b| *b -= half);
        beats.extend(last);
    }
    if on + off <= 0.0 {
        return 0.0;
    }
    on.max(off) / (on + off)
}

/// Half-wave rectified spectral flux of log magnitudes, minus a one-second moving mean; and the
/// linear flux below `LOW_HZ` (kick and bass) for the phase.
fn onset_strength(samples: &[f32], rate: u32) -> (Vec<f64>, Vec<f64>) {
    if samples.len() < WINDOW * 2 {
        return (Vec::new(), Vec::new());
    }
    let low_bins = (LOW_HZ * WINDOW as f64 / f64::from(rate.max(1))).ceil() as usize;
    let fft = FftPlanner::<f32>::new().plan_fft_forward(WINDOW);
    let window: Vec<f32> = (0..WINDOW)
        .map(|i| 0.5 - 0.5 * (2.0 * PI * i as f32 / WINDOW as f32).cos())
        .collect();
    let bins = WINDOW / 2;
    let mut previous = vec![0.0f32; bins];
    let mut previous_low = vec![0.0f32; low_bins];
    let mut buffer = vec![Complex32::new(0.0, 0.0); WINDOW];
    let frames = (samples.len() - WINDOW) / HOP;
    let mut flux = Vec::with_capacity(frames);
    let mut low = Vec::with_capacity(frames);
    for f in 0..frames {
        let start = f * HOP;
        for (i, b) in buffer.iter_mut().enumerate() {
            *b = Complex32::new(samples[start + i] * window[i], 0.0);
        }
        fft.process(&mut buffer);
        let mut sum = 0.0f64;
        let mut low_sum = 0.0f64;
        for (bin, prev) in previous.iter_mut().enumerate() {
            let magnitude = (1.0 + 100.0 * buffer[bin + 1].norm()).ln();
            if f > 0 {
                sum += f64::from((magnitude - *prev).max(0.0));
            }
            *prev = magnitude;
        }
        // Linear, not log: a hi-hat's faint low end must not count as a kick.
        for (bin, prev) in previous_low.iter_mut().enumerate() {
            let magnitude = buffer[bin + 1].norm();
            if f > 0 {
                low_sum += f64::from((magnitude - *prev).max(0.0));
            }
            *prev = magnitude;
        }
        flux.push(sum);
        low.push(low_sum);
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
    (out, low)
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
            let first = t.saturating_sub(hi);
            for (p, previous) in score.iter().enumerate().take(t - lo + 1).skip(first) {
                let gap = (t - p) as f64 / period;
                let s = previous - TIGHTNESS * gap.ln().powi(2);
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

    /// Mean signed distance (found minus true) of each inner true beat to its nearest found one.
    fn mean_error(found: &[f64], truth: &[f64]) -> f64 {
        let inner: Vec<f64> = truth
            .iter()
            .filter(|t| **t > 2.0 && **t < truth.last().unwrap() - 1.0)
            .map(|t| {
                let near = found
                    .iter()
                    .min_by(|a, b| (*a - t).abs().total_cmp(&(*b - t).abs()));
                near.unwrap() - t
            })
            .collect();
        inner.iter().sum::<f64>() / inner.len() as f64
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

    /// A band-like mix: kick on 1 and 3, snare on 2 and 4, quiet hi-hat eighths, a bass note on
    /// each beat; `jitter_s` of human looseness, deterministic.
    fn band_track(beats: &[f64], seconds: f64, jitter_s: f64) -> Vec<f32> {
        let mut seed = 777u32;
        let mut loose = |t: f64| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            t + jitter_s * ((seed >> 8) as f64 / f64::from(1u32 << 24) * 2.0 - 1.0)
        };
        let kicks: Vec<f64> = beats.iter().step_by(2).map(|b| loose(*b)).collect();
        let snares: Vec<f64> = beats.iter().skip(1).step_by(2).map(|b| loose(*b)).collect();
        let hats: Vec<f64> = beats
            .windows(2)
            .flat_map(|w| [w[0], 0.5 * (w[0] + w[1])])
            .map(&mut loose)
            .collect();
        let mut out = click_track(&kicks, seconds);
        let mix = |out: &mut Vec<f32>, part: Vec<f32>, gain: f32| {
            out.iter_mut().zip(part).for_each(|(o, p)| *o += gain * p);
        };
        mix(&mut out, click_track(&snares, seconds), 0.8);
        mix(&mut out, click_track(&hats, seconds), 0.2);

        // Bass: a decaying 55 Hz tone from each beat to the next.
        for w in beats.windows(2) {
            let from = (w[0] * f64::from(RATE)) as usize;
            let to = ((w[1] * f64::from(RATE)) as usize).min(out.len());
            for (i, s) in out[from..to].iter_mut().enumerate() {
                let t = i as f32 / RATE as f32;
                *s += 0.5 * (2.0 * PI * 55.0 * t).sin() * (-t / 0.4).exp();
            }
        }
        out
    }

    #[test]
    fn stays_within_20_ms_over_four_minutes() {
        // LH-BIB-06 acceptance: a four-minute band wandering between 116 and 124 BPM, ±5 ms loose.
        const MINUTES: f64 = 4.0;
        const LIMIT_S: f64 = 0.020;
        let seconds = MINUTES * 60.0;
        let mut truth = Vec::new();
        let mut t = 0.5;
        while t < seconds - 1.0 {
            truth.push(t);
            let bpm = 120.0 + 4.0 * (2.0 * std::f64::consts::PI * t / 90.0).sin();
            t += 60.0 / bpm;
        }
        let grid = track_beats(&band_track(&truth, seconds, 0.005), RATE);
        let error = nearest_error(&grid.beats, &truth);
        assert!(error <= LIMIT_S, "worst beat {:.1} ms off", error * 1000.0);
        assert!(grid.on_beat_share > 0.8, "{}", grid.on_beat_share);

        // No constant lead or lag: the player would be judged late or early with the band.
        const BIAS_LIMIT_S: f64 = 0.005;
        let bias = mean_error(&grid.beats, &truth);
        assert!(
            bias.abs() <= BIAS_LIMIT_S,
            "beats {:.1} ms off on average",
            bias * 1000.0
        );
    }

    /// Kick on 1 and 3 (a low thump), a loud hi-hat (high noise) on every offbeat: the hats
    /// carry most of the onset energy, half a beat after the beat.
    fn offbeat_hats(beats: &[f64], seconds: f64, kick_gain: f32) -> Vec<f32> {
        let mut out = vec![0.0f32; (seconds * f64::from(RATE)) as usize];
        for b in beats.iter().step_by(2) {
            let start = (b * f64::from(RATE)) as usize;
            for (i, s) in out.iter_mut().skip(start).take(6000).enumerate() {
                let t = i as f32 / RATE as f32;
                *s += kick_gain * (2.0 * PI * 55.0 * t).sin() * (-t / 0.08).exp();
            }
        }
        let mut noise = 4242u32;
        for w in beats.windows(2) {
            let start = (0.5 * (w[0] + w[1]) * f64::from(RATE)) as usize;
            let mut last = 0.0f32;
            for (i, s) in out.iter_mut().skip(start).take(3000).enumerate() {
                noise = noise.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let n = (noise >> 9) as f32 / (1u32 << 23) as f32 - 0.5;
                let t = i as f32 / RATE as f32;
                *s += 1.5 * (n - last) * (-t / 0.03).exp();
                last = n;
            }
        }
        out
    }

    #[test]
    fn hats_on_the_offbeat_do_not_shift_the_beats() {
        // QA 27.10. (J3): the grid sat on the offbeat, +310 ms at 97 BPM.
        let truth: Vec<f64> = (0..48).map(|i| 0.5 + f64::from(i) * 60.0 / 97.0).collect();
        let grid = track_beats(&offbeat_hats(&truth, 31.0, 1.0), RATE);
        let error = nearest_error(&grid.beats, &truth);
        assert!(
            error < 0.03,
            "{} BPM, worst beat {:.0} ms off",
            grid.bpm,
            error * 1000.0
        );
        assert!(grid.on_beat_share > 0.9, "{}", grid.on_beat_share);
        // The first downbeat stays (PR-Agent #155: shifting later dropped bar 1).
        let first = grid.beats.first().copied().unwrap_or(f64::MAX);
        assert!(
            (first - truth[0]).abs() < 0.03,
            "first beat at {first:.3} s"
        );
    }

    #[test]
    fn hats_alone_leave_the_phase_open() {
        // Nothing low tells beat from offbeat: the grid must not claim to know.
        let truth: Vec<f64> = (0..48).map(|i| 0.5 + f64::from(i) * 60.0 / 97.0).collect();
        let grid = track_beats(&offbeat_hats(&truth, 31.0, 0.0), RATE);
        assert!(grid.on_beat_share < 0.6, "{}", grid.on_beat_share);
    }

    #[test]
    fn silence_has_no_beats() {
        let grid = track_beats(&vec![0.0; RATE as usize * 5], RATE);
        assert!(grid.beats.is_empty());
    }
}
