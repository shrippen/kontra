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
}
