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
mod device_clock;
mod input;
mod korpus;
mod offline;
mod player;
mod strings_korpus;

pub use backing::{BackingTrack, Resolution};
pub use clock::Clock;
pub use input::{Input, InputKind, Pace, Recording};
pub use kontra_drivers::{
    audio_priority, download_dir, find_separator, in_sandbox, list_devices, list_midi_inputs,
    read_audio, recent_files, write_wav, AudioBuffer, Connection, DeviceInfo, Direction, FileHttp,
    Http, HttpError, MidiPort, Priority, RecentFile, SeparationJob, Separator, SeparatorError,
    StreamConfig, Web, NULL_DEVICE,
};
pub use kontra_dsp::{
    track_beats, BeatGrid, Pick, Shaping, StringModel, LOG_B_PER_FRET, SHAPING_RANGE_DB,
};
pub use korpus::{
    announce_second_tone, band, double_stop_mixtures, evaluate, evaluate_double_stops,
    evaluate_expecting, evaluate_laptop_jack, evaluate_with, generate, generate_double_stops,
    render_performance, same_pitch, Annotation, DoubleStopPlayed, DoubleStopSet, DoubleStopStrike,
    DoubleStopTally, Evaluation, Expecting, KorpusItem, KorpusSet, Matching, Mistakes, Playing,
    ReportLimits, DOUBLE_STOP_INTERVALS,
};
pub use offline::{analyze, analyze_without, retract, revise, revise_pitch};
pub use player::{Accent, BackingAudio, Gains, NullOutput, Output, Player, Sound};
pub use strings_korpus::{tell_strings, StringTold};

use kontra_dsp::TunerReading;
use kontra_model::{
    DetectedNote, DoubleStopVerdict, NoteRetraction, PitchRevision, StringVerdict,
    TechniqueRevision,
};

/// What the engine reports to the layer above.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Note(DetectedNote),
    /// A technique told after its note (the attack needed more audio, LH-BEW-06).
    Technique(TechniqueRevision),
    /// A pitch told after its note, once the tone has settled (decision round 11).
    Pitch(PitchRevision),
    /// The string a note was heard on, told after it (R7).
    String(StringVerdict),
    /// A note taken back: told early, it proved to be no note of its own (R1).
    Retracted(NoteRetraction),
    /// Which tones of a double stop the tab announced sounded, about 160 ms after its note
    /// (R8).
    DoubleStop(DoubleStopVerdict),
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
    /// The device delivers again from `at_s`; it skipped the time since `Stalled`. Told before
    /// the silence (up to 0.5 s) the engine then feeds and reports heard inside that time.
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
