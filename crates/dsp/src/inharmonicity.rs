//! Which string a note was played on, from the stiffness of the string (R7, LH-BEW-16).
//!
//! A stiff string bends its partials sharp: partial k sits at `k·f0·√(1 + B·k²)`. B grows
//! with the string's thickness and falls with the square of its sounding length, so it halves
//! about every six frets. The same pitch on a thicker string higher up the neck has a clearly
//! larger B:
//!
//! ```text
//! log10 B = a_string + fret · log10(2)/6        (B ∝ 1/L², L halves every 12 frets)
//!
//!  log10 B
//!   −3.0 ┤            ╱ E string
//!        │        ╱ ╱ A string        G2 (43): E fret 15, A fret 10, D fret 5, G open
//!   −3.6 ┤    ╱ ╱ ╱                   → four candidates, the measured B picks one
//!        │  ╱ ╱ ╱ ╱ D, G …
//!        └────────────── fret
//! ```
//!
//! Measured on 250 ms of the tone from 30 ms after its peak (the attack is gone, the high
//! partials still ring): one long FFT, up to 25 partials below 4 kHz, each looked for where
//! the B found so far puts it. On IDMT-SMT-Bass, a_string from two basses picked the string of
//! the third in 98.1 % of the plucked, slapped and popped notes more than one string can play
//! (chance 32.7 %; `kontra-cli korpus strings --held-out`, 29.10.2026). Palm-muted notes die
//! too fast to tell.

use kontra_model::{Instrument, InstrumentKind};
use rustfft::num_complex::Complex;
use rustfft::{Fft, FftPlanner};
use std::sync::Arc;

/// The window starts this long after the peak (the attack's noise is gone) …
pub const FROM_PEAK_S: f64 = 0.03;
/// … and is this long: long enough for 1 Hz bins on the low E, short enough for a quaver.
pub const WINDOW_S: f64 = 0.25;
/// The FFT is this many times the window (zero padding: finer peak positions).
const PADDING: usize = 5;
/// Partials looked for at most, and only below this frequency (pickups and strings give
/// little above it).
const MAX_PARTIALS: usize = 25;
const MAX_HZ: f64 = 4000.0;
/// A partial is looked for this far (share of its frequency) around where B predicts it.
const SEARCH: f64 = 0.02;
/// A partial must stand this far above the median of the spectrum (20 dB).
const ABOVE_NOISE: f64 = 10.0;
/// B is fitted once this many partials are found.
const FIT_FROM: usize = 4;
/// What B is told as when none was found (no stiffness heard).
const NO_B_LOG: f64 = -6.0;
/// B halves every six frets: log10(2)/6 per fret.
pub const LOG_B_PER_FRET: f64 = std::f64::consts::LOG10_2 / 6.0;

/// Strings a profile can have (as `Instrument`).
const STRINGS: usize = 12;

/// A measured inharmonicity: log10 B and the partials it rests on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stiffness {
    pub log_b: f64,
    pub partials: u8,
}

/// The strings of a profile with their open pitch and log10 B of the open string (`a_string`),
/// where the profile knows it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StringModel {
    open: [u8; STRINGS],
    log_b: [Option<f64>; STRINGS],
    count: usize,
    frets: u8,
}

/// The string picked for a pitch, or why none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    String(u8),
    /// Only one string can play the pitch: nothing to tell.
    OnlyOne,
    /// A string that could play it has no value in the profile.
    Unknown,
}

impl StringModel {
    /// The model of a fretted string profile; `None` for fretless, double bass and keys (their
    /// strings have no measured values yet).
    pub fn for_instrument(instrument: &Instrument) -> Option<StringModel> {
        if instrument.kind != InstrumentKind::Fretted || !instrument.has_strings() {
            return None;
        }
        let mut model = StringModel {
            open: [0; STRINGS],
            log_b: [None; STRINGS],
            count: instrument.tuning.strings.len().min(STRINGS),
            frets: instrument.frets,
        };
        for (i, open) in instrument.tuning.strings.iter().take(STRINGS).enumerate() {
            model.open[i] = open.0;
            model.log_b[i] = instrument.string_log_b(i as u8);
        }
        Some(model)
    }

