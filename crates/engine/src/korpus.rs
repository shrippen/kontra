//! Synthetic test corpus and its evaluation (Lastenheft 7.3, LH-NF-02, LH-NF-03).
//!
//! Real recordings go to `testdata/korpus/local/` later; until then these generated sets stand in.
//! Each set is a WAV plus an annotation with the true onset and pitch of every note.

use crate::offline::analyze;
use kontra_dsp::synth::{laptop_jack, render, Noise, SynthNote, ToneShape};
use kontra_model::{hz_to_midi, BassKind, DetectedNote, InputSource, Instrument, Tuning, A4_HZ};
use serde::{Deserialize, Serialize};

pub const SAMPLE_RATE: u32 = 48_000;
/// A detection counts as the annotated note if its onset is this close.
const MATCH_S: f64 = 0.03;

/// One annotated note.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Annotation {
    pub onset_s: f64,
    /// Fractional MIDI number including any detuning.
    pub midi: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KorpusSet {
    /// Four-string bass, line input, frets 0–12 on every string, three dynamics.
    Line4,
    /// Five-string bass including the low B.
    Line5,
    /// The Line4 material through a simulated laptop combo jack.
    LaptopJack4,
    /// Fretless five-string, every note detuned by ±5–40 cents.
    Fretless5,
    /// Sixteenth notes at 120 BPM with repeated pitches.
    Sixteenths,
}

impl KorpusSet {
    pub const ALL: [KorpusSet; 5] = [
        KorpusSet::Line4,
        KorpusSet::Line5,
        KorpusSet::LaptopJack4,
        KorpusSet::Fretless5,
        KorpusSet::Sixteenths,
    ];

    pub fn name(self) -> &'static str {
        match self {
            KorpusSet::Line4 => "line-4",
            KorpusSet::Line5 => "line-5",
            KorpusSet::LaptopJack4 => "laptop-jack-4",
            KorpusSet::Fretless5 => "fretless-5",
            KorpusSet::Sixteenths => "sixteenths",
        }
    }

    pub fn instrument(self) -> Instrument {
        let (kind, tuning) = match self {
            KorpusSet::Line4 | KorpusSet::LaptopJack4 | KorpusSet::Sixteenths => {
                (BassKind::Fretted, Tuning::bass4_standard())
            }
            KorpusSet::Line5 => (BassKind::Fretted, Tuning::bass5_standard()),
            KorpusSet::Fretless5 => (BassKind::Fretless, Tuning::bass5_standard()),
        };
        Instrument::new(kind, tuning, 24).unwrap_or_else(|_| Instrument::default_bass())
    }

    pub fn source(self) -> InputSource {
        match self {
            KorpusSet::LaptopJack4 => InputSource::LaptopJack,
            _ => InputSource::Line,
        }
    }
}

/// Generated audio with its annotation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KorpusItem {
    pub set: KorpusSet,
    pub sample_rate: u32,
    pub notes: Vec<Annotation>,
    #[serde(skip)]
    pub samples: Vec<f32>,
}

/// Renders one set (deterministic).
pub fn generate(set: KorpusSet) -> KorpusItem {
    let instrument = set.instrument();
    let sr = f64::from(SAMPLE_RATE);
    let mut noise = Noise::new(set as u64 + 1);
    let mut synth = Vec::new();
    let (spacing, frets) = match set {
        KorpusSet::Sixteenths => (0.125, 5u8),
        _ => (0.3, 12u8),
    };
    let velocities: &[f32] = if set == KorpusSet::Sixteenths {
        &[0.7]
    } else {
        &[0.3, 0.6, 0.9]
    };
    let mut t = 0.3;
    for velocity in velocities {
        for open in &instrument.tuning.strings {
            for fret in 0..=frets {
                let midi = f64::from(open.0) + f64::from(fret);
                let detune = if set == KorpusSet::Fretless5 {
                    f64::from(noise.sample()) * 0.4
                } else {
                    0.0
                };
                let detune = if set == KorpusSet::Fretless5 && detune.abs() < 0.05 {
                    0.05f64.copysign(detune)
                } else {
                    detune
                };
                let hz = A4_HZ * 2f64.powf((midi + detune - 69.0) / 12.0);
                synth.push(SynthNote {
                    onset_s: t,
                    hz,
                    duration_s: spacing * 0.9,
                    velocity: *velocity,
                });
                t += spacing;
                if set == KorpusSet::Sixteenths && fret % 2 == 0 {
                    synth.push(SynthNote {
                        onset_s: t,
                        hz,
                        duration_s: spacing * 0.9,
                        velocity: *velocity,
                    });
                    t += spacing;
                }
            }
        }
    }
    let shape = if set == KorpusSet::LaptopJack4 {
        ToneShape::Dull
    } else {
        ToneShape::Rich
    };
    let mut samples = render(&synth, sr, t + 0.5, shape, set as u64 + 7);
    if set == KorpusSet::LaptopJack4 {
        samples = laptop_jack(&samples, sr, 3);
    }
    let notes = synth
        .iter()
        .map(|n| Annotation {
            onset_s: n.onset_s,
            midi: hz_to_midi(n.hz, A4_HZ),
        })
        .collect();
    KorpusItem {
        set,
        sample_rate: SAMPLE_RATE,
        notes,
        samples,
    }
}

