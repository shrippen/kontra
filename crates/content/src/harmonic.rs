//! Harmonics (LH-BIB-01): a touched string sounds at one of its partials, not at the fretted
//! pitch. Files store string and fret, so the sounding pitch is worked out here.
//!
//! ```text
//! natural:    E string, fret 12 → E2 (+12)   fret 7 → B2 (+19)   fret 5 → E3 (+24)
//! artificial: fretted A1, touched 12 frets higher → A2 (+12), 5 frets higher → A3 (+24)
//! ```
//!
//! An artificial (or tapped) harmonic is a natural one of the string shortened at the fret:
//! the same table, counted from the fretted note.

use kontra_model::Pitch;

/// (fret, semitones above the open string): the 2nd to 8th partial at the frets a player
/// touches (both nodes where they lie on the neck).
const NATURAL: [(f64, i32); 11] = [
    (12.0, 12),
    (24.0, 24),
    (7.0, 19),
    (19.0, 19),
    (5.0, 24),
    (4.0, 28),
    (9.0, 28),
    (16.0, 28),
    (3.2, 31),
    (2.7, 34),
    (2.4, 36),
];
const OCTAVE: i32 = 12;
/// How far (in frets) a stored harmonic fret may lie from a node, e.g. "3" for 3.2.
const FRET_SLACK: f64 = 0.25;

/// The pitch a natural harmonic at `fret` of the string tuned to `open` sounds; `None` where
/// no partial lies.
pub(crate) fn natural_harmonic(open: Pitch, fret: f64) -> Option<Pitch> {
    let (_, semitones) = NATURAL
        .iter()
        .find(|(node, _)| (node - fret).abs() <= FRET_SLACK)?;
    open.transposed(*semitones)
}

/// The pitch an artificial harmonic sounds when the file stores its pitch class and octave
/// (Guitar Pro 5): the partial above `fretted` with that class, `octaves` octaves up, e.g. E
/// over A1 one octave up → E3 (+19). `None` where no partial has that class.
pub(crate) fn harmonic_of_class(fretted: Pitch, class: i32, octaves: i32) -> Option<Pitch> {
    let above = (class - i32::from(fretted.0)).rem_euclid(OCTAVE);
    let semitones = octaves * OCTAVE + above;
    if !NATURAL.iter().any(|(_, s)| *s == semitones) {
        return None;
    }
    fretted.transposed(semitones)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sounds_at_the_partial() {
        let e = Pitch(28);
        assert_eq!(natural_harmonic(e, 12.0), Some(Pitch(40)));
        assert_eq!(natural_harmonic(e, 7.0), Some(Pitch(47)));
        assert_eq!(natural_harmonic(e, 5.0), Some(Pitch(52)));
        assert_eq!(natural_harmonic(e, 3.0), Some(Pitch(59)));
        assert_eq!(natural_harmonic(e, 6.0), None);
    }

    #[test]
    fn class_and_octave_pick_the_partial() {
        let a = Pitch(33);
        assert_eq!(harmonic_of_class(a, 9, 1), Some(Pitch(45)));
        assert_eq!(harmonic_of_class(a, 4, 1), Some(Pitch(52)));
        assert_eq!(harmonic_of_class(a, 9, 2), Some(Pitch(57)));
        assert_eq!(harmonic_of_class(a, 1, 2), Some(Pitch(61)));
        assert_eq!(harmonic_of_class(a, 0, 0), None);
    }
}
