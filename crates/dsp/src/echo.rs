//! Removes Kontra's own backing from a microphone signal (LH-EIN-11): Kontra knows what it sends
//! to the speakers; an adaptive filter learns the way from speaker through the room into the
//! microphone and subtracts it, so the note tracker hears the bass, not the band.
//!
//! Partitioned-block frequency-domain adaptive filter (PBFDAF, overlap-save, constrained),
//! normalised per bin:
//!
//! ```text
//! reference (what plays) ─► FFT per block ─► history of 16 spectra ─┐
//!                                                                    ▼
//! microphone ──────────────────────────────────────► − Σ W·X ─► cleaned (to the tracker)
//!                                                         ▲          │
//!                                       W += μ·X̄·E / power ◄─ FFT ◄──┘
//! ```
//!
//! 16 partitions of 512 samples cover 170 ms at 48 kHz: device round trip and a living room.
//! All buffers are made in `new`; `process` allocates nothing.

use std::sync::Arc;

use rustfft::num_complex::Complex;
use rustfft::{Fft, FftPlanner};

/// Samples per block.
pub const ECHO_BLOCK: usize = 512;
const FFT: usize = 2 * ECHO_BLOCK;
/// Filter length in blocks.
const PARTITIONS: usize = 16;
/// Step size of the normalised update; small, because the bass plays all the time ("double
/// talk") and would otherwise pull the filter away.
const STEP: f32 = 1.0;
/// Smoothing of the reference and error power per bin (per block).
const POWER_SMOOTHING: f32 = 0.9;
/// Keeps the update finite in silent bins.
const POWER_FLOOR: f32 = 1e-6;

/// Subtracts a known reference, as heard through a room, from a signal.
pub struct EchoCanceller {
    fwd: Arc<dyn Fft<f32>>,
    inv: Arc<dyn Fft<f32>>,
    scratch: Vec<Complex<f32>>,
    /// Reference spectra of the last `PARTITIONS` blocks, newest first (ring at `newest`).
    history: Vec<Vec<Complex<f32>>>,
    newest: usize,
    weights: Vec<Vec<Complex<f32>>>,
    power: Vec<f32>,
    /// Error power per bin: where it is far above what the reference explains, the bass plays
    /// ("double talk") and the filter learns slower there.
    error_power: Vec<f32>,
    /// The previous reference block (overlap-save keeps two blocks per FFT).
    previous: Vec<f32>,
    buffer: Vec<Complex<f32>>,
    echo: Vec<Complex<f32>>,
    error: Vec<f32>,
}

impl std::fmt::Debug for EchoCanceller {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EchoCanceller")
    }
}

impl Default for EchoCanceller {
    fn default() -> Self {
        EchoCanceller::new()
    }
}

impl EchoCanceller {
    pub fn new() -> EchoCanceller {
        let mut planner = FftPlanner::new();
        let fwd = planner.plan_fft_forward(FFT);
        let inv = planner.plan_fft_inverse(FFT);
        let scratch_len = fwd
            .get_inplace_scratch_len()
            .max(inv.get_inplace_scratch_len());
        EchoCanceller {
            fwd,
            inv,
            scratch: vec![Complex::default(); scratch_len],
            history: vec![vec![Complex::default(); FFT]; PARTITIONS],
            newest: 0,
            weights: vec![vec![Complex::default(); FFT]; PARTITIONS],
            power: vec![POWER_FLOOR; FFT],
            error_power: vec![POWER_FLOOR; FFT],
            previous: vec![0.0; ECHO_BLOCK],
            buffer: vec![Complex::default(); FFT],
            echo: vec![Complex::default(); FFT],
            error: vec![0.0; ECHO_BLOCK],
        }
    }

    /// Forgets the learnt room (new devices, the speakers moved).
    pub fn reset(&mut self) {
        for p in 0..PARTITIONS {
            self.history[p]
                .iter_mut()
                .for_each(|c| *c = Complex::default());
            self.weights[p]
                .iter_mut()
                .for_each(|c| *c = Complex::default());
        }
        self.power.iter_mut().for_each(|p| *p = POWER_FLOOR);
        self.error_power.iter_mut().for_each(|p| *p = POWER_FLOOR);
        self.previous.iter_mut().for_each(|s| *s = 0.0);
    }

