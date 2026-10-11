//! Pitch shift in real time (LH-WIE-02, round 8): a phase vocoder that moves spectral peaks
//! (Laroche & Dolson 1999) instead of stretching time, so the length stays and the semitones
//! can change while it plays.
//!
//! ```text
//! L,R ─► ring (4096) ─every 1024─► window ─► FFT (L + iR) ─► peaks of |L+R|
//!                                                              │ peak k → k·2^(n/12),
//!                                                              │ its region moves along,
//!                                                              │ phase advances at the new
//!                                                              ▼ frequency
//!        delayed output ◄─ overlap-add ◄─ window ◄─ IFFT ◄─ shifted spectrum
//! ```
//!
//! One rotation per region is applied to both channels, so the stereo image stays. All
//! buffers are made in `new`: `process` allocates nothing and can run in the audio callback.
//! At 0 semitones the input passes exactly, only delayed by `LATENCY`.

use std::f32::consts::TAU;
use std::sync::Arc;

use rustfft::num_complex::Complex;
use rustfft::{Fft, FftPlanner};

/// FFT size: 11.7 Hz bins at 48 kHz, fine enough for the low E (41 Hz) and its neighbours.
const WIN: usize = 4096;
/// Four frames overlap; Hann² then sums to a constant.
const HOP: usize = WIN / 4;
const BINS: usize = WIN / 2 + 1;
/// Hann² overlap-add sum at 4x overlap.
const OLA_SUM: f32 = 1.5;
/// Peaks quieter than this, relative to the loudest bin, are ignored (−100 dB).
const PEAK_FLOOR: f32 = 1e-5;
/// A peak is the loudest bin within this many bins on each side.
const PEAK_REACH: usize = 2;
/// Largest shift offered (±6 semitones, LH-WIE-02).
pub const PITCH_RANGE_SEMITONES: f32 = 6.0;

/// Samples from an input sample to its shifted output: one window (85 ms at 48 kHz). The
/// backing track is read this far ahead, so it stays in time (the engine's job).
pub const LATENCY: usize = WIN;

/// Shifts a stereo signal by a number of semitones.
pub struct PitchShifter {
    fwd: Arc<dyn Fft<f32>>,
    inv: Arc<dyn Fft<f32>>,
    scratch: Vec<Complex<f32>>,
    window: Vec<f32>,
    /// Input history, `WIN` frames per channel, written circularly at `write`.
    input: [Vec<f32>; 2],
    write: usize,
    /// Frames since the last analysis.
    filled: usize,
    /// Overlap-add accumulator and the finished hop being read out.
    acc: [Vec<f32>; 2],
    out: [Vec<f32>; 2],
    read: usize,
    frame: Vec<Complex<f32>>,
    left: Vec<Complex<f32>>,
    right: Vec<Complex<f32>>,
    shifted_left: Vec<Complex<f32>>,
    shifted_right: Vec<Complex<f32>>,
    mag: Vec<f32>,
    phase: Vec<f32>,
    phase_prev: Vec<f32>,
    /// Synthesis phase per output bin, this frame and the last.
    synth: Vec<f32>,
    synth_prev: Vec<f32>,
    peaks: Vec<usize>,
    ratio: f32,
    semitones: f32,
}

impl std::fmt::Debug for PitchShifter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PitchShifter")
            .field("semitones", &self.semitones)
            .finish_non_exhaustive()
    }
}

impl Default for PitchShifter {
    fn default() -> Self {
        PitchShifter::new()
    }
}

impl PitchShifter {
    pub fn new() -> PitchShifter {
        let mut planner = FftPlanner::new();
        let fwd = planner.plan_fft_forward(WIN);
        let inv = planner.plan_fft_inverse(WIN);
        let scratch_len = fwd
            .get_inplace_scratch_len()
            .max(inv.get_inplace_scratch_len());
        let step = TAU / WIN as f32;
        PitchShifter {
            fwd,
            inv,
            scratch: vec![Complex::default(); scratch_len],
            window: (0..WIN)
                .map(|n| 0.5 - 0.5 * (step * n as f32).cos())
                .collect(),
            input: [vec![0.0; WIN], vec![0.0; WIN]],
            write: 0,
            filled: 0,
            acc: [vec![0.0; WIN], vec![0.0; WIN]],
            out: [vec![0.0; HOP], vec![0.0; HOP]],
            read: 0,
            frame: vec![Complex::default(); WIN],
            left: vec![Complex::default(); BINS],
            right: vec![Complex::default(); BINS],
            shifted_left: vec![Complex::default(); BINS],
            shifted_right: vec![Complex::default(); BINS],
            mag: vec![0.0; BINS],
            phase: vec![0.0; BINS],
            phase_prev: vec![0.0; BINS],
            synth: vec![0.0; BINS],
            synth_prev: vec![0.0; BINS],
            peaks: Vec::with_capacity(BINS),
            ratio: 1.0,
            semitones: 0.0,
        }
    }

