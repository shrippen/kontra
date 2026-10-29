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
    /// The pitch is that of a steady tone heard before the attack (mains hum, the note before
    /// still ringing): a click on hum, or the same note struck again. Judged only where a note
    /// is due (LH-NF-16).
    #[serde(default)]
    pub over_background: bool,
}

impl DetectedNote {
    /// An attack without a pitch (`midi` 0): a dead note, or a click or scrape on the strings.
    pub fn is_pitchless(&self) -> bool {
        self.midi <= 0.0
    }

    /// Whether it is a note of its own off the notes: a pitch, and not just the background's.
    pub fn stands_out(&self) -> bool {
        !self.is_pitchless() && !self.over_background
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
