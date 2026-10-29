//! Synthetic test corpus and its evaluation (Lastenheft 7.3, LH-NF-02, LH-NF-03).
//!
//! Real recordings go to `testdata/korpus/local/` later; until then these generated sets stand in.
//! Each set is a WAV plus an annotation with the true onset and pitch of every note.

use crate::offline::{analyze_reported, cancel_echo};
use kontra_dsp::synth::{
    laptop_jack, render, render_articulated, room, Articulation, Noise, Played, SynthNote,
    ToneShape,
};
use kontra_model::{
    hz_to_midi, DetectedNote, InputSource, Instrument, InstrumentKind, Technique, Tuning, A4_HZ,
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
    /// How it is played (LH-BEW-06); `None`: a plain pluck.
    #[serde(default)]
    pub technique: Option<Technique>,
    /// The note slides later by this many cents to a pitch that is not annotated (IDMT: 100 or
    /// 200, direction not given). Kontra hears that as a `Slide` where it arrives.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slide_cents: Option<f64>,
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
    /// Double bass pizzicato in a living room through a microphone (LH-EIN-11, P6).
    RoomDouble,
    /// The same with a band from the speakers in the room (no headphones): kick, snare,
    /// chords and its own bass line, as Kontra plays it.
    RoomDoubleBacking,
    /// Hammer-on, pull-off, slide, slap, pop and ghost notes among plucks (LH-BEW-06).
    Techniques,
}

impl KorpusSet {
    pub const ALL: [KorpusSet; 8] = [
        KorpusSet::Line4,
        KorpusSet::Line5,
        KorpusSet::LaptopJack4,
        KorpusSet::Fretless5,
        KorpusSet::Sixteenths,
        KorpusSet::RoomDouble,
        KorpusSet::RoomDoubleBacking,
        KorpusSet::Techniques,
    ];

    pub fn name(self) -> &'static str {
        match self {
            KorpusSet::Line4 => "line-4",
            KorpusSet::Line5 => "line-5",
            KorpusSet::LaptopJack4 => "laptop-jack-4",
            KorpusSet::Fretless5 => "fretless-5",
            KorpusSet::Sixteenths => "sixteenths",
            KorpusSet::RoomDouble => "room-double",
            KorpusSet::RoomDoubleBacking => "room-double-backing",
            KorpusSet::Techniques => "techniques",
        }
    }

    pub fn instrument(self) -> Instrument {
        let (kind, tuning) = match self {
            KorpusSet::Line4
            | KorpusSet::LaptopJack4
            | KorpusSet::Sixteenths
            | KorpusSet::Techniques => (InstrumentKind::Fretted, Tuning::bass4_standard()),
            KorpusSet::Line5 => (InstrumentKind::Fretted, Tuning::bass5_standard()),
            KorpusSet::Fretless5 => (InstrumentKind::Fretless, Tuning::bass5_standard()),
            KorpusSet::RoomDouble | KorpusSet::RoomDoubleBacking => {
                (InstrumentKind::Double, Tuning::bass4_standard())
            }
        };
        Instrument::new(kind, tuning, 24).unwrap_or_else(|_| Instrument::default_bass())
    }

    pub fn source(self) -> InputSource {
        match self {
            KorpusSet::LaptopJack4 => InputSource::LaptopJack,
            KorpusSet::RoomDouble | KorpusSet::RoomDoubleBacking => InputSource::Microphone,
            _ => InputSource::Line,
        }
    }
}

/// The band in the room, relative to the double bass at the microphone: speakers about as loud
/// as the instrument (a living room, no headphones).
const BAND_IN_ROOM: f32 = 0.8;
const BAND_BPM: f64 = 100.0;

