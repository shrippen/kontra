//! Transposing a piece by semitones (LH-WIE-02), so the notes match a transposed backing track.
//!
//! ```text
//! note  ─► pitch + n ─┬─ in reach: same string, fret + n (fret 0..=frets), else new tab
//!                     └─ out of reach: an octave towards the instrument
//! key   ─► fifths + 7n, kept within −6..=6 (G major +1 → A major +3 at n = 2)
//! ```

use crate::tab::assign_tab;
use kontra_model::{Arrangement, Pitch};

/// Largest transposition offered, in semitones up or down.
pub const TRANSPOSE_MAX: i32 = 6;
const OCTAVE: i32 = 12;
/// Key signatures are written with at most six sharps or flats.
const FIFTHS_MAX: i32 = 6;

/// Moves every note by `semitones` (clamped to ±`TRANSPOSE_MAX`) and the key signatures with
/// it. Fingering from the file is kept where the moved fret exists on the same string.
pub fn transpose(arrangement: &mut Arrangement, semitones: i32) {
    let n = semitones.clamp(-TRANSPOSE_MAX, TRANSPOSE_MAX);
    if n == 0 {
        return;
    }
    let range = arrangement.instrument.range();
    let frets = i32::from(arrangement.instrument.frets);
    let mut lost_tab = false;

    for note in &mut arrangement.notes {
        let mut pitch = i32::from(note.pitch.0) + n;
        // Out of reach: an octave back towards the instrument (E1 − 2 on a four-string → D2).
        if let Some((low, high)) = range {
            if pitch < i32::from(low.0) {
                pitch += OCTAVE;
            } else if pitch > i32::from(high.0) {
                pitch -= OCTAVE;
            }
        }
        let moved = i32::from(note.pitch.0) + n == pitch;
        note.pitch = Pitch(pitch.clamp(0, 127) as u8);

        let fret = note.fret.map(|f| i32::from(f) + n);
        match fret {
            Some(f) if moved && (0..=frets).contains(&f) => note.fret = Some(f as u8),
            _ => {
                note.string = None;
                note.fret = None;
                lost_tab = true;
            }
        }
    }

    for bar in &mut arrangement.bars {
        let fifths = (i32::from(bar.key_fifths) + 7 * n).rem_euclid(OCTAVE);
        let fifths = if fifths > FIFTHS_MAX {
            fifths - OCTAVE
        } else {
            fifths
        };
        bar.key_fifths = fifths as i8;
    }

    if lost_tab {
        assign_tab(arrangement);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kontra_model::{Bar, Instrument, Note, TimeSignature};

    fn piece(notes: &[(u8, u8, u8)]) -> Arrangement {
        let mut a = Arrangement::new("t", Instrument::default_bass());
        for (i, &(midi, string, fret)) in notes.iter().enumerate() {
            let mut note = Note::new(i as u64 * 480, 480, Pitch(midi));
            note.string = Some(string);
            note.fret = Some(fret);
            a.notes.push(note);
        }
        a
    }

    #[test]
    fn notes_and_frets_move_together() {
        // A1 on the E string, fret 5 → B1, fret 7.
        let mut a = piece(&[(33, 0, 5)]);
        transpose(&mut a, 2);
        assert_eq!(a.notes[0].pitch, Pitch(35));
        assert_eq!((a.notes[0].string, a.notes[0].fret), (Some(0), Some(7)));
    }

    #[test]
    fn an_open_string_below_zero_gets_a_new_place() {
        // Open A (A1) down one semitone: G#1, fret 4 on the E string.
        let mut a = piece(&[(33, 1, 0)]);
        transpose(&mut a, -1);
        assert_eq!(a.notes[0].pitch, Pitch(32));
        assert_eq!((a.notes[0].string, a.notes[0].fret), (Some(0), Some(4)));
    }

    #[test]
    fn notes_out_of_reach_move_an_octave() {
        // Open E1 down two semitones does not exist on a four-string: D2 instead.
        let mut a = piece(&[(28, 0, 0)]);
        transpose(&mut a, -2);
        assert_eq!(a.notes[0].pitch, Pitch(38));
        assert!(a.notes[0].fret.is_some());
    }

    #[test]
    fn key_signatures_follow() {
        let mut a = piece(&[(33, 0, 5)]);
        let bar = |fifths| Bar {
            start_ticks: 0,
            signature: TimeSignature::COMMON,
            source_index: 0,
            key_fifths: fifths,
            repeat_open: false,
            repeat_close: false,
            endings: 0,
        };
        a.bars = vec![bar(1), bar(-1), bar(0)];
        transpose(&mut a, 2);
        // G → A (+3), F → G (+1), C → D (+2).
        let keys: Vec<i8> = a.bars.iter().map(|b| b.key_fifths).collect();
        assert_eq!(keys, [3, 1, 2]);
        transpose(&mut a, -6);
        // A → E♭ (−3): six semitones down, written with flats.
        assert_eq!(a.bars[0].key_fifths, -3);
    }

    #[test]
    fn zero_changes_nothing_and_large_values_are_clamped() {
        let mut a = piece(&[(40, 1, 7)]);
        let before = a.clone();
        transpose(&mut a, 0);
        assert_eq!(a, before);
        transpose(&mut a, 20);
        assert_eq!(a.notes[0].pitch, Pitch(46));
    }
}
