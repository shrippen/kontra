//! Generated exercises (LH-LRN-04, LH-LRN-01, LH-BIB-08): scales, arpeggios, walking bass,
//! grooves, finger exercises and odd meters in any key and tempo, written as standard MIDI files
//! so they go through the normal import (tab assignment, library, progress) like any other piece.
//!
//! Chord names travel as MIDI marker events at the bar they belong to; the importer turns them
//! into bar markers. Techniques (ghost, dead, slap, pop) travel as text events at the note.
//!
//! ```text
//! ExerciseSpec ──generate──▶ one pass (tones, chord markers) ──repeat──▶ passes
//!                                                          └──▶ last bar on the tonic
//!                                                          └──▶ drums (grooves) ──write──▶ MIDI
//! ```

mod plan;
mod song;

pub use plan::{bundled, bundled_specs, plan_place};
pub use song::{Level, Style};

use kontra_model::{Judging, Technique};
use midly::num::{u15, u24, u28, u4, u7};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};

/// Resolution of the written files.
const PPQ: u32 = 480;
const QUARTER: u32 = PPQ;
const EIGHTH: u32 = PPQ / 2;
const SIXTEENTH: u32 = PPQ / 4;
const TRIPLET: u32 = PPQ / 3;
const BAR: u32 = 4 * PPQ;
/// Lowest note of a four-string bass (E1) and the highest a beginner reaches comfortably (G3).
const LOWEST: i32 = 28;
const HIGHEST: i32 = 55;
/// Open strings E1 A1 D2 G2 lie a fourth apart.
const STRINGS: u8 = 4;
const STRING_STEP: i32 = 5;
/// Finger exercises start between fret 1 and 9: four frets on the G string stay below G3.
const MIN_FRET: u8 = 1;
const MAX_FRET: u8 = 9;
const OCTAVE: i32 = 12;
/// General MIDI "Electric Bass (finger)".
const BASS_PROGRAM: u8 = 33;
const VELOCITY: u8 = 96;
/// Ghost notes are muted, so they sound quieter than the line around them.
const GHOST_VELOCITY: u8 = 40;
/// Slaps and pops are struck harder than a pluck.
const SLAP_VELOCITY: u8 = 112;
/// General MIDI drum channel (10) and keys of the accompaniment (LH-LRN-01).
const DRUM_CHANNEL: u8 = 9;
const KICK: u8 = 36;
const SNARE: u8 = 38;
const HIHAT: u8 = 42;
const DRUM_VELOCITY: u8 = 90;

/// What to practise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExerciseKind {
    /// A scale up and down; `steps` are quarters (level 1) or eighths.
    Scale(ScaleKind),
    /// Chord tones per bar in quarters over a progression.
    Arpeggio(Progression),
    /// A walking line over a progression: root, chord tones, chromatic approach.
    WalkingBass(Progression),
    /// Roots of a progression in a fixed rhythm (groove before notes, LH-LRN-01).
    Groove(Progression, Rhythm),
    /// A scale in a pattern (thirds, groups of four) up and down: reading and positions.
    Sequence(ScaleKind, Sequence),
    /// Triads per bar: root, third, fifth, octave in quarters.
    Triads(Progression),
    /// Four fingers on four frets across the strings and back; `key` is the start fret.
    Chromatic(Fingers),
    /// A piece in a form (intro, verse, chorus, bridge, outro) to play through.
    Song(Style, Level),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaleKind {
    Major,
    Minor,
    MajorPentatonic,
    MinorPentatonic,
    Blues,
    Dorian,
    Phrygian,
    Lydian,
    Mixolydian,
    Locrian,
    HarmonicMinor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Progression {
    /// One chord (the key's tonic), four bars.
    Tonic,
    /// ii – V – I – I.
    TwoFiveOne,
    /// Twelve-bar blues with dominant sevenths.
    Blues,
    /// i – iv – i – v in minor.
    Minor,
    /// I – IV – V – I.
    OneFourFive,
    /// I – V – vi – IV, the most common pop progression.
    Pop,
    /// I – vi – ii – V, the jazz turnaround.
    Turnaround,
    /// iiø – V7 – i – i in minor.
    MinorTwoFiveOne,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rhythm {
    Quarters,
    Eighths,
    /// Eighth plus two sixteenths per beat.
    Sixteenths,
    /// Beats one and three, the "and" of two: space as part of the groove.
    WithRests,
    /// Swung eighths (2:1).
    Shuffle,
    /// Root and octave alternating in eighths.
    Octaves,
    /// Sixteenths on the root, the "e" and "a" of each beat as ghost notes (funk).
    Ghost,
    /// Root and fifth in half notes (two-feel).
    RootFifth,
    /// Root, fifth, octave, fifth in quarters.
    RootFifthOctave,
    /// Root on the beat, a dead note (x) on the "and".
    DeadNotes,
    /// Only the "and" of each beat: the beat itself is a rest.
    Offbeat,
    /// 3 + 3 + 2 eighths (tresillo), the basic syncopation of pop and latin.
    Tresillo,
    /// Eighth-note triplets on the root.
    Triplets,
    /// A sixteenth funk line with octave, seventh and dead notes.
    Funk,
    /// Thumb slap on the root, pop on the octave, in eighths.
    Slap,
    /// 3/4: root, fifth, octave.
    Waltz,
    /// 6/8 in two groups of three eighths.
    SixEight,
    /// 5/4 as 3 + 2 beats.
    FiveFour,
    /// 7/8 as 2 + 2 + 3 eighths.
    SevenEight,
}

/// Patterns through a scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sequence {
    /// 1-3, 2-4, 3-5 …
    Thirds,
    /// 1-2-3-4, 2-3-4-5 …
    Fours,
}

/// Finger order on four neighbouring frets (1 = index finger).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fingers {
    /// 1-2-3-4
    Straight,
    /// 1-3-2-4
    Crossed,
    /// 1-2-4-3
    SwapTop,
    /// 1-4-2-3
    Wide,
    /// 1-3-4-2
    Return,
    /// 4-3-2-1
    Reverse,
}

/// One exercise to generate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExerciseSpec {
    pub kind: ExerciseKind,
    /// Key root as pitch class, 0 = C .. 11 = B; start fret for `Chromatic`.
    pub key: u8,
    pub bpm: f64,
    /// Scale exercises: quarters instead of eighths.
    pub slow: bool,
    /// How often the pattern is played (each pass is 2–12 bars).
    pub repeats: u8,
    /// BPM added per pass (tempo ramp); 0 keeps the tempo.
    pub ramp_bpm: f64,
    /// Judged by pitch and time, or by time only (DIDAKTIK I4: grooves on the open string).
    pub judging: Judging,
}

/// A written exercise: a title and a standard MIDI file.
#[derive(Debug, Clone, PartialEq)]
pub struct Exercise {
    pub title: String,
    pub midi: Vec<u8>,
}

/// What an exercise is, as text keys, so the UI can name it in its language; the file keeps
/// its German title (progress is keyed by it). Example: `Groove` over `blues` in `shuffle`,
/// key `E`, 90 BPM → "Groove Shuffle 12-Takt-Blues in E (90 BPM)" / "Groove shuffle 12-bar
/// blues in E (90 BPM)".
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Description {
    /// "scale", "sequence", "arpeggio", "triads", "walking", "groove", "chromatic" or "song".
    pub kind: &'static str,
    /// Scale ("major", "minorpenta" …), progression ("tonic", "251", "blues", "minor" …) or
    /// finger order ("1324").
    pub variant: &'static str,
    /// Grooves: the rhythm ("quarters", "eighths" …); sequences: the pattern ("thirds"); pieces:
    /// the level ("easy", "hard"), none for medium.
    pub rhythm: Option<&'static str>,
    /// Key root spelled for its signature ("Bb", "F#") with "m" for minor.
    pub key: &'static str,
    pub minor: bool,
    pub bpm: f64,
    /// Tempo of the last pass when it rises (tempo ramp).
    pub end_bpm: Option<f64>,
    /// Finger exercises start at a fret instead of a key.
    pub fret: Option<u8>,
    /// What the exercise trains: "pulse", "rests", "syncopation", "chordtones", "sixteenths",
    /// "triplets", "meter", "slap", "scale", "modes", "reading", "walking", "fingers", "ramp",
    /// "form".
    pub goal: &'static str,
    /// Time only: the title says so, so it does not share a name with the judged one.
    pub judging: Judging,
}

/// One note of the generated line, in file ticks.
#[derive(Debug, Clone, Copy)]
struct Tone {
    start: u32,
    length: u32,
    key: i32,
    stroke: Stroke,
    /// (string from the lowest, fret) when the exercise wants a fingering, not just a pitch.
    position: Option<(u8, u8)>,
}

/// How a tone is struck.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stroke {
    Plain,
    /// Muted with the fretting hand: a percussive click without pitch (LH-BEW-06).
    Ghost,
    /// Dead note (x): judged as muted.
    Dead,
    /// Thumb slap.
    Slap,
    /// Pop with the index finger.
    Pop,
}

impl Stroke {
    /// Text event that marks the stroke (MIDI has none of these), see `TECHNIQUE_TEXTS`.
    fn text(self) -> Option<&'static str> {
        match self {
            Stroke::Plain => None,
            Stroke::Ghost => Some(GHOST_TEXT),
            Stroke::Dead => Some(DEAD_TEXT),
            Stroke::Slap => Some(SLAP_TEXT),
            Stroke::Pop => Some(POP_TEXT),
        }
    }

    fn velocity(self) -> u8 {
        match self {
            Stroke::Plain => VELOCITY,
            Stroke::Ghost | Stroke::Dead => GHOST_VELOCITY,
            Stroke::Slap | Stroke::Pop => SLAP_VELOCITY,
        }
    }
}

/// Text event that marks the notes starting at its tick as ghost notes (see `midi.rs`).
const GHOST_TEXT: &str = "ghost";
const DEAD_TEXT: &str = "dead";
const SLAP_TEXT: &str = "slap";
const POP_TEXT: &str = "pop";
/// Text event "pos <string> <fret>" that places the note starting at its tick (string 0 is the
/// lowest of a four-string bass in standard tuning).
pub(crate) const POSITION_TEXT: &str = "pos";

/// Text event at the start: the file is judged by time only (`Judging::TimeOnly`, DIDAKTIK I4).
pub(crate) const TIMING_TEXT: &str = "kontra:timing";
/// Suffix of a file title judged by time only; the UI's twin is `exercise.title.timing`.
const TIMING_TITLE: &str = ", nur Timing";

