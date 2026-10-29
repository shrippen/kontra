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

mod backing_cache;
mod cache_files;
mod calibration;
mod chain;
mod check;
mod courses;
mod density;
mod expected;
mod learning;
mod live;
mod matcher;
mod pocket;
mod practice;
mod progress;
mod service;
mod session;
mod settings;
mod songsterr;
mod sources;
mod stems;
mod summary;
mod technique;
mod transport;

pub use backing_cache::{BackingCache, Preparing};
pub use cache_files::CacheEntry;
pub use calibration::{
    CalibrationResult, CLICKS as CALIBRATION_CLICKS, INTERVAL_S as CALIBRATION_INTERVAL_S,
};
pub use chain::{Blocks, Chain, ChainOrder};
pub use check::{
    check_takes, loopback_length_s, loopback_plan, round_trip, CheckNote, CheckTake, LiveTake,
    RoundTrip, Take, TakeSound, CHECK_BPM,
};
pub use courses::{CourseError, CourseFetch, CourseStore};
pub use density::Density;
pub use expected::{expected_notes, ExpectedNote};
pub use kontra_engine::Separator;
pub use kontra_engine::{FileHttp, Http, HttpError, Web};
pub use kontra_engine::{Gains, SeparatorError, Shaping, SHAPING_RANGE_DB};
pub use learning::{
    lesson, place, positions, Answer, LearnState, LearnStore, Learning, QuestionRef, UnitState,
};
pub use live::{
    is_midi_file, soak, InputChoice, Live, LiveCalibration, LiveEar, LiveOutput, LiveSetup,
    LiveTuner, PromptTone, SoakReport,
};
pub use matcher::{NoteResult, Outcome};
pub use pocket::{Pocket, PocketVerdict};
pub use practice::{Practice, PracticeEvent, PracticeOptions};
pub use progress::{run_from, ProgressError, ProgressStore, Run, SongProgress};
pub use service::*;
pub use session::{Mode, Session, SessionEvent, Workshop};
pub use settings::{ScoreSettings, Strictness};
pub use songsterr::{BassTrack, SearchError, SearchStart, SongHit, SongSearch};
pub use sources::{download_dir, recent_scores, search_url, SongSite};
pub use stems::{BassRemoval, StemJob};
pub use summary::{
    Intonation, IntonationPart, Leaning, Observation, OutsideNote, Side, StringTally, Summary,
    TimingHistogram,
};
pub use technique::{
    technique_signs, technique_targets, told_techniques, TechniqueOutcome, TechniqueResult,
    TechniqueTally, TechniqueTarget,
};
pub use transport::{LoopRange, Transport};
