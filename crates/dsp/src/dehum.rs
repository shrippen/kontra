//! Mains hum removed before detection (E8, owner 29.10.2026): the grid's frequency is followed
//! and its hum, a fundamental with harmonics of slowly changing level and phase, subtracted.
//! Notches would cut notes on the same frequencies (G1 49 Hz, G2 98 Hz); a subtracted model
//! leaves them as they are.
//!
//! ```text
//! input ─┬────────────────────────────────────────────── − ──► output
//!        │                                                ▲ Σ Re(a_k · e^{i·k·φ})
//!        └─► every 100 ms: a_k at k · f, k = 1 … 8 ──► steady? (no note) ─► follow f, learn a_k
//!                                                        subtract once steady for 2 s
//! ```
//!
//! A played note never becomes the model: it rotates against the grid (G1 is 1 Hz off 50 Hz),
//! and one inside the grid's tolerance decays faster than hum may change while it is learned.
//! While anything else sounds the model is held, not learned.

use std::f64::consts::TAU;
use std::ops::{Add, Mul, Sub};

/// Mains grids: 50 Hz (Europe, most of the world), 60 Hz (the Americas, parts of Asia).
const GRIDS_HZ: [f64; 2] = [50.0, 60.0];
/// How far a grid's frequency may lie from nominal: 49.8–50.2 Hz, with a margin.
const DRIFT_HZ: f64 = 0.25;
/// Harmonics followed up to the top of the bass's range (G4 392 Hz on 24 frets).
const TOP_HZ: f64 = 420.0;
const MAX_PARTIALS: usize = 8;
/// One analysis block: whole periods of both grids (5 of 50 Hz, 6 of 60 Hz), so each grid's
/// hum vanishes in the other's partials.
const BLOCK_S: f64 = 0.1;
/// A partial is steady when a block differs from the one before (its turn on the grid
/// removed) by at most this share (±1.2 dB or ±9°). G1 on 50 Hz hum turns 36° a block.
const STEADY: f64 = 0.15;
/// Share of a steady block's measurement taken into the model …
const LEARN: f64 = 0.5;
/// … and of the measured frequency offset into the grid's frequency (the offset of one block
/// pair is noisy: ±0.01 Hz on the laptop jack).
const FOLLOW: f64 = 0.25;
/// Steady blocks (summed) before the hum is subtracted.
const PRESENT_S: f64 = 2.0;
/// While that is gathered, the reference partial stays within this factor (±2 dB) of where it
/// began; a plucked note decays faster (synthetic G1: 4.7 dB/s).
const SPREAD: f64 = 1.26;
/// Share of a block an unsteady one takes back from the gathered time: clicks every 1.5 s do
/// not stop it, a note or plain noise does.
const UNSTEADY_COST: f64 = 0.25;
/// Hum is gone when its reference partial stays this far below the model (−12 dB) …
const ABSENT_RATIO: f64 = 0.25;
/// … or changed when it is steady outside `SPREAD` of it, for this long.
const LOST_S: f64 = 1.0;
/// Without a quiet block for this long (a long passage) the model is not subtracted: the grid
/// drifts from it. The next quiet blocks that fit it take it up again.
const HOLD_S: f64 = 8.0;
/// Fainter partials are not followed (−90 dBFS).
const FLOOR: f64 = 3e-5;
/// A block is quiet when what the hum does not explain stays within this factor (+6 dB) of the
/// quietest block lately: a note on the hum, even one turning slowly against it, is not quiet.
const QUIET: f64 = 4.0;
/// How fast that quietest level may rise per block (+0.9 dB/s): it follows a louder room.
const QUIET_RISE: f64 = 1.01;
/// Below this (−120 dBFS) all is quiet …
const QUIET_MIN: f64 = 1e-12;
/// … and this far (−20 dB) below the partials: a grid followed with a lag leaves that much of
/// its hum outside them.
const QUIET_BELOW: f64 = 100.0;

/// Mains hum remover for one input; allocation-free.
#[derive(Debug, Clone)]
pub(crate) struct Dehum {
    grids: [Grid; GRIDS_HZ.len()],
    block: usize,
    filled: usize,
    /// Sum of squares of the running block.
    energy: f64,
}

impl Dehum {
    pub(crate) fn new(sample_rate: f64) -> Dehum {
        let grid = |i: usize| Grid::new(GRIDS_HZ[i], GRIDS_HZ[1 - i], sample_rate);
        Dehum {
            grids: [grid(0), grid(1)],
            block: (BLOCK_S * sample_rate).round() as usize,
            filled: 0,
            energy: 0.0,
        }
    }

