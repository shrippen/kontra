//! Audio engine. Owns the real-time threads and turns audio into events.
//!
//! ```text
//! device / file ─► input callback ─► ring ─► analysis thread ─┬─ NoteTracker ─► Event::Note
//!                                                             ├─ Tuner ───────► Event::Tuner
//!                                                             └─ level ───────► Event::Level
//! control ─► command ring ─► output callback (clicks, reference tones) ─► device
//! ```
//!
//! All times are seconds on one engine clock that starts when the `Clock` is created.

mod clock;
mod input;
mod offline;
mod player;

pub use clock::Clock;
pub use input::{Input, InputKind, Pace};
pub use offline::analyze;
pub use player::{Accent, Player, Sound};

use kontra_dsp::TunerReading;
use kontra_model::DetectedNote;

/// What the engine reports to the layer above.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Note(DetectedNote),
    /// Input level over the last 50 ms.
    Level {
        peak_db: f32,
        rms_db: f32,
        clipped: bool,
    },
    Tuner(TunerReading),
    /// Device problem, e.g. the interface was unplugged.
    Error(String),
    /// File input reached its end.
    Ended,
}

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error(transparent)]
    Audio(#[from] kontra_drivers::AudioError),
    #[error(transparent)]
    File(#[from] kontra_drivers::FileError),
    #[error("cannot start the analysis thread: {0}")]
    Thread(std::io::Error),
}
