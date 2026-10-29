//! Audio engine. Owns the real-time threads and turns audio into events.
//!
//! ```text
//! device / file ─► input callback ─► ring ─► analysis thread ─┬─ NoteTracker ─► Event::Note
//!                                                             ├─ Tuner ───────► Event::Tuner
//!                                                             ├─ level ───────► Event::Level
//!                                                             ├─ heard until ─► Event::Heard
//!                                                             └─ watch ───────► Event::Stalled/Resumed
//! MIDI keyboard / .mid ─► note-on (exact, no analysis) ─────────────────────────► Event::Note
//! control ─► command ring ─► output callback (clicks, reference tones) ─► device
//! ```
//!
//! All times are seconds on one engine clock that starts when the `Clock` is created.

mod backing;
mod clock;
mod input;
mod korpus;
mod offline;
mod player;

pub use backing::{BackingTrack, Resolution};
pub use clock::Clock;
pub use input::{Input, InputKind, Pace, Recording};
pub use kontra_drivers::{
    audio_priority, download_dir, find_separator, in_sandbox, list_devices, list_midi_inputs,
    read_audio, recent_files, write_wav, AudioBuffer, Connection, DeviceInfo, Direction, FileHttp,
    Http, HttpError, MidiPort, Priority, RecentFile, SeparationJob, Separator, SeparatorError,
    StreamConfig, Web, NULL_DEVICE,
};
pub use kontra_dsp::{track_beats, BeatGrid, Shaping, SHAPING_RANGE_DB};
pub use korpus::{
    band, evaluate, evaluate_laptop_jack, evaluate_with, generate, render_performance, same_pitch,
    Annotation, Evaluation, KorpusItem, KorpusSet, Matching, Mistakes, Playing,
};
pub use offline::{analyze, analyze_without, revise, revise_pitch};
pub use player::{Accent, BackingAudio, Gains, NullOutput, Output, Player, Sound};

use kontra_dsp::TunerReading;
use kontra_model::{DetectedNote, PitchRevision, TechniqueRevision};

/// What the engine reports to the layer above.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Note(DetectedNote),
    /// A technique told after its note (the attack needed more audio, LH-BEW-06).
    Technique(TechniqueRevision),
    /// A pitch told after its note, once the tone has settled (decision round 11).
    Pitch(PitchRevision),
    /// Input level over the last 50 ms.
    Level {
        peak_db: f32,
        rms_db: f32,
        clipped: bool,
    },
    Tuner(TunerReading),
    /// Device problem, e.g. the interface was unplugged.
    Error(String),
    /// The analysis has heard the input up to `at_s`; under load it runs behind the clock,
    /// and a note may still come for any time after it.
    Heard {
        at_s: f64,
    },
    /// The device stopped delivering (LH-EIN-15): heard up to `at_s`, nothing since for 250 ms.
    Stalled {
        at_s: f64,
    },
    /// The device delivers again from `at_s`; the time since `Stalled` was not heard.
    Resumed {
        at_s: f64,
    },
    /// File input reached its end.
    Ended,
}

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error(transparent)]
    Audio(#[from] kontra_drivers::AudioError),
    #[error(transparent)]
    File(#[from] kontra_drivers::FileError),
    #[error(transparent)]
    Midi(#[from] kontra_drivers::MidiError),
    #[error("cannot start the analysis thread: {0}")]
    Thread(std::io::Error),
}