/// Text events the importer turns into a technique of the notes starting at their tick.
pub(crate) const TECHNIQUE_TEXTS: [(&str, Technique); 4] = [
    (GHOST_TEXT, Technique::Ghost),
    (DEAD_TEXT, Technique::Muted),
    (SLAP_TEXT, Technique::Slap),
    (POP_TEXT, Technique::Pop),
];

const MAJOR: [i32; 8] = [0, 2, 4, 5, 7, 9, 11, 12];
const MINOR: [i32; 8] = [0, 2, 3, 5, 7, 8, 10, 12];
const MAJOR_PENTA: [i32; 6] = [0, 2, 4, 7, 9, 12];
const MINOR_PENTA: [i32; 6] = [0, 3, 5, 7, 10, 12];
const BLUES: [i32; 7] = [0, 3, 5, 6, 7, 10, 12];
const DORIAN: [i32; 8] = [0, 2, 3, 5, 7, 9, 10, 12];
const PHRYGIAN: [i32; 8] = [0, 1, 3, 5, 7, 8, 10, 12];
const LYDIAN: [i32; 8] = [0, 2, 4, 6, 7, 9, 11, 12];
const MIXOLYDIAN: [i32; 8] = [0, 2, 4, 5, 7, 9, 10, 12];
const LOCRIAN: [i32; 8] = [0, 1, 3, 5, 6, 8, 10, 12];
const HARMONIC_MINOR: [i32; 8] = [0, 2, 3, 5, 7, 8, 11, 12];

/// Semitones from a minor root up to its relative major (A minor → C major).
const RELATIVE_MAJOR: u8 = 3;

/// How the chords of an exercise are named: as played (triads), as seventh chords, or as
/// power chords where the line plays only root, fifth and octave (rock).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Voicing {
    Seventh,
    Triad,
    Power,
}

/// Chord quality of a progression step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Quality {
    Major7,
    Minor7,
    Dominant7,
    HalfDiminished,
}

impl Quality {
    /// Root, third, fifth, seventh in semitones.
    fn tones(self) -> [i32; 4] {
        match self {
            Quality::Major7 => [0, 4, 7, 11],
            Quality::Minor7 => [0, 3, 7, 10],
            Quality::Dominant7 => [0, 4, 7, 10],
            Quality::HalfDiminished => [0, 3, 6, 10],
        }
    }

    /// Chord symbol suffix: "maj7" as a seventh chord, "" as its triad, "5" as a power chord
    /// (minor stays "m": the minor sixth of a pop progression).
    fn suffix_for(self, voicing: Voicing) -> &'static str {
        match (voicing, self) {
            (Voicing::Seventh, _) => self.suffix(),
            (Voicing::Power, Quality::Major7 | Quality::Dominant7) => "5",
            (Voicing::Triad, Quality::Major7 | Quality::Dominant7) => "",
            (_, Quality::Minor7) => "m",
            (_, Quality::HalfDiminished) => "dim",
        }
    }

    fn suffix(self) -> &'static str {
        match self {
            Quality::Major7 => "maj7",
            Quality::Minor7 => "m7",
            Quality::Dominant7 => "7",
            Quality::HalfDiminished => "m7b5",
        }
    }
}

impl Progression {
    pub const ALL: [Progression; 8] = [
        Progression::Tonic,
        Progression::TwoFiveOne,
        Progression::Blues,
        Progression::Minor,
        Progression::OneFourFive,
        Progression::Pop,
        Progression::Turnaround,
        Progression::MinorTwoFiveOne,
    ];

    /// (semitones above the key root, quality) per bar.
    fn bars(self) -> Vec<(i32, Quality)> {
        use Quality::*;
        match self {
            Progression::Tonic => vec![(0, Dominant7); 4],
            Progression::TwoFiveOne => vec![(2, Minor7), (7, Dominant7), (0, Major7), (0, Major7)],
            Progression::Blues => [0, 5, 0, 0, 5, 5, 0, 0, 7, 5, 0, 7]
                .iter()
                .map(|r| (*r, Dominant7))
                .collect(),
            Progression::Minor => vec![(0, Minor7), (5, Minor7), (0, Minor7), (7, Minor7)],
            Progression::OneFourFive => {
                vec![(0, Major7), (5, Major7), (7, Dominant7), (0, Major7)]
            }
            Progression::Pop => vec![(0, Major7), (7, Dominant7), (9, Minor7), (5, Major7)],
            Progression::Turnaround => {
                vec![(0, Major7), (9, Minor7), (2, Minor7), (7, Dominant7)]
            }
            Progression::MinorTwoFiveOne => vec![
                (2, HalfDiminished),
                (7, Dominant7),
                (0, Minor7),
                (0, Minor7),
            ],
        }
    }

    fn minor(self) -> bool {
        matches!(self, Progression::Minor | Progression::MinorTwoFiveOne)
    }

    /// The last bar is not the tonic (blues, minor cadence …): a closing bar resolves it.
    fn open_end(self) -> bool {
        self.bars().last().is_some_and(|(offset, _)| *offset != 0)
    }

    /// Text key ("tonic", "251" …), the inverse of `from_key`.
    pub fn key(self) -> &'static str {
        match self {
            Progression::Tonic => "tonic",
            Progression::TwoFiveOne => "251",
            Progression::Blues => "blues",
            Progression::Minor => "minor",
            Progression::OneFourFive => "145",
            Progression::Pop => "pop",
            Progression::Turnaround => "1625",
            Progression::MinorTwoFiveOne => "minor251",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.key() == key)
    }
}

impl ScaleKind {
    pub const ALL: [ScaleKind; 11] = [
        ScaleKind::Major,
        ScaleKind::Minor,
        ScaleKind::MajorPentatonic,
        ScaleKind::MinorPentatonic,
        ScaleKind::Blues,
        ScaleKind::Dorian,
        ScaleKind::Phrygian,
        ScaleKind::Lydian,
        ScaleKind::Mixolydian,
        ScaleKind::Locrian,
        ScaleKind::HarmonicMinor,
    ];

    fn steps(self) -> &'static [i32] {
        match self {
            ScaleKind::Major => &MAJOR,
            ScaleKind::Minor => &MINOR,
            ScaleKind::MajorPentatonic => &MAJOR_PENTA,
            ScaleKind::MinorPentatonic => &MINOR_PENTA,
            ScaleKind::Blues => &BLUES,
            ScaleKind::Dorian => &DORIAN,
            ScaleKind::Phrygian => &PHRYGIAN,
            ScaleKind::Lydian => &LYDIAN,
            ScaleKind::Mixolydian => &MIXOLYDIAN,
            ScaleKind::Locrian => &LOCRIAN,
            ScaleKind::HarmonicMinor => &HARMONIC_MINOR,
        }
    }

    /// Named as a minor key ("in Am"); modes are named by their root alone ("Dorisch in D").
    fn minor(self) -> bool {
        matches!(
            self,
            ScaleKind::Minor
                | ScaleKind::MinorPentatonic
                | ScaleKind::Blues
                | ScaleKind::HarmonicMinor
        )
    }

    /// Semitones from the root up to the major key whose signature the scale uses
    /// (D Dorian → C major: 10).
    fn to_major(self) -> u8 {
        match self {
            ScaleKind::Major | ScaleKind::MajorPentatonic => 0,
            ScaleKind::Minor
            | ScaleKind::MinorPentatonic
            | ScaleKind::Blues
            | ScaleKind::HarmonicMinor => RELATIVE_MAJOR,
            ScaleKind::Dorian => 10,
            ScaleKind::Phrygian => 8,
            ScaleKind::Lydian => 7,
            ScaleKind::Mixolydian => 5,
            ScaleKind::Locrian => 1,
        }
    }

    fn mode(self) -> bool {
        self.to_major() != 0 && self.to_major() != RELATIVE_MAJOR
    }

    pub fn key(self) -> &'static str {
        match self {
            ScaleKind::Major => "major",
            ScaleKind::Minor => "minor",
            ScaleKind::MajorPentatonic => "majorpenta",
            ScaleKind::MinorPentatonic => "minorpenta",
            ScaleKind::Blues => "blues",
            ScaleKind::Dorian => "dorian",
            ScaleKind::Phrygian => "phrygian",
            ScaleKind::Lydian => "lydian",
            ScaleKind::Mixolydian => "mixolydian",
            ScaleKind::Locrian => "locrian",
            ScaleKind::HarmonicMinor => "harmonicminor",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.key() == key)
    }
}

/// Bar length and how the drums group it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Meter {
    Common,
    ThreeFour,
    SixEight,
    FiveFour,
    SevenEight,
}

impl Meter {
    fn bar(self) -> u32 {
        match self {
            Meter::Common => BAR,
            Meter::ThreeFour | Meter::SixEight => 6 * EIGHTH,
            Meter::FiveFour => 10 * EIGHTH,
            Meter::SevenEight => 7 * EIGHTH,
        }
    }

    /// (numerator, denominator as power of two, MIDI clocks per click).
    fn signature(self) -> (u8, u8, u8) {
        match self {
            Meter::Common => (4, 2, 24),
            Meter::ThreeFour => (3, 2, 24),
            Meter::SixEight => (6, 3, 36),
            Meter::FiveFour => (5, 2, 24),
            Meter::SevenEight => (7, 3, 12),
        }
    }

    /// Eighths of the bar with kick and snare: the groups' first beats and their backbeats.
    /// 5/4 as 3 + 2 beats: kick on 1 and 4, snare on 3 and 5.
    fn drums(self) -> (&'static [u32], &'static [u32]) {
        match self {
            Meter::Common => (&[0, 4], &[2, 6]),
            Meter::ThreeFour => (&[0], &[2, 4]),
            Meter::SixEight => (&[0], &[3]),
            Meter::FiveFour => (&[0, 6], &[4, 8]),
            Meter::SevenEight => (&[0, 4], &[2, 6]),
        }
    }
}

/// A chord tone of a pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Degree {
    Root,
    Fifth,
    Seventh,
    Octave,
}

/// (start, length, degree, stroke) in ticks within a bar.
type Hit = (u32, u32, Degree, Stroke);

impl Rhythm {
    pub const ALL: [Rhythm; 19] = [
        Rhythm::Quarters,
        Rhythm::Eighths,
        Rhythm::Sixteenths,
        Rhythm::WithRests,
        Rhythm::Shuffle,
        Rhythm::Octaves,
        Rhythm::Ghost,
        Rhythm::RootFifth,
        Rhythm::RootFifthOctave,
        Rhythm::DeadNotes,
        Rhythm::Offbeat,
        Rhythm::Tresillo,
        Rhythm::Triplets,
        Rhythm::Funk,
        Rhythm::Slap,
        Rhythm::Waltz,
        Rhythm::SixEight,
        Rhythm::FiveFour,
        Rhythm::SevenEight,
    ];

