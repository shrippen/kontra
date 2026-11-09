//! Note ends in the corpus (E5, LH-BEW-10): how well the tracker tells where a note stopped.
//!
//! ```text
//! annotated note ─ gap ≥ 50 ms to the next? ─ yes: stopped by the player (muted) ─► told Muted?
//!                                                   end told − annotated offset (ms)
//! any note told Muted ≥ 50 ms before its offset ─► an early (false) mute
//! ```
//!
//! Real offsets: IDMT-SMT-Bass-Single-Track (`meta.note_meta[i].offset_s`), score-grid times
//! like its onsets. The synthetic staccato set has exact ones.

use super::{Annotation, KorpusItem, KorpusSet, SAMPLE_RATE};
use crate::offline::analyze_reported;
use kontra_dsp::synth::{render, Noise, SynthNote, ToneShape};
use kontra_model::{hz_to_midi, DetectedNote, Ending, NoteEnd, A4_HZ};

/// A rest at least this long after a note means the player stopped it.
const GAP_S: f64 = 0.05;
/// The synthetic mute: the tone fades over its last 15 ms (`synth::render`); its offset is
/// where the fade starts, the hand touching the string.
const SYNTH_MUTE_S: f64 = 0.015;
/// Staccato set: notes and rests (seconds, drawn per note), frets 0–7 on every string.
const NOTE_S: (f64, f64) = (0.25, 0.5);
const REST_S: (f64, f64) = (0.1, 0.3);
const FRETS: u8 = 7;
const VELOCITIES: [f32; 2] = [0.4, 0.8];
const LEAD_S: f64 = 0.3;
/// A mute told this long before the annotated offset is false: the note still sounded.
const EARLY_S: f64 = 0.05;
/// Two times on one clock name the same note within this.
const SAME_S: f64 = 1e-6;
const MS_PER_S: f64 = 1000.0;
/// Where an early mute is checked for sound: from this long after its end (the fall's tail) to
/// this long before the next note (IDMT onsets lie up to 40 ms off the audio).
const AFTER_END_S: f64 = 0.04;
const BEFORE_NEXT_S: f64 = 0.05;
/// A note's start level: peak over this long from its onset.
const START_S: f64 = 0.15;
/// Sounding again: within 10 dB of the start (−10 dB as a factor on magnitudes).
const SOUNDING: f32 = 0.316;

/// A stretch of a recording (seconds).
#[derive(Debug, Clone, Copy)]
struct Span {
    from_s: f64,
    to_s: f64,
}

impl Span {
    fn start(onset_s: f64) -> Span {
        Span {
            from_s: onset_s,
            to_s: onset_s + START_S,
        }
    }
}

/// Peak magnitude of `samples` over `span` (0 when empty).
fn peak(samples: &[f32], sample_rate: f64, span: Span) -> f32 {
    let at = |s: f64| ((s.max(0.0) * sample_rate) as usize).min(samples.len());
    let (from, to) = (at(span.from_s), at(span.to_s));
    samples
        .get(from..to.max(from))
        .map_or(0.0, |s| s.iter().fold(0.0, |m, x| x.abs().max(m)))
}

/// Ends of one or more recordings.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EndTally {
    /// Annotated notes followed by a rest (`GAP_S`): stopped by the player.
    pub stopped: usize,
    /// … of them found (a detected note within the window), and how their end was told.
    pub stopped_found: usize,
    pub muted: usize,
    pub decayed: usize,
    pub replaced: usize,
    pub untold: usize,
    /// Told end − annotated offset (ms) of each stopped note told `Muted`.
    pub errors_ms: Vec<f64>,
    /// Annotated notes with an offset found, and of them told `Muted` `EARLY_S` or more before
    /// it.
    pub found: usize,
    pub early: usize,
    /// … of the early ones, those after which the recording sounds again within 10 dB of the
    /// note's start before the next note: a false mute. The rest end in silence, so the grid
    /// offset lies late, not the mute (IDMT offsets are score-grid times).
    pub early_sounding: usize,
}

impl EndTally {
    pub fn add(&mut self, other: &EndTally) {
        self.stopped += other.stopped;
        self.stopped_found += other.stopped_found;
        self.muted += other.muted;
        self.decayed += other.decayed;
        self.replaced += other.replaced;
        self.untold += other.untold;
        self.errors_ms.extend_from_slice(&other.errors_ms);
        self.found += other.found;
        self.early += other.early;
        self.early_sounding += other.early_sounding;
    }

    /// Share of the stopped notes found whose end was told `Muted` within `ms`.
    pub fn within(&self, ms: f64) -> f64 {
        let near = self.errors_ms.iter().filter(|e| e.abs() <= ms).count();
        share(near, self.stopped_found)
    }

    /// Quantile `q` (0..=1) of the absolute end error, ms.
    pub fn abs_quantile(&self, q: f64) -> Option<f64> {
        let mut abs: Vec<f64> = self.errors_ms.iter().map(|e| e.abs()).collect();
        quantile(&mut abs, q)
    }

    /// Median signed end error (ms, positive: told late).
    pub fn median_ms(&self) -> Option<f64> {
        quantile(&mut self.errors_ms.clone(), 0.5)
    }

