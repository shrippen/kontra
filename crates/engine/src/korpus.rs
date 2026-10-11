//! Synthetic test corpus and its evaluation (Lastenheft 7.3, LH-NF-02, LH-NF-03).
//!
//! Real recordings go to `testdata/korpus/local/` later; until then these generated sets stand in.
//! Each set is a WAV plus an annotation with the true onset and pitch of every note.

use crate::offline::analyze;
use kontra_dsp::synth::{laptop_jack, render, Noise, SynthNote, ToneShape};
use kontra_model::{
    hz_to_midi, DetectedNote, InputSource, Instrument, InstrumentKind, Tuning, A4_HZ,
};
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
                (InstrumentKind::Fretted, Tuning::bass4_standard())
            }
            KorpusSet::Line5 => (InstrumentKind::Fretted, Tuning::bass5_standard()),
            KorpusSet::Fretless5 => (InstrumentKind::Fretless, Tuning::bass5_standard()),
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
    /// 95th percentile of it: a mean hides one note that is always off (B0 +8 ct, J-B4).
    pub p95_abs_cents: f64,
    /// Largest of it; LH-BEW-05 holds per note.
    pub max_abs_cents: f64,
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
    evaluate_with(set, samples, sample_rate, notes, Matching::default()).0
}

/// How detections are matched to annotated notes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matching {
    /// Largest onset distance (real recordings with score-grid annotations need ±50–80 ms).
    pub window_s: f64,
    /// Only a detection with the right rounded pitch matches. Without it a pitchless or wrong
    /// detection near the note counts as found, which flatters recall on loose annotations.
    pub require_pitch: bool,
}

impl Default for Matching {
    fn default() -> Self {
        Matching {
            window_s: MATCH_S,
            require_pitch: false,
        }
    }
}

/// Like `evaluate`, with chosen matching; also returns the detections for inspection.
pub fn evaluate_with(
    set: KorpusSet,
    samples: &[f32],
    sample_rate: u32,
    notes: &[Annotation],
    matching: Matching,
) -> (Evaluation, Vec<DetectedNote>) {
    let detected = analyze(
        samples,
        f64::from(sample_rate),
        &set.instrument(),
        set.source(),
    );
    let evaluation = compare(set, notes, &detected, matching);
    (evaluation, detected)
}

/// `samples` as heard through a laptop combo jack (level, band limit, hum, noise).
pub fn simulate_laptop_jack(samples: &[f32], sample_rate: u32) -> Vec<f32> {
    laptop_jack(samples, f64::from(sample_rate), 3)
}

fn compare(
    set: KorpusSet,
    notes: &[Annotation],
    detected: &[DetectedNote],
    matching: Matching,
) -> Evaluation {
    let mut used = vec![false; detected.len()];
    let mut onset_errors = Vec::new();
    let mut cents_errors = Vec::new();
    let mut pitch_correct = 0;
    for note in notes {
        let best = detected
            .iter()
            .enumerate()
            .filter(|(i, d)| {
                !used[*i]
                    && (d.onset_s - note.onset_s).abs() <= matching.window_s
                    && (!matching.require_pitch || d.midi.round() == note.midi.round())
            })
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
    cents_errors.sort_by(f64::total_cmp);
    let mean = |v: &[f64]| {
        if v.is_empty() {
            0.0
        } else {
            v.iter().sum::<f64>() / v.len() as f64
        }
    };
    // Of a sorted list.
    let p95 = |v: &[f64]| {
        v.get((v.len() as f64 * 0.95) as usize)
            .or(v.last())
            .copied()
            .unwrap_or(0.0)
    };
    Evaluation {
        set,
        annotated: notes.len(),
        detected: detected.len(),
        matched: onset_errors.len(),
        pitch_correct,
        mean_abs_onset_ms: mean(&onset_errors),
        p95_abs_onset_ms: p95(&onset_errors),
        max_abs_onset_ms: onset_errors.last().copied().unwrap_or(0.0),
        mean_abs_cents: mean(&cents_errors),
        p95_abs_cents: p95(&cents_errors),
        max_abs_cents: cents_errors.last().copied().unwrap_or(0.0),
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
                // LH-BEW-05: intonation shown within 3 cents, for every note.
                assert!(
                    e.p95_abs_cents <= 3.0 && e.max_abs_cents <= 3.0,
                    "fretless cents p95 {:.1}, max {:.1}",
                    e.p95_abs_cents,
                    e.max_abs_cents
                );
            }
        }
    }
}

/// Deliberate mistakes for a rendered performance (QA journeys).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Mistakes {
    /// Constant timing offset in ms (positive = late).
    pub offset_ms: f64,
    /// Every n-th note one semitone too high (0 = never).
    pub wrong_every: usize,
    /// Every n-th note left out (0 = never).
    pub miss_every: usize,
    /// Detuning in cents for every note (fretless intonation tests).
    pub detune_cents: f64,
    /// More detuning per note, repeating: `[12.0, -25.0]` makes note 1 +12, note 2 −25,
    /// note 3 +12 … cents (on top of `detune_cents`).
    pub detune_cycle_cents: &'static [f64],
}

/// Renders an arrangement as a synthetic bass performance at `speed`, starting `lead_in_s` into
/// the audio (e.g. the count-in), at `sample_rate`.
pub fn render_performance(
    arrangement: &kontra_model::Arrangement,
    speed: f64,
    lead_in_s: f64,
    mistakes: Mistakes,
    sample_rate: u32,
) -> Vec<f32> {
    let speed = speed.clamp(0.1, 2.0);
    let mut notes = Vec::new();
    for (i, note) in arrangement
        .notes
        .iter()
        .filter(|n| n.needs_attack())
        .enumerate()
    {
        let n = i + 1;
        if mistakes.miss_every > 0 && n % mistakes.miss_every == 0 {
            continue;
        }
        let wrong = mistakes.wrong_every > 0 && n % mistakes.wrong_every == 0;
        let cycle = mistakes
            .detune_cycle_cents
            .get(i % mistakes.detune_cycle_cents.len().max(1))
            .copied()
            .unwrap_or(0.0);
        let cents = mistakes.detune_cents + cycle;
        let midi = f64::from(note.pitch.0) + if wrong { 1.0 } else { 0.0 } + cents / 100.0;
        let start = arrangement.tempo.seconds_at(note.start_ticks) / speed;
        let end = arrangement.tempo.seconds_at(note.end_ticks()) / speed;
        notes.push(SynthNote {
            onset_s: lead_in_s + start + mistakes.offset_ms / 1000.0,
            hz: arrangement.instrument.a4_hz * 2f64.powf((midi - 69.0) / 12.0),
            duration_s: (end - start).max(0.05),
            velocity: 0.7,
        });
    }
    let total = lead_in_s + arrangement.duration_s() / speed + 1.0;
    render(&notes, f64::from(sample_rate), total, ToneShape::Rich, 42)
}
