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
use crate::noise_floor::FloorMeter;
use rustfft::num_complex::Complex32;
use rustfft::{Fft, FftPlanner};
use std::f32::consts::PI;
use std::fmt;
use std::ops::Range;
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
/// Longer than one period of the lowest bass note (B0: 32 ms).
const PERIOD_S: f64 = 0.035;

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
    /// Samples where no onset is taken (`mute`).
    muted: Range<u64>,
    /// Band levels at the calibration's count-in (LH-EIN-16).
    floor: FloorMeter,
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
            muted: 0..0,
            floor: FloorMeter::default(),
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

    /// Takes no onset from frames that see `samples` (counted like `Onset::sample`), and lets
    /// them raise neither the minimum interval nor the mean: an attack 30 ms later is found.
    /// E.g. the jump back from inserted silence to a ringing string.
    pub fn mute(&mut self, samples: Range<u64>) {
        self.muted = samples;
    }

    /// Measures the band's noise floor over `samples` (counted like `Onset::sample`).
    pub fn measure_floor(&mut self, samples: Range<u64>) {
        self.floor.start(samples);
    }

    /// The noise floor measured (dB, like `OnsetConfig::gate_db`), once its span has passed.
    pub fn noise_floor_db(&self) -> Option<f32> {
        self.floor.floor_db()
    }

    /// Attacks below `gate_db` in the band are ignored from now on.
    pub fn set_gate(&mut self, gate_db: f32) {
        self.config.gate_db = gate_db;
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
        // Frames before the stream started count as silence (the ring starts zeroed), so an
        // attack right at the start is still found.
        if self.frames >= LAG as u64 {
            let bins = self.spectra[slot].len();
            let reach = self.memory.min(self.frames as usize + 1);
            for k in 0..bins {
                let lo = k.saturating_sub(1);
                let hi = (k + 1).min(bins - 1);
                let mut reference = f32::MIN;
                for back in LAG..reach {
                    let old = ((self.frames + self.memory as u64 - back as u64)
                        % self.memory as u64) as usize;
                    reference = self.spectra[old][lo..=hi]
                        .iter()
                        .fold(reference, |m, v| m.max(*v));
                }
                flux += (self.spectra[slot][k] - reference).max(0.0);
            }
        }
        let level_db = band_db(band_energy);
        let gate = level_db > self.config.gate_db;
        let first = self.pushed.saturating_sub(WINDOW as u64);
        self.floor.add(first..self.pushed, level_db);
        // A frame that sees muted samples adds nothing: neither an onset nor to the mean a
        // later attack must stand out of.
        let muted = first < self.muted.end && self.pushed > self.muted.start;
        self.flux.rotate_left(1);
        let last = self.flux.len() - 1;
        self.flux[last] = if gate && !muted { flux } else { 0.0 };
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

    /// Locates the attack near `estimate` in the high-passed history.
    ///
    /// 1. The first 0.5 ms block whose peak exceeds 1.5× the highest peak of the period before it
    ///    (`PERIOD_S`): a ringing note never outgrows its own last period, a new attack does.
    /// 2. If none does (a soft note after a loud one), the steepest rise within ±8 ms.
    ///
    /// The onset is the first sample in that block's rise that crosses `REFINE_FRACTION`.
    fn refine(&self, estimate: u64) -> u64 {
        let block = ((BLOCK_S * self.sample_rate) as u64).max(1);
        let len = self.history.len() as u64;
        let oldest = self.pushed.saturating_sub(len);
        let period = (PERIOD_S * self.sample_rate) as u64;
        // An attack after muted samples starts after them, however steep the rise in them; and
        // a block is compared with the period before it only once a whole one came after them
        // (the signal jumped there, its peaks of one period are not all in yet).
        let after_mute = !self.muted.is_empty() && estimate >= self.muted.end;
        let earliest = if after_mute {
            oldest.max(self.muted.end)
        } else {
            oldest
        };
        let compared = if after_mute {
            earliest.max(self.muted.start + period)
        } else {
            earliest
        };
        let from = estimate
            .saturating_sub((0.025 * self.sample_rate) as u64)
            .max(compared);
        let to = (estimate + (0.025 * self.sample_rate) as u64).min(self.pushed);
        if to < from + block * 4 {
            return estimate;
        }
        // Sample n was written at index n % len; samples outside the history count as silence.
        let at = |s: u64| {
            if s >= oldest && s < self.pushed {
                self.history[(s % len) as usize]
            } else {
                0.0
            }
        };
        let peak = |start: u64, end: u64| (start..end).map(|s| at(s).abs()).fold(0.0f32, f32::max);

        let mut chosen = None;
        let mut start = from;
        while start + block <= to {
            let level = peak(start, start + block);
            let before = peak(start.saturating_sub(period).max(oldest), start);
            if level > before * 1.5 && level > 1e-4 {
                chosen = Some((start, before));
                break;
            }
            start += block;
        }
        let (block_start, floor) = match chosen {
            Some(found) => found,
            None => self.steepest_rise(estimate, block, earliest, &at),
        };
        let level = peak(block_start, block_start + block);
        let threshold = floor + (level - floor) * REFINE_FRACTION;
        (block_start.saturating_sub(block)..block_start + block)
            .find(|s| at(*s).abs() >= threshold)
            .unwrap_or(block_start)
    }

    /// Fallback: block with the steepest rise over `RISE_BLOCKS` blocks within ±8 ms of `estimate`,
    /// from `earliest` on.
    fn steepest_rise(
        &self,
        estimate: u64,
        block: u64,
        earliest: u64,
        at: &dyn Fn(u64) -> f32,
    ) -> (u64, f32) {
        let span = (0.008 * self.sample_rate) as u64;
        let peak = |start: u64| {
            (start..start + block)
                .map(|s| at(s).abs())
                .fold(0.0f32, f32::max)
        };
        let mut best = (estimate, 0.0f32, 0.0f32);
        let mut start = estimate.saturating_sub(span).max(earliest);
        while start <= estimate + span {
            let floor = (1..=RISE_BLOCKS as u64)
                .map(|k| peak(start.saturating_sub(k * block)))
                .fold(f32::MAX, f32::min);
            let rise = peak(start) - floor;
            if rise > best.1 {
                best = (start, rise, floor);
            }
            start += block;
        }
        (best.0, best.2)
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
    fn note_at_the_very_start() {
        let notes = [SynthNote {
            onset_s: 0.0,
            hz: 41.2,
            duration_s: 0.3,
            velocity: 0.7,
        }];
        check(&render(&notes, SR, 0.5, ToneShape::Rich, 5), &notes, 5.0);
    }

    #[test]
    fn silence_has_no_onsets() {
        assert!(detect(&vec![0.0; 48_000]).is_empty());
        let mut hum = vec![0.0; 48_000];
        crate::synth::add_noise(&mut hum, -70.0, 1);
        assert!(detect(&hum).is_empty());
    }
}
