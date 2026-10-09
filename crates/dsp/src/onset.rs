//! Streaming onset detection tuned for bass plucks.
//!
//! A bass fundamental needs 25–35 ms per period, far too slow to time an attack. The attack
//! itself is broadband (finger or pick noise, bright upper partials), so the detector looks only
//! at 150 Hz – 5 kHz:
//!
//! 1. Spectral flux of log magnitudes against a reference that is the maximum over the last
//!    `MEMORY_S` (longer than the slowest bass period) and over neighbouring bins. Sustained
//!    notes beat and pulse once per period; taking the maximum over a full period hides that,
//!    while a new attack still rises above it.
//! 2. Peak picking against a moving mean plus an adaptive margin, gated by a noise floor.
//! 3. Refinement in the time domain: the first sample of the high-passed signal that crosses a
//!    fraction of the attack peak gives millisecond precision.

use crate::biquad::{Biquad, FilterKind};
use rustfft::num_complex::Complex32;
use rustfft::{Fft, FftPlanner};
use std::f32::consts::PI;
use std::fmt;
use std::sync::Arc;

const WINDOW: usize = 512;
/// Frames closer than this are too similar to compare against.
const LAG: usize = 2;
/// Reference memory: longer than one period of B0 (32 ms).
const MEMORY_S: f64 = 0.04;
const MEAN_FRAMES: usize = 24;
const PEAK_HALF: usize = 2;
const LOW_HZ: f32 = 200.0;
const HIGH_HZ: f32 = 5000.0;
const LOG_GAMMA: f32 = 1000.0;
/// History of the high-passed signal kept for time refinement.
const HISTORY_S: f64 = 0.12;
/// Fraction of the attack rise that marks the onset sample.
const REFINE_FRACTION: f32 = 0.3;
/// Envelope block for refinement and the look-back that defines a rise.
const BLOCK_S: f64 = 0.0005;
const RISE_BLOCKS: usize = 6;

/// Tunable thresholds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OnsetConfig {
    /// Minimum time between two onsets.
    pub min_interval_s: f64,
    /// Flux must exceed the moving mean by this factor …
    pub ratio: f32,
    /// … and by at least this absolute margin.
    pub margin: f32,
    /// Ignore attacks whose high band stays below this level.
    pub gate_db: f32,
}

impl Default for OnsetConfig {
    fn default() -> Self {
        OnsetConfig {
            min_interval_s: 0.035,
            ratio: 2.0,
            margin: 8.0,
            gate_db: -62.0,
        }
    }
}

/// A detected attack. `sample` counts from the first sample ever pushed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Onset {
    pub sample: u64,
    /// Strength of the flux peak (arbitrary units, for diagnostics).
    pub strength: f32,
}

pub struct OnsetDetector {
    config: OnsetConfig,
    sample_rate: f64,
    hop: usize,
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    frame: Vec<Complex32>,
    scratch: Vec<Complex32>,
    bins: (usize, usize),
    /// Last log spectra (ring of `memory` frames).
    spectra: Vec<Vec<f32>>,
    memory: usize,
    flux: Vec<f32>,
    input: Vec<f32>,
    input_len: usize,
    highpass: Biquad,
    history: Vec<f32>,
    history_pos: usize,
    frames: u64,
    pushed: u64,
    last_onset: Option<u64>,
    found: Vec<Onset>,
}

impl fmt::Debug for OnsetDetector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OnsetDetector")
            .field("sample_rate", &self.sample_rate)
            .field("hop", &self.hop)
            .finish()
    }
}

impl OnsetDetector {
    pub fn new(sample_rate: f64, config: OnsetConfig) -> OnsetDetector {
        let hop = ((sample_rate * 0.001_33).round() as usize).max(16);
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(WINDOW);
        let bin_hz = sample_rate as f32 / WINDOW as f32;
        let low = ((LOW_HZ / bin_hz) as usize).max(1);
        let high = ((HIGH_HZ / bin_hz) as usize).min(WINDOW / 2 - 1);
        let window = (0..WINDOW)
            .map(|i| 0.5 - 0.5 * (2.0 * PI * i as f32 / WINDOW as f32).cos())
            .collect();
        let history_len = (HISTORY_S * sample_rate) as usize;
        let memory = ((MEMORY_S * sample_rate) as usize / hop).max(LAG + 1) + 1;
        OnsetDetector {
            config,
            sample_rate,
            hop,
            scratch: vec![Complex32::default(); fft.get_inplace_scratch_len()],
            fft,
            window,
            frame: vec![Complex32::default(); WINDOW],
            bins: (low, high),
            spectra: vec![vec![0.0; high - low + 1]; memory],
            memory,
            flux: vec![0.0; MEAN_FRAMES + PEAK_HALF * 2 + 1],
            input: vec![0.0; WINDOW],
            input_len: 0,
            highpass: Biquad::new(FilterKind::HighPass, sample_rate, f64::from(LOW_HZ)),
            history: vec![0.0; history_len],
            history_pos: 0,
            frames: 0,
            pushed: 0,
            last_onset: None,
            found: Vec::with_capacity(64),
        }
    }