    fn meter(self) -> Meter {
        match self {
            Rhythm::Waltz => Meter::ThreeFour,
            Rhythm::SixEight => Meter::SixEight,
            Rhythm::FiveFour => Meter::FiveFour,
            Rhythm::SevenEight => Meter::SevenEight,
            _ => Meter::Common,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Rhythm::Quarters => "quarters",
            Rhythm::Eighths => "eighths",
            Rhythm::Sixteenths => "sixteenths",
            Rhythm::WithRests => "rests",
            Rhythm::Shuffle => "shuffle",
            Rhythm::Octaves => "octaves",
            Rhythm::Ghost => "ghost",
            Rhythm::RootFifth => "rootfifth",
            Rhythm::RootFifthOctave => "rootfifthoctave",
            Rhythm::DeadNotes => "dead",
            Rhythm::Offbeat => "offbeat",
            Rhythm::Tresillo => "tresillo",
            Rhythm::Triplets => "triplets",
            Rhythm::Funk => "funk",
            Rhythm::Slap => "slap",
            Rhythm::Waltz => "34",
            Rhythm::SixEight => "68",
            Rhythm::FiveFour => "54",
            Rhythm::SevenEight => "78",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|r| r.key() == key)
    }

    /// The bar of the rhythms after the first seven (those keep their own code in `groove_bar`).
    fn hits(self) -> Vec<Hit> {
        use Degree::*;
        use Stroke::*;
        let e = EIGHTH;
        let s = SIXTEENTH;
        match self {
            Rhythm::RootFifth => vec![
                (0, 2 * QUARTER, Root, Plain),
                (2 * QUARTER, 2 * QUARTER, Fifth, Plain),
            ],
            Rhythm::RootFifthOctave => [Root, Fifth, Octave, Fifth]
                .iter()
                .enumerate()
                .map(|(i, d)| (i as u32 * QUARTER, QUARTER, *d, Plain))
                .collect(),
            Rhythm::DeadNotes => (0..8)
                .map(|i| (i * e, e, Root, if i % 2 == 0 { Plain } else { Dead }))
                .collect(),
            Rhythm::Offbeat => (0..4).map(|b| (b * QUARTER + e, e, Root, Plain)).collect(),
            // 1 . . & . . 4 . : dotted quarter, dotted quarter, quarter.
            Rhythm::Tresillo => vec![
                (0, 3 * e, Root, Plain),
                (3 * e, 3 * e, Root, Plain),
                (6 * e, 2 * e, Fifth, Plain),
            ],
            Rhythm::Triplets => (0..12)
                .map(|i| (i * TRIPLET, TRIPLET, Root, Plain))
                .collect(),
            // 1 e & a | 2 e & a | 3 e & a | 4 e & a
            // R . x R | . x 8 . | R . x R | 7 . 8 x
            Rhythm::Funk => vec![
                (0, 2 * s, Root, Plain),
                (2 * s, s, Root, Dead),
                (3 * s, s, Root, Plain),
                (5 * s, s, Root, Dead),
                (6 * s, 2 * s, Octave, Plain),
                (8 * s, 2 * s, Root, Plain),
                (10 * s, s, Root, Dead),
                (11 * s, s, Root, Plain),
                (12 * s, 2 * s, Seventh, Plain),
                (14 * s, s, Octave, Plain),
                (15 * s, s, Root, Dead),
            ],
            Rhythm::Slap => (0..8)
                .map(|i| {
                    if i % 2 == 0 {
                        (i * e, e, Root, Slap)
                    } else {
                        (i * e, e, Octave, Pop)
                    }
                })
                .collect(),
            Rhythm::Waltz => [Root, Fifth, Octave]
                .iter()
                .enumerate()
                .map(|(i, d)| (i as u32 * QUARTER, QUARTER, *d, Plain))
                .collect(),
            Rhythm::SixEight => vec![
                (0, 3 * e, Root, Plain),
                (3 * e, 2 * e, Fifth, Plain),
                (5 * e, e, Root, Plain),
            ],
            // 1 2 3 | 4 5: R . . 5 8 . 5 . R .
            Rhythm::FiveFour => vec![
                (0, 3 * e, Root, Plain),
                (3 * e, e, Fifth, Plain),
                (4 * e, 2 * e, Octave, Plain),
                (6 * e, 2 * e, Fifth, Plain),
                (8 * e, 2 * e, Root, Plain),
            ],
            // 1 2 | 1 2 | 1 2 3: R . 5 . 8 5 .
            Rhythm::SevenEight => vec![
                (0, 2 * e, Root, Plain),
                (2 * e, 2 * e, Fifth, Plain),
                (4 * e, e, Octave, Plain),
                (5 * e, 2 * e, Fifth, Plain),
            ],
            _ => Vec::new(),
        }
    }

    /// The line plays the chord's seventh (funk): its chords stay seventh chords.
    fn plays_seventh(self) -> bool {
        self.hits().iter().any(|hit| hit.2 == Degree::Seventh)
    }

    fn goal(self) -> &'static str {
        match self {
            Rhythm::Quarters | Rhythm::Eighths => "pulse",
            Rhythm::WithRests => "rests",
            Rhythm::Offbeat | Rhythm::Tresillo => "syncopation",
            Rhythm::Octaves | Rhythm::RootFifth | Rhythm::RootFifthOctave => "chordtones",
            Rhythm::Sixteenths | Rhythm::Ghost | Rhythm::DeadNotes | Rhythm::Funk => "sixteenths",
            Rhythm::Shuffle | Rhythm::Triplets => "triplets",
            Rhythm::Slap => "slap",
            Rhythm::Waltz | Rhythm::SixEight | Rhythm::FiveFour | Rhythm::SevenEight => "meter",
        }
    }
}

impl Sequence {
    pub const ALL: [Sequence; 2] = [Sequence::Thirds, Sequence::Fours];

    pub fn key(self) -> &'static str {
        match self {
            Sequence::Thirds => "thirds",
            Sequence::Fours => "fours",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.key() == key)
    }
}

impl Fingers {
    pub const ALL: [Fingers; 6] = [
        Fingers::Straight,
        Fingers::Crossed,
        Fingers::SwapTop,
        Fingers::Wide,
        Fingers::Return,
        Fingers::Reverse,
    ];

    /// The finger order as text, also the text key: "1324".
    pub fn key(self) -> &'static str {
        match self {
            Fingers::Straight => "1234",
            Fingers::Crossed => "1324",
            Fingers::SwapTop => "1243",
            Fingers::Wide => "1423",
            Fingers::Return => "1342",
            Fingers::Reverse => "4321",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.key() == key)
    }

    /// Frets above the start fret, one per finger.
    fn frets(self) -> [u8; 4] {
        let mut frets = [0; 4];
        for (fret, digit) in frets.iter_mut().zip(self.key().bytes()) {
            *fret = digit - b'1';
        }
        frets
    }

    /// "1-3-2-4", as in the title.
    fn label(self) -> String {
        self.key()
            .chars()
            .map(String::from)
            .collect::<Vec<_>>()
            .join("-")
    }
}

const SHARP_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];
const FLAT_NAMES: [&str; 12] = [
    "C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B",
];

/// Key signature in fifths for a major or minor key on `root`.
fn key_fifths(root: u8, minor: bool) -> i8 {
    signature(root, if minor { RELATIVE_MAJOR } else { 0 })
}

/// Key signature in fifths of the major key `to_major` semitones above `root`.
fn signature(root: u8, to_major: u8) -> i8 {
    // Fifths of each major key root: C G D A E B F# | F Bb Eb Ab Db.
    const MAJOR_FIFTHS: [i8; 12] = [0, -5, 2, -3, 4, -1, 6, 1, -4, 3, -2, 5];
    // E♭ minor (six flats) is far more common than D♯ minor (six sharps).
    const EB_MINOR: (u8, u8) = (3, RELATIVE_MAJOR);
    const SIX_FLATS: i8 = -6;
    if (root % 12, to_major) == EB_MINOR {
        return SIX_FLATS;
    }
    let major_root = (root % 12 + to_major) % 12;
    MAJOR_FIFTHS[major_root as usize]
}

/// Note name of pitch class `pc` spelled for a key with `fifths`.
fn name(pc: i32, fifths: i8) -> &'static str {
    let pc = pc.rem_euclid(12) as usize;
    if fifths < 0 {
        FLAT_NAMES[pc]
    } else {
        SHARP_NAMES[pc]
    }
}

/// The key root placed as low as possible on the bass (E1..D#2).
fn low_root(key: u8) -> i32 {
    let pc = i32::from(key % 12);
    LOWEST + (pc - LOWEST).rem_euclid(12)
}

/// `pc` in the octave nearest to `near`, kept inside the bass range.
fn nearest(pc: i32, near: i32) -> i32 {
    let mut key = near + (pc - near).rem_euclid(12);
    if key - near > 6 {
        key -= 12;
    }
    while key < LOWEST {
        key += 12;
    }
    while key > HIGHEST {
        key -= 12;
    }
    key
}

/// A root low enough that its octave stays playable (A♭2 → A♭1).
fn octave_root(root: i32) -> i32 {
    if root + OCTAVE > HIGHEST && root - OCTAVE >= LOWEST {
        root - OCTAVE
    } else {
        root
    }
}

/// What one pass of an exercise holds.
struct Pass {
    tones: Vec<Tone>,
    markers: Vec<(u32, String)>,
    length: u32,
    minor: bool,
    /// Semitones from the key root to the major key of the signature.
    to_major: u8,
    label: String,
}

