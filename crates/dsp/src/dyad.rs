//! Which tones of an announced double stop sound (R8): Klapuri's harmonic summation (ISMIR
//! 2006) on a whitened spectrum, with cancellation by spectral smoothness (Klapuri 2003).
//!
//! The tab names both tones. Each is looked for after the other is taken out: a tone whose
//! partials stay after cancelling the other sounds. Cancelling keeps a partial down to the mean
//! of its neighbours only, so a partial both tones share (a fifth's third harmonic) keeps the
//! other tone's part.
//!
//! ```text
//! 140 ms from attack + 20 ms ─► Hann, FFT (4× zero pad) ─► whiten (30 bands, ν 0.33) ─► Y
//! Y − lower (smooth) ─► salience near upper ─► a = s / max(s_lower, s_upper) > 0.44?
//! Y − upper (smooth) ─► salience near lower ─► b = …                         > 0.46?
//!                        both ─► Both, only b ─► Lower, only a ─► Upper, neither ─► Unknown
//! ```
//!
//! Measured (experiment dyadstretch, 29.10.2026, thresholds fitted on half the data): IDMT
//! mixtures 95 % told as both, 2.1 % of single notes told as both; NSynth 87 % / 0.4 %;
//! synthetic single notes 0 of 4774. A single note never named the wrong tone. Octaves 0 %:
//! every partial of the upper tone is one of the lower's.

use rustfft::num_complex::Complex;
use rustfft::{Fft, FftPlanner};
use std::sync::Arc;

/// Analysis window from `SKIP_S` after the attack (the pick noise passed).
pub(crate) const FRAME_S: f64 = 0.14;
pub(crate) const SKIP_S: f64 = 0.02;
/// Zero padding: finer bins for the partial search.
const PAD: usize = 4;
/// Salience weight g(f, m) = (f + α) / (m·f + β) (Klapuri 2006).
const ALPHA_HZ: f64 = 52.0;
const BETA_HZ: f64 = 320.0;
/// Whitening: 30 bands with centres 229·(10^(b/21.4) − 1) Hz, gain σ^(ν−1).
const BANDS: usize = 30;
const BAND_SCALE_HZ: f64 = 229.0;
const BAND_STEP: f64 = 21.4;
const NU: f64 = 0.33;
const MAX_PARTIAL_HZ: f64 = 5000.0;
/// Partials stay this far below Nyquist.
const NYQUIST_MARGIN_HZ: f64 = 100.0;
const MAX_PARTIALS: usize = 30;
/// Stiff strings: partial m lies up to √(1 + B·m²) higher.
const B_MAX: f64 = 0.0003;
/// A partial is looked for this far around its place …
const TOLERANCE_CENTS: f64 = 20.0;
/// … for fundamentals on this grid, this far around the tab's tone (a bent or detuned string).
const GRID_CENTS: f64 = 5.0;
const AROUND_CENTS: f64 = 40.0;
/// Main lobe half width of the Hann window in unpadded bins, removed when cancelling.
const LOBE_BINS: f64 = 1.5;
/// The upper tone after the lower is cancelled, and the lower after the upper, against the
/// stronger tone: above these it sounds (fitted at ≤ 1 % false alarms on the training half).
const UPPER_SOUNDS: f32 = 0.44;
const LOWER_SOUNDS: f32 = 0.46;
const CENTS_PER_OCTAVE: f64 = 1200.0;
const SEMITONES_PER_OCTAVE: i32 = 12;

/// Which tones of the double stop sound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Heard {
    Both,
    Lower,
    Upper,
    /// An octave, or neither tone clear.
    Unknown,
}

/// The checker; allocation-free after `new`.
pub(crate) struct DyadCheck {
    sample_rate: f64,
    nfft: usize,
    fft: Arc<dyn Fft<f32>>,
    fft_scratch: Vec<Complex<f32>>,
    window: Vec<f32>,
    /// The analysed audio (`FRAME_S`), filled by the caller.
    frame: Vec<f32>,
    buf: Vec<Complex<f32>>,
    /// Whitened magnitude spectrum, and a copy with one tone cancelled.
    spectrum: Vec<f32>,
    residual: Vec<f32>,
    /// Band centres (index 0 = 0 Hz) and, per bin, the band below it and the share of the next.
    centres: [f64; BANDS + 2],
    bin_band: Vec<(usize, f64)>,
    gains: [f64; BANDS + 2],
    /// (bin, magnitude) of each partial of the tone being cancelled.
    peaks: [(usize, f32); MAX_PARTIALS],
}

