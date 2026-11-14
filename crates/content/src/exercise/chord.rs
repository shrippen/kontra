//! Chord symbols of a piece plan ("G", "F#m7", "Bb", "C/E"): root, quality and an optional
//! bass note, and what the bass plays from them.
//!
//! ```text
//! "Am7/G" ─▶ root A (9), Min7 [0 3 7 10], bass G (7), name "Am7/G"
//! ```

use super::{Quality, Voicing};
use kontra_model::read_note;

/// What a chord is made of, in semitones above its root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Major,
    Minor,
    Power,
    Sixth,
    MinorSixth,
    Dominant7,
    Major7,
    Minor7,
    HalfDiminished,
    Diminished,
    Diminished7,
    Sus2,
    Sus4,
    Dominant7Sus4,
    Add9,
    Augmented,
}

/// Seventh the bass plays over a chord without one (funk grooves): the flat seventh.
const FLAT_SEVENTH: i32 = 10;

/// (suffix as written, quality) for every accepted spelling; the name keeps `Kind::suffix`.
const SUFFIXES: [(&str, Kind); 22] = [
    ("", Kind::Major),
    ("maj", Kind::Major),
    ("m", Kind::Minor),
    ("min", Kind::Minor),
    ("5", Kind::Power),
    ("6", Kind::Sixth),
    ("m6", Kind::MinorSixth),
    ("7", Kind::Dominant7),
    ("maj7", Kind::Major7),
    ("m7", Kind::Minor7),
    ("min7", Kind::Minor7),
    ("m7b5", Kind::HalfDiminished),
    ("ø", Kind::HalfDiminished),
    ("ø7", Kind::HalfDiminished),
    ("dim", Kind::Diminished),
    ("dim7", Kind::Diminished7),
    ("sus2", Kind::Sus2),
    ("sus4", Kind::Sus4),
    ("sus", Kind::Sus4),
    ("7sus4", Kind::Dominant7Sus4),
    ("add9", Kind::Add9),
    ("aug", Kind::Augmented),
];

impl Kind {
    /// The suffix a name is written with.
    fn suffix(self) -> &'static str {
        match self {
            Kind::Major => "",
            Kind::Minor => "m",
            Kind::Power => "5",
            Kind::Sixth => "6",
            Kind::MinorSixth => "m6",
            Kind::Dominant7 => "7",
            Kind::Major7 => "maj7",
            Kind::Minor7 => "m7",
            Kind::HalfDiminished => "m7b5",
            Kind::Diminished => "dim",
            Kind::Diminished7 => "dim7",
            Kind::Sus2 => "sus2",
            Kind::Sus4 => "sus4",
            Kind::Dominant7Sus4 => "7sus4",
            Kind::Add9 => "add9",
            Kind::Augmented => "aug",
        }
    }

    /// (third, fifth) of the triad; a power chord has no third.
    fn triad(self) -> (Option<i32>, i32) {
        match self {
            Kind::Major | Kind::Sixth | Kind::Dominant7 | Kind::Major7 | Kind::Add9 => (Some(4), 7),
            Kind::Minor | Kind::MinorSixth | Kind::Minor7 => (Some(3), 7),
            Kind::Power => (None, 7),
            Kind::HalfDiminished | Kind::Diminished | Kind::Diminished7 => (Some(3), 6),
            Kind::Sus2 => (Some(2), 7),
            Kind::Sus4 | Kind::Dominant7Sus4 => (Some(5), 7),
            Kind::Augmented => (Some(4), 8),
        }
    }

    /// The fourth tone the name asks for: a seventh, or the sixth of a 6 chord.
    fn color(self) -> Option<i32> {
        match self {
            Kind::Sixth | Kind::MinorSixth | Kind::Diminished7 => Some(9),
            Kind::Dominant7 | Kind::Minor7 | Kind::HalfDiminished | Kind::Dominant7Sus4 => Some(10),
            Kind::Major7 => Some(11),
            _ => None,
        }
    }
}

