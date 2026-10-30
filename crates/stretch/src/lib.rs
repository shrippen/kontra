//! Time-stretch for backing tracks: tempo change without pitch change.
//!
//! ```text
//! push ─► input buffer ──┬──► onset detector (short FFT, spectral flux)
//!                        │            │ onsets + per-bin reset masks
//!                        │            ▼
//!                        │     planner: analysis hop per frame,
//!                        │     1:1 runs over transients, drift control
//!                        │            │
//!                        ▼            ▼
//!                  vocoder: FFT ─► phase locking on the channel sum
//!                           ─► same rotation for every channel ─► IFFT
//!                        │
//!                        ▼
//!                  overlap-add ─► output FIFO ─► pull
//! ```
//!
//! Output sample `o` lines up with input sample `o * speed`; the
//! stretcher only buffers (see [`Stretcher::latency_samples`]).
//!
//! Limits: an onset within about 0.1 s after `reset` comes out early (no
//! time to steer): measured up to 67 ms (Standard, 25 % speed) and 240 ms
//! (Fine); when transients follow closer than
//! about one window at low speed, runs may be skipped (phases still reset).

mod offline;
mod onset;
mod plan;
mod sizes;
mod vocoder;

use std::fmt;

use onset::{Detector, Onsets};
use plan::Planner;
use sizes::Sizes;
use vocoder::Vocoder;

pub use offline::{stretch_buffer, stretch_buffer_with};

/// Frequency resolution of the analysis window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Resolution {
    /// 4096 samples at 48 kHz: clean down to E1 (41 Hz partial spacing), 96 ms latency.
    #[default]
    Standard,
    /// 8192 samples at 48 kHz: clean down to B0 (31 Hz), 96 ms more latency (192 ms) and
    /// slightly softer non-percussive attacks. For five- and six-string basses.
    Fine,
}

/// Slowest supported speed (25 % tempo).
const SPEED_MIN: f64 = 0.25;
/// Fastest supported speed (150 % tempo).
const SPEED_MAX: f64 = 1.5;

/// Streaming time-stretcher. All buffers are allocated in [`Stretcher::new`];
/// `push`, `pull`, `set_speed` and `reset` never allocate.
///
/// Real-time use (fill one output block):
/// ```
/// # use kontra_stretch::Stretcher;
/// let mut st = Stretcher::new(2, 48_000, 512);
/// st.set_speed(0.75);
/// let source = vec![vec![0.0_f32; 512]; 2];
/// let mut out = vec![vec![0.0_f32; 256]; 2];
/// let mut done = 0;
/// while done < 256 {
///     let mut views: Vec<&mut [f32]> = out.iter_mut().map(|c| &mut c[done..]).collect();
///     let got = st.pull(&mut views);
///     done += got;
///     if got == 0 {
///         let need = st.required_input().min(512);
///         let chunk: Vec<&[f32]> = source.iter().map(|c| &c[..need]).collect();
///         st.push(&chunk);
///     }
/// }
/// ```
pub struct Stretcher {
    sizes: Sizes,
    channels: usize,

    /// Input per channel; `input[c][0]` is absolute sample `in_start`.
    input: Vec<Vec<f32>>,
    in_start: i64,
    in_len: usize,

    /// Overlap-add accumulator, one window long.
    acc: Vec<Vec<f32>>,

    /// Finished output, at most one synthesis hop.
    fifo: Vec<Vec<f32>>,
    fifo_pos: usize,
    fifo_len: usize,

    /// Output samples before index 0 (fade-in of the first frames), dropped.
    skip: usize,

    /// Reset mask for the current frame, at detector resolution.
    mask: Vec<bool>,

    detector: Detector,
    onsets: Onsets,
    planner: Planner,
    vocoder: Vocoder,
}

impl fmt::Debug for Stretcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Stretcher")
            .field("channels", &self.channels)
            .field("window", &self.sizes.win)
            .field("speed", &self.planner.speed())
            .finish_non_exhaustive()
    }
}

impl Stretcher {
    /// `max_block`: largest number of samples passed to one `push` or `pull`.
    /// Zero channels or block size are treated as one.
    pub fn new(channels: usize, sample_rate: u32, max_block: usize) -> Self {
        Self::with_resolution(channels, sample_rate, max_block, Resolution::Standard)
    }

    /// Like `new`, with a chosen analysis `resolution`.
    pub fn with_resolution(
        channels: usize,
        sample_rate: u32,
        max_block: usize,
        resolution: Resolution,
    ) -> Self {
        let channels = channels.max(1);
        let sizes = Sizes::new(sample_rate, resolution);

        // Room for the lookahead, one block and one maximal hop, twice over
        // so compaction stays rare.
        let cap = 2 * (sizes.lookahead() + sizes.win) + max_block.max(1);

        let mut st = Self {
            sizes,
            channels,
            input: vec![vec![0.0; cap]; channels],
            in_start: 0,
            in_len: 0,
            acc: vec![vec![0.0; sizes.win]; channels],
            fifo: vec![vec![0.0; sizes.hop]; channels],
            fifo_pos: 0,
            fifo_len: 0,
            skip: 0,
            mask: vec![false; sizes.det_bins()],
            detector: Detector::new(sizes),
            onsets: Onsets::new(sizes.det_bins()),
            planner: Planner::new(sizes),
            vocoder: Vocoder::new(sizes, channels),
        };
        st.reset();
        st
    }

