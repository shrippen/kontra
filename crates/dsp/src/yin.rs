//! YIN pitch estimation (de Cheveigné & Kawahara 2002) with an FFT-based difference function.
//!
//! The window must hold `window + tau_max` samples. For a bass down to B0 (30.9 Hz) at 48 kHz
//! `tau_max` is about 1700 samples, so one estimate needs roughly 75 ms of audio.

use crate::biquad::{Biquad, FilterKind};
use rustfft::num_complex::Complex32;
use rustfft::{Fft, FftPlanner};
use std::fmt;
use std::sync::Arc;

/// Below this cumulative-mean-normalized difference a dip counts as the period.
const THRESHOLD: f64 = 0.15;
/// A dip at half the period wins if it is at most this much shallower (normalized difference).
const OCTAVE_TOLERANCE: f64 = 0.1;
/// Low-pass cutoff relative to the coarse estimate for the refining pass.
const REFINE_CUTOFF: f64 = 1.5;
/// The refined value replaces the coarse one unless they differ by this much (octaves): a
/// quarter octave catches octave and fifth slips, but lets the refinement correct a coarse
/// value that noise pulled off by most of a semitone (a weak note through a laptop jack).
const REFINE_MAX_OCTAVES: f64 = 0.25;
/// A thump an octave below the note (kick drum in a separated stem, a double bass's body) makes
/// the signal repeat at twice the note's period. All three must hold to halve the period:
/// the half period repeats this well (normalized difference; FiloBass notes over a thump
/// 0.1–0.5; 0.6 and more took room-double notes an octave up) …
const THUMP_DIP: f64 = 0.5;
/// … the odd harmonics 3, 5, 7 of the long period carry less than this share of the power of
/// the even ones 2, 4, 6 (the note's own harmonics; over a thump mostly below 0.03) …
const THUMP_ODD_SHARE: f64 = 0.15;
/// … and the lowest partial fades against the note's: its power ratio from the first to the
/// last period of the window, over the same ratio of the 2nd harmonic. FiloBass: a thump
/// mostly below 0.25, a played low note mostly above 0.5 (its fundamental rings longest).
const THUMP_FADE: f64 = 0.5;
/// Highest harmonic compared for `THUMP_ODD_SHARE`.
const THUMP_TOP: usize = 7;

/// Whether a fading thump an octave below may be taken for the note (`THUMP_*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Thump {
    Counts,
    Ignored,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PitchEstimate {
    pub hz: f64,
    /// 1 − normalized difference at the chosen period; 1 = perfectly periodic.
    pub confidence: f32,
}

/// Reusable YIN estimator; all buffers are allocated in `new`.
pub struct Yin {
    sample_rate: f64,
    window: usize,
    tau_min: usize,
    tau_max: usize,
    fft: Arc<dyn Fft<f32>>,
    ifft: Arc<dyn Fft<f32>>,
    a: Vec<Complex32>,
    b: Vec<Complex32>,
    scratch: Vec<Complex32>,
    diff: Vec<f64>,
    filtered: Vec<f32>,
}

impl fmt::Debug for Yin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Yin")
            .field("sample_rate", &self.sample_rate)
            .field("window", &self.window)
            .field("tau_min", &self.tau_min)
            .field("tau_max", &self.tau_max)
            .finish()
    }
}

impl Yin {
    /// Estimator for frequencies in `[min_hz, max_hz]` with an integration window of `window_s`.
    pub fn new(sample_rate: f64, min_hz: f64, max_hz: f64, window_s: f64) -> Yin {
        let tau_max = (sample_rate / min_hz.max(1.0)).ceil() as usize + 2;
        let tau_min = ((sample_rate / max_hz.max(min_hz + 1.0)).floor() as usize).max(2);
        let window = ((window_s * sample_rate) as usize).max(tau_max);
        let size = (window + tau_max).next_power_of_two();
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(size);
        let ifft = planner.plan_fft_inverse(size);
        let scratch_len = fft
            .get_inplace_scratch_len()
            .max(ifft.get_inplace_scratch_len());
        Yin {
            sample_rate,
            window,
            tau_min,
            tau_max,
            fft,
            ifft,
            a: vec![Complex32::default(); size],
            b: vec![Complex32::default(); size],
            scratch: vec![Complex32::default(); scratch_len],
            diff: vec![0.0; tau_max + 2],
            filtered: vec![0.0; 2 * (window + tau_max)],
        }
    }

