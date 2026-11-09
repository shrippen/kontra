//! Chords in the corpus as whole sets (M2, LH-BEW-11), MIDI first: every tone of a chord
//! played, none missing, none extra. Audio inputs hear at most two tones (`ChordHearing`);
//! a polyphonic detector later plugs in here.
//!
//! ```text
//! annotation: tones with one onset = a chord ─┐
//! detected notes within the window of it  ────┴─► set against set ─► complete / missing / extra
//! ```
//!
//! The generated set rolls each chord as a keyboard player does (tones 0–25 ms apart, any
//! order, the next chord's keys down before the last ones are up).

use super::Annotation;
use crate::input::midi_note;
use crate::EngineError;
use kontra_drivers::{read_midi_notes, MidiNote};
use kontra_dsp::synth::Noise;
use kontra_model::DetectedNote;
use std::path::Path;

/// Annotated tones this close in time form one chord (a score puts them on one onset).
const SAME_ONSET_S: f64 = 0.005;
/// Generated set: chord shapes (semitones over the root), roots, timing and velocities.
const SHAPES: [&[u8]; 6] = [
    &[0, 4, 7],
    &[0, 3, 7],
    &[0, 4, 7, 10],
    &[0, 3, 7, 10],
    &[0, 7, 12],
    &[0, 4, 7, 11],
];
const ROOTS: std::ops::RangeInclusive<u8> = 36..=55;
const ROLL_S: f64 = 0.025;
const CHORD_S: f64 = 0.6;
/// Keys go up this long after the next chord started (legato playing).
const OVERLAP_S: f64 = 0.03;
const LEAD_S: f64 = 0.5;
const VELOCITY_LOW: f32 = 40.0;
const VELOCITY_SPAN: f32 = 70.0;

/// Chords of one or more recordings, scored as sets.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChordTally {
    /// Annotated chords (two tones or more) and their tones.
    pub chords: usize,
    pub tones: usize,
    /// Chords with every tone and nothing else.
    pub complete: usize,
    /// Annotated tones not heard, and tones heard that the chord does not have.
    pub missing: usize,
    pub extra: usize,
}

impl ChordTally {
    pub fn add(&mut self, other: &ChordTally) {
        self.chords += other.chords;
        self.tones += other.tones;
        self.complete += other.complete;
        self.missing += other.missing;
        self.extra += other.extra;
    }

    /// Share of the chords played complete.
    pub fn complete_share(&self) -> f64 {
        match self.chords {
            0 => 0.0,
            c => self.complete as f64 / c as f64,
        }
    }
}

/// Scores each annotated chord against the detections within `window_s` of its onset (each
/// detection counted once, rounded pitch).
///
/// ```text
/// annotated C3 E3 G3   heard C3 G3 A3 ─► missing 1 (E3), extra 1 (A3), not complete
/// ```
pub fn score_chords(notes: &[Annotation], detected: &[DetectedNote], window_s: f64) -> ChordTally {
    let mut used = vec![false; detected.len()];
    let mut tally = ChordTally::default();
    let mut start = 0;
    while start < notes.len() {
        let onset_s = notes[start].onset_s;
        let len = notes[start..]
            .iter()
            .take_while(|n| n.onset_s - onset_s <= SAME_ONSET_S)
            .count();
        let chord = &notes[start..start + len];
        start += len;
        if chord.len() < 2 {
            continue;
        }
        let mut heard: Vec<i64> = Vec::new();
        for (d, u) in detected.iter().zip(used.iter_mut()) {
            if *u || (d.onset_s - onset_s).abs() > window_s || d.is_pitchless() {
                continue;
            }
            *u = true;
            heard.push(d.midi.round() as i64);
        }
        let wanted: Vec<i64> = chord.iter().map(|n| n.midi.round() as i64).collect();
        let missing = wanted.iter().filter(|w| !heard.contains(w)).count();
        let extra = heard.iter().filter(|h| !wanted.contains(h)).count();
        tally.chords += 1;
        tally.tones += chord.len();
        tally.complete += usize::from(missing == 0 && extra == 0);
        tally.missing += missing;
        tally.extra += extra;
    }
    tally
}

