//! Onset detection (spectral flux) and the queue of detected onsets.
//!
//! The detector runs on the mono sum with a short window (¼ of the main
//! window). An onset is a peak of the log-magnitude flux above an adaptive
//! threshold. With it goes a reset mask: the bins whose magnitude rose by
//! more than 12 dB, i.e. the bins the transient owns. Phases are reset only
//! there, so a sustained bass note under a hi-hat keeps its phase.

use std::fmt;
use std::sync::Arc;

use rustfft::num_complex::Complex;
use rustfft::{Fft, FftPlanner};

use crate::plan::{Lead, Touch};
use crate::sizes::Sizes;

/// Linear magnitude spectra kept (current and four before).
const HISTORY: usize = 5;
/// Frames between "before" and "after" spectra for the reset mask.
const MASK_SPAN: usize = 4;
/// Flux values averaged for the adaptive threshold (≈ 170 ms at 48 kHz).
const MEAN_LEN: usize = 32;
/// Threshold = factor × mean flux + floor.
const MEAN_FACTOR: f32 = 1.5;
const FLUX_FLOOR: f32 = 0.02;
/// Log compression: ln(1 + COMPRESS · magnitude).
const COMPRESS: f32 = 1000.0;
/// Magnitude rise that marks a bin as part of the transient (12 dB).
const RISE: f32 = 4.0;
/// Bins quieter than this, relative to the loudest, are never reset (-60 dB).
const MASK_FLOOR: f32 = 1e-3;
/// Minimum distance between onsets, in detector hops (≈ 21 ms).
const MIN_GAP_HOPS: i64 = 4;
/// Onsets queued at most; more are dropped (never reached in music).
const CAPACITY: usize = 64;
/// Frames holding the attack only in the first 1/16 of the window
/// (Hann weight < 0.04) may leave the run.
const GUARD_DIV: usize = 16;

pub(crate) struct Detector {
    win: usize,
    hop: usize,
    bins: usize,
    fft: Arc<dyn Fft<f32>>,
    buf: Vec<Complex<f32>>,
    scratch: Vec<Complex<f32>>,
    window: Vec<f32>,

    /// Ring of linear magnitude spectra, slot = frame % HISTORY.
    lin: Vec<Vec<f32>>,
    log_prev: Vec<f32>,
    /// Flux of frames t-2, t-1, t.
    flux: [f32; 3],
    mean: Vec<f32>,
    mean_sum: f32,

    frames: usize,
    /// Absolute start of the next detector frame.
    next: i64,
    last_onset: i64,
}

impl fmt::Debug for Detector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Detector")
            .field("win", &self.win)
            .field("next", &self.next)
            .finish_non_exhaustive()
    }
}

impl Detector {
    pub(crate) fn new(sizes: Sizes) -> Self {
        let win = sizes.det_win;
        let fft = FftPlanner::new().plan_fft_forward(win);
        let scratch = vec![Complex::default(); fft.get_inplace_scratch_len()];
        let bins = sizes.det_bins();

        Self {
            win,
            hop: sizes.det_hop,
            bins,
            fft,
            buf: vec![Complex::default(); win],
            scratch,
            window: hann(win),
            lin: vec![vec![0.0; bins]; HISTORY],
            log_prev: vec![0.0; bins],
            flux: [0.0; 3],
            mean: vec![0.0; MEAN_LEN],
            mean_sum: 0.0,
            frames: 0,
            next: 0,
            last_onset: i64::MIN / 2,
        }
    }

    pub(crate) fn reset(&mut self, start: i64) {
        for spec in &mut self.lin {
            spec.fill(0.0);
        }
        self.log_prev.fill(0.0);
        self.flux = [0.0; 3];
        self.mean.fill(0.0);
        self.mean_sum = 0.0;
        self.frames = 0;
        self.next = start;
        self.last_onset = i64::MIN / 2;
    }

    pub(crate) fn next(&self) -> i64 {
        self.next
    }