    /// Samples one estimate needs.
    pub fn needed(&self) -> usize {
        self.window + self.tau_max
    }

    /// Estimates the pitch of the first `needed()` samples; `None` if too short, silent or aperiodic.
    pub fn estimate(&mut self, samples: &[f32]) -> Option<PitchEstimate> {
        self.estimate_with(samples, Thump::Counts)
    }

    /// Like `estimate`, for audio right after an attack: a thump an octave below that fades (kick
    /// drum, a double bass's body) is not taken for the note. Only there: on a ringing note the
    /// check would follow other low sounds (a band's bass line from the speakers).
    pub fn estimate_attack(&mut self, samples: &[f32]) -> Option<PitchEstimate> {
        self.estimate_with(samples, Thump::Ignored)
    }

    fn estimate_with(&mut self, samples: &[f32], thump: Thump) -> Option<PitchEstimate> {
        let needed = self.needed();
        if samples.len() < needed {
            return None;
        }
        let samples = &samples[..needed];
        self.difference(samples)?;
        self.normalize();
        let (mut tau, mut value) = self.pick()?;
        if thump == Thump::Ignored {
            (tau, value) = self.above_thump(samples, tau, value);
        }
        let refined = self.interpolate(tau);
        Some(PitchEstimate {
            hz: self.sample_rate / refined,
            confidence: (1.0 - value).clamp(0.0, 1.0) as f32,
        })
    }

    /// Like `estimate`, on the last `needed()` samples, then refined on a low-passed copy so that
    /// inharmonic upper partials of stiff strings do not pull the result sharp. Earlier samples
    /// (up to `needed()` more) warm up the filter. This is what a tuner should show.
    pub fn estimate_fundamental(&mut self, samples: &[f32]) -> Option<PitchEstimate> {
        let needed = self.needed();
        if samples.len() < needed {
            return None;
        }
        let tail = &samples[samples.len() - needed..];
        let coarse = self.estimate(tail)?;
        let take = samples.len().min(self.filtered.len());
        let source = &samples[samples.len() - take..];
        let cutoff = coarse.hz * REFINE_CUTOFF;
        let mut first = Biquad::new(FilterKind::LowPass, self.sample_rate, cutoff);
        let mut second = Biquad::new(FilterKind::LowPass, self.sample_rate, cutoff);
        for (out, x) in self.filtered.iter_mut().zip(source) {
            *out = second.process(first.process(*x));
        }
        let mut filtered = std::mem::take(&mut self.filtered);
        let refined = self.estimate(&filtered[take - needed..take]);
        std::mem::swap(&mut self.filtered, &mut filtered);
        let refined = refined?;
        // Only trust the refinement if it stays near the coarse value (no octave jump).
        if (refined.hz / coarse.hz).log2().abs() > REFINE_MAX_OCTAVES {
            return Some(coarse);
        }
        Some(PitchEstimate {
            hz: refined.hz,
            confidence: coarse.confidence,
        })
    }

    /// d(τ) = e(0) + e(τ) − 2·r(τ), with r from one FFT cross-correlation.
    fn difference(&mut self, samples: &[f32]) -> Option<()> {
        let w = self.window;
        for (i, c) in self.a.iter_mut().enumerate() {
            *c = Complex32::new(if i < w { samples[i] } else { 0.0 }, 0.0);
        }
        for (i, c) in self.b.iter_mut().enumerate() {
            *c = Complex32::new(samples.get(i).copied().unwrap_or(0.0), 0.0);
        }
        self.fft
            .process_with_scratch(&mut self.a, &mut self.scratch);
        self.fft
            .process_with_scratch(&mut self.b, &mut self.scratch);
        for (a, b) in self.a.iter_mut().zip(self.b.iter()) {
            *a = a.conj() * b;
        }
        self.ifft
            .process_with_scratch(&mut self.a, &mut self.scratch);
        let scale = 1.0 / self.a.len() as f64;

        // Energy of the reference window and of the sliding window.
        let energy0: f64 = samples[..w].iter().map(|x| f64::from(*x).powi(2)).sum();
        if energy0 < 1e-10 {
            return None;
        }
        let mut sliding = energy0;
        for tau in 0..=self.tau_max {
            if tau > 0 {
                sliding +=
                    f64::from(samples[tau + w - 1]).powi(2) - f64::from(samples[tau - 1]).powi(2);
            }
            let r = f64::from(self.a[tau].re) * scale;
            self.diff[tau] = (energy0 + sliding - 2.0 * r).max(0.0);
        }
        Some(())
    }