impl std::fmt::Debug for DyadCheck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DyadCheck")
            .field("sample_rate", &self.sample_rate)
            .field("nfft", &self.nfft)
            .finish_non_exhaustive()
    }
}

impl DyadCheck {
    pub(crate) fn new(sample_rate: f64) -> DyadCheck {
        let len = ((FRAME_S * sample_rate).round() as usize).max(1);
        let nfft = len.next_power_of_two() * PAD;
        let fft = FftPlanner::new().plan_fft_forward(nfft);
        let window = (0..len)
            .map(|i| (0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / len as f64).cos()) as f32)
            .collect();
        let mut centres = [0.0; BANDS + 2];
        for (b, c) in centres.iter_mut().enumerate() {
            *c = BAND_SCALE_HZ * (10f64.powf(b as f64 / BAND_STEP) - 1.0);
        }
        let bins = nfft / 2 + 1;
        let bin_hz = sample_rate / nfft as f64;
        let bin_band = (0..bins)
            .map(|k| band_of(&centres, k as f64 * bin_hz))
            .collect();
        DyadCheck {
            sample_rate,
            nfft,
            fft_scratch: vec![Complex::default(); fft.get_inplace_scratch_len()],
            fft,
            window,
            frame: vec![0.0; len],
            buf: vec![Complex::default(); nfft],
            spectrum: vec![0.0; bins],
            residual: vec![0.0; bins],
            centres,
            bin_band,
            gains: [0.0; BANDS + 2],
            peaks: [(0, 0.0); MAX_PARTIALS],
        }
    }

    /// The audio to analyse: `FRAME_S` from `SKIP_S` after the attack. The caller fills it and
    /// zeroes what it does not have.
    pub(crate) fn frame_mut(&mut self) -> &mut [f32] {
        &mut self.frame
    }

    /// Whether an interval can be told at all: an octave's upper tone has only the lower's
    /// partials.
    pub(crate) fn can_tell(lower_midi: u8, upper_midi: u8) -> bool {
        let semitones = i32::from(upper_midi) - i32::from(lower_midi);
        semitones != 0 && semitones % SEMITONES_PER_OCTAVE != 0
    }

    /// Which of `lower_hz` and `upper_hz` sound in the frame.
    pub(crate) fn check(&mut self, lower_hz: f64, upper_hz: f64) -> Heard {
        self.analyze();
        let around = 2f64.powf(AROUND_CENTS / CENTS_PER_OCTAVE);
        let (lower, lower_salience) = self.best(Spectrum::Whole, lower_hz, around);
        let (upper, upper_salience) = self.best(Spectrum::Whole, upper_hz, around);
        let strongest = lower_salience.max(upper_salience);
        if strongest <= 0.0 {
            return Heard::Unknown;
        }
        self.cancel(lower);
        let upper_left = self.best(Spectrum::Residual, upper_hz, around).1 / strongest;
        self.cancel(upper);
        let lower_left = self.best(Spectrum::Residual, lower_hz, around).1 / strongest;
        match (lower_left > LOWER_SOUNDS, upper_left > UPPER_SOUNDS) {
            (true, true) => Heard::Both,
            (true, false) => Heard::Lower,
            (false, true) => Heard::Upper,
            (false, false) => Heard::Unknown,
        }
    }

    /// Windowed, zero-padded spectrum of `frame`, whitened into `spectrum`.
    fn analyze(&mut self) {
        let len = self.frame.len();
        for (i, c) in self.buf.iter_mut().enumerate() {
            let x = if i < len {
                self.frame[i] * self.window[i]
            } else {
                0.0
            };
            *c = Complex::new(x, 0.0);
        }
        self.fft
            .process_with_scratch(&mut self.buf, &mut self.fft_scratch);
        for (y, c) in self.spectrum.iter_mut().zip(&self.buf) {
            *y = c.norm();
        }
        self.whiten();
    }

    /// Flattens the spectral envelope: each band's energy σ scaled by σ^(ν−1), linear between
    /// band centres (Klapuri 2006, section 2.1).
    fn whiten(&mut self) {
        let bin_hz = self.bin_hz();
        let bins = self.spectrum.len();
        for b in 1..=BANDS {
            let (lo, mid, hi) = (self.centres[b - 1], self.centres[b], self.centres[b + 1]);
            let first = (lo / bin_hz).floor() as usize;
            let last = ((hi / bin_hz).ceil() as usize).min(bins - 1);
            let mut energy = 0.0;
            for k in first..=last {
                let f = k as f64 * bin_hz;
                let weight = if f < lo || f > hi {
                    0.0
                } else if f <= mid {
                    (f - lo) / (mid - lo)
                } else {
                    (hi - f) / (hi - mid)
                };
                energy += weight * f64::from(self.spectrum[k]).powi(2);
            }
            let sigma = (energy / bins as f64).sqrt().max(f64::MIN_POSITIVE);
            self.gains[b] = sigma.powf(NU - 1.0);
        }
        for (y, &(b, t)) in self.spectrum.iter_mut().zip(&self.bin_band) {
            let gain = self.gains[b] * (1.0 - t) + self.gains[(b + 1).min(BANDS)] * t;
            *y *= gain as f32;
        }
    }

    fn bin_hz(&self) -> f64 {
        self.sample_rate / self.nfft as f64
    }

    /// Bins where partial `m` of `f` may lie (a stiff string's lie higher).
    fn partial_bins(&self, f: f64, m: usize) -> (usize, usize) {
        let mf = m as f64 * f;
        let tolerance = 2f64.powf(TOLERANCE_CENTS / CENTS_PER_OCTAVE);
        let lo = mf / tolerance;
        let hi = mf * (1.0 + B_MAX * (m * m) as f64).sqrt() * tolerance;
        let last = self.spectrum.len() - 1;
        let bin_hz = self.bin_hz();
        (
            ((lo / bin_hz).round() as usize).min(last),
            ((hi / bin_hz).round() as usize).min(last),
        )
    }

    fn partials(&self, f: f64) -> usize {
        let top = MAX_PARTIAL_HZ.min(self.sample_rate / 2.0 - NYQUIST_MARGIN_HZ);
        ((top / f) as usize).min(MAX_PARTIALS)
    }

    /// Weighted sum of the strongest bin near each partial of `f`.
    fn salience(&self, of: Spectrum, f: f64) -> f32 {
        let s = match of {
            Spectrum::Whole => &self.spectrum,
            Spectrum::Residual => &self.residual,
        };
        let mut sum = 0.0f32;
        for m in 1..=self.partials(f) {
            let (first, last) = self.partial_bins(f, m);
            let peak = s[first..=last].iter().fold(0f32, |a, &b| a.max(b));
            let weight = (f + ALPHA_HZ) / (m as f64 * f + BETA_HZ);
            sum += weight as f32 * peak;
        }
        sum
    }

    /// The most salient fundamental within `around` (a ratio) of `hz`, and its salience.
    fn best(&self, of: Spectrum, hz: f64, around: f64) -> (f64, f32) {
        let step = 2f64.powf(GRID_CENTS / CENTS_PER_OCTAVE);
        let top = hz * around;
        let mut f = hz / around;
        let mut best = (hz, 0.0f32);
        while f <= top {
            let s = self.salience(of, f);
            if s > best.1 {
                best = (f, s);
            }
            f *= step;
        }
        best
    }

    /// `residual` = `spectrum` with the partials of `f` lowered to their smooth envelope (the
    /// mean of each partial and its neighbours).
    fn cancel(&mut self, f: f64) {
        self.residual.copy_from_slice(&self.spectrum);
        let count = self.partials(f);
        for m in 1..=count {
            let (first, last) = self.partial_bins(f, m);
            let mut peak = (first, 0.0f32);
            for k in first..=last {
                if self.residual[k] > peak.1 {
                    peak = (k, self.residual[k]);
                }
            }
            self.peaks[m - 1] = peak;
        }
        let lobe = (LOBE_BINS * self.nfft as f64 / self.frame.len() as f64).round() as usize;
        let last = self.residual.len() - 1;
        for i in 0..count {
            let (bin, a) = self.peaks[i];
            if a <= 0.0 {
                continue;
            }
            let before = if i > 0 { self.peaks[i - 1].1 } else { a };
            let after = if i + 1 < count {
                self.peaks[i + 1].1
            } else {
                a
            };
            let smooth = a.min((before + a + after) / 3.0);
            let keep = 1.0 - smooth / a;
            for y in &mut self.residual[bin.saturating_sub(lobe)..=(bin + lobe).min(last)] {
                *y *= keep;
            }
        }
    }
}