    /// Analyses every complete detector frame in the buffer.
    pub(crate) fn run(&mut self, input: &[Vec<f32>], start: i64, len: usize, out: &mut Onsets) {
        let end = start + len as i64;
        while self.next + self.win as i64 <= end {
            let off = (self.next - start) as usize;
            self.step(input, off, out);
            self.frames += 1;
            self.next += self.hop as i64;
        }
    }

    fn step(&mut self, input: &[Vec<f32>], off: usize, out: &mut Onsets) {
        self.spectrum(input, off);
        let flux = self.flux_now();
        self.flux = [self.flux[1], self.flux[2], flux];

        // Peak picking needs frame t to confirm the peak at t-1.
        if self.frames >= 2 {
            self.pick(input, off, out);
        }

        let slot = self.frames % MEAN_LEN;
        self.mean_sum += self.flux[1] - self.mean[slot];
        self.mean[slot] = self.flux[1];
    }

    /// Windowed mono FFT into the magnitude ring.
    fn spectrum(&mut self, input: &[Vec<f32>], off: usize) {
        let gain = 1.0 / input.len().max(1) as f32;
        for (n, (b, w)) in self.buf.iter_mut().zip(&self.window).enumerate() {
            let sum: f32 = input.iter().map(|c| c[off + n]).sum();
            *b = Complex::new(sum * gain * w, 0.0);
        }
        self.fft
            .process_with_scratch(&mut self.buf, &mut self.scratch);

        // Normalise so a full-scale sine peaks near 1.
        let norm = 4.0 / self.win as f32;
        let slot = self.frames % HISTORY;
        for (m, b) in self.lin[slot].iter_mut().zip(&self.buf) {
            *m = b.norm() * norm;
        }
    }

    /// Mean rectified rise of the log-compressed magnitudes.
    fn flux_now(&mut self) -> f32 {
        let slot = self.frames % HISTORY;
        let mut sum = 0.0;
        for (prev, m) in self.log_prev.iter_mut().zip(&self.lin[slot]) {
            let log = (COMPRESS * m).ln_1p();
            sum += (log - *prev).max(0.0);
            *prev = log;
        }
        sum / self.bins as f32
    }

    /// Reports frame t-1 as onset when its flux is a peak above threshold.
    fn pick(&mut self, input: &[Vec<f32>], off: usize, out: &mut Onsets) {
        let [before, cand, after] = self.flux;
        let thr = MEAN_FACTOR * self.mean_sum / MEAN_LEN as f32 + FLUX_FLOOR;
        if cand <= before || cand < after || cand <= thr {
            return;
        }

        let frame = self.next - self.hop as i64;
        if frame - self.last_onset < MIN_GAP_HOPS * self.hop as i64 {
            return;
        }
        self.last_onset = frame;

        // The flux peaks while the onset sits in the right half of the
        // detector frame: between a quarter and the end.
        let quarter = self.win as i64 / 4;
        let (from, end) = (frame + quarter, frame + self.win as i64);
        let off = off - self.hop;
        let peak = from + attack(input, off + self.win / 4, off + self.win) as i64;

        // The onset starts a little before its steepest step (rise time).
        let Some(mask) = out.push(peak - quarter, end, peak) else {
            return;
        };

        let now = &self.lin[self.frames % HISTORY];
        let old = &self.lin[(self.frames + HISTORY - MASK_SPAN) % HISTORY];
        let loud = now.iter().fold(0.0_f32, |a, &b| a.max(b));
        for ((m, &n), &o) in mask.iter_mut().zip(now).zip(old) {
            *m = n > RISE * o && n > MASK_FLOOR * loud;
        }
    }
}

/// Offset (from `from`) of the steepest mono step in `input[..][from..to]`:
/// the attack. The first difference ignores a loud bass under the hit.
fn attack(input: &[Vec<f32>], from: usize, to: usize) -> usize {
    let mono = |i: usize| input.iter().map(|c| c[i]).sum::<f32>();
    let mut best = (0, 0.0_f32);
    let mut prev = mono(from.saturating_sub(1));
    for i in from..to {
        let now = mono(i);
        let step = (now - prev).abs();
        prev = now;
        if step > best.1 {
            best = (i - from, step);
        }
    }
    best.0
}