    /// Delay between an attack and its report, from look-ahead and window size.
    pub fn latency_samples(&self) -> usize {
        WINDOW + self.hop * (PEAK_HALF + 1)
    }

    /// Feeds samples; returns onsets found in this call (valid until the next call).
    pub fn push(&mut self, samples: &[f32]) -> &[Onset] {
        self.found.clear();
        for &x in samples {
            let hp = self.highpass.process(x);
            self.history[self.history_pos] = hp;
            self.history_pos = (self.history_pos + 1) % self.history.len();
            self.input[self.input_len] = x;
            self.input_len += 1;
            self.pushed += 1;
            if self.input_len == WINDOW {
                self.analyze_frame();
                self.input.copy_within(self.hop.., 0);
                self.input_len = WINDOW - self.hop;
            }
        }
        &self.found
    }

    /// Onsets returned by the last `push`.
    pub fn last_found(&self) -> &[Onset] {
        &self.found
    }

    pub fn reset(&mut self) {
        self.input_len = 0;
        self.frames = 0;
        self.last_onset = None;
        self.flux.iter_mut().for_each(|v| *v = 0.0);
        self.spectra
            .iter_mut()
            .for_each(|s| s.iter_mut().for_each(|v| *v = 0.0));
        self.highpass.reset();
    }

    fn analyze_frame(&mut self) {
        for (i, c) in self.frame.iter_mut().enumerate() {
            *c = Complex32::new(self.input[i] * self.window[i], 0.0);
        }
        self.fft
            .process_with_scratch(&mut self.frame, &mut self.scratch);

        // Log spectrum of the band into the ring slot of this frame.
        let slot = (self.frames % self.memory as u64) as usize;
        let low = self.bins.0;
        let mut band_energy = 0.0f32;
        for (k, value) in self.spectra[slot].iter_mut().enumerate() {
            let mag = self.frame[low + k].norm();
            band_energy += mag * mag;
            *value = (1.0 + LOG_GAMMA * mag).ln();
        }

        // Flux against the maximum over older frames (LAG..memory back) and ±1 bin.
        let mut flux = 0.0f32;
        if self.frames >= self.memory as u64 {
            let bins = self.spectra[slot].len();
            for k in 0..bins {
                let lo = k.saturating_sub(1);
                let hi = (k + 1).min(bins - 1);
                let mut reference = f32::MIN;
                for back in LAG..self.memory {
                    let old = ((self.frames + self.memory as u64 - back as u64)
                        % self.memory as u64) as usize;
                    reference = self.spectra[old][lo..=hi]
                        .iter()
                        .fold(reference, |m, v| m.max(*v));
                }
                flux += (self.spectra[slot][k] - reference).max(0.0);
            }
        }
        let gate = band_db(band_energy) > self.config.gate_db;
        self.flux.rotate_left(1);
        let last = self.flux.len() - 1;
        self.flux[last] = if gate { flux } else { 0.0 };
        self.frames += 1;
        self.pick_peak();
    }

    /// Checks whether the frame PEAK_HALF hops back is a local maximum above the threshold.
    fn pick_peak(&mut self) {
        let center = self.flux.len() - 1 - PEAK_HALF;
        let value = self.flux[center];
        if value <= 0.0 {
            return;
        }
        let neighbours = &self.flux[center - PEAK_HALF..=center + PEAK_HALF];
        if neighbours.iter().any(|v| *v > value) {
            return;
        }
        let mean =
            self.flux[..center - PEAK_HALF].iter().sum::<f32>() / (center - PEAK_HALF) as f32;
        if value < mean * self.config.ratio || value < mean + self.config.margin {
            return;
        }
        let frame = self.frames - 1 - PEAK_HALF as u64;
        let estimate = frame * self.hop as u64 + (WINDOW / 2) as u64;
        let sample = self.refine(estimate);
        let min_gap = (self.config.min_interval_s * self.sample_rate) as u64;
        if self.last_onset.is_some_and(|last| sample < last + min_gap) {
            return;
        }
        self.last_onset = Some(sample);
        self.found.push(Onset {
            sample,
            strength: value,
        });
    }

