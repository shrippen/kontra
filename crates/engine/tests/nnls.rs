//! Feasibility study M4 (planned after 1.0, no product code): NNLS with one template per
//! string and fret on a log-frequency spectrum, on IDMT double-stop mixtures. Ignored; run with
//!
//! ```sh
//! KONTRA_IDMT=testdata/korpus/local/conv/idmt-st \
//!   cargo test --release -p kontra-engine --test nnls -- --ignored --nocapture
//! ```
//!
//! ```text
//! frame (attack + 20 ms, 140 ms) ─► Hann, FFT ─► log-frequency (36 bins/octave, √ magnitude) ─► v
//! templates: a stiff string per (string, fret), B from the profile, same transform ─► W
//! v ≈ W·h, h ≥ 0 (multiplicative updates) ─► activation per pitch (sum over strings)
//!   blind:    pitches over θ · max            ─► the set heard
//!   informed: the tab's two tones, weaker / stronger over θ ─► both sound
//! ```
//!
//! Results go to `planung/FORSCHUNG-AUDIO.md` („NNLS mit Saitenvorlagen (M4)“).

use kontra_engine::{double_stop_mixtures, read_audio, KorpusItem, LOG_B_PER_FRET};
use kontra_model::{midi_to_hz, Instrument, A4_HZ};
use rustfft::num_complex::Complex;
use rustfft::FftPlanner;
use std::path::{Path, PathBuf};

const SKIP_S: f64 = 0.02;
const FRAME_S: f64 = 0.14;
const FFT_LEN: usize = 32_768;
const LOW_HZ: f64 = 35.0;
const BINS_PER_OCTAVE: f64 = 36.0;
const OCTAVES: f64 = 5.7;
const PARTIALS: usize = 30;
const FRETS: u8 = 20;
const ITERATIONS: usize = 300;
const PER_INTERVAL: usize = 6;
/// Single notes announced with these second tones (as `korpus double-stops`).
const SECOND_TONES: [i32; 5] = [5, 7, -7, -16, 12];
const THRESHOLDS: [f64; 5] = [0.2, 0.3, 0.4, 0.5, 0.6];
const OCTAVE: u8 = 12;
const SEGMENT_MIN_S: f64 = 0.3;

/// One template: its pitch and string.
struct Atom {
    midi: u8,
    string: u8,
    spectrum: Vec<f64>,
}

/// Which templates: one per string and fret (stiff), or one per pitch (harmonic, B = 0).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Templates {
    PerString,
    PerPitch,
}

struct Analyzer {
    sample_rate: f64,
    fft: std::sync::Arc<dyn rustfft::Fft<f64>>,
    bins: Vec<(usize, usize)>,
}

impl Analyzer {
    fn new(sample_rate: f64) -> Analyzer {
        let fft = FftPlanner::new().plan_fft_forward(FFT_LEN);
        let hz_per_bin = sample_rate / FFT_LEN as f64;
        let count = (OCTAVES * BINS_PER_OCTAVE) as usize;
        let half = 0.5 / BINS_PER_OCTAVE;
        let bins = (0..count)
            .map(|i| {
                let centre = LOW_HZ * 2f64.powf(i as f64 / BINS_PER_OCTAVE);
                let low = (centre * 2f64.powf(-half) / hz_per_bin).floor() as usize;
                let high = (centre * 2f64.powf(half) / hz_per_bin).ceil() as usize;
                (low, high.max(low + 1))
            })
            .collect();
        Analyzer {
            sample_rate,
            fft,
            bins,
        }
    }

    /// √ magnitude on the log-frequency bins, of `frame` (Hann).
    fn spectrum(&self, frame: &[f32]) -> Vec<f64> {
        let n = frame.len();
        let mut buf = vec![Complex::new(0.0, 0.0); FFT_LEN];
        for (i, (slot, x)) in buf.iter_mut().zip(frame).enumerate() {
            let w = 0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / n as f64).cos();
            *slot = Complex::new(f64::from(*x) * w, 0.0);
        }
        self.fft.process(&mut buf);
        self.bins
            .iter()
            .map(|&(lo, hi)| {
                buf[lo..hi.min(FFT_LEN / 2)]
                    .iter()
                    .map(|c| c.norm())
                    .fold(0.0, f64::max)
                    .sqrt()
            })
            .collect()
    }

    fn frame_len(&self) -> usize {
        (FRAME_S * self.sample_rate) as usize
    }

    /// A stiff string's tone (partial k at k·f0·√(1 + B·k²), falling 1/k) through `spectrum`.
    fn template(&self, midi: u8, log_b: Option<f64>) -> Vec<f64> {
        let f0 = midi_to_hz(f64::from(midi), A4_HZ);
        let b = log_b.map_or(0.0, |l| 10f64.powf(l));
        let frame: Vec<f32> = (0..self.frame_len())
            .map(|i| {
                let t = i as f64 / self.sample_rate;
                (1..=PARTIALS)
                    .map(|k| {
                        let k = k as f64;
                        let f = k * f0 * (1.0 + b * k * k).sqrt();
                        if f > self.sample_rate / 2.0 {
                            return 0.0;
                        }
                        (std::f64::consts::TAU * f * t).sin() / k
                    })
                    .sum::<f64>() as f32
            })
            .collect();
        normalised(self.spectrum(&frame))
    }

    fn atoms(&self, instrument: &Instrument, templates: Templates) -> Vec<Atom> {
        let mut atoms = Vec::new();
        for (string, open) in instrument.tuning.strings.iter().enumerate() {
            let string = string as u8;
            for fret in 0..=FRETS {
                let midi = open.0 + fret;
                let log_b = match templates {
                    Templates::PerString => instrument
                        .string_log_b(string)
                        .map(|b| b + f64::from(fret) * LOG_B_PER_FRET),
                    Templates::PerPitch => None,
                };
                if templates == Templates::PerPitch && atoms.iter().any(|a: &Atom| a.midi == midi) {
                    continue;
                }
                atoms.push(Atom {
                    midi,
                    string,
                    spectrum: self.template(midi, log_b),
                });
            }
        }
        atoms
    }
}

