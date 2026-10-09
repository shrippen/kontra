//! Phase vocoder frame: analysis, phase locking, synthesis.
//!
//! Phases are computed once, on the sum of all channels, with scaled phase
//! locking (Laroche & Dolson 1999): each spectral peak advances by its
//! instantaneous frequency × synthesis hop, continuing the previous frame's
//! peak whose region it falls in; the bins around a peak keep their phase
//! offset to it. The result is one rotation per bin, applied to every
//! channel alike, so phase differences between channels (the stereo image)
//! stay exactly as in the input.
//!
//! Frames use zero-phase windowing (rotated by half a window), so a
//! sinusoid has the same phase in all bins of its main lobe.
//!
//! Two real channels share one complex FFT (left + i·right).

use std::f32::consts::{PI, TAU};
use std::fmt;
use std::sync::Arc;

use rustfft::num_complex::Complex;
use rustfft::{Fft, FftPlanner};

use crate::onset::hann;
use crate::sizes::Sizes;

/// Hann² overlap-add sum at 4x overlap.
const OLA_SUM: f32 = 1.5;
/// Peaks quieter than this, relative to the loudest bin, are ignored (-100 dB).
const PEAK_FLOOR: f32 = 1e-5;

/// Whether the next frame continues phases or starts from analysis phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Start {
    Fresh,
    Running,
}

pub(crate) struct Vocoder {
    win: usize,
    hop_s: usize,
    bins: usize,
    det_win: usize,
    det_bins: usize,
    channels: usize,

    fwd: Arc<dyn Fft<f32>>,
    inv: Arc<dyn Fft<f32>>,
    scratch: Vec<Complex<f32>>,
    window: Vec<f32>,

    /// Packed time / spectrum buffer for one channel pair.
    frame: Vec<Complex<f32>>,
    /// Half spectrum per channel.
    spec: Vec<Vec<Complex<f32>>>,

    /// Magnitude of the channel sum.
    mag: Vec<f32>,
    /// Analysis and synthesis phase (valid at peaks), current and previous.
    phase: Vec<f32>,
    phase_prev: Vec<f32>,
    synth: Vec<f32>,
    synth_prev: Vec<f32>,
    /// Peak bin owning each bin, current and previous.
    owner: Vec<usize>,
    owner_prev: Vec<usize>,
    peaks: Vec<usize>,
    /// First bin of each peak's region.
    starts: Vec<usize>,
    /// Rotation per bin (unit complex).
    rot: Vec<Complex<f32>>,

    start: Start,
}

impl fmt::Debug for Vocoder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Vocoder")
            .field("win", &self.win)
            .field("channels", &self.channels)
            .finish_non_exhaustive()
    }
}

impl Vocoder {
    pub(crate) fn new(sizes: Sizes, channels: usize) -> Self {
        let win = sizes.win;
        let bins = sizes.bins();
        let mut planner = FftPlanner::new();
        let fwd = planner.plan_fft_forward(win);
        let inv = planner.plan_fft_inverse(win);
        let scratch_len = fwd
            .get_inplace_scratch_len()
            .max(inv.get_inplace_scratch_len());

        Self {
            win,
            hop_s: sizes.hop,
            bins,
            det_win: sizes.det_win,
            det_bins: sizes.det_bins(),
            channels,
            fwd,
            inv,
            scratch: vec![Complex::default(); scratch_len],
            window: hann(win),
            frame: vec![Complex::default(); win],
            spec: vec![vec![Complex::default(); bins]; channels],
            mag: vec![0.0; bins],
            phase: vec![0.0; bins],
            phase_prev: vec![0.0; bins],
            synth: vec![0.0; bins],
            synth_prev: vec![0.0; bins],
            owner: vec![0; bins],
            owner_prev: vec![0; bins],
            peaks: Vec::with_capacity(bins),
            starts: Vec::with_capacity(bins),
            rot: vec![Complex::new(1.0, 0.0); bins],
            start: Start::Fresh,
        }
    }

    pub(crate) fn reset(&mut self) {
        self.start = Start::Fresh;
    }

    /// Processes the frame starting at `input[..][off]` and adds it to `acc`.
    /// `hop_a`: analysis hop since the previous frame. `reset`: bins (at
    /// detector resolution) whose phases restart from the analysis.
    pub(crate) fn process(
        &mut self,
        input: &[Vec<f32>],
        off: usize,
        hop_a: usize,
        reset: &[bool],
        acc: &mut [Vec<f32>],
    ) {
        for c in (0..self.channels).step_by(2) {
            self.analyse(input, off, c);
        }

        self.reference();
        self.find_peaks();
        self.lock(hop_a, reset);

        for c in (0..self.channels).step_by(2) {
            self.synthesise(acc, c);
        }

        std::mem::swap(&mut self.phase, &mut self.phase_prev);
        std::mem::swap(&mut self.synth, &mut self.synth_prev);
        std::mem::swap(&mut self.owner, &mut self.owner_prev);
        self.start = Start::Running;
    }

    /// FFT of channels `c` and `c + 1` (or silence), split into two spectra.
    fn analyse(&mut self, input: &[Vec<f32>], off: usize, c: usize) {
        let n = self.win;
        let half = n / 2;
        let left = &input[c][off..off + n];
        let right = input.get(c + 1).map(|r| &r[off..off + n]);

        // Zero-phase: sample n goes to (n + N/2) mod N.
        for (i, &w) in self.window.iter().enumerate() {
            let re = left[i] * w;
            let im = right.map_or(0.0, |r| r[i] * w);
            self.frame[(i + half) % n] = Complex::new(re, im);
        }
        self.fwd
            .process_with_scratch(&mut self.frame, &mut self.scratch);

        // Z = X + iY  →  X[k] = (Z[k] + Z*[N-k]) / 2,  Y[k] = (Z[k] - Z*[N-k]) / 2i
        for k in 0..self.bins {
            let z = self.frame[k];
            let m = self.frame[(n - k) % n].conj();
            self.spec[c][k] = (z + m) * 0.5;
            if let Some(spec) = self.spec.get_mut(c + 1) {
                spec[k] = Complex::new(z.im - m.im, m.re - z.re) * 0.5;
            }
        }
    }