    /// Cumulative mean normalized difference, in place.
    fn normalize(&mut self) {
        self.diff[0] = 1.0;
        let mut running = 0.0;
        for tau in 1..=self.tau_max {
            running += self.diff[tau];
            self.diff[tau] = if running > 0.0 {
                self.diff[tau] * tau as f64 / running
            } else {
                1.0
            };
        }
    }

    /// First dip below the threshold (then its local minimum); else the global minimum, moved
    /// to half its period while a dip there is nearly as deep.
    ///
    /// Without a dip below the threshold (noisy attack, beating, weak fundamental) the global
    /// minimum often sits at twice the period, a note an octave too low. Real recordings showed
    /// this on 20 % of double-bass and some electric-bass notes; the halving check fixes it.
    fn pick(&self) -> Option<(usize, f64)> {
        let range = self.tau_min..self.tau_max;
        for tau in range.clone() {
            if self.diff[tau] >= THRESHOLD {
                continue;
            }
            let mut best = tau;
            while best + 1 < self.tau_max && self.diff[best + 1] < self.diff[best] {
                best += 1;
            }
            return Some((best, self.diff[best]));
        }
        let mut best = range.min_by(|a, b| self.diff[*a].total_cmp(&self.diff[*b]))?;
        while let Some(half) = self.dip_near(best / 2) {
            if self.diff[half] > self.diff[best] + OCTAVE_TOLERANCE {
                break;
            }
            best = half;
        }
        Some((best, self.diff[best]))
    }

    /// Half the period `tau` when `tau` only repeats because of a thump an octave below the
    /// note. A played low note sounds odd and even harmonics and its fundamental rings on; the
    /// note over a thump has only the even ones, and the lowest partial fades.
    ///
    /// ```text
    /// harmonic of tau:  1   2   3   4   5   6   7
    /// G1 played:        G1  ·   ·   ·   ·   ·   ·     odd ones there, 1 rings  → G1
    /// G2 over a thump:  ∿   G2      ·       ·         odd ones missing, ∿ fades → G2
    /// ```
    fn above_thump(&self, samples: &[f32], tau: usize, value: f64) -> (usize, f64) {
        let Some(half) = self.dip_near(tau / 2) else {
            return (tau, value);
        };
        if self.diff[half] > THUMP_DIP || !self.odd_missing(samples, tau) {
            return (tau, value);
        }
        if !self.lowest_fades(samples, tau) {
            return (tau, value);
        }
        (half, self.diff[half])
    }

    /// Whether the odd harmonics of period `tau` are missing (`THUMP_ODD_SHARE`). Measured over
    /// whole periods, so the harmonics do not leak into each other.
    fn odd_missing(&self, samples: &[f32], tau: usize) -> bool {
        let whole = &samples[..samples.len() / tau * tau];
        let f0 = self.sample_rate / tau as f64;
        let (mut odd, mut even) = (0.0, 0.0);
        for k in 2..=THUMP_TOP {
            let hz = f0 * k as f64;
            if hz >= self.sample_rate / 2.0 {
                break;
            }
            let power = self.power_at(whole, hz);
            if k % 2 == 0 {
                even += power;
            } else {
                odd += power;
            }
        }
        odd < even * THUMP_ODD_SHARE
    }

    /// Whether the lowest partial of period `tau` fades against the 2nd harmonic between the
    /// first and the last period of `samples` (`THUMP_FADE`).
    fn lowest_fades(&self, samples: &[f32], tau: usize) -> bool {
        let f0 = self.sample_rate / tau as f64;
        let (first, last) = (&samples[..tau], &samples[samples.len() - tau..]);
        let lowest = (self.power_at(first, f0), self.power_at(last, f0));
        let note = (
            self.power_at(first, 2.0 * f0),
            self.power_at(last, 2.0 * f0),
        );
        lowest.1 * note.0 < THUMP_FADE * lowest.0 * note.1
    }