/// Which spectrum a salience is taken on.
#[derive(Debug, Clone, Copy)]
enum Spectrum {
    Whole,
    Residual,
}

/// The band whose centre lies at or below `f`, and how far `f` lies towards the next (0..1).
/// Below the first centre and above the last the gain is the edge band's.
fn band_of(centres: &[f64; BANDS + 2], f: f64) -> (usize, f64) {
    if f <= centres[1] {
        return (1, 0.0);
    }
    if f >= centres[BANDS] {
        return (BANDS, 0.0);
    }
    let b = (1..BANDS)
        .find(|&b| f < centres[b + 1])
        .unwrap_or(BANDS - 1);
    (b, (f - centres[b]) / (centres[b + 1] - centres[b]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::{bass_tone, ToneShape};
    use kontra_model::{midi_to_hz, A4_HZ};

    const SR: f64 = 48_000.0;

    /// The check on `tones` (MIDI) sounding together, the upper `delay_s` later.
    fn heard(tones: &[u8], lower: u8, upper: u8, shape: ToneShape) -> Heard {
        let mut mix = vec![0.0f32; SR as usize];
        for (i, &m) in tones.iter().enumerate() {
            let tone = bass_tone(
                midi_to_hz(f64::from(m), A4_HZ),
                SR,
                1.0,
                shape,
                5 + i as u64,
            );
            for (out, x) in mix.iter_mut().zip(tone) {
                *out += x;
            }
        }
        let mut check = DyadCheck::new(SR);
        let skip = (SKIP_S * SR) as usize;
        let frame = check.frame_mut();
        let len = frame.len();
        frame.copy_from_slice(&mix[skip..skip + len]);
        let hz = |m: u8| midi_to_hz(f64::from(m), A4_HZ);
        check.check(hz(lower), hz(upper))
    }

    #[test]
    fn tells_both_tones_of_a_fifth() {
        // A1 + E2 on the E and A strings, A2 + E3 higher up.
        for (lo, hi) in [(33, 40), (45, 52), (40, 44), (35, 38)] {
            for shape in [ToneShape::Rich, ToneShape::Dull] {
                assert_eq!(heard(&[lo, hi], lo, hi, shape), Heard::Both, "{lo} {hi}");
            }
        }
    }

    #[test]
    fn names_the_tone_a_single_note_is() {
        for shape in [ToneShape::Rich, ToneShape::Dull] {
            assert_eq!(heard(&[33], 33, 40, shape), Heard::Lower);
            assert_eq!(heard(&[40], 33, 40, shape), Heard::Upper);
        }
    }

    #[test]
    fn octaves_cannot_be_told() {
        assert!(!DyadCheck::can_tell(33, 45));
        assert!(!DyadCheck::can_tell(33, 57));
        assert!(!DyadCheck::can_tell(33, 33));
        assert!(DyadCheck::can_tell(33, 40));
    }

    #[test]
    fn silence_tells_nothing() {
        let mut check = DyadCheck::new(SR);
        check.frame_mut().fill(0.0);
        assert_eq!(check.check(55.0, 82.4), Heard::Unknown);
    }
}