/// Detection quality of one set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Evaluation {
    pub set: KorpusSet,
    pub annotated: usize,
    pub detected: usize,
    /// Annotated notes with a detection within ±30 ms.
    pub matched: usize,
    /// Matched notes whose rounded pitch is right.
    pub pitch_correct: usize,
    pub mean_abs_onset_ms: f64,
    pub p95_abs_onset_ms: f64,
    pub max_abs_onset_ms: f64,
    /// Mean absolute pitch error of matched notes in cents (meaningful for fretless).
    pub mean_abs_cents: f64,
}

impl Evaluation {
    pub fn recall(&self) -> f64 {
        ratio(self.matched, self.annotated)
    }

    pub fn precision(&self) -> f64 {
        ratio(self.matched, self.detected)
    }

    pub fn pitch_accuracy(&self) -> f64 {
        ratio(self.pitch_correct, self.matched)
    }
}

fn ratio(a: usize, b: usize) -> f64 {
    if b == 0 {
        return 0.0;
    }
    a as f64 / b as f64
}

/// Analyzes `samples` and compares with `notes`.
pub fn evaluate(
    set: KorpusSet,
    samples: &[f32],
    sample_rate: u32,
    notes: &[Annotation],
) -> Evaluation {
    let detected = analyze(
        samples,
        f64::from(sample_rate),
        &set.instrument(),
        set.source(),
    );
    compare(set, notes, &detected)
}

fn compare(set: KorpusSet, notes: &[Annotation], detected: &[DetectedNote]) -> Evaluation {
    let mut used = vec![false; detected.len()];
    let mut onset_errors = Vec::new();
    let mut cents_errors = Vec::new();
    let mut pitch_correct = 0;
    for note in notes {
        let best = detected
            .iter()
            .enumerate()
            .filter(|(i, d)| !used[*i] && (d.onset_s - note.onset_s).abs() <= MATCH_S)
            .min_by(|a, b| {
                (a.1.onset_s - note.onset_s)
                    .abs()
                    .total_cmp(&(b.1.onset_s - note.onset_s).abs())
            });
        let Some((index, d)) = best else {
            continue;
        };
        used[index] = true;
        onset_errors.push(((d.onset_s - note.onset_s) * 1000.0).abs());
        cents_errors.push(((d.midi - note.midi) * 100.0).abs());
        if d.midi.round() == note.midi.round() {
            pitch_correct += 1;
        }
    }
    onset_errors.sort_by(f64::total_cmp);
    let mean = |v: &[f64]| {
        if v.is_empty() {
            0.0
        } else {
            v.iter().sum::<f64>() / v.len() as f64
        }
    };
    let p95 = onset_errors
        .get((onset_errors.len() as f64 * 0.95) as usize)
        .or(onset_errors.last())
        .copied()
        .unwrap_or(0.0);
    Evaluation {
        set,
        annotated: notes.len(),
        detected: detected.len(),
        matched: onset_errors.len(),
        pitch_correct,
        mean_abs_onset_ms: mean(&onset_errors),
        p95_abs_onset_ms: p95,
        max_abs_onset_ms: onset_errors.last().copied().unwrap_or(0.0),
        mean_abs_cents: mean(&cents_errors),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lastenheft LH-NF-02 (onset < 10 ms) and LH-NF-03 (pitch ≥ 98 % line, ≥ 95 % laptop jack).
    #[test]
    fn corpus_meets_requirements() {
        for set in KorpusSet::ALL {
            let item = generate(set);
            let e = evaluate(set, &item.samples, item.sample_rate, &item.notes);
            eprintln!("{e:?}");
            assert!(
                e.recall() >= 0.99,
                "{}: recall {:.3}",
                set.name(),
                e.recall()
            );
            assert!(
                e.precision() >= 0.99,
                "{}: precision {:.3}",
                set.name(),
                e.precision()
            );
            assert!(
                e.p95_abs_onset_ms < 10.0,
                "{}: p95 onset {:.1} ms",
                set.name(),
                e.p95_abs_onset_ms
            );
            let needed = if set == KorpusSet::LaptopJack4 {
                0.95
            } else {
                0.98
            };
            assert!(
                e.pitch_accuracy() >= needed,
                "{}: pitch {:.3}",
                set.name(),
                e.pitch_accuracy()
            );
            if set == KorpusSet::Fretless5 {
                assert!(
                    e.mean_abs_cents < 6.0,
                    "fretless cents {:.1}",
                    e.mean_abs_cents
                );
            }
        }
    }
}
