//! Reading note names like "F#" or "B♭" from text.

const SEMITONES: i32 = 12;

/// A note name read from the start of a text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadNote<'a> {
    /// Pitch class, C = 0.
    pub class: u8,
    /// The letter, upper case A to G.
    pub letter: char,
    /// +1 for a sharp, -1 for a flat, 0 for none.
    pub shift: i8,
    /// What follows the name ("m7" in "F#m7").
    pub rest: &'a str,
}

/// Reads a note name at the start of `text`: an upper case letter A to G, then `#` or `♯`
/// (sharp) or `b` or `♭` (flat). Lower case and German "H" are no names here; callers that
/// want them normalise first.
pub fn read_note(text: &str) -> Option<ReadNote<'_>> {
    let mut chars = text.chars();
    let letter = chars.next()?;
    let natural: i32 = match letter {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    };
    let rest = chars.as_str();
    let (shift, rest) = if let Some(r) = rest.strip_prefix(['#', '♯']) {
        (1, r)
    } else if let Some(r) = rest.strip_prefix(['b', '♭']) {
        (-1, r)
    } else {
        (0, rest)
    };
    let class = u8::try_from((natural + i32::from(shift)).rem_euclid(SEMITONES)).ok()?;
    Some(ReadNote {
        class,
        letter,
        shift,
        rest,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_letter_accidental_and_rest() {
        let note = read_note("Bb7").unwrap();
        assert_eq!(
            (note.class, note.letter, note.shift, note.rest),
            (10, 'B', -1, "7")
        );

        let note = read_note("F♯m").unwrap();
        assert_eq!(
            (note.class, note.letter, note.shift, note.rest),
            (6, 'F', 1, "m")
        );

        assert_eq!(read_note("Cb").unwrap().class, 11);
        assert_eq!(read_note("B#").unwrap().class, 0);
    }

    #[test]
    fn rejects_lower_case_german_h_and_junk() {
        for text in ["", "c", "H", "x", "#"] {
            assert_eq!(read_note(text), None, "{text}");
        }
    }
}