/// Generates `spec` as a titled MIDI file.
pub fn generate(spec: &ExerciseSpec) -> Exercise {
    // A piece has its own form and drums for one pass: never repeated.
    let repeats = match spec.kind {
        ExerciseKind::Song(..) => 1,
        _ => u32::from(spec.repeats.max(1)),
    };
    let pass = one_pass(spec);
    let (mut tones, mut markers) = repeat(&pass.tones, &pass.markers, pass.length, repeats);
    let end = pass.length * repeats;
    let meter = spec.meter();

    // A blues or minor cadence ends on the dominant: one more bar on the tonic resolves it.
    let closing = spec.progression().filter(|p| p.open_end());
    if let Some(progression) = closing {
        let (offset, quality) = progression.bars()[0];
        let root = nearest(i32::from(spec.key) + offset, low_root(spec.key));
        let fifths = key_fifths(spec.key, progression.minor());
        let suffix = quality.suffix_for(spec.voicing());
        markers.push((end, format!("{}{}", name(root, fifths), suffix)));
        tones.push(Tone {
            start: end,
            length: meter.bar(),
            key: root,
            stroke: Stroke::Plain,
            position: None,
        });
    }

    let fifths = signature(spec.key, pass.to_major);
    let title = title(spec, &pass, fifths);
    let drums = match spec.kind {
        ExerciseKind::Song(style, level) => song::song(style, level, spec.key).1,
        ExerciseKind::Groove(_, rhythm) => {
            let bars = end / meter.bar();
            let mut hits: Vec<(u32, u8)> = (0..bars)
                .flat_map(|b| drum_bar(b * meter.bar(), rhythm))
                .collect();
            if closing.is_some() {
                hits.extend([(end, KICK), (end, HIHAT)]);
            }
            hits
        }
        _ => Vec::new(),
    };
    // A tempo ramp changes the tempo at each pass; else one tempo for all.
    let changes = if spec.end_bpm().is_some() { repeats } else { 1 };
    let tempos: Vec<(u32, f64)> = (0..changes)
        .map(|p| (p * pass.length, spec.bpm + f64::from(p) * spec.ramp_bpm))
        .collect();
    let line = Line {
        tones: &tones,
        markers: &markers,
        drums: &drums,
        tempos: &tempos,
        meter,
        judging: spec.judging,
    };
    let midi = write(&title, fifths, pass.minor, &line);
    Exercise { title, midi }
}

impl ExerciseSpec {
    fn progression(&self) -> Option<Progression> {
        match self.kind {
            ExerciseKind::Arpeggio(p)
            | ExerciseKind::WalkingBass(p)
            | ExerciseKind::Groove(p, _)
            | ExerciseKind::Triads(p) => Some(p),
            _ => None,
        }
    }

    /// Triads where the line plays them, and for a groove over the pop progression that plays
    /// no seventh (round 16: maj7 read like jazz to a beginner).
    fn voicing(&self) -> Voicing {
        match self.kind {
            ExerciseKind::Triads(_) => Voicing::Triad,
            ExerciseKind::Groove(Progression::Pop, rhythm) if !rhythm.plays_seventh() => {
                Voicing::Triad
            }
            _ => Voicing::Seventh,
        }
    }

    fn meter(&self) -> Meter {
        match self.kind {
            ExerciseKind::Groove(_, rhythm) => rhythm.meter(),
            _ => Meter::Common,
        }
    }

    /// Tempo of the last pass when it rises.
    fn end_bpm(&self) -> Option<f64> {
        let passes = f64::from(self.repeats.max(1) - 1);
        (self.ramp_bpm > 0.0 && passes > 0.0).then_some(self.bpm + passes * self.ramp_bpm)
    }

    fn fret(&self) -> u8 {
        self.key.clamp(MIN_FRET, MAX_FRET)
    }
}

/// The German title the file keeps: "Groove Achtel I-IV-V in A (80 BPM)",
/// "Fingerübung 1-3-2-4 ab Bund 5 (60 BPM)", "… in E (80-100 BPM)".
fn title(spec: &ExerciseSpec, pass: &Pass, fifths: i8) -> String {
    let bpm = match spec.end_bpm() {
        Some(end) => format!("{:.0}-{end:.0}", spec.bpm),
        None => format!("{:.0}", spec.bpm),
    };
    // Judged by time only, the file is another exercise: its own name and progress.
    let timing = match spec.judging {
        Judging::TimeOnly => TIMING_TITLE,
        Judging::PitchAndTime => "",
    };
    if let ExerciseKind::Chromatic(_) = spec.kind {
        return format!("{} ab Bund {} ({bpm} BPM){timing}", pass.label, spec.fret());
    }
    let key_name = format!(
        "{}{}",
        name(i32::from(spec.key), fifths),
        if pass.minor { "m" } else { "" }
    );
    format!("{} in {key_name} ({bpm} BPM){timing}", pass.label)
}

/// Notes, markers, length and label of one pass of `spec`.
fn one_pass(spec: &ExerciseSpec) -> Pass {
    let chords = |progression: Progression, label: String, tones, markers| Pass {
        tones,
        markers,
        length: bars_len(progression, Meter::Common),
        minor: progression.minor(),
        to_major: if progression.minor() {
            RELATIVE_MAJOR
        } else {
            0
        },
        label,
    };
    match spec.kind {
        ExerciseKind::Scale(scale) => {
            let (tones, length) = scale_line(scale, spec.key, spec.slow);
            scale_pass(scale, tones, length, scale_label(scale).to_string())
        }
        ExerciseKind::Sequence(scale, sequence) => {
            let (tones, length) = sequence_line(scale, sequence, spec.key);
            let label = format!("{} {}", scale_label(scale), sequence_label(sequence));
            scale_pass(scale, tones, length, label)
        }
        ExerciseKind::Arpeggio(progression) => {
            let (tones, markers) = chords_line(
                progression,
                spec.key,
                Voicing::Seventh,
                Meter::Common,
                arpeggio_bar,
            );
            let label = format!("Arpeggios {}", progression_label(progression));
            chords(progression, label, tones, markers)
        }
        ExerciseKind::Triads(progression) => {
            let (tones, markers) = chords_line(
                progression,
                spec.key,
                Voicing::Triad,
                Meter::Common,
                triad_bar,
            );
            let label = format!("Dreiklänge {}", progression_label(progression));
            chords(progression, label, tones, markers)
        }
        ExerciseKind::WalkingBass(progression) => {
            let (tones, markers) = walking_line(progression, spec.key);
            let label = format!("Walking Bass {}", progression_label(progression));
            chords(progression, label, tones, markers)
        }
        ExerciseKind::Groove(progression, rhythm) => {
            let meter = rhythm.meter();
            let (tones, markers) = chords_line(
                progression,
                spec.key,
                spec.voicing(),
                meter,
                |start, root, q| groove_bar(start, root, q, rhythm),
            );
            Pass {
                length: bars_len(progression, meter),
                ..chords(
                    progression,
                    format!(
                        "Groove {} {}",
                        rhythm_label(rhythm),
                        progression_label(progression)
                    ),
                    tones,
                    markers,
                )
            }
        }
        ExerciseKind::Song(style, level) => song::song(style, level, spec.key).0,
        ExerciseKind::Chromatic(fingers) => {
            let (tones, length) = chromatic_line(fingers, spec.fret(), spec.slow);
            Pass {
                tones,
                markers: Vec::new(),
                length,
                minor: false,
                to_major: 0,
                label: format!("Fingerübung {}", fingers.label()),
            }
        }
    }
}

fn scale_pass(scale: ScaleKind, tones: Vec<Tone>, length: u32, label: String) -> Pass {
    Pass {
        tones,
        markers: Vec::new(),
        length,
        minor: scale.minor(),
        to_major: scale.to_major(),
        label,
    }
}

/// What `write` puts into the file besides the header data.
struct Line<'a> {
    tones: &'a [Tone],
    markers: &'a [(u32, String)],
    /// (tick, General MIDI drum key).
    drums: &'a [(u32, u8)],
    /// (tick, BPM): the first at 0, more for a tempo ramp.
    tempos: &'a [(u32, f64)],
    meter: Meter,
    judging: Judging,
}

/// One bar of drums under a groove: kick on one and three, snare on two and four, hi-hat in
/// eighths (rock beat), swung for the shuffle, in triplets for triplets; odd meters by group.
fn drum_bar(start: u32, rhythm: Rhythm) -> Vec<(u32, u8)> {
    let meter = rhythm.meter();
    if meter != Meter::Common {
        let (kicks, snares) = meter.drums();
        let mut hits: Vec<(u32, u8)> = (0..meter.bar() / EIGHTH)
            .map(|e| (start + e * EIGHTH, HIHAT))
            .collect();
        hits.extend(kicks.iter().map(|e| (start + e * EIGHTH, KICK)));
        hits.extend(snares.iter().map(|e| (start + e * EIGHTH, SNARE)));
        return hits;
    }
    let offbeat = if rhythm == Rhythm::Shuffle {
        QUARTER * 2 / 3
    } else {
        EIGHTH
    };
    let mut hits = Vec::new();
    for beat in 0..4 {
        let at = start + beat * QUARTER;
        hits.push((at, if beat % 2 == 0 { KICK } else { SNARE }));
        hits.push((at, HIHAT));
        if rhythm == Rhythm::Triplets {
            hits.push((at + TRIPLET, HIHAT));
            hits.push((at + 2 * TRIPLET, HIHAT));
            continue;
        }
        hits.push((at + offbeat, HIHAT));
    }
    // Rock beat: a second kick on the "and" of three drives the straight grooves.
    if !matches!(rhythm, Rhythm::Shuffle | Rhythm::Triplets) {
        hits.push((start + 2 * QUARTER + EIGHTH, KICK));
    }
    hits
}

fn bars_len(progression: Progression, meter: Meter) -> u32 {
    progression.bars().len() as u32 * meter.bar()
}

fn scale_label(scale: ScaleKind) -> &'static str {
    match scale {
        ScaleKind::Major => "Dur-Tonleiter",
        ScaleKind::Minor => "Moll-Tonleiter",
        ScaleKind::MajorPentatonic => "Dur-Pentatonik",
        ScaleKind::MinorPentatonic => "Moll-Pentatonik",
        ScaleKind::Blues => "Blues-Tonleiter",
        ScaleKind::Dorian => "Dorisch",
        ScaleKind::Phrygian => "Phrygisch",
        ScaleKind::Lydian => "Lydisch",
        ScaleKind::Mixolydian => "Mixolydisch",
        ScaleKind::Locrian => "Lokrisch",
        ScaleKind::HarmonicMinor => "Harmonisch Moll",
    }
}

fn sequence_label(sequence: Sequence) -> &'static str {
    match sequence {
        Sequence::Thirds => "in Terzen",
        Sequence::Fours => "in Vierergruppen",
    }
}

fn progression_label(progression: Progression) -> &'static str {
    match progression {
        Progression::Tonic => "auf einem Akkord",
        Progression::TwoFiveOne => "II-V-I",
        Progression::Blues => "12-Takt-Blues",
        Progression::Minor => "Moll-Kadenz",
        Progression::OneFourFive => "I-IV-V",
        Progression::Pop => "I-V-VI-IV",
        Progression::Turnaround => "I-VI-II-V",
        Progression::MinorTwoFiveOne => "Moll-II-V-I",
    }
}