    /// One block: `mic` (what the microphone heard) and `reference` (what played at the same
    /// time), both `ECHO_BLOCK` long. Writes the cleaned microphone signal into `mic`.
    pub fn process(&mut self, mic: &mut [f32], reference: &[f32]) {
        if mic.len() != ECHO_BLOCK || reference.len() != ECHO_BLOCK {
            return;
        }
        let gain = 1.0 / FFT as f32;

        // Reference spectrum of [previous block, this block].
        self.newest = (self.newest + PARTITIONS - 1) % PARTITIONS;
        let (older, newer) = self.buffer.split_at_mut(ECHO_BLOCK);
        for ((o, n), (p, r)) in older
            .iter_mut()
            .zip(newer)
            .zip(self.previous.iter().zip(reference))
        {
            *o = Complex::new(*p, 0.0);
            *n = Complex::new(*r, 0.0);
        }
        self.previous.copy_from_slice(reference);
        self.fwd
            .process_with_scratch(&mut self.buffer, &mut self.scratch);
        self.history[self.newest].copy_from_slice(&self.buffer);
        for k in 0..FFT {
            let x = self.buffer[k].norm_sqr();
            self.power[k] = POWER_SMOOTHING * self.power[k] + (1.0 - POWER_SMOOTHING) * x;
        }

        // Echo estimate: Σ W[p]·X[p], the last block of its inverse (overlap-save).
        self.echo.iter_mut().for_each(|c| *c = Complex::default());
        for p in 0..PARTITIONS {
            let x = &self.history[(self.newest + p) % PARTITIONS];
            for (e, (w, x)) in self.echo.iter_mut().zip(self.weights[p].iter().zip(x)) {
                *e += w * x;
            }
        }
        self.inv
            .process_with_scratch(&mut self.echo, &mut self.scratch);
        for ((m, e), y) in mic
            .iter_mut()
            .zip(self.error.iter_mut())
            .zip(&self.echo[ECHO_BLOCK..])
        {
            *e = *m - y.re * gain;
            *m = *e;
        }

        // Error spectrum of [zeros, error].
        for i in 0..ECHO_BLOCK {
            self.buffer[i] = Complex::default();
            self.buffer[ECHO_BLOCK + i] = Complex::new(self.error[i], 0.0);
        }
        self.fwd
            .process_with_scratch(&mut self.buffer, &mut self.scratch);
        for k in 0..FFT {
            let e = self.buffer[k].norm_sqr();
            self.error_power[k] =
                POWER_SMOOTHING * self.error_power[k] + (1.0 - POWER_SMOOTHING) * e;
        }

        // Constrained update per partition: gradient X̄·E / power, its second half zeroed in
        // time (a linear, not a circular, correlation). Per bin the step shrinks where the error
        // outweighs the reference: there the bass, not the room, makes the error.
        for p in 0..PARTITIONS {
            let x = &self.history[(self.newest + p) % PARTITIONS];
            for (k, (g, x)) in self.echo.iter_mut().zip(x).enumerate() {
                let (power, error) = (self.power[k], self.error_power[k]);
                let share = power / (power + error + POWER_FLOOR);
                *g = x.conj() * self.buffer[k] * share / (power + POWER_FLOOR);
            }
            self.inv
                .process_with_scratch(&mut self.echo, &mut self.scratch);
            for i in 0..FFT {
                self.echo[i] = if i < ECHO_BLOCK {
                    self.echo[i] * gain
                } else {
                    Complex::default()
                };
            }
            self.fwd
                .process_with_scratch(&mut self.echo, &mut self.scratch);
            // All partitions learn from the same error: together one step of `STEP`.
            for (w, g) in self.weights[p].iter_mut().zip(&self.echo) {
                *w += g * (STEP / PARTITIONS as f32);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::Noise;

    /// A room: 3 ms direct path at 0.6, a reflection at 20 ms, one at 60 ms.
    fn through_room(reference: &[f32]) -> Vec<f32> {
        let taps = [(144usize, 0.6f32), (960, 0.3), (2880, -0.15)];
        (0..reference.len())
            .map(|i| {
                taps.iter()
                    .filter_map(|&(d, g)| i.checked_sub(d).map(|j| reference[j] * g))
                    .sum()
            })
            .collect()
    }

    fn energy(s: &[f32]) -> f32 {
        s.iter().map(|x| x * x).sum::<f32>() / s.len().max(1) as f32
    }

    #[test]
    fn learns_the_room_and_removes_the_band() {
        let mut noise = Noise::new(3);
        let band: Vec<f32> = (0..48_000 * 6).map(|_| noise.sample() * 0.3).collect();
        let heard = through_room(&band);
        let mut canceller = EchoCanceller::new();
        let mut cleaned = Vec::new();
        let blocks = heard.as_chunks::<ECHO_BLOCK>().0;
        for (mic, reference) in blocks.iter().zip(band.as_chunks::<ECHO_BLOCK>().0) {
            let mut block = mic.to_vec();
            canceller.process(&mut block, reference);
            cleaned.extend(block);
        }
        // After 3 s the band is 30 dB down.
        let tail = 48_000 * 3..cleaned.len().min(heard.len());
        let drop = 10.0 * (energy(&cleaned[tail.clone()]) / energy(&heard[tail])).log10();
        assert!(drop < -30.0, "{drop} dB");
    }

    #[test]
    fn keeps_what_the_band_did_not_play() {
        // The bass (a sine nobody sent to the speakers) survives while the band goes.
        let mut noise = Noise::new(5);
        let band: Vec<f32> = (0..48_000 * 6).map(|_| noise.sample() * 0.3).collect();
        let bass: Vec<f32> = (0..band.len())
            .map(|i| 0.3 * (std::f32::consts::TAU * 55.0 * i as f32 / 48_000.0).sin())
            .collect();
        let heard: Vec<f32> = through_room(&band)
            .iter()
            .zip(&bass)
            .map(|(a, b)| a + b)
            .collect();
        let mut canceller = EchoCanceller::new();
        let mut cleaned = Vec::new();
        let blocks = heard.as_chunks::<ECHO_BLOCK>().0;
        for (mic, reference) in blocks.iter().zip(band.as_chunks::<ECHO_BLOCK>().0) {
            let mut block = mic.to_vec();
            canceller.process(&mut block, reference);
            cleaned.extend(block);
        }
        let tail = 48_000 * 4..cleaned.len().min(heard.len());
        let residue: Vec<f32> = cleaned[tail.clone()]
            .iter()
            .zip(&bass[tail.clone()])
            .map(|(c, b)| c - b)
            .collect();
        let left = 10.0 * (energy(&residue) / energy(&bass[tail])).log10();
        assert!(left < -20.0, "bass kept, band gone: residue {left} dB");
    }
}