fn normalised(v: Vec<f64>) -> Vec<f64> {
    let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt().max(1e-12);
    v.into_iter().map(|x| x / norm).collect()
}

/// h ≥ 0 minimising ‖v − W·h‖² (Lee–Seung multiplicative updates, W fixed).
fn nnls(atoms: &[Atom], v: &[f64]) -> Vec<f64> {
    let wt_v: Vec<f64> = atoms
        .iter()
        .map(|a| a.spectrum.iter().zip(v).map(|(w, x)| w * x).sum())
        .collect();
    let gram: Vec<Vec<f64>> = atoms
        .iter()
        .map(|a| {
            atoms
                .iter()
                .map(|b| a.spectrum.iter().zip(&b.spectrum).map(|(x, y)| x * y).sum())
                .collect()
        })
        .collect();
    let mut h = vec![1.0; atoms.len()];
    for _ in 0..ITERATIONS {
        for i in 0..h.len() {
            let wt_wh: f64 = gram[i].iter().zip(&h).map(|(g, x)| g * x).sum();
            h[i] *= wt_v[i] / wt_wh.max(1e-12);
        }
    }
    h
}

/// Activation per pitch (summed over strings) and the strongest string of each pitch.
fn per_pitch(atoms: &[Atom], h: &[f64]) -> Vec<(u8, f64, u8)> {
    let mut out: Vec<(u8, f64, u8, f64)> = Vec::new();
    for (a, x) in atoms.iter().zip(h) {
        match out.iter_mut().find(|p| p.0 == a.midi) {
            Some(p) => {
                p.1 += x;
                if *x > p.3 {
                    p.2 = a.string;
                    p.3 = *x;
                }
            }
            None => out.push((a.midi, *x, a.string, *x)),
        }
    }
    out.into_iter().map(|(m, x, s, _)| (m, x, s)).collect()
}

/// Which files: all, or every other one (θ chosen on one half, reported on the other).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Half {
    All,
    Odd,
    Even,
}

impl Half {
    fn takes(self, index: usize) -> bool {
        match self {
            Half::All => true,
            Half::Odd => !index.is_multiple_of(2),
            Half::Even => index.is_multiple_of(2),
        }
    }
}

/// Counts per threshold: blind sets right / heard, informed both / told.
#[derive(Default, Clone, Copy)]
struct Count {
    blind_exact: usize,
    informed_both: usize,
}

#[derive(Default)]
struct Tally {
    dyads: usize,
    singles: usize,
    on_dyads: [Count; THRESHOLDS.len()],
    on_singles: [Count; THRESHOLDS.len()],
    single_string_right: usize,
    single_string_known: usize,
}

fn activation(p: &[(u8, f64, u8)], midi: u8) -> f64 {
    p.iter().find(|x| x.0 == midi).map_or(0.0, |x| x.1)
}

/// The pitches over `theta` of the strongest.
fn blind(p: &[(u8, f64, u8)], theta: f64) -> Vec<u8> {
    let max = p.iter().map(|x| x.1).fold(0.0, f64::max);
    p.iter()
        .filter(|x| x.1 >= theta * max && max > 0.0)
        .map(|x| x.0)
        .collect()
}

fn informed(p: &[(u8, f64, u8)], a: u8, b: u8, theta: f64) -> bool {
    let (x, y) = (activation(p, a), activation(p, b));
    let max = p.iter().map(|x| x.1).fold(0.0, f64::max);
    x.min(y) >= theta * max
}

fn frame_at(samples: &[f32], sr: f64, onset_s: f64, len: usize) -> Option<&[f32]> {
    let from = ((onset_s + SKIP_S) * sr) as usize;
    samples.get(from..from + len)
}