    /// Magnitude of the channel sum.
    fn reference(&mut self) {
        for k in 0..self.bins {
            let sum: Complex<f32> = self.spec.iter().map(|s| s[k]).sum();
            self.mag[k] = sum.norm();
        }
    }

    /// Local maxima over ±2 bins and the region each bin belongs to
    /// (split at the lowest bin between two peaks).
    fn find_peaks(&mut self) {
        let mag = &self.mag;
        let len = self.bins;
        let loud = mag.iter().fold(0.0_f32, |a, &b| a.max(b));
        let floor = loud * PEAK_FLOOR;

        self.peaks.clear();
        for k in 0..len {
            let m = mag[k];
            if m <= floor {
                continue;
            }
            let lo = k.saturating_sub(2);
            let hi = (k + 2).min(len - 1);
            let top = (lo..=hi).all(|j| j == k || mag[j] < m || (mag[j] == m && j > k));
            if top {
                self.peaks.push(k);
            }
        }
        if self.peaks.is_empty() {
            self.peaks.push(0);
        }

        // Regions: bins up to the minimum between peaks belong to the left one.
        self.starts.clear();
        let mut from = 0;
        for i in 0..self.peaks.len() {
            let peak = self.peaks[i];
            let to = match self.peaks.get(i + 1) {
                Some(&next) => valley(mag, peak, next),
                None => len,
            };
            self.starts.push(from);
            self.owner[from..to].fill(peak);
            from = to;
        }
    }

    /// Synthesis phase per peak, rotation per bin.
    fn lock(&mut self, hop_a: usize, reset: &[bool]) {
        for i in 0..self.peaks.len() {
            let k = self.peaks[i];
            let sum: Complex<f32> = self.spec.iter().map(|s| s[k]).sum();
            let phase = sum.arg();
            self.phase[k] = phase;

            let restart = self.start == Start::Fresh || hop_a == 0 || reset[self.det_bin(k)];
            self.synth[k] = match restart {
                true => phase,
                false => self.advance(k, phase, hop_a),
            };

            // Bins of the region keep their offset to the peak.
            let (sin, cos) = (self.synth[k] - phase).sin_cos();
            let rot = Complex::new(cos, sin);
            let lo = self.starts[i];
            let hi = self.starts.get(i + 1).copied().unwrap_or(self.bins);
            self.rot[lo..hi].fill(rot);
        }
    }

    /// Phase of peak `k`, continuing the previous frame's peak that owned it.
    fn advance(&self, k: usize, phase: f32, hop_a: usize) -> f32 {
        let n = self.win;
        let prev = self.owner_prev[k];

        // Expected advance of bin k over the analysis hop, reduced mod 2π
        // in integers to keep f32 precision: 2π·(k·hop mod N)/N.
        let expect_a = TAU * ((k * hop_a) % n) as f32 / n as f32;
        let dev = wrap(phase - self.phase_prev[prev] - expect_a);

        let expect_s = TAU * ((k * self.hop_s) % n) as f32 / n as f32;
        let step = expect_s + dev * self.hop_s as f32 / hop_a as f32;
        wrap(self.synth_prev[prev] + step)
    }

    /// Main-window bin → detector bin.
    fn det_bin(&self, k: usize) -> usize {
        ((k * self.det_win + self.win / 2) / self.win).min(self.det_bins - 1)
    }

    /// Rotates channels `c`, `c + 1`, inverse FFT, windowed overlap-add.
    fn synthesise(&mut self, acc: &mut [Vec<f32>], c: usize) {
        let n = self.win;
        let half = n / 2;
        let nyq = self.bins - 1;
        let pair = self.spec.get(c + 1).is_some();

        for k in 0..self.bins {
            let mut x = self.spec[c][k] * self.rot[k];
            let mut y = match pair {
                true => self.spec[c + 1][k] * self.rot[k],
                false => Complex::default(),
            };

            // DC and Nyquist must stay real: keep the projection.
            if k == 0 || k == nyq {
                x = Complex::new(x.re, 0.0);
                y = Complex::new(y.re, 0.0);
            }

            // Z = X + iY, Hermitian halves mirrored.
            self.frame[k] = Complex::new(x.re - y.im, x.im + y.re);
            if k != 0 && k != nyq {
                self.frame[n - k] = Complex::new(x.re + y.im, y.re - x.im);
            }
        }
        self.inv
            .process_with_scratch(&mut self.frame, &mut self.scratch);

        let gain = 1.0 / (OLA_SUM * n as f32);
        for (i, &w) in self.window.iter().enumerate() {
            let z = self.frame[(i + half) % n] * (w * gain);
            acc[c][i] += z.re;
            if let Some(right) = acc.get_mut(c + 1) {
                right[i] += z.im;
            }
        }
    }
}

/// Lowest bin strictly between two peaks (inclusive boundary to the right).
fn valley(mag: &[f32], from: usize, to: usize) -> usize {
    let mut best = from + 1;
    for k in from + 1..to {
        if mag[k] < mag[best] {
            best = k;
        }
    }
    best + 1
}

/// Wraps a phase to [-π, π).
fn wrap(x: f32) -> f32 {
    x - TAU * ((x + PI) / TAU).floor()
}