    /// Share of the notes found told `Muted` `EARLY_S` before their annotated offset.
    pub fn early_mutes(&self) -> f64 {
        share(self.early, self.found)
    }

    /// Share of the notes found told `Muted` while they still sounded (`early_sounding`).
    pub fn false_mutes(&self) -> f64 {
        share(self.early_sounding, self.found)
    }
}

fn share(part: usize, whole: usize) -> f64 {
    match whole {
        0 => 0.0,
        w => part as f64 / w as f64,
    }
}

fn quantile(values: &mut [f64], q: f64) -> Option<f64> {
    values.sort_by(f64::total_cmp);
    let last = values.len().checked_sub(1)?;
    values.get(((last as f64) * q).round() as usize).copied()
}

/// Runs the tracker over `samples` and compares the ends told with `offsets` (seconds, one per
/// annotated note, `None` where not annotated); a detection belongs to the note within
/// `window_s`.
pub fn evaluate_ends(
    set: KorpusSet,
    samples: &[f32],
    sample_rate: u32,
    notes: &[Annotation],
    offsets: &[Option<f64>],
    window_s: f64,
) -> EndTally {
    let sr = f64::from(sample_rate);
    let told = analyze_reported(samples, sr, &set.instrument(), set.source(), &[]);
    let mut tally = EndTally::default();
    for (i, note) in notes.iter().enumerate() {
        let Some(offset_s) = offsets.get(i).copied().flatten() else {
            continue;
        };
        let next_s = notes.get(i + 1).map_or(f64::INFINITY, |n| n.onset_s);
        let stopped = next_s - offset_s >= GAP_S;
        tally.stopped += usize::from(stopped);
        let Some(found) = nearest(&told.notes, note.onset_s, window_s) else {
            continue;
        };
        let end = told
            .ends
            .iter()
            .find(|e| (e.onset_s - found.onset_s).abs() < SAME_S);
        tally.found += 1;
        let early = |e: &NoteEnd| e.ending == Ending::Muted && e.end_s <= offset_s - EARLY_S;
        tally.early += usize::from(end.is_some_and(early));
        let sounding = |e: &NoteEnd| {
            let back = Span {
                from_s: e.end_s + AFTER_END_S,
                to_s: next_s.min(offset_s) - BEFORE_NEXT_S,
            };
            peak(samples, sr, back) > peak(samples, sr, Span::start(note.onset_s)) * SOUNDING
        };
        tally.early_sounding += usize::from(end.is_some_and(|e| early(e) && sounding(e)));
        if !stopped {
            continue;
        }
        tally.stopped_found += 1;
        count(&mut tally, end, offset_s);
    }
    tally
}

/// Counts how the end of a stopped note was told.
fn count(tally: &mut EndTally, end: Option<&NoteEnd>, offset_s: f64) {
    let Some(end) = end else {
        tally.untold += 1;
        return;
    };
    match end.ending {
        Ending::Muted => {
            tally.muted += 1;
            tally.errors_ms.push((end.end_s - offset_s) * MS_PER_S);
        }
        Ending::Decayed => tally.decayed += 1,
        Ending::Replaced => tally.replaced += 1,
    }
}

fn nearest(notes: &[DetectedNote], onset_s: f64, window_s: f64) -> Option<&DetectedNote> {
    notes
        .iter()
        .filter(|n| (n.onset_s - onset_s).abs() <= window_s && !n.is_pitchless())
        .min_by(|a, b| {
            (a.onset_s - onset_s)
                .abs()
                .total_cmp(&(b.onset_s - onset_s).abs())
        })
}

/// A four-string line of muted notes with rests, every other pair played without a rest (the
/// next note replaces it): the annotation and each note's offset (exact).
pub fn generate_staccato() -> (KorpusItem, Vec<Option<f64>>) {
    let set = KorpusSet::Line4;
    let sr = f64::from(SAMPLE_RATE);
    let mut noise = Noise::new(set as u64 + 31);
    let mut draw = |(low, high): (f64, f64)| low + (high - low) * f64::from(noise.sample().abs());
    let mut synth = Vec::new();
    let mut offsets = Vec::new();
    let mut t = LEAD_S;
    for velocity in VELOCITIES {
        for open in &set.instrument().tuning.strings {
            for fret in 0..=FRETS {
                let duration_s = draw(NOTE_S);
                let midi = f64::from(open.0 + fret);
                synth.push(SynthNote {
                    onset_s: t,
                    hz: A4_HZ * 2f64.powf((midi - 69.0) / 12.0),
                    duration_s,
                    velocity,
                });
                offsets.push(Some(t + duration_s - SYNTH_MUTE_S));
                let rest = if fret % 4 < 2 { draw(REST_S) } else { 0.0 };
                t += duration_s + rest;
            }
        }
    }
    let samples = render(&synth, sr, t + LEAD_S, ToneShape::Rich, set as u64 + 37);
    let notes = synth
        .iter()
        .map(|n| Annotation {
            onset_s: n.onset_s,
            midi: hz_to_midi(n.hz, A4_HZ),
            technique: None,
            slide_cents: None,
        })
        .collect();
    let item = KorpusItem {
        set,
        sample_rate: SAMPLE_RATE,
        notes,
        samples,
    };
    (item, offsets)
}