/// What Kontra plays to the speakers for `RoomDoubleBacking` (mono, dry): kick on 1 and 3,
/// snare on 2 and 4, a chord per bar and the band's own bass line, an octave apart from the
/// player's notes. Deterministic; the same length as the take.
pub fn band(len: usize, sample_rate: f64) -> Vec<f32> {
    let beat_s = 60.0 / BAND_BPM;
    let total_s = len as f64 / sample_rate;
    let beats = (total_s / beat_s) as usize;
    let mut out = vec![0.0f32; len];
    let mut noise = Noise::new(11);

    // Drums: a falling sine for the kick, a noise burst over 190 Hz for the snare.
    for b in 0..beats {
        let start = (b as f64 * beat_s * sample_rate) as usize;
        let kick = b % 2 == 0;
        for i in 0..(0.15 * sample_rate) as usize {
            let Some(s) = out.get_mut(start + i) else {
                break;
            };
            let t = i as f64 / sample_rate;
            let decay = (-t / 0.05).exp() as f32;
            *s += if kick {
                let hz = 50.0 + 70.0 * (-t / 0.03).exp();
                0.6 * decay * (std::f64::consts::TAU * hz * t).sin() as f32
            } else {
                0.25 * decay * (noise.sample() + (std::f64::consts::TAU * 190.0 * t).sin() as f32)
            };
        }
    }

    // Chords (A minor, F, C, G; one per bar) and the band's bass on each beat.
    const CHORDS: [[f64; 3]; 4] = [
        [57.0, 60.0, 64.0],
        [53.0, 57.0, 60.0],
        [48.0, 52.0, 55.0],
        [55.0, 59.0, 62.0],
    ];
    const ROOTS: [f64; 4] = [45.0, 41.0, 48.0, 43.0];
    let hz = |midi: f64| A4_HZ * 2f64.powf((midi - 69.0) / 12.0);
    let mut notes = Vec::new();
    for bar in 0..beats / 4 {
        let at = bar as f64 * 4.0 * beat_s;
        for &midi in &CHORDS[bar % 4] {
            notes.push(SynthNote {
                onset_s: at,
                hz: hz(midi),
                duration_s: 4.0 * beat_s * 0.95,
                velocity: 0.25,
            });
        }
    }
    let chords = render(&notes, sample_rate, total_s, ToneShape::Dull, 12);
    let bass_notes: Vec<SynthNote> = (0..beats)
        .map(|b| SynthNote {
            onset_s: b as f64 * beat_s,
            hz: hz(ROOTS[(b / 4) % 4]),
            duration_s: beat_s * 0.8,
            velocity: 0.5,
        })
        .collect();
    let bass = render(&bass_notes, sample_rate, total_s, ToneShape::Rich, 13);
    for (i, s) in out.iter_mut().enumerate() {
        *s += chords.get(i).copied().unwrap_or(0.0) + bass.get(i).copied().unwrap_or(0.0);
    }
    out
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

/// The technique set: on each open string a pluck, a hammer-on (+2), a pull-off back, a slide
/// (+5), a slap (low strings) or pop (high strings, an octave up) and a ghost note.
fn techniques() -> KorpusItem {
    use Articulation as A;
    let sr = f64::from(SAMPLE_RATE);
    let hz = |midi: f64| A4_HZ * 2f64.powf((midi - 69.0) / 12.0);
    let mut line = Vec::new();
    let mut notes = Vec::new();
    let mut t = 0.3;
    for velocity in [0.5f32, 0.8] {
        for root in [28.0, 33.0, 38.0, 43.0] {
            let (strike, strike_midi) = if root < 38.0 {
                (A::Slap, root)
            } else {
                (A::Pop, root + 12.0)
            };
            let steps: [(f64, A, Option<Technique>); 6] = [
                (root, A::Pluck, None),
                (root + 2.0, A::Legato, Some(Technique::HammerOn)),
                (root, A::Legato, Some(Technique::PullOff)),
                (
                    root + 5.0,
                    A::Slide {
                        glide_s: TECHNIQUE_GLIDE_S,
                    },
                    Some(Technique::Slide),
                ),
                (
                    strike_midi,
                    strike,
                    Some(if root < 38.0 {
                        Technique::Slap
                    } else {
                        Technique::Pop
                    }),
                ),
                (root, A::Ghost, Some(Technique::Muted)),
            ];
            for (midi, how, technique) in steps {
                line.push(Played {
                    note: SynthNote {
                        onset_s: t,
                        hz: hz(midi),
                        duration_s: TECHNIQUE_STEP_S * 0.9,
                        velocity,
                    },
                    how,
                });
                notes.push(Annotation {
                    onset_s: t,
                    midi,
                    technique,
                    slide_cents: None,
                });
                t += TECHNIQUE_STEP_S;
            }
        }
    }
    KorpusItem {
        set: KorpusSet::Techniques,
        sample_rate: SAMPLE_RATE,
        notes,
        samples: render_articulated(&line, sr, t + 0.5, 21),
    }
}

/// Spacing of the technique set and the glide of its slides.
const TECHNIQUE_STEP_S: f64 = 0.4;
const TECHNIQUE_GLIDE_S: f64 = 0.08;

/// Renders one set (deterministic).
pub fn generate(set: KorpusSet) -> KorpusItem {
    if set == KorpusSet::Techniques {
        return techniques();
    }
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
    let shape = match set {
        KorpusSet::LaptopJack4 | KorpusSet::RoomDouble | KorpusSet::RoomDoubleBacking => {
            ToneShape::Dull
        }
        _ => ToneShape::Rich,
    };
    let mut samples = render(&synth, sr, t + 0.5, shape, set as u64 + 7);
    match set {
        KorpusSet::LaptopJack4 => samples = laptop_jack(&samples, sr, 3),
        KorpusSet::RoomDouble => samples = room(&samples, sr, 5),
        KorpusSet::RoomDoubleBacking => {
            let band = band(samples.len(), sr);
            let mixed: Vec<f32> = samples
                .iter()
                .zip(&band)
                .map(|(bass, band)| bass + band * BAND_IN_ROOM)
                .collect();
            samples = room(&mixed, sr, 5);
        }
        _ => {}
    }
    let notes = synth
        .iter()
        .map(|n| Annotation {
            onset_s: n.onset_s,
            midi: hz_to_midi(n.hz, A4_HZ),
            technique: None,
            slide_cents: None,
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
    /// Matched notes whose technique (or plain pluck) is recognised (LH-BEW-06).
    #[serde(default)]
    pub technique_correct: usize,
    pub mean_abs_onset_ms: f64,
    pub p95_abs_onset_ms: f64,
    pub max_abs_onset_ms: f64,
    /// Mean absolute pitch error of matched notes in cents (meaningful for fretless).
    pub mean_abs_cents: f64,
    /// 95th percentile of it: a mean hides one note that is always off (B0 +8 ct, J-B4).
    pub p95_abs_cents: f64,
    /// Largest of it; LH-BEW-05 holds per note.
    pub max_abs_cents: f64,
    /// From the annotated onset to the first report with a pitch, 95th percentile (LH-NF-15).
    #[serde(default)]
    pub p95_report_ms: f64,
    #[serde(default)]
    pub max_report_ms: f64,
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

    /// Share of all annotated notes found with the right technique.
    pub fn technique_accuracy(&self) -> f64 {
        ratio(self.technique_correct, self.annotated)
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
    evaluate_as(set, set.source(), samples, sample_rate, notes, matching)
}

/// Like `evaluate_with`, with `samples` as heard through a laptop combo jack (level, band limit,
/// hum, noise) and analyzed with the laptop-jack profile, as P1 would play them.
pub fn evaluate_laptop_jack(
    set: KorpusSet,
    samples: &[f32],
    sample_rate: u32,
    notes: &[Annotation],
    matching: Matching,
) -> (Evaluation, Vec<DetectedNote>) {
    // Only a line recording goes through the jack; the others were heard through theirs.
    if set.source() != InputSource::Line {
        return evaluate_as(set, set.source(), samples, sample_rate, notes, matching);
    }
    let heard = laptop_jack(samples, f64::from(sample_rate), 3);
    evaluate_as(
        set,
        InputSource::LaptopJack,
        &heard,
        sample_rate,
        notes,
        matching,
    )
}

fn evaluate_as(
    set: KorpusSet,
    source: InputSource,
    samples: &[f32],
    sample_rate: u32,
    notes: &[Annotation],
    matching: Matching,
) -> (Evaluation, Vec<DetectedNote>) {
    // With a band in the room, Kontra subtracts what it played (the band) first.
    let rate = f64::from(sample_rate);
    let cleaned;
    let heard = match set {
        KorpusSet::RoomDoubleBacking => {
            cleaned = cancel_echo(samples, &band(samples.len(), rate));
            &cleaned
        }
        _ => samples,
    };
    let (detected, reported) = analyze_reported(heard, rate, &set.instrument(), source);
    let evaluation = compare(set, notes, &detected, &reported, matching);
    (evaluation, detected)
}

/// A harmonic counts in any octave, as in the session (the tracker often hears it lower).
fn cents_off(note: &Annotation, midi: f64) -> f64 {
    let cents = (midi - note.midi) * 100.0;
    if note.technique != Some(Technique::Harmonic) {
        return cents;
    }
    cents - (cents / 1200.0).round() * 1200.0
}

/// The right note: the same rounded pitch, for a harmonic in any octave.
pub fn same_pitch(note: &Annotation, midi: f64) -> bool {
    let semitones = midi.round() - note.midi.round();
    if note.technique == Some(Technique::Harmonic) {
        return semitones.rem_euclid(12.0) == 0.0;
    }
    semitones == 0.0
}

/// `reported`: when each detection was first told, seconds from the start.
fn compare(
    set: KorpusSet,
    notes: &[Annotation],
    detected: &[DetectedNote],
    reported: &[f64],
    matching: Matching,
) -> Evaluation {
    let mut used = vec![false; detected.len()];
    let mut onset_errors = Vec::new();
    let mut report_delays = Vec::new();
    let mut cents_errors = Vec::new();
    let mut pitch_correct = 0;
    let mut technique_correct = 0;
    for note in notes {
        let best = detected
            .iter()
            .enumerate()
            .filter(|(i, d)| {
                !used[*i]
                    && (d.onset_s - note.onset_s).abs() <= matching.window_s
                    && (!matching.require_pitch || same_pitch(note, d.midi))
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
        if d.technique == note.technique {
            technique_correct += 1;
        }
        // A ghost note has no pitch to get right.
        if note.technique == Some(Technique::Muted) {
            pitch_correct += 1;
            continue;
        }
        cents_errors.push(cents_off(note, d.midi).abs());
        if let Some(at_s) = reported.get(index) {
            report_delays.push((at_s - note.onset_s) * 1000.0);
        }
        if same_pitch(note, d.midi) {
            pitch_correct += 1;
        }
    }
    onset_errors.sort_by(f64::total_cmp);
    cents_errors.sort_by(f64::total_cmp);
    report_delays.sort_by(f64::total_cmp);
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
        technique_correct,
        mean_abs_onset_ms: mean(&onset_errors),
        p95_abs_onset_ms: p95(&onset_errors),
        max_abs_onset_ms: onset_errors.last().copied().unwrap_or(0.0),
        mean_abs_cents: mean(&cents_errors),
        p95_abs_cents: p95(&cents_errors),
        max_abs_cents: cents_errors.last().copied().unwrap_or(0.0),
        p95_report_ms: p95(&report_delays),
        max_report_ms: report_delays.last().copied().unwrap_or(0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Guard for LH-NF-15 until its target is confirmed: 10 ms over the first measurement.
    const REPORT_GUARD_MS: f64 = 110.0;

    /// Sets already heard through another input keep it: `korpus measure --laptop-jack` over a
    /// `korpus write` folder simulated laptop-jack-4 twice (second reader, 13.10.).
    #[test]
    fn laptop_jack_keeps_other_inputs() {
        for set in [KorpusSet::LaptopJack4, KorpusSet::RoomDoubleBacking] {
            let item = generate(set);
            let simulated = evaluate_laptop_jack(
                set,
                &item.samples,
                item.sample_rate,
                &item.notes,
                Matching::default(),
            )
            .0;
            let own = evaluate(set, &item.samples, item.sample_rate, &item.notes);
            assert_eq!(
                format!("{simulated:?}"),
                format!("{own:?}"),
                "{}",
                set.name()
            );
        }
    }

    /// Lastenheft LH-NF-02 (onset < 10 ms) and LH-NF-03 (pitch ≥ 98 % line, ≥ 95 % laptop jack,
    /// ≥ 90 % at a microphone in a room, LH-EIN-11).
    #[test]
    fn corpus_meets_requirements() {
        for set in KorpusSet::ALL {
            let item = generate(set);
            let e = evaluate(set, &item.samples, item.sample_rate, &item.notes);
            eprintln!("{e:?}");
            // LH-EIN-11: a microphone in a room needs ≥ 90 %.
            let room = matches!(set, KorpusSet::RoomDouble | KorpusSet::RoomDoubleBacking);
            let found = if room { 0.9 } else { 0.99 };
            assert!(
                e.recall() >= found,
                "{}: recall {:.3}",
                set.name(),
                e.recall()
            );
            assert!(
                e.precision() >= found,
                "{}: precision {:.3}",
                set.name(),
                e.precision()
            );
            // Legato notes have no attack: their time comes from the pitch window (±30 ms).
            let onset_ms = if set == KorpusSet::Techniques {
                30.0
            } else {
                10.0
            };
            assert!(
                e.p95_abs_onset_ms < onset_ms,
                "{}: p95 onset {:.1} ms",
                set.name(),
                e.p95_abs_onset_ms
            );
            let needed = match set {
                KorpusSet::LaptopJack4 => 0.95,
                // LH-EIN-11: ≥ 90 % at a microphone in a room.
                KorpusSet::RoomDouble | KorpusSet::RoomDoubleBacking => 0.9,
                _ => 0.98,
            };
            assert!(
                e.pitch_accuracy() >= needed,
                "{}: pitch {:.3}",
                set.name(),
                e.pitch_accuracy()
            );
            // LH-BEW-06: ≥ 85 % of the technique set recognised; elsewhere none invented.
            let technique_needed = if set == KorpusSet::Techniques {
                0.85
            } else {
                0.95
            };
            assert!(
                e.technique_accuracy() >= technique_needed,
                "{}: technique {:.3}",
                set.name(),
                e.technique_accuracy()
            );
            // LH-NF-15: told with a pitch soon after the attack. The target (70 ms, p95, line
            // sets) waits for the owner's word on the first measurement (29.10.: line-4 81 ms,
            // line-5 100 ms); until then a guard against getting slower.
            if matches!(set, KorpusSet::Line4 | KorpusSet::Line5) {
                assert!(
                    e.p95_report_ms > 0.0 && e.p95_report_ms <= REPORT_GUARD_MS,
                    "{}: p95 report {:.1} ms",
                    set.name(),
                    e.p95_report_ms
                );
            }
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
    /// How the notes are played: all plucked, or with the score's techniques.
    pub playing: Playing,
}

/// Articulation of a rendered performance (LH-BEW-06 journeys).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Playing {
    /// Every note plucked; hammer-on and pull-off targets are left out (no attack).
    #[default]
    Plucked,
    /// Slaps, pops, ghost notes, hammer-ons, pull-offs and slides as the score says; every
    /// `plain_every`-th note with a technique plucked instead (0 = never).
    Articulated { plain_every: usize },
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
    if let Playing::Articulated { plain_every } = mistakes.playing {
        return render_techniques(arrangement, speed, lead_in_s, plain_every, sample_rate);
    }
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

/// Every note with its technique: a slide marked on a note glides into the next one on its
/// string, a hammer-on or pull-off target changes the ringing pitch without an attack.
fn render_techniques(
    arrangement: &kontra_model::Arrangement,
    speed: f64,
    lead_in_s: f64,
    plain_every: usize,
    sample_rate: u32,
) -> Vec<f32> {
    use Articulation as A;
    let notes = &arrangement.notes;
    let slid_into: Vec<bool> = (0..notes.len())
        .map(|j| {
            notes[..j].iter().rev().any(|n| {
                n.techniques.contains(&Technique::Slide)
                    && n.string == notes[j].string
                    && n.end_ticks() <= notes[j].start_ticks + 1
                    && notes[j].start_ticks <= n.end_ticks() + kontra_model::TICKS_PER_QUARTER
            })
        })
        .collect();
    let mut line = Vec::new();
    let mut with_technique = 0;
    for (i, note) in notes.iter().enumerate() {
        let has = |t: Technique| note.techniques.contains(&t);
        let how = if slid_into[i] {
            A::Slide {
                glide_s: TECHNIQUE_GLIDE_S,
            }
        } else if has(Technique::HammerOn) || has(Technique::PullOff) {
            A::Legato
        } else if has(Technique::Slap) {
            A::Slap
        } else if has(Technique::Pop) {
            A::Pop
        } else if has(Technique::Muted) {
            A::Ghost
        } else {
            A::Pluck
        };
        let how = if how == A::Pluck {
            how
        } else {
            with_technique += 1;
            if plain_every > 0 && with_technique % plain_every == 0 {
                A::Pluck
            } else {
                how
            }
        };
        let start = arrangement.tempo.seconds_at(note.start_ticks) / speed;
        let end = arrangement.tempo.seconds_at(note.end_ticks()) / speed;
        line.push(Played {
            note: SynthNote {
                onset_s: lead_in_s + start,
                hz: arrangement.instrument.a4_hz
                    * 2f64.powf((f64::from(note.pitch.0) - 69.0) / 12.0),
                duration_s: (end - start).max(0.05) * 0.9,
                velocity: 0.7,
            },
            how,
        });
    }
    let total = lead_in_s + arrangement.duration_s() / speed + 1.0;
    render_articulated(&line, f64::from(sample_rate), total, 42)
}