    /// New shift, within ±`PITCH_RANGE_SEMITONES`; phases run on, so it changes without a gap.
    pub fn set_semitones(&mut self, semitones: f32) {
        self.semitones = semitones.clamp(-PITCH_RANGE_SEMITONES, PITCH_RANGE_SEMITONES);
        self.ratio = 2f32.powf(self.semitones / 12.0);
    }

    pub fn semitones(&self) -> f32 {
        self.semitones
    }

    /// Forgets all history (after a jump in the input).
    pub fn reset(&mut self) {
        for c in 0..2 {
            self.input[c].iter_mut().for_each(|s| *s = 0.0);
            self.acc[c].iter_mut().for_each(|s| *s = 0.0);
            self.out[c].iter_mut().for_each(|s| *s = 0.0);
        }
        self.phase_prev.iter_mut().for_each(|p| *p = 0.0);
        self.synth_prev.iter_mut().for_each(|p| *p = 0.0);
        self.write = 0;
        self.filled = 0;
        self.read = 0;
    }

    /// One stereo frame in, the frame from `LATENCY` frames ago (shifted) out.
    pub fn process(&mut self, left: f32, right: f32) -> (f32, f32) {
        let out = (self.out[0][self.read], self.out[1][self.read]);
        self.read += 1;
        self.input[0][self.write] = left;
        self.input[1][self.write] = right;
        self.write = (self.write + 1) % WIN;
        self.filled += 1;
        if self.filled == HOP {
            self.filled = 0;
            self.read = 0;
            self.step();
        }
        out
    }

    /// One analysis-synthesis frame over the last `WIN` input frames; fills `out` with the
    /// next finished hop.
    fn step(&mut self) {
        if self.semitones == 0.0 {
            self.pass_through();
        } else {
            self.analyse();
            self.find_peaks();
            self.shift();
            self.synthesise();
        }
        std::mem::swap(&mut self.phase, &mut self.phase_prev);
        std::mem::swap(&mut self.synth, &mut self.synth_prev);

        // The oldest hop of the accumulator is complete: hand it out and move the rest up.
        for c in 0..2 {
            self.out[c].copy_from_slice(&self.acc[c][..HOP]);
            self.acc[c].copy_within(HOP.., 0);
            self.acc[c][WIN - HOP..].iter_mut().for_each(|s| *s = 0.0);
        }
    }

    /// At 0 semitones: the oldest hop of the input, exactly, at the same delay as a shift.
    /// The analysis still runs, so a later shift starts with valid phases.
    fn pass_through(&mut self) {
        self.analyse();
        for c in 0..2 {
            for i in 0..HOP {
                let sample = self.input[c][(self.write + i) % WIN];
                self.acc[c][i] = sample;
            }
            self.acc[c][HOP..].iter_mut().for_each(|s| *s = 0.0);
        }
        // Keep the synthesis phases in step with the analysis.
        self.synth.copy_from_slice(&self.phase);
    }

    /// Windowed FFT of both channels in one complex transform (zero-phase: the window centre
    /// at sample 0), split into the two half spectra; magnitude and phase of their sum.
    fn analyse(&mut self) {
        let half = WIN / 2;
        for i in 0..WIN {
            let at = (self.write + i) % WIN;
            let w = self.window[i];
            self.frame[(i + half) % WIN] =
                Complex::new(self.input[0][at] * w, self.input[1][at] * w);
        }
        self.fwd
            .process_with_scratch(&mut self.frame, &mut self.scratch);
        // Z = X + iY  →  X[k] = (Z[k] + Z*[N−k]) / 2,  Y[k] = (Z[k] − Z*[N−k]) / 2i
        for k in 0..BINS {
            let z = self.frame[k];
            let m = self.frame[(WIN - k) % WIN].conj();
            self.left[k] = (z + m) * 0.5;
            self.right[k] = Complex::new(z.im - m.im, m.re - z.re) * 0.5;
            let sum = self.left[k] + self.right[k];
            self.mag[k] = sum.norm();
            self.phase[k] = sum.arg();
        }
    }