fn rhythm_label(rhythm: Rhythm) -> &'static str {
    match rhythm {
        Rhythm::Quarters => "Viertel",
        Rhythm::Eighths => "Achtel",
        Rhythm::Sixteenths => "Sechzehntel",
        Rhythm::WithRests => "mit Pausen",
        Rhythm::Shuffle => "Shuffle",
        Rhythm::Octaves => "Oktaven",
        Rhythm::Ghost => "Ghost Notes",
        Rhythm::RootFifth => "Grundton-Quinte",
        Rhythm::RootFifthOctave => "Grundton-Quinte-Oktave",
        Rhythm::DeadNotes => "Dead Notes",
        Rhythm::Offbeat => "Offbeat",
        Rhythm::Tresillo => "Tresillo",
        Rhythm::Triplets => "Triolen",
        Rhythm::Funk => "Funk",
        Rhythm::Slap => "Slap",
        Rhythm::Waltz => "3/4",
        Rhythm::SixEight => "6/8",
        Rhythm::FiveFour => "5/4",
        Rhythm::SevenEight => "7/8",
    }
}

/// Text keys for `spec` (see `Description`).
pub fn describe(spec: &ExerciseSpec) -> Description {
    let (kind, variant, rhythm, minor, to_major) = match spec.kind {
        ExerciseKind::Scale(s) => ("scale", s.key(), None, s.minor(), s.to_major()),
        ExerciseKind::Sequence(s, q) => {
            ("sequence", s.key(), Some(q.key()), s.minor(), s.to_major())
        }
        ExerciseKind::Arpeggio(p) => ("arpeggio", p.key(), None, p.minor(), 0),
        ExerciseKind::Triads(p) => ("triads", p.key(), None, p.minor(), 0),
        ExerciseKind::WalkingBass(p) => ("walking", p.key(), None, p.minor(), 0),
        ExerciseKind::Groove(p, r) => ("groove", p.key(), Some(r.key()), p.minor(), 0),
        ExerciseKind::Chromatic(f) => ("chromatic", f.key(), None, false, 0),
        // The level stands where grooves name their rhythm; medium is the plain piece.
        ExerciseKind::Song(style, level) => {
            let level = (level != Level::Medium).then(|| level.key());
            ("song", style.key(), level, style.minor(), 0)
        }
    };
    let to_major = if minor { RELATIVE_MAJOR } else { to_major };
    let fret = matches!(spec.kind, ExerciseKind::Chromatic(_)).then(|| spec.fret());
    Description {
        kind,
        variant,
        rhythm,
        key: name(i32::from(spec.key), signature(spec.key, to_major)),
        minor,
        bpm: spec.bpm,
        end_bpm: spec.end_bpm(),
        fret,
        goal: goal(spec),
        judging: spec.judging,
    }
}

/// What `spec` trains (`Description::goal`); a tempo ramp trains staying calm when it rises.
fn goal(spec: &ExerciseSpec) -> &'static str {
    if spec.end_bpm().is_some() {
        return "ramp";
    }
    if spec.judging == Judging::TimeOnly {
        return "timing";
    }
    match spec.kind {
        ExerciseKind::Scale(s) if s.mode() => "modes",
        ExerciseKind::Scale(_) => "scale",
        ExerciseKind::Sequence(..) => "reading",
        ExerciseKind::Arpeggio(_) | ExerciseKind::Triads(_) => "chordtones",
        ExerciseKind::WalkingBass(_) => "walking",
        ExerciseKind::Groove(_, r) => r.goal(),
        ExerciseKind::Chromatic(_) => "fingers",
        ExerciseKind::Song(..) => "form",
    }
}

/// Repeats a line of `length` ticks `times`, shifting notes and markers.
fn repeat(
    tones: &[Tone],
    markers: &[(u32, String)],
    length: u32,
    times: u32,
) -> (Vec<Tone>, Vec<(u32, String)>) {
    let mut all = Vec::new();
    let mut all_markers = Vec::new();
    for pass in 0..times {
        let shift = pass * length;
        all.extend(tones.iter().map(|t| Tone {
            start: t.start + shift,
            ..*t
        }));
        all_markers.extend(markers.iter().map(|(at, text)| (at + shift, text.clone())));
    }
    (all, all_markers)
}

/// Up and down the scale (one octave, two from a low root when it fits), padded to whole bars.
fn scale_line(scale: ScaleKind, key: u8, slow: bool) -> (Vec<Tone>, u32) {
    let root = low_root(key);
    let step = if slow { QUARTER } else { EIGHTH };
    let mut degrees: Vec<i32> = scale.steps().to_vec();
    if !slow && root + 24 <= HIGHEST {
        let top: Vec<i32> = scale.steps()[1..].iter().map(|d| d + 12).collect();
        degrees.extend(top);
    }
    let down: Vec<i32> = degrees.iter().rev().skip(1).copied().collect();
    degrees.extend(down);
    even_line(&degrees, root, step)
}

/// `degrees` above `root` in even steps; the last note rings to the bar line.
fn even_line(degrees: &[i32], root: i32, step: u32) -> (Vec<Tone>, u32) {
    let mut tones: Vec<Tone> = degrees
        .iter()
        .enumerate()
        .map(|(i, d)| Tone {
            start: i as u32 * step,
            length: step,
            key: root + d,
            stroke: Stroke::Plain,
            position: None,
        })
        .collect();
    let end = tones.len() as u32 * step;
    let length = end.div_ceil(BAR) * BAR;
    if let Some(last) = tones.last_mut() {
        last.length = held(last.start, length);
    }
    (tones, length)
}

/// The longest plain value a closing tone at `start` can hold before `end`, starting on a
/// multiple of itself, so the staff writes it as one note and rests fill the bar.
/// Example: a tone on the second eighth of the last bar → an eighth (seven eighths before).
fn held(start: u32, end: u32) -> u32 {
    let offset = start % BAR;
    [BAR, BAR / 2, QUARTER, EIGHTH, SIXTEENTH]
        .into_iter()
        .find(|v| *v <= end - start && offset.is_multiple_of(*v))
        .unwrap_or(end - start)
}

/// A scale pattern in eighths over one octave: thirds (1-3 2-4 … up, 8-6 7-5 … down, then the
/// root) or groups of four (1234 2345 … 5678 up, 8765 … 4321 down).
fn sequence_line(scale: ScaleKind, sequence: Sequence, key: u8) -> (Vec<Tone>, u32) {
    let steps = scale.steps();
    let n = steps.len() - 1;
    // Degree i of the scale, continued into the next octave.
    let degree = |i: usize| steps[i % n] + OCTAVE * (i / n) as i32;
    let mut degrees = Vec::new();
    match sequence {
        Sequence::Thirds => {
            for i in 0..n {
                degrees.extend([degree(i), degree(i + 2)]);
            }
            // Down from the octave; the last pair ends on the root.
            for i in (2..=n).rev() {
                degrees.extend([degree(i), degree(i - 2)]);
            }
        }
        Sequence::Fours => {
            for i in 0..=n - 3 {
                degrees.extend((i..i + 4).map(degree));
            }
            for i in (3..=n).rev() {
                degrees.extend((i - 3..=i).rev().map(degree));
            }
        }
    }
    even_line(&degrees, low_root(key), EIGHTH)
}

/// Four frets from `fret` with `fingers` on each string from E to G, back with the order
/// reversed from G to E; quarters when `slow`, else eighths.
fn chromatic_line(fingers: Fingers, fret: u8, slow: bool) -> (Vec<Tone>, u32) {
    let step = if slow { QUARTER } else { EIGHTH };
    let frets = fingers.frets();
    let strings = 0..STRINGS;
    let up = strings.clone().flat_map(|s| frets.map(|f| (s, f)));
    let down = strings
        .rev()
        .flat_map(|s| frets.into_iter().rev().map(move |f| (s, f)));
    // The tab generator would move the pattern onto open strings; the file says where it lies.
    let tones: Vec<Tone> = up
        .chain(down)
        .enumerate()
        .map(|(i, (string, offset))| {
            let fret = fret + offset;
            Tone {
                start: i as u32 * step,
                length: step,
                key: LOWEST + STRING_STEP * i32::from(string) + i32::from(fret),
                stroke: Stroke::Plain,
                position: Some((string, fret)),
            }
        })
        .collect();
    let length = (tones.len() as u32 * step).div_ceil(BAR) * BAR;
    (tones, length)
}

/// One bar per chord, filled by `bar(start, root, quality)`; markers carry the chord names.
fn chords_line(
    progression: Progression,
    key: u8,
    voicing: Voicing,
    meter: Meter,
    bar: impl Fn(u32, i32, Quality) -> Vec<Tone>,
) -> (Vec<Tone>, Vec<(u32, String)>) {
    let fifths = key_fifths(key, progression.minor());
    let base = low_root(key);
    let mut tones = Vec::new();
    let mut markers = Vec::new();
    for (i, (offset, quality)) in progression.bars().into_iter().enumerate() {
        let start = i as u32 * meter.bar();
        let root = nearest(i32::from(key) + offset, base);
        let chord = format!("{}{}", name(root, fifths), quality.suffix_for(voicing));
        markers.push((start, chord));
        tones.extend(bar(start, root, quality));
    }
    (tones, markers)
}

fn arpeggio_bar(start: u32, root: i32, quality: Quality) -> Vec<Tone> {
    quality
        .tones()
        .iter()
        .enumerate()
        .map(|(i, t)| Tone {
            start: start + i as u32 * QUARTER,
            length: QUARTER,
            key: fit(root + t),
            stroke: Stroke::Plain,
            position: None,
        })
        .collect()
}

/// Root, third, fifth, octave of the chord's triad in quarters.
fn triad_bar(start: u32, root: i32, quality: Quality) -> Vec<Tone> {
    let tones = quality.tones();
    let root = octave_root(root);
    [0, tones[1], tones[2], OCTAVE]
        .iter()
        .enumerate()
        .map(|(i, t)| Tone {
            start: start + i as u32 * QUARTER,
            length: QUARTER,
            key: root + t,
            stroke: Stroke::Plain,
            position: None,
        })
        .collect()
}

/// Keeps a chord tone playable: an octave down when it is above the range.
fn fit(key: i32) -> i32 {
    if key > HIGHEST {
        key - 12
    } else {
        key
    }
}