    /// The sample without hum (unchanged while none is found).
    pub(crate) fn process(&mut self, x: f32) -> f32 {
        let x = f64::from(x);
        let mut y = x;
        for grid in &mut self.grids {
            let hum = grid.sample(x);
            if grid.subtracts() {
                y -= hum;
            }
        }

        self.filled += 1;
        self.energy += x * x;
        if self.filled == self.block {
            // Mean power of the block; one mains frequency at a time: a grid is found only
            // while the other is not.
            let power = self.energy / self.block as f64;
            self.filled = 0;
            self.energy = 0.0;
            let [a, b] = &mut self.grids;
            a.end_block(self.block, power, b.present);
            b.end_block(self.block, power, a.present);
        }
        y as f32
    }

    #[cfg(test)]
    fn present(&self) -> bool {
        self.grids.iter().any(|g| g.present)
    }
}

/// Hum on one grid: its frequency and the level and phase of each partial (`model`).
#[derive(Debug, Clone)]
struct Grid {
    nominal_hz: f64,
    hz: f64,
    sample_rate: f64,
    /// e^{i·φ}, φ the fundamental's phase, turned by `step` each sample (cheaper than sin_cos;
    /// set back to length 1 each block).
    phase: C,
    step: C,
    partials: usize,
    /// The partial the other grid shares (300 Hz): no reference, it may be the other's hum.
    shared: Option<usize>,
    /// Demodulated input of the running block, per partial.
    sums: [C; MAX_PARTIALS],
    /// The last block's measurement, per partial.
    last: [C; MAX_PARTIALS],
    model: [C; MAX_PARTIALS],
    /// The partial that tells whether the hum is steady, while it is gathered or present.
    reference: Option<usize>,
    /// Level of `reference` when gathering began.
    anchor: f64,
    gathered_s: f64,
    present: bool,
    lost_s: f64,
    since_learned_s: f64,
    /// Power the hum did not explain in the quietest block lately.
    quiet: f64,
    /// Whether the last block was quiet: only two quiet blocks in a row tell how the hum turns
    /// (after a note the first quiet one pulled 60 Hz to 59.95).
    last_quiet: bool,
}

impl Grid {
    fn new(nominal_hz: f64, other_hz: f64, sample_rate: f64) -> Grid {
        let partials = ((TOP_HZ / nominal_hz) as usize).min(MAX_PARTIALS);
        let shared = (0..partials).find(|k| ((k + 1) as f64 * nominal_hz) % other_hz == 0.0);
        Grid {
            nominal_hz,
            hz: nominal_hz,
            sample_rate,
            phase: C::at(0.0),
            step: C::at(TAU * nominal_hz / sample_rate),
            partials,
            shared,
            sums: [C::ZERO; MAX_PARTIALS],
            last: [C::ZERO; MAX_PARTIALS],
            model: [C::ZERO; MAX_PARTIALS],
            reference: None,
            anchor: 0.0,
            gathered_s: 0.0,
            present: false,
            lost_s: 0.0,
            since_learned_s: 0.0,
            quiet: f64::INFINITY,
            last_quiet: false,
        }
    }

    fn subtracts(&self) -> bool {
        self.present && self.since_learned_s <= HOLD_S
    }

    /// Demodulates `x` at each partial; returns the model's hum at this sample.
    fn sample(&mut self, x: f64) -> f64 {
        let base = self.phase;
        let mut turn = base;
        let mut hum = 0.0;
        for k in 0..self.partials {
            self.sums[k] = self.sums[k] + turn.conj() * x;
            hum += (self.model[k] * turn).re;
            turn = turn * base;
        }
        self.phase = self.phase * self.step;
        hum
    }

    fn end_block(&mut self, len: usize, power: f64, other_present: bool) {
        // Complex amplitude per partial: x ≈ Re(z · e^{i·k·φ}).
        let scale = 2.0 / len as f64;
        let mut z = [C::ZERO; MAX_PARTIALS];
        for (zk, sum) in z.iter_mut().zip(&mut self.sums) {
            *zk = *sum * scale;
            *sum = C::ZERO;
        }
        let last = std::mem::replace(&mut self.last, z);
        self.since_learned_s += BLOCK_S;
        self.phase = self.phase * (1.0 / self.phase.abs());

        // Power besides the partials (each carries |z|²/2): a note, a click, the noise floor.
        let partials: f64 = z.iter().map(|c| c.abs() * c.abs() / 2.0).sum();
        let besides = (power - partials).max(0.0);
        self.quiet = (self.quiet * QUIET_RISE).min(besides.max(QUIET_MIN));
        let quiet = besides <= QUIET * self.quiet || besides * QUIET_BELOW <= partials;
        let both_quiet = quiet && self.last_quiet;
        self.last_quiet = quiet;

        match self.steady(&z, &last).filter(|_| both_quiet) {
            Some((reference, offset_hz)) if self.fits(z[reference]) => {
                self.learn(&z, &last, offset_hz);
                self.gather(reference, z[reference].abs(), other_present);
            }
            // Steady hum at another level (a charger plugged in): learned anew.
            Some(_) => self.lose(),
            None => self.unsteady(&z),
        }
    }