    fn find_peaks(&mut self) {
        let loud = self.mag.iter().fold(0.0_f32, |a, &b| a.max(b));
        let floor = loud * PEAK_FLOOR;
        self.peaks.clear();
        for k in 1..BINS - 1 {
            let m = self.mag[k];
            if m <= floor {
                continue;
            }
            let from = k.saturating_sub(PEAK_REACH);
            let to = (k + PEAK_REACH).min(BINS - 1);
            if (from..=to).all(|j| j == k || self.mag[j] < m) {
                self.peaks.push(k);
            }
        }
    }

    /// Moves each peak's region to `k · ratio` and turns it to the peak's new phase: the
    /// previous synthesis phase there, advanced by the peak's frequency × ratio over one hop.
    fn shift(&mut self) {
        self.shifted_left
            .iter_mut()
            .for_each(|b| *b = Complex::default());
        self.shifted_right
            .iter_mut()
            .for_each(|b| *b = Complex::default());
        self.synth.iter_mut().for_each(|p| *p = 0.0);
        let hop = HOP as f32;
        for (i, &k) in self.peaks.iter().enumerate() {
            // The region runs halfway to the neighbouring peaks.
            let start = if i == 0 {
                0
            } else {
                (self.peaks[i - 1] + k).div_ceil(2)
            };
            let end = self
                .peaks
                .get(i + 1)
                .map_or(BINS, |&next| (k + next).div_ceil(2));

            let bin_w = TAU * k as f32 / WIN as f32;
            let deviation = wrap(self.phase[k] - self.phase_prev[k] - bin_w * hop);
            let omega = bin_w + deviation / hop;
            let target = (k as f32 * self.ratio).round() as usize;
            if target == 0 || target >= BINS {
                continue;
            }
            let new_phase = self.synth_prev[target] + omega * self.ratio * hop;
            self.synth[target] = new_phase;
            let turn = Complex::from_polar(1.0, new_phase - self.phase[k]);

            let offset = target as isize - k as isize;
            for j in start..end {
                let to = j as isize + offset;
                if to <= 0 || to >= BINS as isize {
                    continue;
                }
                let to = to as usize;
                self.shifted_left[to] += self.left[j] * turn;
                self.shifted_right[to] += self.right[j] * turn;
            }
        }
    }

    /// Inverse FFT of both shifted spectra at once, windowed, added to the accumulator.
    fn synthesise(&mut self) {
        let half = WIN / 2;
        // Rebuild the full spectrum of L + iR from the two half spectra of real signals:
        // Z[k] = X[k] + i·Y[k],  Z[N−k] = X̄[k] + i·Ȳ[k].
        let times_i = |z: Complex<f32>| Complex::new(-z.im, z.re);
        for k in 0..BINS {
            let l = self.shifted_left[k];
            let r = self.shifted_right[k];
            self.frame[k] = l + times_i(r);
            if k > 0 && k < WIN - k {
                self.frame[WIN - k] = l.conj() + times_i(r.conj());
            }
        }
        self.inv
            .process_with_scratch(&mut self.frame, &mut self.scratch);
        let gain = 1.0 / (WIN as f32 * OLA_SUM);
        for i in 0..WIN {
            let z = self.frame[(i + half) % WIN] * gain;
            let w = self.window[i];
            self.acc[0][i] += z.re * w;
            self.acc[1][i] += z.im * w;
        }
    }
}