/// The notes a MIDI file's keys give, as the MIDI input tells them (key down, exact pitch).
pub fn analyze_midi_file(path: &Path) -> Result<Vec<DetectedNote>, EngineError> {
    let notes = read_midi_notes(path)?;
    Ok(midi_detections(&notes))
}

fn midi_detections(notes: &[(f64, MidiNote)]) -> Vec<DetectedNote> {
    notes
        .iter()
        .filter_map(|(at_s, n)| midi_note(*at_s, *n))
        .collect()
}

/// The generated MIDI chord set: every shape on every root, rolled; the annotation (one onset
/// per chord) and the notes the MIDI input tells.
pub fn generate_midi_chords() -> (Vec<Annotation>, Vec<DetectedNote>) {
    let mut noise = Noise::new(41);
    let mut annotation = Vec::new();
    let mut keys: Vec<(f64, MidiNote)> = Vec::new();
    let mut t = LEAD_S;
    for root in ROOTS {
        for shape in SHAPES {
            for interval in shape {
                let key = root + interval;
                annotation.push(Annotation {
                    onset_s: t,
                    midi: f64::from(key),
                    technique: None,
                    slide_cents: None,
                });
                let down_s = t + ROLL_S * f64::from(noise.sample().abs());
                let velocity = VELOCITY_LOW + VELOCITY_SPAN * noise.sample().abs();
                keys.push((
                    down_s,
                    MidiNote {
                        key,
                        velocity: velocity as u8,
                    },
                ));
                keys.push((t + CHORD_S + OVERLAP_S, MidiNote { key, velocity: 0 }));
            }
            t += CHORD_S;
        }
    }
    keys.sort_by(|a, b| a.0.total_cmp(&b.0));
    (annotation, midi_detections(&keys))
}

#[cfg(test)]
mod tests {
    use super::*;
    use kontra_model::InputSource;

    fn tone(onset_s: f64, midi: f64) -> Annotation {
        Annotation {
            onset_s,
            midi,
            technique: None,
            slide_cents: None,
        }
    }

    fn heard(onset_s: f64, midi: f64) -> DetectedNote {
        DetectedNote {
            onset_s,
            midi,
            confidence: 1.0,
            level_db: -10.0,
            source: InputSource::Midi,
            technique: None,
            over_background: false,
        }
    }

    #[test]
    fn a_chord_counts_as_a_set() {
        let c = [tone(1.0, 48.0), tone(1.0, 52.0), tone(1.0, 55.0)];
        let all = [heard(1.01, 55.0), heard(1.0, 48.0), heard(1.02, 52.0)];
        assert_eq!(score_chords(&c, &all, 0.05).complete, 1);

        // E3 missing, A3 extra; a tone outside the window is not the chord's.
        let wrong = [
            heard(1.0, 48.0),
            heard(1.01, 55.0),
            heard(1.02, 57.0),
            heard(1.2, 52.0),
        ];
        let t = score_chords(&c, &wrong, 0.05);
        assert_eq!((t.chords, t.complete, t.missing, t.extra), (1, 0, 1, 1));
    }

    #[test]
    fn single_notes_are_no_chords() {
        let line = [tone(1.0, 40.0), tone(1.5, 43.0)];
        assert_eq!(score_chords(&line, &[], 0.05).chords, 0);
    }

    #[test]
    fn the_generated_set_is_complete_through_midi() {
        let (annotation, detected) = generate_midi_chords();
        let t = score_chords(&annotation, &detected, ROLL_S + 0.005);
        assert_eq!(t.chords, ROOTS.count() * SHAPES.len());
        assert_eq!(t.complete, t.chords, "{t:?}");
    }
}