    /// Whether the reference partial matches the model, once the hum is present.
    fn fits(&self, now: C) -> bool {
        let Some(r) = self.reference.filter(|_| self.present) else {
            return true;
        };
        let model = self.model[r].abs();
        (model / SPREAD..=model * SPREAD).contains(&now.abs())
    }

    /// The reference partial and the grid's frequency offset (Hz), if this block is hum alone:
    /// the reference as strong as in the last block, turned by no more than the grid may drift.
    fn steady(&self, z: &[C; MAX_PARTIALS], last: &[C; MAX_PARTIALS]) -> Option<(usize, f64)> {
        let r = self.reference.or_else(|| self.strongest(z))?;
        let (now, before) = (z[r], last[r]);
        if now.abs() < FLOOR || before.abs() < FLOOR {
            return None;
        }
        if ((now.abs() / before.abs()) - 1.0).abs() > STEADY {
            return None;
        }

        // Turn per block at partial r+1 → offset of the fundamental.
        let ratio = now * before.conj();
        let offset_hz = ratio.im.atan2(ratio.re) / (TAU * (r + 1) as f64 * BLOCK_S);
        if (self.hz + offset_hz - self.nominal_hz).abs() > DRIFT_HZ {
            return None;
        }
        Some((r, offset_hz))
    }

    /// The strongest partial that may be a reference.
    fn strongest(&self, z: &[C; MAX_PARTIALS]) -> Option<usize> {
        (0..self.partials)
            .filter(|k| Some(*k) != self.shared)
            .max_by(|a, b| z[*a].abs().total_cmp(&z[*b].abs()))
    }

    /// Follows the grid's frequency and learns each partial that held steady.
    fn learn(&mut self, z: &[C; MAX_PARTIALS], last: &[C; MAX_PARTIALS], offset_hz: f64) {
        let lo = self.nominal_hz - DRIFT_HZ;
        let hi = self.nominal_hz + DRIFT_HZ;
        self.hz = (self.hz + FOLLOW * offset_hz).clamp(lo, hi);
        self.step = C::at(TAU * self.hz / self.sample_rate);
        self.since_learned_s = 0.0;
        for k in 0..self.partials {
            let harmonic = (k + 1) as f64;
            let expected = last[k] * C::at(TAU * harmonic * offset_hz * BLOCK_S);
            if (z[k] - expected).abs() > STEADY * z[k].abs().max(expected.abs()) {
                continue;
            }
            // The measurement holds for the block's middle; the model runs from its end on, half
            // a block at the old frequency and half at the new one later.
            let ahead = TAU * harmonic * offset_hz * BLOCK_S * (1.0 - FOLLOW / 2.0);
            let target = z[k] * C::at(ahead);
            self.model[k] = self.model[k] + (target - self.model[k]) * LEARN;
        }
    }

    /// Counts steady time towards finding the hum: from one reference level, within `SPREAD`.
    fn gather(&mut self, reference: usize, level: f64, other_present: bool) {
        self.lost_s = 0.0;
        if self.present {
            return;
        }
        let near = self.reference == Some(reference)
            && (self.anchor / SPREAD..=self.anchor * SPREAD).contains(&level);
        if !near {
            self.reference = Some(reference);
            self.anchor = level;
            self.gathered_s = 0.0;
        }
        self.gathered_s += BLOCK_S;
        if self.gathered_s >= PRESENT_S && !other_present {
            self.present = true;
        }
    }

    /// A block with something else in it (a note, a click) or without the hum.
    fn unsteady(&mut self, z: &[C; MAX_PARTIALS]) {
        let Some(r) = self.reference else {
            return;
        };
        if !self.present {
            self.gathered_s = (self.gathered_s - UNSTEADY_COST * BLOCK_S).max(0.0);
            if self.gathered_s == 0.0 {
                self.reference = None;
            }
            return;
        }

        // Present: gone once the reference stays far below the model.
        if z[r].abs() >= ABSENT_RATIO * self.model[r].abs() {
            self.lost_s = 0.0;
            return;
        }
        self.lose();
    }

    /// The hum is not as modelled (gone, or at another level): after `LOST_S` it is forgotten
    /// and gathered anew.
    fn lose(&mut self) {
        self.lost_s += BLOCK_S;
        if self.lost_s < LOST_S {
            return;
        }
        self.present = false;
        self.reference = None;
        self.gathered_s = 0.0;
        self.lost_s = 0.0;
        self.model = [C::ZERO; MAX_PARTIALS];
    }
}