/// Periodic Hann window.
pub(crate) fn hann(len: usize) -> Vec<f32> {
    let step = std::f64::consts::TAU / len as f64;
    (0..len)
        .map(|n| (0.5 - 0.5 * (step * n as f64).cos()) as f32)
        .collect()
}

/// What the planner did with an onset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// Not yet inside any frame.
    Pending,
    /// Phases reset, hop held at 1:1 while the onset is in the window.
    Run,
    /// Phases reset only; drift too large for a run.
    Plain,
}

#[derive(Debug, Clone, Copy)]
struct Onset {
    /// Interval that holds the onset (absolute input samples).
    start: i64,
    end: i64,
    /// Steepest step in the interval: the attack the run is centred on.
    peak: i64,
    state: State,
}

/// Fixed-capacity FIFO of onsets with their reset masks.
#[derive(Debug)]
pub(crate) struct Onsets {
    items: Vec<Onset>,
    masks: Vec<bool>,
    bins: usize,
    head: usize,
    tail: usize,
}

impl Onsets {
    pub(crate) fn new(bins: usize) -> Self {
        let idle = Onset {
            start: 0,
            end: 0,
            peak: 0,
            state: State::Plain,
        };
        Self {
            items: vec![idle; CAPACITY],
            masks: vec![false; CAPACITY * bins],
            bins,
            head: 0,
            tail: 0,
        }
    }

    pub(crate) fn clear(&mut self) {
        self.head = 0;
        self.tail = 0;
    }

    /// Queues an onset; returns its mask to fill, or `None` when full.
    fn push(&mut self, start: i64, end: i64, peak: i64) -> Option<&mut [bool]> {
        if self.tail - self.head == CAPACITY {
            return None;
        }
        let slot = self.tail % CAPACITY;
        self.tail += 1;
        self.items[slot] = Onset {
            start,
            end,
            peak,
            state: State::Pending,
        };
        Some(&mut self.masks[slot * self.bins..(slot + 1) * self.bins])
    }

    /// Applies the onsets that the frame at `pos` overlaps: ORs the reset
    /// masks of first touches into `mask` and reports whether a run holds.
    pub(crate) fn touch(
        &mut self,
        pos: i64,
        sizes: &Sizes,
        drift: f64,
        mask: &mut [bool],
    ) -> Touch {
        let win = sizes.win as i64;
        let guard = win / GUARD_DIV as i64;

        // Onsets behind the frame are done.
        while self.head < self.tail && self.items[self.head % CAPACITY].end < pos {
            self.head += 1;
        }

        let mut touch = Touch::Free;
        for i in self.head..self.tail {
            let slot = i % CAPACITY;
            let onset = &mut self.items[slot];
            if onset.start >= pos + win {
                break;
            }

            if onset.state == State::Pending {
                onset.state = match drift.abs() <= sizes.win as f64 {
                    true => State::Run,
                    false => State::Plain,
                };
                let own = &self.masks[slot * self.bins..(slot + 1) * self.bins];
                for (m, &o) in mask.iter_mut().zip(own) {
                    *m |= o;
                }
            }

            // Keep 1:1 while this frame holds the attack clear of its left
            // edge; later frames see it only under the guard (or not).
            if onset.state == State::Run && pos + guard <= onset.peak {
                touch = Touch::Run;
            }
        }
        touch
    }

    /// Whether an untouched onset enters the window within the lead.
    pub(crate) fn next_pending(&self, pos: i64, sizes: &Sizes) -> Lead {
        let reach = pos + (sizes.win + sizes.lead + sizes.hop) as i64;
        let near = (self.head..self.tail)
            .map(|i| &self.items[i % CAPACITY])
            .take_while(|o| o.start < reach)
            .find(|o| o.state == State::Pending);
        match near {
            Some(o) => Lead::Onset(o.peak),
            None => Lead::Clear,
        }
    }
}