fn groove_bar(start: u32, root: i32, quality: Quality, rhythm: Rhythm) -> Vec<Tone> {
    let note = |at: u32, length: u32, key: i32| Tone {
        start: start + at,
        length,
        key,
        stroke: Stroke::Plain,
        position: None,
    };
    match rhythm {
        Rhythm::Quarters => (0..4).map(|b| note(b * QUARTER, QUARTER, root)).collect(),
        Rhythm::Eighths => (0..8).map(|e| note(e * EIGHTH, EIGHTH, root)).collect(),
        Rhythm::Sixteenths => (0..4)
            .flat_map(|b| {
                let beat = b * QUARTER;
                [
                    note(beat, EIGHTH, root),
                    note(beat + EIGHTH, SIXTEENTH, root),
                    note(beat + EIGHTH + SIXTEENTH, SIXTEENTH, root),
                ]
            })
            .collect(),
        Rhythm::WithRests => vec![
            note(0, QUARTER, root),
            note(QUARTER + EIGHTH, EIGHTH, root),
            note(2 * QUARTER, QUARTER, root),
        ],
        Rhythm::Shuffle => (0..4)
            .flat_map(|b| {
                let beat = b * QUARTER;
                let long = QUARTER * 2 / 3;
                [
                    note(beat, long, root),
                    note(beat + long, QUARTER - long, root),
                ]
            })
            .collect(),
        Rhythm::Octaves => {
            let root = octave_root(root);
            (0..8)
                .map(|e| {
                    let key = if e % 2 == 0 { root } else { root + OCTAVE };
                    note(e * EIGHTH, EIGHTH, key)
                })
                .collect()
        }
        // 1 e & a: root on the beat and the "and", muted on the sixteenths between.
        Rhythm::Ghost => (0..16)
            .map(|s| {
                let tone = note(s * SIXTEENTH, SIXTEENTH, root);
                if s % 2 == 0 {
                    return tone;
                }
                Tone {
                    stroke: Stroke::Ghost,
                    ..tone
                }
            })
            .collect(),
        _ => {
            let root = octave_root(root);
            let tones = quality.tones();
            rhythm
                .hits()
                .into_iter()
                .map(|(at, length, degree, stroke)| {
                    let interval = match degree {
                        Degree::Root => 0,
                        Degree::Fifth => tones[2],
                        Degree::Seventh => tones[3],
                        Degree::Octave => OCTAVE,
                    };
                    Tone {
                        stroke,
                        ..note(at, length, root + interval)
                    }
                })
                .collect()
        }
    }
}

/// Root on one, chord tones on two and three, a chromatic approach to the next root on four.
fn walking_line(progression: Progression, key: u8) -> (Vec<Tone>, Vec<(u32, String)>) {
    let bars = progression.bars();
    let base = low_root(key);
    let roots: Vec<i32> = bars
        .iter()
        .map(|(offset, _)| nearest(i32::from(key) + offset, base))
        .collect();
    let fifths = key_fifths(key, progression.minor());
    let mut tones = Vec::new();
    let mut markers = Vec::new();
    for (i, (_, quality)) in bars.iter().enumerate() {
        let start = i as u32 * BAR;
        let root = roots[i];
        let next = roots[(i + 1) % roots.len()];
        markers.push((start, format!("{}{}", name(root, fifths), quality.suffix())));
        let chord = quality.tones();
        // Alternate the shape so the line does not repeat itself bar after bar.
        let (second, third) = if i % 2 == 0 {
            (chord[1], chord[2])
        } else {
            (chord[2], chord[3])
        };
        // Approach from below when the next root lies higher, else from above; below the
        // lowest note there is nothing, so from above there too.
        let approach = if next >= root && next > LOWEST {
            next - 1
        } else {
            next + 1
        };
        let line = [root, fit(root + second), fit(root + third), approach];
        for (beat, key) in line.iter().enumerate() {
            tones.push(Tone {
                start: start + beat as u32 * QUARTER,
                length: QUARTER,
                key: (*key).clamp(LOWEST, HIGHEST),
                stroke: Stroke::Plain,
                position: None,
            });
        }
    }
    (tones, markers)
}

/// A type 1 file: a conductor track (title, tempo, metre, key, chord markers), a bass track and,
/// for grooves, a drum track on channel 10.
fn write(title: &str, fifths: i8, minor: bool, line: &Line) -> Vec<u8> {
    let (tones, markers) = (line.tones, line.markers);
    let (numerator, pow, clocks) = line.meter.signature();
    let mut conductor: Vec<(u32, TrackEventKind)> = vec![(
        0,
        TrackEventKind::Meta(MetaMessage::TrackName(title.as_bytes())),
    )];
    conductor.extend(line.tempos.iter().map(|(at, bpm)| {
        let micros = (60_000_000.0 / bpm.clamp(20.0, 300.0)).round() as u32;
        (
            *at,
            TrackEventKind::Meta(MetaMessage::Tempo(u24::new(micros))),
        )
    }));
    conductor.extend([
        (
            0,
            TrackEventKind::Meta(MetaMessage::TimeSignature(numerator, pow, clocks, 8)),
        ),
        (
            0,
            TrackEventKind::Meta(MetaMessage::KeySignature(fifths, minor)),
        ),
    ]);
    if line.judging == Judging::TimeOnly {
        conductor.push((
            0,
            TrackEventKind::Meta(MetaMessage::Text(TIMING_TEXT.as_bytes())),
        ));
    }
    conductor.extend(markers.iter().map(|(at, text)| {
        (
            *at,
            TrackEventKind::Meta(MetaMessage::Marker(text.as_bytes())),
        )
    }));
    let end = tones.iter().map(|t| t.start + t.length).max().unwrap_or(0);
    let channel = u4::new(0);
    let mut bass: Vec<(u32, TrackEventKind)> = vec![
        (0, TrackEventKind::Meta(MetaMessage::TrackName(b"Bass"))),
        (
            0,
            TrackEventKind::Midi {
                channel,
                message: MidiMessage::ProgramChange {
                    program: u7::new(BASS_PROGRAM),
                },
            },
        ),
    ];
    // "pos 1 7": string (from the lowest) and fret, read by the importer like the techniques.
    let positions: Vec<Option<String>> = tones
        .iter()
        .map(|t| t.position.map(|(s, f)| format!("{POSITION_TEXT} {s} {f}")))
        .collect();
    for (t, position) in tones.iter().zip(&positions) {
        let key = u7::new(t.key.clamp(0, 127) as u8);
        if let Some(text) = position {
            bass.push((
                t.start,
                TrackEventKind::Meta(MetaMessage::Text(text.as_bytes())),
            ));
        }
        let velocity = t.stroke.velocity();
        // MIDI has no ghost note or slap: a text event at the note's tick tells the importer.
        if let Some(text) = t.stroke.text() {
            bass.push((
                t.start,
                TrackEventKind::Meta(MetaMessage::Text(text.as_bytes())),
            ));
        }
        bass.push((
            t.start,
            TrackEventKind::Midi {
                channel,
                message: MidiMessage::NoteOn {
                    key,
                    vel: u7::new(velocity),
                },
            },
        ));
        // Release slightly early so repeated notes re-attack cleanly.
        let off = t.start + t.length.saturating_sub(t.length / 16).max(1);
        bass.push((
            off,
            TrackEventKind::Midi {
                channel,
                message: MidiMessage::NoteOff {
                    key,
                    vel: u7::new(0),
                },
            },
        ));
    }
    let mut tracks = vec![track(conductor, end), track(bass, end)];
    if !line.drums.is_empty() {
        tracks.push(track(drum_track(line.drums), end));
    }
    let smf = Smf {
        header: Header::new(Format::Parallel, Timing::Metrical(u15::new(PPQ as u16))),
        tracks,
    };
    let mut out = Vec::new();
    // Writing into memory only fails on invalid events, which `track` never builds.
    if smf.write_std(&mut out).is_err() {
        out.clear();
    }
    out
}

/// Drum strokes as short notes on channel 10.
fn drum_track(drums: &[(u32, u8)]) -> Vec<(u32, TrackEventKind<'static>)> {
    let channel = u4::new(DRUM_CHANNEL);
    let mut events = vec![(0, TrackEventKind::Meta(MetaMessage::TrackName(b"Drums")))];
    for &(at, key) in drums {
        let key = u7::new(key);
        let on = MidiMessage::NoteOn {
            key,
            vel: u7::new(DRUM_VELOCITY),
        };
        let off = MidiMessage::NoteOff {
            key,
            vel: u7::new(0),
        };
        events.push((
            at,
            TrackEventKind::Midi {
                channel,
                message: on,
            },
        ));
        events.push((
            at + SIXTEENTH / 2,
            TrackEventKind::Midi {
                channel,
                message: off,
            },
        ));
    }
    events
}

/// Sorts absolute events (note-offs before note-ons at the same tick) into delta time.
fn track(mut events: Vec<(u32, TrackEventKind)>, end: u32) -> Vec<TrackEvent> {
    let order = |kind: &TrackEventKind| match kind {
        TrackEventKind::Meta(_) => 0,
        TrackEventKind::Midi {
            message: MidiMessage::NoteOff { .. },
            ..
        } => 1,
        _ => 2,
    };
    events.sort_by_key(|(at, kind)| (*at, order(kind)));
    let mut out = Vec::with_capacity(events.len() + 1);
    let mut last = 0;
    for (at, kind) in events {
        out.push(TrackEvent {
            delta: u28::new(at - last),
            kind,
        });
        last = at;
    }
    out.push(TrackEvent {
        delta: u28::new(end.saturating_sub(last)),
        kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
    });
    out
}