/// One chord of a bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Chord {
    /// Pitch class of the root, 0 = C.
    pub(super) root: u8,
    kind: Kind,
    /// Pitch class of the bass note of a slash chord ("C/E" → 4).
    pub(super) bass: Option<u8>,
    /// The symbol as it goes into the file (suffix normalised, spelling kept).
    pub(super) name: String,
}

/// Pitch class and written form of the note name at the start of `text` ("Bb7" → 10, "Bb"),
/// and what follows it.
pub(super) fn note(text: &str) -> Option<(u8, String, &str)> {
    let read = read_note(text)?;
    let mark = match read.shift {
        1 => "#",
        -1 => "b",
        _ => "",
    };
    Some((read.class, format!("{}{mark}", read.letter), read.rest))
}

impl Chord {
    /// Parses a symbol; `None` for anything outside the supported set.
    pub(super) fn parse(symbol: &str) -> Option<Chord> {
        let (root, written, rest) = note(symbol.trim())?;
        let (suffix, bass) = match rest.split_once('/') {
            Some((suffix, bass)) => (suffix, Some(bass)),
            None => (rest, None),
        };
        let kind = SUFFIXES.iter().find(|(s, _)| *s == suffix)?.1;
        let mut name = format!("{written}{}", kind.suffix());
        let bass = match bass {
            None => None,
            Some(text) => {
                let (pc, written, tail) = note(text)?;
                if !tail.is_empty() {
                    return None;
                }
                name = format!("{name}/{written}");
                Some(pc)
            }
        };
        Some(Chord {
            root,
            kind,
            bass,
            name,
        })
    }

    /// The plain triad on `root`, written `spelled` ("Bb", "F#") with "m" when minor.
    pub(super) fn triad(root: u8, minor: bool, spelled: &str) -> Chord {
        let (kind, suffix) = if minor {
            (Kind::Minor, "m")
        } else {
            (Kind::Major, "")
        };
        Chord {
            root,
            kind,
            bass: None,
            name: format!("{spelled}{suffix}"),
        }
    }

    /// A built-in piece's chord: `quality` named as `voicing` reads it (`Quality::suffix_for`),
    /// built directly from the style's own data, never from a written symbol, so it cannot
    /// fail to parse.
    pub(super) fn built(root: u8, spelled: &str, quality: Quality, voicing: Voicing) -> Chord {
        // The "as written" voicing drops the seventh from every chord but keeps minor and
        // half-diminished as such (`Quality::suffix_for`): only `Voicing::Seventh` keeps it.
        let kind = match (quality, voicing) {
            (Quality::Minor7, Voicing::Seventh) => Kind::Minor7,
            (Quality::Minor7, _) => Kind::Minor,
            (Quality::HalfDiminished, Voicing::Seventh) => Kind::HalfDiminished,
            (Quality::HalfDiminished, _) => Kind::Diminished,
            (Quality::Major7, Voicing::Seventh) => Kind::Major7,
            (Quality::Dominant7, Voicing::Seventh) => Kind::Dominant7,
            (Quality::Major7 | Quality::Dominant7, Voicing::Power) => Kind::Power,
            (Quality::Major7 | Quality::Dominant7, Voicing::Triad) => Kind::Major,
        };
        Chord {
            root,
            kind,
            bass: None,
            name: format!("{spelled}{}", kind.suffix()),
        }
    }

    /// Root, third, fifth and seventh in semitones; a chord without a seventh gets the flat
    /// seventh (funk grooves play it over any chord).
    pub(super) fn tones(&self) -> [i32; 4] {
        let (third, fifth) = self.kind.triad();
        [
            0,
            third.unwrap_or(fifth),
            fifth,
            self.kind.color().unwrap_or(FLAT_SEVENTH),
        ]
    }

    /// Whether the name asks for a seventh or sixth.
    pub(super) fn has_color(&self) -> bool {
        self.kind.color().is_some()
    }