fn read_item(json: &Path) -> Option<(KorpusItem, Vec<Option<u8>>)> {
    let text = std::fs::read_to_string(json).ok()?;
    let mut item: KorpusItem = serde_json::from_str(&text).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    let strings = item
        .notes
        .iter()
        .enumerate()
        .map(|(i, _)| {
            value["meta"]["note_meta"][i]["string"]
                .as_u64()
                .and_then(|s| u8::try_from(s).ok()?.checked_sub(1))
        })
        .collect();
    let audio = read_audio(&json.with_extension("wav")).ok()?;
    item.sample_rate = audio.sample_rate;
    item.samples = audio.mono();
    Some((item, strings))
}

fn run(dir: &Path, templates: Templates, half: Half) -> Tally {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|e| e.filter_map(Result::ok).map(|e| e.path()).collect())
        .unwrap_or_default();
    files.retain(|p| p.extension().is_some_and(|x| x == "json"));
    files.sort();
    let mut tally = Tally::default();
    for (n, json) in files.iter().enumerate() {
        if !half.takes(n) {
            continue;
        }
        let Some((item, strings)) = read_item(json) else {
            continue;
        };
        let sr = f64::from(item.sample_rate);
        let analyzer = Analyzer::new(sr);
        let atoms = analyzer.atoms(&item.set.instrument(), templates);
        let len = analyzer.frame_len();
        let mixed = double_stop_mixtures(&item, PER_INTERVAL, n as u64 + 1);
        for s in mixed.strikes.iter().filter(|s| s.upper - s.lower != OCTAVE) {
            let Some(frame) = frame_at(&mixed.samples, sr, s.onset_s, len) else {
                continue;
            };
            let p = per_pitch(&atoms, &nnls(&atoms, &analyzer.spectrum(frame)));
            tally.dyads += 1;
            for (i, theta) in THRESHOLDS.iter().enumerate() {
                let mut heard = blind(&p, *theta);
                heard.sort_unstable();
                let c = &mut tally.on_dyads[i];
                c.blind_exact += usize::from(heard == [s.lower, s.upper]);
                c.informed_both += usize::from(informed(&p, s.lower, s.upper, *theta));
            }
        }
        for (k, note) in item.notes.iter().enumerate() {
            let next = item.notes.get(k + 1).map_or(f64::INFINITY, |m| m.onset_s);
            if note.technique.is_some() || next - note.onset_s < SEGMENT_MIN_S {
                continue;
            }
            let Some(frame) = frame_at(&item.samples, sr, note.onset_s, len) else {
                continue;
            };
            let midi = note.midi.round() as u8;
            let p = per_pitch(&atoms, &nnls(&atoms, &analyzer.spectrum(frame)));
            tally.singles += 1;
            if let Some(string) = strings[k] {
                tally.single_string_known += 1;
                let told = p.iter().find(|x| x.0 == midi).map(|x| x.2);
                tally.single_string_right += usize::from(told == Some(string));
            }
            for (i, theta) in THRESHOLDS.iter().enumerate() {
                let c = &mut tally.on_singles[i];
                c.blind_exact += usize::from(blind(&p, *theta) == [midi]);
                let alarms = SECOND_TONES
                    .iter()
                    .filter_map(|d| u8::try_from(i32::from(midi) + d).ok())
                    .filter(|other| {
                        let (a, b) = (midi.min(*other), midi.max(*other));
                        informed(&p, a, b, *theta)
                    })
                    .count();
                c.informed_both += alarms;
            }
        }
    }
    tally
}

#[test]
#[ignore = "feasibility study M4: needs the converted IDMT lines (KONTRA_IDMT)"]
fn nnls_with_string_templates() {
    let Some(dir) = std::env::var_os("KONTRA_IDMT").map(PathBuf::from) else {
        println!("KONTRA_IDMT not set: nothing measured");
        return;
    };
    let pct = |a: usize, b: usize| a as f64 / b.max(1) as f64 * 100.0;
    let runs = [
        (Templates::PerString, Half::All),
        (Templates::PerPitch, Half::All),
        (Templates::PerString, Half::Odd),
        (Templates::PerString, Half::Even),
    ];
    for (templates, half) in runs {
        let t = run(&dir, templates, half);
        println!(
            "{templates:?}, {half:?} files: {} dyads (no octaves), {} single notes; string of single notes \
             right {:.1} % of {}",
            t.dyads,
            t.singles,
            pct(t.single_string_right, t.single_string_known),
            t.single_string_known
        );
        for (i, theta) in THRESHOLDS.iter().enumerate() {
            let (d, s) = (t.on_dyads[i], t.on_singles[i]);
            println!(
                "  θ {theta:.1}: blind dyad set right {:5.1} %, single set right {:5.1} % | \
                 informed both {:5.1} %, single false alarm {:5.1} %",
                pct(d.blind_exact, t.dyads),
                pct(s.blind_exact, t.singles),
                pct(d.informed_both, t.dyads),
                pct(s.informed_both, t.singles * SECOND_TONES.len())
            );
        }
    }
}