    /// Locates the attack near `estimate` in the high-passed history: the steepest rise of a
    /// 0.5 ms peak envelope, then the first sample that crosses `REFINE_FRACTION` of that rise.
    /// Ringing of the previous note is ignored because only the rise counts.
    fn refine(&self, estimate: u64) -> u64 {
        let block = ((BLOCK_S * self.sample_rate) as u64).max(1);
        let span = (0.025 * self.sample_rate) as u64;
        let len = self.history.len() as u64;
        let oldest = self.pushed.saturating_sub(len);
        let from = estimate.saturating_sub(span).max(oldest);
        let to = (estimate + span).min(self.pushed);
        if to < from + block * (RISE_BLOCKS as u64 + 2) {
            return estimate;
        }
        let at = |s: u64| self.history[((s - oldest + self.history_pos as u64) % len) as usize];
        let envelope = |b: u64| {
            (from + b * block..from + (b + 1) * block)
                .map(|s| at(s).abs())
                .fold(0.0f32, f32::max)
        };
        let blocks = (to - from) / block;

        // Steepest rise over RISE_BLOCKS blocks.
        let mut best = (0u64, 0.0f32, 0.0f32);
        for b in RISE_BLOCKS as u64..blocks {
            let level = envelope(b);
            let floor = (b - RISE_BLOCKS as u64..b)
                .map(&envelope)
                .fold(f32::MAX, f32::min);
            if level - floor > best.1 {
                best = (b, level - floor, floor);
            }
        }
        let (peak_block, rise, floor) = best;
        if rise <= 0.0 {
            return estimate;
        }
        let threshold = floor + rise * REFINE_FRACTION;
        let start = from + (peak_block - RISE_BLOCKS as u64) * block;
        let end = from + (peak_block + 1) * block;
        (start..end)
            .find(|s| at(*s).abs() >= threshold)
            .unwrap_or(estimate)
    }
}

fn band_db(energy: f32) -> f32 {
    if energy <= 0.0 {
        return -200.0;
    }
    10.0 * (energy / (WINDOW as f32 * WINDOW as f32 / 4.0)).log10()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::{laptop_jack, render, SynthNote, ToneShape};

    const SR: f64 = 48_000.0;

    fn riff(velocity: f32) -> Vec<SynthNote> {
        // Eighth notes at 100 BPM with repeated pitches, octave jumps and low B.
        let hz = [
            41.2, 41.2, 82.4, 41.2, 55.0, 55.0, 30.87, 61.7, 73.4, 73.4, 98.0, 41.2,
        ];
        hz.iter()
            .enumerate()
            .map(|(i, h)| SynthNote {
                onset_s: 0.2 + i as f64 * 0.3,
                hz: *h,
                duration_s: 0.28,
                velocity,
            })
            .collect()
    }

    fn detect(signal: &[f32]) -> Vec<u64> {
        let mut d = OnsetDetector::new(SR, OnsetConfig::default());
        let mut out = Vec::new();
        for block in signal.chunks(256) {
            out.extend(d.push(block).iter().map(|o| o.sample));
        }
        out
    }

    fn check(signal: &[f32], notes: &[SynthNote], max_error_ms: f64) {
        let found = detect(signal);
        assert_eq!(found.len(), notes.len(), "found {:?}", found);
        for (sample, note) in found.iter().zip(notes) {
            let error_ms = (*sample as f64 / SR - note.onset_s) * 1000.0;
            assert!(
                error_ms.abs() < max_error_ms,
                "note at {} s: error {error_ms:.2} ms",
                note.onset_s
            );
        }
    }

    #[test]
    fn clean_line_input() {
        let notes = riff(0.8);
        check(&render(&notes, SR, 4.0, ToneShape::Rich, 1), &notes, 5.0);
    }

    #[test]
    fn soft_playing() {
        let notes = riff(0.25);
        check(&render(&notes, SR, 4.0, ToneShape::Rich, 2), &notes, 5.0);
    }

    #[test]
    fn laptop_jack_dull() {
        let notes = riff(0.6);
        let signal = laptop_jack(&render(&notes, SR, 4.0, ToneShape::Dull, 3), SR, 4);
        check(&signal, &notes, 8.0);
    }

    #[test]
    fn silence_has_no_onsets() {
        assert!(detect(&vec![0.0; 48_000]).is_empty());
        let mut hum = vec![0.0; 48_000];
        crate::synth::add_noise(&mut hum, -70.0, 1);
        assert!(detect(&hum).is_empty());
    }
}