    /// Target speed (0.25–1.5, clamped). Changes glide over about 0.1 s.
    pub fn set_speed(&mut self, speed: f64) {
        let speed = if speed.is_finite() { speed } else { 1.0 };
        self.planner.set_speed(speed.clamp(SPEED_MIN, SPEED_MAX));
    }

    /// Forget all audio; keeps the speed. Call on seek.
    pub fn reset(&mut self) {
        // Pretend half a window of silence precedes the stream so the first
        // frame is centred on input sample 0.
        let half = self.sizes.win / 2;
        for ch in &mut self.input {
            ch[..half].fill(0.0);
        }
        self.in_start = -(half as i64);
        self.in_len = half;

        for ch in &mut self.acc {
            ch.fill(0.0);
        }
        self.fifo_pos = 0;
        self.fifo_len = 0;
        self.skip = half;

        self.detector.reset(self.in_start);
        self.onsets.clear();
        self.planner.reset(self.in_start);
        self.vocoder.reset();
    }

    /// Buffering delay at speed 1: after `n` pushed samples at most
    /// `n - latency_samples()` output samples are available. The signal
    /// itself is not delayed (output `o` ↔ input `o * speed`).
    pub fn latency_samples(&self) -> usize {
        self.sizes.lookahead() - self.sizes.hop
    }

    /// Input samples still missing before `pull` can return more output.
    pub fn required_input(&self) -> usize {
        if self.fifo_len > self.fifo_pos {
            return 0;
        }
        let need = self.planner.pos() + self.sizes.lookahead() as i64;
        (need - self.in_end()).max(0) as usize
    }

    /// Appends input (one slice per channel; missing channels are silent).
    /// Returns the number of samples taken, less than offered when full.
    pub fn push(&mut self, input: &[&[f32]]) -> usize {
        let offered = input.iter().map(|c| c.len()).min().unwrap_or(0);
        if offered == 0 {
            return 0;
        }

        self.compact();
        let cap = self.input[0].len();
        let n = offered.min(cap - self.in_len);
        let at = self.in_len;
        for (c, dst) in self.input.iter_mut().enumerate() {
            let dst = &mut dst[at..at + n];
            match input.get(c) {
                Some(src) => dst.copy_from_slice(&src[..n]),
                None => dst.fill(0.0),
            }
        }
        self.in_len += n;

        self.detector
            .run(&self.input, self.in_start, self.in_len, &mut self.onsets);
        n
    }

    /// Writes stretched output (one slice per channel). Returns the number of
    /// samples written; fewer than asked means more input is required.
    pub fn pull(&mut self, output: &mut [&mut [f32]]) -> usize {
        let want = output.iter().map(|c| c.len()).min().unwrap_or(0);
        let mut done = 0;

        while done < want {
            if self.fifo_pos == self.fifo_len {
                if !self.ready() {
                    break;
                }
                self.frame();
                continue;
            }

            let n = (want - done).min(self.fifo_len - self.fifo_pos);
            for (c, dst) in output.iter_mut().enumerate() {
                let dst = &mut dst[done..done + n];
                match self.fifo.get(c) {
                    Some(src) => dst.copy_from_slice(&src[self.fifo_pos..self.fifo_pos + n]),
                    None => dst.fill(0.0),
                }
            }
            self.fifo_pos += n;
            done += n;
        }
        done
    }

    fn in_end(&self) -> i64 {
        self.in_start + self.in_len as i64
    }

    /// True when the next frame and all onsets it may meet are buffered.
    fn ready(&self) -> bool {
        self.planner.pos() + self.sizes.lookahead() as i64 <= self.in_end()
    }

    /// Drops input that no frame or detector step needs any more.
    fn compact(&mut self) {
        // The detector looks back one window when it confirms an onset.
        let det = self.detector.next() - self.sizes.det_win as i64;
        let keep = self.planner.pos().min(det);
        let drop = (keep - self.in_start).clamp(0, self.in_len as i64) as usize;
        if drop == 0 {
            return;
        }
        for ch in &mut self.input {
            ch.copy_within(drop..self.in_len, 0);
        }
        self.in_start += drop as i64;
        self.in_len -= drop;
    }

    /// One analysis/synthesis frame: adds one hop of finished output.
    fn frame(&mut self) {
        let pos = self.planner.pos();
        let drift = self.planner.drift();

        // Which onsets this frame touches: first touch resets phases, an
        // active run keeps the hop at 1:1 so the transient is copied once.
        self.mask.fill(false);
        let touch = self.onsets.touch(pos, &self.sizes, drift, &mut self.mask);

        let off = (pos - self.in_start) as usize;
        self.vocoder.process(
            &self.input,
            off,
            self.planner.hop_a(),
            &self.mask,
            &mut self.acc,
        );
        self.finish();

        let lead = self.onsets.next_pending(pos, &self.sizes);
        self.planner.advance(touch, lead);
    }

    /// Moves one synthesis hop from the accumulator to the FIFO.
    fn finish(&mut self) {
        let hop = self.sizes.hop;
        for (acc, fifo) in self.acc.iter_mut().zip(&mut self.fifo) {
            fifo.copy_from_slice(&acc[..hop]);
            acc.copy_within(hop.., 0);
            let len = acc.len();
            acc[len - hop..].fill(0.0);
        }

        let dropped = self.skip.min(hop);
        self.skip -= dropped;
        self.fifo_pos = dropped;
        self.fifo_len = hop;
    }
}