/// A complex number, just what the demodulation needs.
#[derive(Debug, Clone, Copy, PartialEq)]
struct C {
    re: f64,
    im: f64,
}

impl C {
    const ZERO: C = C { re: 0.0, im: 0.0 };

    /// e^{i·angle}
    fn at(angle: f64) -> C {
        let (im, re) = angle.sin_cos();
        C { re, im }
    }

    fn conj(self) -> C {
        C {
            re: self.re,
            im: -self.im,
        }
    }

    fn abs(self) -> f64 {
        self.re.hypot(self.im)
    }
}

impl Add for C {
    type Output = C;
    fn add(self, o: C) -> C {
        C {
            re: self.re + o.re,
            im: self.im + o.im,
        }
    }
}

impl Sub for C {
    type Output = C;
    fn sub(self, o: C) -> C {
        C {
            re: self.re - o.re,
            im: self.im - o.im,
        }
    }
}

impl Mul for C {
    type Output = C;
    fn mul(self, o: C) -> C {
        C {
            re: self.re * o.re - self.im * o.im,
            im: self.re * o.im + self.im * o.re,
        }
    }
}

impl Mul<f64> for C {
    type Output = C;
    fn mul(self, s: f64) -> C {
        C {
            re: self.re * s,
            im: self.im * s,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::{add_noise, bass_tone, ToneShape};
    use kontra_model::{Pitch, A4_HZ};

    const SR: f64 = 48_000.0;

    fn run(signal: &[f32]) -> (Vec<f32>, Dehum) {
        let mut dehum = Dehum::new(SR);
        let out = signal.iter().map(|x| dehum.process(*x)).collect();
        (out, dehum)
    }

    fn rms(x: &[f32]) -> f64 {
        (x.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>() / x.len() as f64).sqrt()
    }

    /// −40 dBFS hum at `hz` with its third harmonic, `len_s` long.
    fn hum(hz: f64, len_s: f64) -> Vec<f32> {
        (0..(len_s * SR) as usize)
            .map(|i| {
                let t = i as f64 / SR;
                (0.01 * (TAU * hz * t).sin() + 0.004 * (TAU * 3.0 * hz * t + 1.0).sin()) as f32
            })
            .collect()
    }

    /// A sustained note is no hum: G1 (49 Hz) for 5 s, and one tuned into the grid's tolerance
    /// (49.9 Hz), come out untouched.
    #[test]
    fn sustained_note_is_not_learned() {
        let g1 = Pitch::parse("G1").unwrap().hz(A4_HZ);
        for hz in [g1, 49.9] {
            let mut tone = bass_tone(hz, SR, 5.0, ToneShape::Rich, 3);
            add_noise(&mut tone, -66.0, 2);
            let (out, dehum) = run(&tone);
            assert!(!dehum.present(), "{hz} Hz taken for hum");
            assert_eq!(out, tone, "{hz} Hz changed");
        }
    }

    /// Hum alone is gone after the time to find it: at least 30 dB less.
    #[test]
    fn hum_is_removed() {
        for hz in [50.0, 49.8, 60.2] {
            let mut signal = hum(hz, 5.0);
            add_noise(&mut signal, -80.0, 4);
            let (out, dehum) = run(&signal);
            assert!(dehum.present(), "{hz} Hz");
            let tail = (4.0 * SR) as usize;
            let left = rms(&out[tail..]) / rms(&signal[tail..]);
            assert!(left < 0.03, "{hz} Hz: {:.1} dB left", 20.0 * left.log10());
        }
    }

    /// Hum that gets louder (a charger plugged in) is learned anew and removed again.
    #[test]
    fn louder_hum_is_learned_anew() {
        let mut signal = hum(50.0, 4.0);
        signal.extend(hum(50.0, 5.0).iter().map(|x| x * 2.0));
        add_noise(&mut signal, -80.0, 6);
        let (out, dehum) = run(&signal);
        assert!(dehum.present());
        let tail = (8.0 * SR) as usize;
        let left = rms(&out[tail..]) / rms(&signal[tail..]);
        assert!(left < 0.03, "{:.1} dB left", 20.0 * left.log10());
    }

    /// Hum that stops (a charger unplugged) is no longer subtracted: no hum is added.
    #[test]
    fn stopped_hum_is_not_added() {
        let mut signal = hum(50.0, 4.0);
        signal.extend(std::iter::repeat_n(0.0, (3.0 * SR) as usize));
        add_noise(&mut signal, -80.0, 5);
        let (out, dehum) = run(&signal);
        assert!(!dehum.present());
        let tail = (5.5 * SR) as usize;
        assert_eq!(out[tail..], signal[tail..]);
    }
}