    /// Power at `hz` over `samples` (Goertzel).
    fn power_at(&self, samples: &[f32], hz: f64) -> f64 {
        let coeff = 2.0 * (std::f64::consts::TAU * hz / self.sample_rate).cos();
        let (mut s1, mut s2) = (0.0, 0.0);
        for x in samples {
            let s = f64::from(*x) + coeff * s1 - s2;
            s2 = s1;
            s1 = s;
        }
        s1 * s1 + s2 * s2 - coeff * s1 * s2
    }

    /// The local minimum within ±3 % of `tau`, if that range lies inside the search range.
    fn dip_near(&self, tau: usize) -> Option<usize> {
        let spread = (tau / 33).max(1);
        let from = tau.checked_sub(spread)?.max(self.tau_min);
        let to = (tau + spread).min(self.tau_max - 1);
        if from >= to || tau < self.tau_min {
            return None;
        }
        (from..=to).min_by(|a, b| self.diff[*a].total_cmp(&self.diff[*b]))
    }

    /// Parabolic interpolation around `tau` for sub-sample precision.
    fn interpolate(&self, tau: usize) -> f64 {
        if tau == 0 || tau + 1 > self.tau_max {
            return tau as f64;
        }
        let (a, b, c) = (self.diff[tau - 1], self.diff[tau], self.diff[tau + 1]);
        let denom = a - 2.0 * b + c;
        if denom.abs() < 1e-12 {
            return tau as f64;
        }
        tau as f64 + 0.5 * (a - c) / denom
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::{bass_tone, ToneShape};
    use kontra_model::{cents_between, Pitch, A4_HZ};

    const SR: f64 = 48_000.0;

    fn yin() -> Yin {
        Yin::new(SR, 27.0, 420.0, 0.04)
    }

    #[test]
    fn detects_all_bass_notes_within_two_cents() {
        let mut yin = yin();
        for midi in 23..=67u8 {
            let hz = Pitch(midi).hz(A4_HZ);
            let tone = bass_tone(hz, SR, 0.3, ToneShape::Rich, 7);
            let est = yin.estimate_fundamental(&tone[2400..]).unwrap();
            let cents = cents_between(hz, est.hz);
            assert!(cents.abs() < 2.0, "midi {midi}: {cents} cents");
            assert!(
                est.confidence > 0.8,
                "midi {midi}: confidence {}",
                est.confidence
            );
        }
    }

    #[test]
    fn detects_cent_offsets() {
        let mut yin = yin();
        let base = Pitch::parse("A1").unwrap().hz(A4_HZ);
        for offset in [-40.0, -10.0, 5.0, 25.0] {
            let hz = base * 2f64.powf(offset / 1200.0);
            let tone = bass_tone(hz, SR, 0.3, ToneShape::Rich, 3);
            let est = yin.estimate_fundamental(&tone[2400..]).unwrap();
            assert!((cents_between(base, est.hz) - offset).abs() < 1.5);
        }
    }

    /// The thump fades within the first YIN window, the note rings on.
    const THUMP_DECAY_S: f64 = 0.03;

    /// A thump an octave below the note, as at a double bass (FiloBass, "All the Things You
    /// Are" 4.876 s: G2 at 96.7 Hz, 48 Hz at −6…−11 dB in the first 55 ms, likely the kick
    /// drum left in the stem or the body's resonance). It makes the signal repeat at twice the
    /// period: YIN heard G1.
    #[test]
    fn thump_an_octave_below_keeps_the_note() {
        let mut yin = yin();
        let skip = (0.012 * SR) as usize;
        for midi in [38u8, 43, 45] {
            let hz = Pitch(midi).hz(A4_HZ);
            let mut tone = bass_tone(hz, SR, 0.3, ToneShape::Rich, 5);
            let peak = tone.iter().fold(0.0f32, |m, s| m.max(s.abs()));
            for (i, s) in tone.iter_mut().enumerate() {
                let t = i as f64 / SR;
                let thump = f64::from(peak) * (-t / THUMP_DECAY_S).exp();
                *s += (thump * (std::f64::consts::TAU * hz / 2.0 * t).sin()) as f32;
            }
            let est = yin.estimate_attack(&tone[skip..]).unwrap();
            let cents = cents_between(hz, est.hz);
            assert!(cents.abs() < 20.0, "midi {midi}: {cents} cents");
        }
    }

    #[test]
    fn silence_and_short_input() {
        let mut yin = yin();
        assert!(yin.estimate(&vec![0.0; yin.needed()]).is_none());
        assert!(yin.estimate(&[0.1; 10]).is_none());
    }
}