    /// Pitch classes (relative to the root) the chord is made of.
    pub(super) fn pitch_classes(&self) -> Vec<i32> {
        let tones = self.tones();
        let mut classes = vec![0, tones[1], tones[2]];
        if self.has_color() {
            classes.push(tones[3]);
        }
        classes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_reads_the_start_of_a_text() {
        let table = [
            ("Bb7", Some((10, "Bb", "7"))),
            ("F♯m", Some((6, "F#", "m"))),
            ("A♭", Some((8, "Ab", ""))),
            ("C", Some((0, "C", ""))),
            ("c", None),
            ("H", None),
            ("x", None),
        ];
        for (text, want) in table {
            let got = note(text).map(|(pc, name, rest)| (pc, name, rest.to_string()));
            let want = want.map(|(pc, name, rest)| (pc, name.to_string(), rest.to_string()));
            assert_eq!(got, want, "{text}");
        }
    }

    /// (symbol, root, name kept, tones).
    #[test]
    fn symbols_parse_to_their_tones() {
        let table = [
            ("C", 0, "C", [0, 4, 7, 10]),
            ("F#m", 6, "F#m", [0, 3, 7, 10]),
            ("Bb", 10, "Bb", [0, 4, 7, 10]),
            ("Eb5", 3, "Eb5", [0, 7, 7, 10]),
            ("G6", 7, "G6", [0, 4, 7, 9]),
            ("Am6", 9, "Am6", [0, 3, 7, 9]),
            ("D7", 2, "D7", [0, 4, 7, 10]),
            ("Fmaj7", 5, "Fmaj7", [0, 4, 7, 11]),
            ("Em7", 4, "Em7", [0, 3, 7, 10]),
            ("Bm7b5", 11, "Bm7b5", [0, 3, 6, 10]),
            ("Bø", 11, "Bm7b5", [0, 3, 6, 10]),
            ("Bø7", 11, "Bm7b5", [0, 3, 6, 10]),
            ("C#dim", 1, "C#dim", [0, 3, 6, 10]),
            ("C#dim7", 1, "C#dim7", [0, 3, 6, 9]),
            ("Dsus2", 2, "Dsus2", [0, 2, 7, 10]),
            ("Asus4", 9, "Asus4", [0, 5, 7, 10]),
            ("E7sus4", 4, "E7sus4", [0, 5, 7, 10]),
            ("Gadd9", 7, "Gadd9", [0, 4, 7, 10]),
            ("Caug", 0, "Caug", [0, 4, 8, 10]),
            ("A♭maj7", 8, "Abmaj7", [0, 4, 7, 11]),
            ("Cmin", 0, "Cm", [0, 3, 7, 10]),
        ];
        for (symbol, root, name, tones) in table {
            let chord = Chord::parse(symbol).unwrap_or_else(|| panic!("{symbol}"));
            assert_eq!(chord.root, root, "{symbol}");
            assert_eq!(chord.name, name, "{symbol}");
            assert_eq!(chord.tones(), tones, "{symbol}");
            assert_eq!(chord.bass, None, "{symbol}");
        }
    }

    #[test]
    fn slash_chords_keep_their_bass() {
        let chord = Chord::parse("C/E").expect("C/E");
        assert_eq!(
            (chord.root, chord.bass, chord.name.as_str()),
            (0, Some(4), "C/E")
        );
        let chord = Chord::parse("Am7/G").expect("Am7/G");
        assert_eq!((chord.root, chord.bass), (9, Some(7)));
        let chord = Chord::parse("D/F#").expect("D/F#");
        assert_eq!((chord.bass, chord.name.as_str()), (Some(6), "D/F#"));
    }

    #[test]
    fn unknown_symbols_do_not_parse() {
        for symbol in [
            "", "H", "Hm", "c", "Cx", "C/", "C/H", "C/E7", "Cm8", "C9", "N.C.",
        ] {
            assert!(Chord::parse(symbol).is_none(), "{symbol:?}");
        }
    }
}
