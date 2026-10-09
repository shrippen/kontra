//! Practice session logic, independent of audio and UI.
//!
//! ```text
//!            DetectedNote (engine clock, s)
//!                     │
//! Transport ── position_at(real time) ──► Matcher ──► NoteResult ──► Summary / Progress
//!   ▲  speed, loop, wait                     │
//!   └──────────── Mode (Play, Song, Wait, Workshop) decides stops and speed steps
//! ```
//!
//! Two clocks: *real time* (seconds on the engine's audio clock) and *position* (seconds of the
//! arrangement at original tempo). At speed 0.5 the position advances half as fast.

mod expected;
mod matcher;
mod practice;
mod progress;
mod service;
mod session;
mod settings;
mod summary;
mod transport;

pub use expected::{expected_notes, ExpectedNote};
pub use matcher::{NoteResult, Outcome};
pub use practice::{Practice, PracticeEvent, PracticeOptions};
pub use progress::{run_from, ProgressError, ProgressStore, Run, SongProgress};
pub use service::*;
pub use session::{Mode, Session, SessionEvent, Workshop};
pub use settings::{ScoreSettings, Strictness};
pub use summary::{Summary, TimingHistogram};
pub use transport::{LoopRange, Transport};