/// Phase into −π..π.
fn wrap(x: f32) -> f32 {
    x - TAU * (x / TAU).round()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::yin::Yin;
    use std::f64::consts::PI;

    const SR: f64 = 48_000.0;

    fn run(shifter: &mut PitchShifter, input: &[f32]) -> Vec<f32> {
        input.iter().map(|&x| shifter.process(x, x).0).collect()
    }

    fn sine(hz: f64, n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| (0.5 * (2.0 * PI * hz * i as f64 / SR).sin()) as f32)
            .collect()
    }

    fn cents(samples: &[f32], expected_hz: f64) -> f64 {
        let mut yin = Yin::new(SR, 40.0, 800.0, 0.04);
        let hz = yin.estimate(samples).map_or(0.0, |e| e.hz);
        1200.0 * (hz / expected_hz).log2()
    }

    #[test]
    fn zero_semitones_is_the_input_delayed() {
        let mut shifter = PitchShifter::new();
        let input = sine(110.0, 20_000);
        let out = run(&mut shifter, &input);
        let max_diff = (LATENCY..out.len())
            .map(|i| (out[i] - input[i - LATENCY]).abs())
            .fold(0.0, f32::max);
        assert!(max_diff < 1e-6, "{max_diff}");
    }

    #[test]
    fn shifts_bass_notes_by_semitones() {
        for (hz, semitones) in [(55.0, 2.0), (110.0, -6.0), (82.4, 5.0), (220.0, 3.0)] {
            let mut shifter = PitchShifter::new();
            shifter.set_semitones(semitones);
            let out = run(&mut shifter, &sine(hz, 48_000));
            let expected = hz * 2f64.powf(f64::from(semitones) / 12.0);
            let off = cents(&out[20_000..44_000], expected);
            assert!(off.abs() < 5.0, "{hz} Hz {semitones:+}: {off} cents");
            // The level stays about the same.
            let rms = |s: &[f32]| (s.iter().map(|x| x * x).sum::<f32>() / s.len() as f32).sqrt();
            let gain = rms(&out[20_000..44_000]) / rms(&sine(hz, 24_000));
            assert!(
                (0.7..1.3).contains(&gain),
                "{hz} Hz {semitones:+}: gain {gain}"
            );
        }
    }

    #[test]
    fn changing_the_shift_keeps_playing() {
        let mut shifter = PitchShifter::new();
        shifter.set_semitones(2.0);
        let input = sine(110.0, 96_000);
        let mut out = run(&mut shifter, &input[..48_000]);
        shifter.set_semitones(4.0);
        out.extend(run(&mut shifter, &input[48_000..]));
        let expected = 110.0 * 2f64.powf(4.0 / 12.0);
        assert!(cents(&out[70_000..94_000], expected).abs() < 5.0);
        // No gap: the level never drops out around the change.
        let quietest = out[48_000..60_000]
            .chunks(480)
            .map(|c| c.iter().fold(0.0f32, |m, s| m.max(s.abs())))
            .fold(f32::INFINITY, f32::min);
        assert!(quietest > 0.2, "{quietest}");
    }

    #[test]
    fn shifted_notes_start_on_time() {
        // A note starting at 1 s comes out LATENCY later, shifted or not (±5 ms), so the
        // backing stays on the beat.
        let onset = |out: &[f32]| {
            let peak = out.iter().fold(0.0f32, |m, s| m.max(s.abs()));
            out.iter().position(|s| s.abs() > peak * 0.3).unwrap_or(0)
        };
        let mut input = vec![0.0f32; 48_000];
        input.extend(sine(110.0, 24_000));
        for semitones in [0.0, 4.0, -5.0] {
            let mut shifter = PitchShifter::new();
            shifter.set_semitones(semitones);
            let out = run(&mut shifter, &input);
            let late = onset(&out) as f64 - (48_000 + LATENCY) as f64;
            assert!(late.abs() < 0.005 * SR, "{semitones:+}: {late} samples");
        }
    }

    #[test]
    fn stereo_stays_apart() {
        let mut shifter = PitchShifter::new();
        shifter.set_semitones(3.0);
        let left = sine(110.0, 48_000);
        let mut right_out = Vec::new();
        let mut left_out = Vec::new();
        for &l in &left {
            let (a, b) = shifter.process(l, 0.0);
            left_out.push(a);
            right_out.push(b);
        }
        let energy = |s: &[f32]| s.iter().map(|x| x * x).sum::<f32>();
        assert!(energy(&right_out[20_000..]) < energy(&left_out[20_000..]) * 1e-3);
    }

    #[test]
    fn range_is_clamped() {
        let mut shifter = PitchShifter::new();
        shifter.set_semitones(20.0);
        assert_eq!(shifter.semitones(), PITCH_RANGE_SEMITONES);
    }
}
