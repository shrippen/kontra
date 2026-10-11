//! Scoring tolerances (LH-BEW-02, LH-BEW-05).

use kontra_model::{InputSource, InstrumentKind};
use serde::{Deserialize, Serialize};

/// Pitch limit at a microphone, whatever the instrument.
pub const MICROPHONE_PITCH_CENTS: f64 = 50.0;

/// How strict the timing window is.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Strictness {
    Loose,
    Normal,
    Strict,
    Custom { window_ms: f64 },
}

impl Strictness {
    /// Half width of the timing window in real milliseconds.
    pub fn window_ms(self) -> f64 {
        match self {
            Strictness::Loose => 90.0,
            Strictness::Normal => 60.0,
            Strictness::Strict => 25.0,
            Strictness::Custom { window_ms } => window_ms.clamp(5.0, 250.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ScoreSettings {
    pub strictness: Strictness,
    /// Allowed pitch deviation in cents for a hit.
    pub pitch_cents: f64,
    /// Measured input latency, subtracted from every detected onset (LH-EIN-05).
    pub latency_ms: f64,
}

impl ScoreSettings {
    /// Defaults for an instrument kind and input. Fretted basses only need the right semitone;
    /// fretless intonation is scored (±30 cents); microphones get wider windows and pitch limits
    /// (LH-EIN-11): the room's reverb pulls each estimate towards the note before (corpus
    /// "room-double": p95 44 ct on notes played in tune).
    pub fn defaults(kind: InstrumentKind, source: InputSource) -> ScoreSettings {
        let pitch_cents = match (kind, source) {
            (_, InputSource::Microphone) => MICROPHONE_PITCH_CENTS,
            (InstrumentKind::Fretless | InstrumentKind::Double, _) => 30.0,
            (InstrumentKind::Fretted | InstrumentKind::Keys, _) => 50.0,
        };
        let strictness = match source {
            InputSource::Microphone => Strictness::Loose,
            InputSource::Line | InputSource::LaptopJack | InputSource::Midi => Strictness::Normal,
        };
        ScoreSettings {
            strictness,
            pitch_cents,
            latency_ms: 0.0,
        }
    }
}

impl Default for ScoreSettings {
    fn default() -> Self {
        ScoreSettings::defaults(InstrumentKind::Fretted, InputSource::Line)
    }
}
