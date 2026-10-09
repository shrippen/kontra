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

mod biquad;
mod intonation;
mod level;
mod onset;
mod resample;
mod tracker;
mod tuner;
mod yin;

pub mod synth;

pub use biquad::{Biquad, FilterKind};
pub use intonation::refine_hz;
pub use level::{amplitude_to_db, peak_db, rms_db, SILENCE_DB};
pub use onset::{Onset, OnsetConfig, OnsetDetector};
pub use resample::resample;
pub use tracker::{NoteTracker, TrackerConfig};
pub use tuner::{Tuner, TunerReading};
pub use yin::{PitchEstimate, Yin};
