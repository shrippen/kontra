//! Natural harmonics (LH-BIB-01): a touched string sounds at one of its partials, not at the
//! fretted pitch. Guitar Pro stores string and fret, so the sounding pitch is worked out here.
//!
//! ```text
//! E string, fret 12 → E2 (+12)   fret 7 → B2 (+19)   fret 5 → E3 (+24)   fret 4 → G♯3 (+28)
//! ```

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
}
