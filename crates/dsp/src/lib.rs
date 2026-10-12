//! Signal processing building blocks. Everything here is deterministic and free of I/O,
//! so it runs the same in the audio engine, the CLI and the tests.
//!
//! ```text
//! samples ─┬─► OnsetDetector ──onset──┐
//!          │                          ▼
//!          └─────────────────────► NoteTracker ──► DetectedNote
//!                                     │ (waits until Yin agrees twice)
//!                                     ▼
//!                                    Yin
//! ```

mod beats;
mod biquad;
mod echo;
mod intonation;
mod level;
mod onset;
mod pitch;
mod resample;
mod shaping;
mod strike;
mod tracker;
mod tuner;
mod yin;

pub mod synth;

pub use beats::{track_beats, BeatGrid};
pub use biquad::{Biquad, EqKind, FilterKind};
pub use echo::{EchoCanceller, ECHO_BLOCK};
pub use intonation::refine_hz;
pub use level::{amplitude_to_db, peak_db, rms_db, SILENCE_DB};
pub use onset::{Onset, OnsetConfig, OnsetDetector};
pub use pitch::{PitchShifter, LATENCY as PITCH_LATENCY, PITCH_RANGE_SEMITONES};
pub use resample::resample;
pub use shaping::{Shaper, Shaping, SHAPING_RANGE_DB};
pub use tracker::{NoteTracker, TrackerConfig};
pub use tuner::{Tuner, TunerReading};
pub use yin::{PitchEstimate, Yin};