    /// The same with `log_b` for `string` (a calibration, a held-out measurement).
    pub fn with_log_b(mut self, string: u8, log_b: Option<f64>) -> StringModel {
        if let Some(slot) = self.log_b.get_mut(usize::from(string)) {
            *slot = log_b;
        }
        self
    }

    /// The fret `string` plays `midi` on, if it can.
    pub fn fret(&self, string: u8, midi: u8) -> Option<u8> {
        let open = *self.open[..self.count].get(usize::from(string))?;
        let fret = midi.checked_sub(open)?;
        (fret <= self.frets).then_some(fret)
    }

    /// The string whose expected B lies nearest to the measured one, among those that can play
    /// `midi`. Example: G2 with log10 B −3.2 on a bass whose open E is −3.6: E fret 15 expects
    /// −3.6 + 15·0.05 = −2.85, A fret 10 −3.12 … the nearest wins.
    pub fn pick(&self, midi: u8, log_b: f64) -> Pick {
        let mut best: Option<(u8, f64)> = None;
        let mut candidates = 0;
        for string in 0..self.count as u8 {
            let Some(fret) = self.fret(string, midi) else {
                continue;
            };
            candidates += 1;
            let Some(a) = self.log_b[usize::from(string)] else {
                return Pick::Unknown;
            };
            let off = (log_b - a - f64::from(fret) * LOG_B_PER_FRET).abs();
            if best.is_none_or(|(_, b)| off < b) {
                best = Some((string, off));
            }
        }
        match (candidates, best) {
            (0 | 1, _) | (_, None) => Pick::OnlyOne,
            (_, Some((string, _))) => Pick::String(string),
        }
    }
}

/// Measures B on a stretch of a tone; allocation-free after `new`.
pub struct InharmonicityMeter {
    sample_rate: f64,
    window: usize,
    fft: Arc<dyn Fft<f64>>,
    buffer: Vec<Complex<f64>>,
    scratch: Vec<Complex<f64>>,
    magnitude: Vec<f64>,
    sorted: Vec<f64>,
}

impl std::fmt::Debug for InharmonicityMeter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InharmonicityMeter")
    }
}

/// Partials found so far and the weighted fit of `(f_k/k)² = f0² + f0²·B·k²`.
#[derive(Default)]
struct Fit {
    found: usize,
    /// Sums of the weighted normal equations: w, w·k², w·k⁴, w·y, w·k²·y.
    w: f64,
    wx: f64,
    wxx: f64,
    wy: f64,
    wxy: f64,
}

impl Fit {
    fn add(&mut self, k: f64, hz: f64, weight: f64) {
        let x = k * k;
        let y = (hz / k) * (hz / k);
        self.found += 1;
        self.w += weight;
        self.wx += weight * x;
        self.wxx += weight * x * x;
        self.wy += weight * y;
        self.wxy += weight * x * y;
    }

    /// (f0, B) of the fit, if it has a positive f0².
    fn solve(&self) -> Option<(f64, f64)> {
        let det = self.w * self.wxx - self.wx * self.wx;
        if det.abs() < f64::EPSILON {
            return None;
        }
        let c0 = (self.wxx * self.wy - self.wx * self.wxy) / det;
        let c1 = (self.w * self.wxy - self.wx * self.wy) / det;
        (c0 > 0.0).then(|| (c0.sqrt(), (c1 / c0).max(0.0)))
    }
}

