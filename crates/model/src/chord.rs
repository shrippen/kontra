//! Chord symbols and keys as pitch classes (LH-BEW-13): which tones belong to the chord of a bar
//! and which to the key, so a free line can be judged by how it fits.
//!
//! ```text
//! "Am7"  →  root A (9), intervals 0 3 7 10  →  pitch classes {A, C, E, G}
//! "F#m7b5/E"  →  {F#, A, C, E} plus the bass E
//! ```

use crate::read_note;

const PITCH_CLASSES: u8 = 12;
/// Semitones from the tonic of the major scale; the key's minor uses the same tones.
const MAJOR_SCALE: [u8; 7] = [0, 2, 4, 5, 7, 9, 11];
/// Semitones one fifth up the circle.
const FIFTH: i32 = 7;

/// Chord suffixes and their intervals above the root; the longest match wins ("maj7" over "m").
/// Covers every suffix the MusicXML import writes (`chord_suffix` in kontra-content).
const QUALITIES: [(&str, &[u8]); 33] = [
    ("m(maj7)", &[0, 3, 7, 11]),
    ("maj11", &[0, 4, 7, 11, 2, 5]),
    ("maj13", &[0, 4, 7, 11, 2, 9]),
    ("aug7", &[0, 4, 8, 10]),
    ("min7", &[0, 3, 7, 10]),
    ("m11", &[0, 3, 7, 10, 2, 5]),
    ("m13", &[0, 3, 7, 10, 2, 9]),
    ("11", &[0, 4, 7, 10, 2, 5]),
    ("13", &[0, 4, 7, 10, 2, 9]),
    ("m7b5", &[0, 3, 6, 10]),
    ("maj7", &[0, 4, 7, 11]),
    ("maj9", &[0, 4, 7, 11, 2]),
    ("dim7", &[0, 3, 6, 9]),
    ("sus2", &[0, 2, 7]),
    ("sus4", &[0, 5, 7]),
    ("sus", &[0, 5, 7]),
    ("min", &[0, 3, 7]),
    ("dim", &[0, 3, 6]),
    ("aug", &[0, 4, 8]),
    ("add9", &[0, 4, 7, 2]),
    ("m7", &[0, 3, 7, 10]),
    ("m9", &[0, 3, 7, 10, 2]),
    ("m6", &[0, 3, 7, 9]),
    ("M7", &[0, 4, 7, 11]),
    ("ø", &[0, 3, 6, 10]),
    ("°", &[0, 3, 6]),
    ("+", &[0, 4, 8]),
    ("m", &[0, 3, 7]),
    ("-", &[0, 3, 7]),
    ("7", &[0, 4, 7, 10]),
    ("9", &[0, 4, 7, 10, 2]),
    ("6", &[0, 4, 7, 9]),
    ("5", &[0, 7]),
];
/// No suffix: the major triad.
const MAJOR: &[u8] = &[0, 4, 7];

/// A chord as a set of pitch classes (bit `pc` set, C = 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chord {
    root: u8,
    tones: u16,
}

impl Chord {
    /// The chord a symbol names ("C", "Bbm7", "F#ø", "E5", "C/G"); `None` if it is none.
    /// Unknown extensions after a known quality are ignored ("C7#9" is C7).
    pub fn parse(symbol: &str) -> Option<Chord> {
        let (body, bass) = match symbol.trim().split_once('/') {
            Some((body, bass)) => (body, Some(bass)),
            None => (symbol.trim(), None),
        };
        let (root, rest) = root_of(body)?;
        let intervals = QUALITIES
            .iter()
            .filter(|(suffix, _)| rest.starts_with(suffix))
            .max_by_key(|(suffix, _)| suffix.len())
            .map_or(MAJOR, |(_, intervals)| intervals);
        // Anything else after the root is not a chord name ("Chorus", "Bridge").
        if intervals == MAJOR && !rest.is_empty() && !rest.starts_with(['(', '#', 'b']) {
            return None;
        }
        let mut tones = intervals
            .iter()
            .fold(0u16, |set, i| set | 1 << ((root + i) % PITCH_CLASSES));
        if let Some((bass, _)) = bass.and_then(root_of) {
            tones |= 1 << bass;
        }
        Some(Chord { root, tones })
    }

    /// Pitch class of the root, C = 0.
    pub fn root(self) -> u8 {
        self.root
    }

    /// True if `midi` is a tone of the chord, in any octave.
    pub fn has(self, midi: u8) -> bool {
        self.tones & 1 << (midi % PITCH_CLASSES) != 0
    }
}

/// True if `midi` belongs to the key with `key_fifths` (1..7 sharps, -1..-7 flats): the major
/// scale on its tonic, or the same tones of its relative minor.
pub fn in_key(key_fifths: i8, midi: u8) -> bool {
    let tonic = (i32::from(key_fifths) * FIFTH).rem_euclid(i32::from(PITCH_CLASSES));
    let step = (i32::from(midi) - tonic).rem_euclid(i32::from(PITCH_CLASSES));
    MAJOR_SCALE.iter().any(|s| i32::from(*s) == step)
}

/// Pitch class of a note name at the start of `text` and what follows it.
fn root_of(text: &str) -> Option<(u8, &str)> {
    // Lower case is no root ("am" could be a word); "H" is German for B and left out.
    let note = read_note(text)?;
    Some((note.class, note.rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    const A1: u8 = 33;
    const C2: u8 = 36;
    const E2: u8 = 40;
    const G2: u8 = 43;

    #[test]
    fn roots_take_upper_case_letters_only() {
        let table = [
            ("C", Some(0)),
            ("F#m", Some(6)),
            ("F♯m", Some(6)),
            ("Bb", Some(10)),
            ("B♭", Some(10)),
            ("c", None),
            ("am", None),
            ("H", None),
        ];
        for (symbol, root) in table {
            assert_eq!(Chord::parse(symbol).map(Chord::root), root, "{symbol}");
        }
    }

    #[test]
    fn chord_symbols_give_their_tones() {
        let am7 = Chord::parse("Am7").unwrap();
        assert_eq!(am7.root(), 9);
        assert!([A1, C2, E2, G2].iter().all(|m| am7.has(*m)));
        assert!(!am7.has(A1 + 2));
        let c = Chord::parse("C").unwrap();
        assert!(c.has(E2) && !c.has(A1));
        assert!(Chord::parse("Bbmaj7").unwrap().has(A1));
        assert!(Chord::parse("C/A").unwrap().has(A1));
        assert!(
            Chord::parse("E5").unwrap().has(E2 + 7) && !Chord::parse("E5").unwrap().has(G2 + 1)
        );
        assert!(Chord::parse("F#ø").unwrap().has(C2));
    }

    #[test]
    fn section_names_are_no_chords() {
        assert_eq!(Chord::parse("Chorus"), None);
        assert_eq!(Chord::parse("Bridge"), None);
        assert_eq!(Chord::parse("intro"), None);
        assert_eq!(Chord::parse(""), None);
    }

    #[test]
    fn keys_hold_their_major_scale() {
        // C major: no F#; G major (1 sharp): F#, no F.
        assert!(in_key(0, C2) && !in_key(0, 42));
        assert!(in_key(1, 42) && !in_key(1, 41));
        // F major (1 flat): Bb.
        assert!(in_key(-1, 34) && !in_key(-1, 35));
    }
}
