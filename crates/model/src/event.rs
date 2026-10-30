//! Notes detected from an input, the common currency between engine and session.

use serde::{Deserialize, Serialize};

/// Where a detected note came from. Scoring tolerances depend on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputSource {
    /// Line or Hi-Z input: clean signal.
    Line,
    /// Laptop combo jack: weak, noisy signal.
    LaptopJack,
    /// Microphone: room, noise, backing track bleed (phase 4).
    Microphone,
    /// MIDI note-on (phase 4): exact.
    Midi,
}

/// One played note as detected by the engine.
///
/// `onset_s` is final when the event is created; `midi` arrives with it because the engine waits
/// until the pitch is stable (see concept 6.3), so the event is emitted 40 to 80 ms after the onset.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DetectedNote {
    /// Onset on the session clock, latency already removed.
    pub onset_s: f64,
    /// Fractional MIDI note number, e.g. 28.12 for E1 twelve cents sharp.
    pub midi: f64,
    /// 0..=1, how sure the pitch estimate is.
    pub confidence: f32,
    /// Peak level in dBFS around the onset.
    pub level_db: f32,
    pub source: InputSource,
    /// How it was played, if the tracker recognised a technique (LH-BEW-06); `None`: a plain
    /// pluck or not known.
    #[serde(default)]
    pub technique: Option<crate::Technique>,
    /// The pitch is that of a steady tone heard before the attack (mains hum not yet removed,
    /// the note before still ringing): a click on hum, or the same note struck again. Judged
    /// only where a note is due (LH-NF-16).
    #[serde(default)]
    pub over_background: bool,
}

impl DetectedNote {
    /// An attack without a pitch (`midi` 0): a dead note, or a click or scrape on the strings.
    pub fn is_pitchless(&self) -> bool {
        self.midi <= 0.0
    }

    /// Whether it is a note of its own off the notes: a pitch, not just the background's, and
    /// not a dead note (a muted attack's pitch is noise: a hand on the strings, E8).
    pub fn stands_out(&self) -> bool {
        !self.is_pitchless()
            && !self.over_background
            && self.technique != Some(crate::Technique::Muted)
    }
}

/// A technique told after its note was emitted (LH-BEW-06): judging an attack needs about 110 ms
/// of it, more than the note waits for its pitch. Matches the note by its `onset_s`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TechniqueRevision {
    pub onset_s: f64,
    pub technique: crate::Technique,
}

/// A pitch told after its note was emitted, once the tone has settled (decision round 11): a
/// note may start up to a semitone flat, or with a component an octave below, and reach its
/// pitch within about 100 ms; the note goes out at once and is corrected after. Matches the
/// note by its `onset_s`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PitchRevision {
    pub onset_s: f64,
    /// Fractional MIDI note number of the settled tone.
    pub midi: f64,
}

/// A note taken back after it was emitted (R1): it went out early on a short window, and what
/// came after shows it was no note of its own, e.g. the fretting finger tapping the string just
/// before a much louder pluck. Matches the note by its `onset_s`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NoteRetraction {
    pub onset_s: f64,
}

/// A tone of an `ExpectedStrike`: the pitch, and the string the tab puts it on, where known.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ExpectedTone {
    pub midi: u8,
    /// String index, 0 = lowest (as `Note::string`).
    pub string: Option<u8>,
}

/// A strike the tab expects, told to the pitch detection ahead of time (R2). The detector may
/// report an expected pitch sooner when its own estimate agrees; it never decides the pitch.
/// A double stop has two tones. `onset_s` is on the receiver's clock: engine time for the
/// engine, seconds from the first sample for the tracker (as `DetectedNote::onset_s`).
///
/// ```text
///            onset_s
///   ├─ window_s ─┼─ window_s ─┤   an attack in here may be this strike
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExpectedStrike {
    pub onset_s: f64,
    /// Half width of the time window around `onset_s`.
    pub window_s: f64,
    tones: [ExpectedTone; ExpectedStrike::MAX_TONES],
    len: u8,
}

impl ExpectedStrike {
    /// One bass strikes at most six strings at once.
    pub const MAX_TONES: usize = 6;

    pub fn new(onset_s: f64, window_s: f64) -> ExpectedStrike {
        ExpectedStrike {
            onset_s,
            window_s,
            tones: [ExpectedTone::default(); ExpectedStrike::MAX_TONES],
            len: 0,
        }
    }

    /// Adds a tone; beyond `MAX_TONES` it is left out.
    pub fn with(mut self, tone: ExpectedTone) -> ExpectedStrike {
        if let Some(slot) = self.tones.get_mut(usize::from(self.len)) {
            *slot = tone;
            self.len += 1;
        }
        self
    }

    pub fn tones(&self) -> &[ExpectedTone] {
        &self.tones[..usize::from(self.len)]
    }
}

/// Which string a struck note was heard on, told about 280 ms after its attack (R7,
/// LH-BEW-16): the string's stiffness bends the partials sharp, more on a thick string and
/// on a short one (high fret). Only for a strike the tab puts on a string, and only feedback:
/// a note on another string still counts as played. Matches the note by its `onset_s`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StringVerdict {
    pub onset_s: f64,
    pub heard: StringHeard,
    /// The measured inharmonicity, log10 B (e.g. −3.6 on an open E string); `None` when
    /// nothing was measured.
    pub log_b: Option<f64>,
    /// Partials found for it: the more, the surer.
    pub partials: u8,
}

/// The string a note was heard on, against the one the tab expects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StringHeard {
    Expected,
    /// Another string (index, 0 = lowest, as `Note::string`).
    Other(u8),
    /// No statement, and why.
    Unclear(Unclear),
}

/// Why a string could not be told.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Unclear {
    /// The next attack came too soon: too little of the tone to measure.
    ShortNote,
    /// Muted (dead note, palm mute): the partials die before they can be measured.
    Muted,
    /// Too few partials stood out of the noise.
    FewPartials,
    /// The profile has no string values for this (fretless, a string not measured yet such as
    /// a low B, another tuning).
    NotSupported,
}

/// Which tones of a double stop the input heard (R8, LH-BEW-11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DoubleStopHeard {
    Both,
    /// Only the lower tone; the upper was not played or did not sound.
    Lower,
    Upper,
    /// No statement: an octave (its partials lie on the lower tone's), or neither tone clear.
    Unknown,
}

/// What a double stop the tab expects sounded like, told about 160 ms after its attack (R8).
/// Matches the note by its `onset_s`, as `TechniqueRevision`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DoubleStopVerdict {
    pub onset_s: f64,
    pub heard: DoubleStopHeard,
}

/// Which simultaneous tones an input can tell apart (LH-BEW-11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChordHearing {
    /// Every tone is its own event (MIDI).
    Every,
    /// Two-tone double stops the tab announces, not octaves (audio, R8). Three tones or more,
    /// and anything not announced, are heard as one note.
    AnnouncedDoubleStops,
    /// One tone per attack: a double stop is judged like a chord. A microphone until recordings
    /// exist (Runde 17: 12.2 % false alarms on FiloBass).
    OneTone,
}

impl InputSource {
    pub fn chord_hearing(self) -> ChordHearing {
        match self {
            InputSource::Midi => ChordHearing::Every,
            InputSource::Microphone => ChordHearing::OneTone,
            _ => ChordHearing::AnnouncedDoubleStops,
        }
    }
}