impl InharmonicityMeter {
    pub fn new(sample_rate: f64) -> InharmonicityMeter {
        let window = (WINDOW_S * sample_rate) as usize;
        let len = (window * PADDING).next_power_of_two();
        let fft = FftPlanner::new().plan_fft_forward(len);
        let scratch = vec![Complex::default(); fft.get_inplace_scratch_len()];
        InharmonicityMeter {
            sample_rate,
            window,
            fft,
            buffer: vec![Complex::default(); len],
            scratch,
            magnitude: vec![0.0; len / 2 + 1],
            sorted: vec![0.0; len / 2 + 1],
        }
    }

    /// Samples the window holds at most.
    pub fn window(&self) -> usize {
        self.window
    }

    /// B of the tone in `samples` (from `FROM_PEAK_S` after its peak, at most `window()` long)
    /// near `hz`.
    pub fn measure(&mut self, samples: &[f32], hz: f64) -> Stiffness {
        self.spectrum(samples);
        let len = self.buffer.len();
        let bin_hz = self.sample_rate / len as f64;
        let noise = self.noise();
        let top = self.magnitude.len() - 1;

        // Each partial where the B so far puts it; B refitted with every one found.
        let mut fit = Fit::default();
        let (mut f0, mut b) = (hz, 0.0);
        for k in 1..=MAX_PARTIALS {
            let k = k as f64;
            let predicted = k * f0 * (1.0 + b * k * k).sqrt();
            if predicted > MAX_HZ {
                break;
            }
            let lo = (predicted * (1.0 - SEARCH) / bin_hz) as usize;
            let hi = ((predicted * (1.0 + SEARCH) / bin_hz) as usize + 1).min(top);
            let Some(at) = self.peak_in(lo, hi) else {
                continue;
            };
            if self.magnitude[at] < ABOVE_NOISE * noise || at == lo || at + 1 == hi {
                continue;
            }
            fit.add(
                k,
                self.refine(at) * bin_hz,
                (self.magnitude[at] / noise).ln(),
            );
            if fit.found < FIT_FROM {
                continue;
            }
            if let Some((f, fitted)) = fit.solve() {
                f0 = f;
                b = fitted;
            }
        }
        let log_b = if b > 0.0 { b.log10() } else { NO_B_LOG };
        Stiffness {
            log_b,
            partials: u8::try_from(fit.found).unwrap_or(u8::MAX),
        }
    }