/// A title as a portable file name (no slashes, sharps spelled out).
fn file_name(title: &str) -> String {
    title
        .replace('#', "is")
        .chars()
        .map(|c| if c == '/' || c == ':' { '-' } else { c })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{import_bytes, FileKind};

    #[test]
    fn level_1_order_holds_every_level_1_exercise() {
        let level_1: Vec<usize> = bundled_specs()
            .iter()
            .enumerate()
            .filter(|(_, (level, _))| level.starts_with("1 "))
            .map(|(i, _)| i + 1)
            .collect();
        let mut placed = level_1.clone();
        placed.sort_by_key(|&n| plan_place(n));

        // The order holds exactly the level-1 files; the pulse comes first, scales last.
        let places: Vec<usize> = placed.iter().map(|&n| plan_place(n).0).collect();
        assert_eq!(places, (0..level_1.len()).collect::<Vec<_>>());
        assert_eq!(&placed[..5], &[1, 2, 3, 46, 52]);
        assert_eq!(placed.last(), Some(&8));
    }

    fn spec(kind: ExerciseKind, key: u8) -> ExerciseSpec {
        ExerciseSpec {
            kind,
            key,
            bpm: 90.0,
            slow: false,
            repeats: 1,
            ramp_bpm: 0.0,
            judging: Judging::PitchAndTime,
        }
    }

    fn notes(exercise: &Exercise) -> Vec<u8> {
        let imported = import_bytes(&exercise.midi, FileKind::Midi).expect("imports");
        let track = imported.suggested().expect("bass track");
        let arrangement = imported.arrangement(track).expect("arrangement");
        arrangement.notes.iter().map(|n| n.pitch.0).collect()
    }

    fn arrangement(exercise: &Exercise) -> kontra_model::Arrangement {
        let imported = import_bytes(&exercise.midi, FileKind::Midi).expect("imports");
        imported
            .arrangement(imported.suggested().expect("bass track"))
            .expect("arrangement")
    }

    #[test]
    fn odd_meters_write_their_bars() {
        let e = generate(&spec(
            ExerciseKind::Groove(Progression::Tonic, Rhythm::SevenEight),
            4,
        ));
        let a = arrangement(&e);
        assert_eq!(a.bars.len(), 4);
        for bar in &a.bars {
            assert_eq!((bar.signature.numerator, bar.signature.denominator), (7, 8));
        }
        // 960 model ticks per quarter: a 7/8 bar is 3360 ticks, and every note stays in its bar.
        let bar_ticks = a.bars[0].signature.bar_ticks();
        assert_eq!(bar_ticks, 3360);
        for n in &a.notes {
            let offset = n.start_ticks % bar_ticks;
            assert!(offset + n.duration_ticks <= bar_ticks, "{n:?}");
        }
        assert!(!a.drums.is_empty());
        assert!(e.title.starts_with("Groove 7/8"), "{}", e.title);
    }

    #[test]
    fn slap_and_dead_notes_carry_their_technique() {
        use kontra_model::Technique;
        let slap = arrangement(&generate(&spec(
            ExerciseKind::Groove(Progression::Tonic, Rhythm::Slap),
            4,
        )));
        for (i, n) in slap.notes.iter().enumerate() {
            let want = if i % 2 == 0 {
                Technique::Slap
            } else {
                Technique::Pop
            };
            assert_eq!(n.techniques, [want], "note {i}");
        }
        let dead = arrangement(&generate(&spec(
            ExerciseKind::Groove(Progression::Tonic, Rhythm::DeadNotes),
            9,
        )));
        let muted = dead
            .notes
            .iter()
            .filter(|n| n.techniques.contains(&Technique::Muted))
            .count();
        assert_eq!(muted, dead.notes.len() / 2);
    }

    #[test]
    fn finger_exercise_keeps_four_frets_per_string() {
        let e = generate(&spec(ExerciseKind::Chromatic(Fingers::Crossed), 5));
        assert_eq!(e.title, "Fingerübung 1-3-2-4 ab Bund 5 (90 BPM)");
        let a = arrangement(&e);
        // Up: E A D G with frets 5 7 6 8; without the file's places the tab would use open strings.
        let first: Vec<(Option<u8>, Option<u8>)> =
            a.notes[..8].iter().map(|n| (n.string, n.fret)).collect();
        let want = [
            (0, 5),
            (0, 7),
            (0, 6),
            (0, 8),
            (1, 5),
            (1, 7),
            (1, 6),
            (1, 8),
        ];
        let want: Vec<_> = want.iter().map(|(s, f)| (Some(*s), Some(*f))).collect();
        assert_eq!(first, want);
        // The start fret stays where four frets fit below G3.
        let high = describe(&spec(ExerciseKind::Chromatic(Fingers::Straight), 11));
        assert_eq!(high.fret, Some(MAX_FRET));
    }

    #[test]
    fn tempo_ramp_rises_each_pass() {
        let ramp = ExerciseSpec {
            repeats: 3,
            ramp_bpm: 10.0,
            ..spec(ExerciseKind::Groove(Progression::Tonic, Rhythm::Eighths), 4)
        };
        let e = generate(&ramp);
        assert!(e.title.ends_with("(90-110 BPM)"), "{}", e.title);
        let a = arrangement(&e);
        let bpm: Vec<f64> = a.tempo.changes().iter().map(|c| c.bpm.round()).collect();
        assert_eq!(bpm, [90.0, 100.0, 110.0]);
        assert_eq!(describe(&ramp).end_bpm, Some(110.0));
        assert_eq!(describe(&ramp).goal, "ramp");
    }

    #[test]
    fn scale_in_thirds() {
        let e = generate(&spec(
            ExerciseKind::Sequence(ScaleKind::Major, Sequence::Thirds),
            0,
        ));
        let line = notes(&e);
        // C2 E2, D2 F2, E2 G2 … up, then down from the octave, ending on the root.
        assert_eq!(line[..6], [36, 40, 38, 41, 40, 43]);
        // Down in thirds to the root, which comes once (review #62: twice before).
        assert_eq!(line[line.len() - 4..], [41, 38, 40, 36]);
        assert_eq!(e.title, "Dur-Tonleiter in Terzen in C (90 BPM)");
    }

    #[test]
    fn scale_lines_end_on_a_written_value() {
        // Sixteenths a staff writes as one note: plain or dotted, sixteenth to whole.
        const WRITTEN: [u64; 8] = [1, 2, 3, 4, 6, 8, 12, 16];
        let sixteenth = kontra_model::TICKS_PER_QUARTER / 4;
        let kinds = ScaleKind::ALL.into_iter().flat_map(|s| {
            let sequences = Sequence::ALL.map(|q| ExerciseKind::Sequence(s, q));
            std::iter::once(ExerciseKind::Scale(s)).chain(sequences)
        });
        for kind in kinds {
            for key in 0..12 {
                let a = arrangement(&generate(&spec(kind, key)));
                for n in &a.notes {
                    // The file ends each tone 1/16 of its length early (articulation).
                    let ticks = n.duration_ticks as f64 * 16.0 / 15.0;
                    let value = (ticks / sixteenth as f64).round() as u64;
                    assert!(WRITTEN.contains(&value), "{kind:?} in {key}: {value}/16");
                }
            }
        }
    }

    /// Round 16: a groove over the pop progression names triads (it plays no seventh); a
    /// groove over a jazz progression keeps its seventh chords.
    #[test]
    fn pop_grooves_name_triads() {
        let names = |progression, rhythm| {
            let e = generate(&spec(ExerciseKind::Groove(progression, rhythm), 0));
            let a = arrangement(&e);
            a.markers.iter().map(|m| m.name.clone()).collect::<Vec<_>>()
        };
        assert_eq!(
            names(Progression::Pop, Rhythm::Quarters)[..4],
            ["C", "G", "Am", "F"]
        );
        assert_eq!(
            names(Progression::Turnaround, Rhythm::Quarters)[..4],
            ["Cmaj7", "Am7", "Dm7", "G7"]
        );
    }

    #[test]
    fn triads_are_named_as_triads() {
        let names = |progression, key| {
            let a = arrangement(&generate(&spec(ExerciseKind::Triads(progression), key)));
            a.markers.iter().map(|m| m.name.clone()).collect::<Vec<_>>()
        };
        assert_eq!(
            names(Progression::OneFourFive, 7)[..4],
            ["G", "C", "D", "G"]
        );
        assert_eq!(
            names(Progression::MinorTwoFiveOne, 9)[..4],
            ["Bdim", "E", "Am", "Am"]
        );
        // A closing bar is named as a triad too.
        assert_eq!(names(Progression::Minor, 9).last().unwrap(), "Am");
    }

    #[test]
    fn modes_and_minor_keys_take_their_signature() {
        let dorian = generate(&spec(ExerciseKind::Scale(ScaleKind::Dorian), 2));
        assert_eq!(dorian.title, "Dorisch in D (90 BPM)");
        assert_eq!(arrangement(&dorian).bars[0].key_fifths, 0);
        let minor = generate(&spec(
            ExerciseKind::Groove(Progression::Minor, Rhythm::FiveFour),
            3,
        ));
        assert!(minor.title.contains("in Ebm"), "{}", minor.title);
        assert_eq!(arrangement(&minor).bars[0].key_fifths, -6);
        let d = describe(&spec(
            ExerciseKind::Arpeggio(Progression::MinorTwoFiveOne),
            7,
        ));
        assert_eq!((d.key, d.minor), ("G", true));
    }

    #[test]
    fn text_keys_round_trip() {
        for s in ScaleKind::ALL {
            assert_eq!(ScaleKind::from_key(s.key()), Some(s));
        }
        for p in Progression::ALL {
            assert_eq!(Progression::from_key(p.key()), Some(p));
        }
        for r in Rhythm::ALL {
            assert_eq!(Rhythm::from_key(r.key()), Some(r));
        }
        for q in Sequence::ALL {
            assert_eq!(Sequence::from_key(q.key()), Some(q));
        }
        for f in Fingers::ALL {
            assert_eq!(Fingers::from_key(f.key()), Some(f));
        }
        assert_eq!(Rhythm::from_key("nope"), None);
    }

    #[test]
    fn every_bundled_exercise_fills_whole_bars() {
        for (_, file, e) in bundled() {
            let a = arrangement(&e);
            let last = a.bars.last().expect("bars");
            let end = last.start_ticks + last.signature.bar_ticks();
            assert!(a.notes.iter().all(|n| n.string.is_some()), "{file}: tab");
            let played = a
                .notes
                .iter()
                .map(|n| n.start_ticks + n.duration_ticks)
                .max();
            // The last note ends in the last bar: no empty bar, no note past the end.
            assert!(
                played.is_some_and(|p| p > last.start_ticks && p <= end),
                "{file}"
            );
        }
    }

    #[test]
    fn g_major_scale_goes_up_and_down() {
        let e = generate(&ExerciseSpec {
            slow: true,
            ..spec(ExerciseKind::Scale(ScaleKind::Major), 7)
        });
        assert_eq!(
            notes(&e),
            [31, 33, 35, 36, 38, 40, 42, 43, 42, 40, 38, 36, 35, 33, 31]
        );
        assert!(e.title.contains("Dur-Tonleiter in G"), "{}", e.title);
    }

    #[test]
    fn every_note_is_playable_on_four_strings() {
        for (_, _, e) in bundled() {
            for key in notes(&e) {
                assert!((28..=55).contains(&key), "{}: {key}", e.title);
            }
        }
    }

    #[test]
    fn walking_bass_approaches_the_next_root() {
        let e = generate(&spec(ExerciseKind::WalkingBass(Progression::TwoFiveOne), 0));
        let line = notes(&e);
        assert_eq!(line.len(), 16);
        // Beat four of each bar is a semitone from the next bar's root.
        for bar in 0..3 {
            let approach = i32::from(line[bar * 4 + 3]);
            let next = i32::from(line[(bar + 1) * 4]);
            assert_eq!((approach - next).abs(), 1, "bar {bar}: {line:?}");
        }
    }

    #[test]
    fn walking_approach_on_the_low_e_comes_from_above() {
        // One chord in E: the next root is E1, the lowest note; below it there is no fret.
        let line = notes(&generate(&spec(
            ExerciseKind::WalkingBass(Progression::Tonic),
            4,
        )));
        for bar in 0..3 {
            let approach = i32::from(line[bar * 4 + 3]);
            let next = i32::from(line[(bar + 1) * 4]);
            assert_eq!((approach - next).abs(), 1, "bar {bar}: {line:?}");
        }
    }

    #[test]
    fn octave_groove_keeps_the_octave_on_high_roots() {
        // E♭ blues: the IV root A♭2 lies high; its octave must not fall back onto the root.
        let e = generate(&spec(
            ExerciseKind::Groove(Progression::Blues, Rhythm::Octaves),
            3,
        ));
        let line = notes(&e);
        for pair in line.as_chunks::<2>().0 {
            assert_eq!(i32::from(pair[1]) - i32::from(pair[0]), 12, "{line:?}");
        }
    }

    #[test]
    fn progressions_end_on_the_tonic() {
        use ExerciseKind::*;
        // A blues or minor cadence ends on the dominant; a last bar resolves it.
        for kind in [
            Groove(Progression::Blues, Rhythm::Quarters),
            Groove(Progression::Minor, Rhythm::Eighths),
            WalkingBass(Progression::Blues),
            Arpeggio(Progression::Minor),
        ] {
            let e = generate(&spec(kind, 9));
            let imported = import_bytes(&e.midi, FileKind::Midi).unwrap();
            let a = imported.arrangement(imported.suggested().unwrap()).unwrap();
            let last = a.notes.last().unwrap();
            assert_eq!(last.pitch.0 % 12, 9, "{}", e.title);
            let marker = a.markers.last().unwrap();
            assert!(marker.name.starts_with('A'), "{}: {}", e.title, marker.name);
        }
    }

    #[test]
    fn chord_names_become_bar_markers() {
        let e = generate(&spec(ExerciseKind::Arpeggio(Progression::TwoFiveOne), 5));
        let imported = import_bytes(&e.midi, FileKind::Midi).unwrap();
        let a = imported.arrangement(imported.suggested().unwrap()).unwrap();
        let names: Vec<&str> = a.markers.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, ["Gm7", "C7", "Fmaj7", "Fmaj7"]);
        assert_eq!(a.bars[0].key_fifths, -1);
    }

    #[test]
    fn bundled_set_has_nine_levels() {
        let set = bundled();
        assert!(set.len() >= 140, "{}", set.len());
        // Appended exercises go into the earlier levels too: count the folders, not the runs.
        let levels: std::collections::BTreeSet<&str> = set.iter().map(|s| s.0).collect();
        assert_eq!(levels.len(), 9, "{levels:?}");
        let files: std::collections::BTreeSet<&str> = set.iter().map(|s| s.1.as_str()).collect();
        assert_eq!(files.len(), set.len(), "file names are unique");
        // LH-LRN-01 asks for twenty groove exercises.
        let grooves = set
            .iter()
            .filter(|s| s.2.title.starts_with("Groove"))
            .count();
        assert!(grooves >= 20, "{grooves}");
    }

    #[test]
    fn levels_hold_their_kind() {
        // Runde 12 swapped the names of levels 4 and 5, not the exercises in them.
        for (level, spec) in bundled_specs() {
            let walking = matches!(spec.kind, ExerciseKind::WalkingBass(_));
            match level {
                "4 Groove" => assert!(matches!(spec.kind, ExerciseKind::Groove(..)), "{spec:?}"),
                "5 Walking Bass" => assert!(walking, "{spec:?}"),
                _ => {}
            }
        }
    }

    #[test]
    fn time_only_survives_the_file() {
        // Open-string grooves judge only the attack (DIDAKTIK I4); the file carries that.
        let timing = ExerciseSpec {
            judging: Judging::TimeOnly,
            ..spec(
                ExerciseKind::Groove(Progression::Tonic, Rhythm::Quarters),
                4,
            )
        };
        assert_eq!(arrangement(&generate(&timing)).judging, Judging::TimeOnly);
        // Its own title, so it does not overwrite or share progress with the judged one.
        assert_eq!(
            generate(&timing).title,
            "Groove Viertel auf einem Akkord in E (90 BPM), nur Timing"
        );

        let normal = spec(
            ExerciseKind::Groove(Progression::Tonic, Rhythm::Quarters),
            4,
        );
        assert_eq!(
            arrangement(&generate(&normal)).judging,
            Judging::PitchAndTime
        );

        let open = bundled_specs()
            .iter()
            .filter(|s| s.1.judging == Judging::TimeOnly)
            .count();
        assert_eq!(open, 5, "the open-string grooves of level 1");
    }

    /// A piece goes through the normal import: bars with chord names, drums, a title with its
    /// style; the funk piece is in A minor.
    #[test]
    fn pieces_import_with_chords_and_drums() {
        for style in Style::ALL {
            let e = generate(&spec(ExerciseKind::Song(style, Level::Medium), 9));
            let a = arrangement(&e);
            assert!(a.bars.len() >= 40, "{style:?}: {} bars", a.bars.len());
            assert!(!a.drums.is_empty(), "{style:?}");
            assert!(
                a.markers.len() >= a.bars.len(),
                "{style:?}: a chord name per bar"
            );
            assert!(e.title.starts_with("Stück"), "{}", e.title);

            // One closing bar on the tonic, with drums under it.
            let last = a.bars.last().unwrap();
            let closing = a.notes.iter().filter(|n| n.start_ticks >= last.start_ticks);
            assert_eq!(closing.count(), 1, "{style:?}");
            assert!(
                a.drums.iter().any(|d| d.tick >= last.start_ticks),
                "{style:?}"
            );
        }

        // The bundled pieces at their tempos last two to three minutes (LH-BIB-12).
        let set = bundled();
        let pieces: Vec<_> = set
            .iter()
            .filter(|s| s.2.title.starts_with("Stück"))
            .collect();
        assert_eq!(pieces.len(), 6);
        for (_, _, piece) in pieces {
            let a = arrangement(piece);
            let seconds = a.tempo.seconds_at(a.bars.last().unwrap().end_ticks());
            assert!(
                (120.0..=180.0).contains(&seconds),
                "{}: {seconds:.0} s",
                piece.title
            );
        }
        // Repeats do not apply to a piece: one form, drums to the end.
        let twice = ExerciseSpec {
            repeats: 2,
            ..spec(ExerciseKind::Song(Style::Rock, Level::Medium), 9)
        };
        assert_eq!(
            generate(&twice).midi,
            generate(&spec(ExerciseKind::Song(Style::Rock, Level::Medium), 9)).midi
        );

        let funk = generate(&spec(ExerciseKind::Song(Style::Funk, Level::Medium), 9));
        assert_eq!(funk.title, "Stück Funk in Am (90 BPM)");
        let d = describe(&spec(ExerciseKind::Song(Style::Funk, Level::Medium), 9));
        assert_eq!(
            (d.kind, d.variant, d.minor, d.goal),
            ("song", "funk", true, "form")
        );
        assert_eq!(Style::from_key("shuffle"), Some(Style::Shuffle));
    }

    #[test]
    fn shuffle_swings_the_eighths() {
        let e = generate(&spec(
            ExerciseKind::Groove(Progression::Tonic, Rhythm::Shuffle),
            4,
        ));
        let imported = import_bytes(&e.midi, FileKind::Midi).unwrap();
        let a = imported.arrangement(imported.suggested().unwrap()).unwrap();
        // 960 ticks per quarter in the model: the off-beat lands on the triplet (640).
        assert_eq!(a.notes[1].start_ticks, 640);
    }

    #[test]
    fn grooves_bring_a_drum_part() {
        use kontra_model::DrumKind;
        let groove = |rhythm| {
            let e = generate(&spec(ExerciseKind::Groove(Progression::Tonic, rhythm), 4));
            let imported = import_bytes(&e.midi, FileKind::Midi).unwrap();
            imported.arrangement(imported.suggested().unwrap()).unwrap()
        };
        let rock = groove(Rhythm::Eighths);
        // The drum track is not offered as the bass and does not add notes to play.
        assert_eq!(rock.notes.len(), 32);
        let kind_at = |a: &kontra_model::Arrangement, tick, kind| {
            a.drums.iter().any(|d| d.tick == tick && d.kind == kind)
        };
        assert!(kind_at(&rock, 0, DrumKind::Kick));
        assert!(kind_at(&rock, 960, DrumKind::Snare));
        assert!(kind_at(&rock, 480, DrumKind::HiHat));
        // Shuffle: the hi-hat swings onto the triplet.
        let shuffle = groove(Rhythm::Shuffle);
        assert!(kind_at(&shuffle, 640, DrumKind::HiHat));
        assert!(!kind_at(&shuffle, 480, DrumKind::HiHat));
        // Scales stay without drums.
        let scale = generate(&spec(ExerciseKind::Scale(ScaleKind::Major), 0));
        let imported = import_bytes(&scale.midi, FileKind::Midi).unwrap();
        let a = imported.arrangement(imported.suggested().unwrap()).unwrap();
        assert!(a.drums.is_empty());
    }

    #[test]
    fn ghost_groove_mutes_the_offbeat_sixteenths() {
        use kontra_model::Technique;
        let e = generate(&spec(
            ExerciseKind::Groove(Progression::Tonic, Rhythm::Ghost),
            4,
        ));
        let imported = import_bytes(&e.midi, FileKind::Midi).unwrap();
        let a = imported.arrangement(imported.suggested().unwrap()).unwrap();
        assert_eq!(a.notes.len(), 64);
        // 240 model ticks per sixteenth: "e" and "a" are odd sixteenths.
        for n in &a.notes {
            let offbeat = (n.start_ticks / 240) % 2 == 1;
            let ghost = n.techniques.contains(&Technique::Ghost);
            assert_eq!(ghost, offbeat, "tick {}", n.start_ticks);
            assert_eq!(n.pitch.0, 28, "root E1 at tick {}", n.start_ticks);
        }
        assert!(!a.drums.is_empty());
        // Other grooves stay without ghost notes.
        let plain = generate(&spec(
            ExerciseKind::Groove(Progression::Tonic, Rhythm::Sixteenths),
            4,
        ));
        let imported = import_bytes(&plain.midi, FileKind::Midi).unwrap();
        let a = imported.arrangement(imported.suggested().unwrap()).unwrap();
        assert!(a.notes.iter().all(|n| n.techniques.is_empty()));
    }
}
