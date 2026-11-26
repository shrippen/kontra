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
mod band;
mod cache_files;
mod calibration;
mod call;
mod chain;
mod check;
mod choice;
mod courses;
mod density;
mod expected;
pub mod external;
mod improv;
mod learning;
mod live;
mod llm;
mod loopback;
mod matcher;
mod monitor;
pub mod plugin_host;
mod pocket;
mod practice;
mod progress;
mod rig;
mod service;
mod session;
mod settings;
mod songsterr;
mod sources;
mod spacing;
mod stems;
mod summary;
mod technique;
mod transport;
mod update;

pub use backing_cache::{BackingCache, Preparing};
pub use band::Band;
pub use cache_files::CacheEntry;
pub use calibration::CalibrationResult;
pub use call::call_and_response;
pub use chain::{Blocks, Chain, ChainOrder};
pub use check::{
    check_takes, loopback_length_s, loopback_plan, round_trip, tone_delays_s, CheckTake, LiveTake,
    Take, TakeSound, CHECK_BPM, LOOPBACK_TONES,
};
pub use choice::{or_default, Choice};
pub use courses::{CourseError, CourseFetch, CourseStore};
pub use density::Density;

pub use improv::{fit, Improvisation};
pub use kontra_engine::{
    describe_plugin_later, PendingSpec, PluginBank, PluginProblems, PluginRack,
};
pub use kontra_engine::{group_of_effect, group_of_plugin, PluginGroup};
pub use kontra_engine::{EngineError, Http, HttpError};
pub use kontra_engine::{Gains, Limit, SeparatorError, Shaping, SHAPING_RANGE_DB};
/// Where an LLM key lives (LH-BIB-16): appcore's layer only, never session's own logic.
pub use kontra_engine::{MemoryStore, SecretError, SecretStore};
pub use learning::{
    lesson, place, positions, Answer, LearnState, LearnStore, Learning, QuestionRef,
};
pub use live::{
    is_midi_file, soak, InputChoice, Live, LiveCalibration, LiveEar, LiveError, LiveSetup,
    LiveTuner, PromptTone, SoakReport, TunerMonitor,
};
pub use llm::{
    generate_plan, list_models, provider, provider_message, GenError, KeyNeed, Provider, CUSTOM,
    PROVIDERS,
};
pub use loopback::{LiveLoopback, LoopbackResult};
pub use matcher::{NoteResult, Outcome};
pub use monitor::LiveMonitor;
pub use pocket::{Pocket, PocketVerdict};
pub use practice::{Practice, PracticeEvent, PracticeOptions};
pub use progress::{run_from, ProgressStore, Run, SongProgress};
pub use rig::{Rig, RigOutput};
pub use service::*;
pub use session::{Mode, Session, SessionEvent, Workshop};
pub use settings::{ScoreSettings, Strictness};
pub use songsterr::{SearchError, SearchStart, SongHit, SongSearch};
pub use sources::{
    cache_dir_of, data_dir_of, download_dir, file_url, local_path, openable_extensions,
    recent_scores, search_url, SongSite,
};
pub use spacing::{spacing, streak_days, PassageEvent};
pub use stems::{BassRemoval, StemJob};
pub use summary::{
    DetunedString, EndHeard, Intonation, IntonationPart, Leaning, MutingTally, NoteEndMark,
    Observation, OutsideNote, Side, StringTally, Summary,
};
pub use technique::{technique_signs, TechniqueOutcome, TechniqueResult, TechniqueTally};
pub use transport::{LoopRange, Transport};
pub use update::{is_newer, NewRelease, UpdateCheck};