    /// Magnitudes of the Hann-windowed, zero-padded `samples`.
    fn spectrum(&mut self, samples: &[f32]) {
        let n = samples.len().min(self.window);
        let span = (n.max(2) - 1) as f64;
        for (i, slot) in self.buffer.iter_mut().enumerate() {
            let x = samples.get(i).filter(|_| i < n).copied().unwrap_or(0.0);
            let hann = 0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / span).cos();
            *slot = Complex::new(f64::from(x) * hann, 0.0);
        }
        self.fft
            .process_with_scratch(&mut self.buffer, &mut self.scratch);
        for (m, c) in self.magnitude.iter_mut().zip(&self.buffer) {
            *m = c.norm();
        }
    }

    /// The median magnitude: the noise floor between the partials.
    fn noise(&mut self) -> f64 {
        self.sorted.copy_from_slice(&self.magnitude);
        let mid = self.sorted.len() / 2;
        let (_, median, _) = self.sorted.select_nth_unstable_by(mid, f64::total_cmp);
        median.max(f64::MIN_POSITIVE)
    }

    /// The loudest bin in `[lo, hi)`.
    fn peak_in(&self, lo: usize, hi: usize) -> Option<usize> {
        (lo..hi).max_by(|a, b| self.magnitude[*a].total_cmp(&self.magnitude[*b]))
    }

    /// The peak's position between bins (parabola through the log magnitudes).
    fn refine(&self, at: usize) -> f64 {
        let ln = |i: usize| (self.magnitude[i] + 1e-12).ln();
        let (a, b, c) = (ln(at - 1), ln(at), ln(at + 1));
        let curve = a - 2.0 * b + c;
        if curve.abs() < f64::EPSILON {
            return at as f64;
        }
        at as f64 + 0.5 * (a - c) / curve
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kontra_model::{Pitch, Tuning};

    const RATE: f64 = 44_100.0;

    /// A stiff string's tone: partials at `k·f0·√(1 + B·k²)`, falling 1/k, from the peak on.
    fn stiff_tone(hz: f64, b: f64, seconds: f64) -> Vec<f32> {
        (0..(seconds * RATE) as usize)
            .map(|i| {
                let t = i as f64 / RATE;
                (1..=30)
                    .map(|k| {
                        let k = f64::from(k);
                        let f = k * hz * (1.0 + b * k * k).sqrt();
                        (std::f64::consts::TAU * f * t).sin() / k * (-t * 3.0).exp()
                    })
                    .sum::<f64>() as f32
                    * 0.2
            })
            .collect()
    }

    #[test]
    fn measures_the_stiffness_of_a_synthetic_string() {
        let mut meter = InharmonicityMeter::new(RATE);
        for b in [1e-4, 4e-4, 1.5e-3] {
            let tone = stiff_tone(55.0, b, WINDOW_S);
            let s = meter.measure(&tone, 55.0);
            assert!((s.log_b - b.log10()).abs() < 0.05, "B {b}: {s:?}");
            assert!(s.partials >= 20, "{s:?}");
        }
    }

    #[test]
    fn a_pure_tone_has_too_few_partials() {
        let mut meter = InharmonicityMeter::new(RATE);
        // A sine in faint noise (−40 dB): its leakage alone is no partial.
        let mut seed = 1u32;
        let tone: Vec<f32> = (0..(WINDOW_S * RATE) as usize)
            .map(|i| {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let noise = f64::from(seed >> 8) / f64::from(1u32 << 24) - 0.5;
                ((std::f64::consts::TAU * 98.0 * i as f64 / RATE).sin() + 0.02 * noise) as f32
            })
            .collect();
        assert!(meter.measure(&tone, 98.0).partials < FIT_FROM as u8);
    }

    fn bass() -> StringModel {
        let mut model = StringModel::for_instrument(&Instrument::default_bass()).unwrap();
        for (s, a) in [-3.6, -3.7, -3.8, -3.9].into_iter().enumerate() {
            model = model.with_log_b(s as u8, Some(a));
        }
        model
    }

    #[test]
    fn picks_the_string_whose_stiffness_fits() {
        let model = bass();
        // G2 (43): E fret 15, A fret 10, D fret 5, G open.
        let on = |s: u8, fret: u8| -3.6 - 0.1 * f64::from(s) + f64::from(fret) * LOG_B_PER_FRET;
        assert_eq!(model.pick(43, on(0, 15)), Pick::String(0));
        assert_eq!(model.pick(43, on(1, 10)), Pick::String(1));
        assert_eq!(model.pick(43, on(3, 0)), Pick::String(3));
        // E1 (28) only on the E string.
        assert_eq!(model.pick(28, -3.0), Pick::OnlyOne);
    }

    #[test]
    fn an_unmeasured_string_is_unknown() {
        let model = bass().with_log_b(1, None);
        assert_eq!(model.pick(35, -3.5), Pick::Unknown);
    }

    #[test]
    fn fretless_and_keys_have_no_model() {
        let mut fretless = Instrument::default_bass();
        fretless.kind = InstrumentKind::Fretless;
        assert!(StringModel::for_instrument(&fretless).is_none());
        let keys = Instrument::keyboard(InstrumentKind::Keys, Pitch(28), Pitch(67)).unwrap();
        assert!(StringModel::for_instrument(&keys).is_none());
        let five = Instrument::new(InstrumentKind::Fretted, Tuning::bass5_standard(), 24).unwrap();
        let model = StringModel::for_instrument(&five).unwrap();
        // B0 to D#1 only on the low B, which has no value: E1 on B fret 5 or E open is unknown.
        assert_eq!(model.pick(28, -3.5), Pick::Unknown);
    }
}
