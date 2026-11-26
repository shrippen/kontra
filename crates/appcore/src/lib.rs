//! Qt-free core of the app: everything the QML bridge calls, testable without a GUI.
//!
//! ```text
//! QML ──invokables──► bridge (cxx-qt) ──► Core ──► kontra-session (Live, LiveTuner, service)
//!     ◄──properties/signals── bridge ◄── UiEvent
//! ```

mod backing;
mod bluetooth;
mod bundled;
mod caches;
mod calibrate;
mod call;
mod chain;
mod choices;
mod course;
mod data;
mod density;
mod device_lists;
pub mod ear;
mod ear_training;
mod effects;
mod folders;
pub mod fx_plugins;
mod goal;
mod i18n;
mod input_choice;
mod input_state;
mod ladder;
mod learn;
pub mod level;
mod library;
mod library_view;
mod live_course;
mod llm;
mod loops;
mod mixer;
mod notation;
mod origin;
mod package;
mod prepare;
mod progress;
pub mod quiz;
pub mod quiz_stats;
mod quizzing;
mod report;
mod rig;
mod save;
mod song_map;
mod songsterr;
mod sources;
mod standby;
mod stems;
mod store;
mod summary;
mod tuner;
mod unplug;
mod update;
mod util;

pub use bundled::PieceVariety;
pub use choices::*;
pub use data::DataImport;
pub use i18n::{Catalog, Language};
pub use input_state::{InputState, QaOptions};
pub use kontra_session::{exercise, file_url, local_path, Blocks, ChainOrder, Density};
pub use library::BackingChange;
pub use llm::{KeyState, LlmEvent};
pub use package::PackageAudio;
pub use store::{
    data_dir, AudioPath, Bass, InputKind, LatencyFrom, Library, PracticeLoop, RecentEntry,
    SectionBars, SectionDensity, Setlist, Settings,
};

use input_choice::{chosen_input, chosen_output, preferred_input};
use input_state::{InputUse, Unstarted};
use kontra_model::{Arrangement, InputSource, InstrumentKind, Technique, Tuning};
use kontra_session::{
    import_file, list_devices, Chain, Direction, ImportResult, InputChoice, Live, LiveCalibration,
    LiveEar, LiveError, LiveLoopback, LiveSetup, LiveTuner, LoopRange, Mode, Outcome,
    PracticeEvent, PracticeOptions, ProgressStore, Run, ScoreSettings, SessionEvent,
    TechniqueOutcome, Workshop,
};
use mixer::gains;
use notation::key_at;
use serde_json::json;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
pub(crate) use util::{file_name, piece_name, quantum_too_long_ms, time_seed, unix_now};

/// Crates linked into the app and their licences, from `scripts/licences.py` (LH-NF-11).
const LICENCES: &str = include_str!("../licences.json");

/// Seconds from the input's start to the first measured calibration click; the count-in
/// (four clicks, 2.4 s) starts 0.6 s after the input.
const CALIBRATION_LEAD_S: f64 = 3.0;
/// Reference pitch range for A4 (LH-TUN-04).
const A4_MIN_HZ: f64 = 430.0;
const A4_MAX_HZ: f64 = 450.0;
/// Range of the free timing window (LH-BEW-02).
const WINDOW_MIN_MS: f64 = 20.0;
const WINDOW_MAX_MS: f64 = 150.0;
/// Beat map trusted to sit on the beat: this share of kick and bass onsets on the beats (0.5 is
/// a coin toss; a band with kick on 1 and 3 reaches 0.9).
const SURE_ON_BEAT_SHARE: f64 = 0.7;
/// Notes a string of a custom tuning may have: G0 (MIDI 19) to G3 (MIDI 55).
const CUSTOM_NOTES: std::ops::RangeInclusive<u8> = 19..=55;
/// Keyboard view without an instrument range: E1 to G4, a four-string bass.
const DEFAULT_KEYS: (u8, u8) = (28, 67);
/// Middle C (MIDI 60): any octave names the twelve keys of the keyboard view.
const MIDDLE_C: u8 = 60;

/// What the UI has to show after a call or a tick.
#[derive(Debug, Clone, PartialEq)]
pub enum UiEvent {
    /// A note was judged: index into the arrangement's notes and its new state.
    Judged {
        index: usize,
        state: &'static str,
        offset_ms: f64,
        cents: f64,
    },
    Extra,
    /// How a note's technique was played (LH-BEW-06): Kante's technique key and result
    /// ("played", "other", "unheard"); apart from `Judged`.
    Technique {
        index: usize,
        technique: &'static str,
        result: &'static str,
    },
    Holding {
        index: usize,
        text: String,
    },
    Released,
    PassComplete {
        clean: bool,
        /// Hits among the pass's notes and extras (0..=1); the workshop keeps it in a band.
        hit_rate: f64,
    },
    SpeedChanged(f64),
    Finished {
        summary_json: String,
    },
    Level {
        peak_db: f64,
        rms_db: f64,
        clipped: bool,
    },
    Tuner {
        note: String,
        cents: f64,
        /// The heard pitch as fractional MIDI number (guided tuning compares it with a string).
        midi: f64,
    },
    Message(String),
    Error(String),
    /// Ear training changed: a note was judged or the prompt ended (`Core::ear_json`).
    Ear(String),
    /// Level advice changed (LH-EIN-03/04): "", "silent", "quiet", "good", "loud" or "clip",
    /// and whether to offer the laptop-jack profile.
    LevelAdvice {
        state: &'static str,
        suggest_laptop: bool,
    },
    /// An audio device failed (unplugged): which side, the system's reason; the run was stopped.
    DeviceLost(Direction, String),
    /// Between runs: offer another share of the notes (LH-MOD-06), in percent, with the question.
    DensitySuggestion {
        percent: u8,
        text: String,
    },
    /// Another input could be used meanwhile (LH-ALL-04): its id and a question for the user.
    InputOffer {
        id: String,
        text: String,
    },
    /// The lost input is back and used again.
    InputBack(String),
    /// The player chose an input: the offer of a stand-in is no longer open.
    InputOfferEnded,
    /// A new USB input without output: JSON `{name, text}` of the question whether an instrument
    /// or a microphone hangs on it (LH-EIN-02); empty: the question is gone.
    UsbAsk(String),
    /// Bass removal of the backing track changed (`Core::stems_json`).
    Stems(String),
    /// The backing track is being prepared (true) or ready (false); play waits for it.
    BackingLoading(bool),
    /// Building blocks moved on (DIDAKTIK I6): the new step (`Core::chain_json`) is the loop,
    /// its run starts with a count-in.
    ChainStep {
        chain_json: String,
    },
    /// The effects page changed: a search ended or the chain latency moved (`effects_json`).
    EffectsChanged,
    /// Calibration finished: new latency (if measured) and a text for the user.
    Calibrated {
        latency_ms: Option<f64>,
        text: String,
    },
    /// A piece wish, a model list or a key job (LH-BIB-16); see `llm::LlmEvent`.
    Llm(LlmEvent),
}

/// A fixed note density gets no suggestions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DensityLock {
    Free,
    Fixed,
}

/// Which end of the loop a key sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopEdge {
    Start,
    End,
}

/// Result of opening a file.
#[derive(Debug, Clone, PartialEq)]
pub enum Opened {
    Ready,
    /// No bass track was recognized; the user picks one of these names.
    ChooseTrack(Vec<String>),
}

#[derive(Debug)]
struct Loaded {
    path: PathBuf,
    /// All notes as imported; `arrangement` holds those of the current density (LH-MOD-06).
    full: Arrangement,
    arrangement: Arrangement,
    /// The file's tracks a bass can play (index in the file, name) and the one played, so the
    /// player can switch (QA P2 R9).
    tracks: Vec<(usize, String)>,
    track: usize,
}

#[derive(Debug)]
pub struct Core {
    catalog: Catalog,
    settings: Settings,
    library: Library,
    data_dir: PathBuf,
    qa: QaOptions,
    loaded: Option<Loaded>,
    pending: Option<(PathBuf, ImportResult)>,
    live: Option<Live>,
    /// Where a finished run stood when a recording was chosen: the run is gone, the page keeps
    /// its summary and position (review #242). Ends with the next run or piece.
    finished_at_s: Option<f64>,
    tuner: Option<LiveTuner>,
    calibration: Option<LiveCalibration>,
    loopback: Option<LiveLoopback>,
    /// Input and output held open while idle (LH-EIN-18).
    standby: standby::Standby,
    /// Engine-independent start of the current run, for practice time.
    run_started: Option<std::time::Instant>,
    /// The audio server's block size was checked for this run (LH-EIN-07).
    quantum_checked: bool,
    /// The input was checked for a missing PipeWire node once this program run (review #73).
    direct_checked: bool,
    /// When the current live input (practice or tuner) started.
    input_since: Option<std::time::Instant>,
    speed: f64,
    looping: Option<(usize, usize)>,
    /// Call and response (LH-LRN-19).
    call: call::CallState,
    /// A note for the user from the last `open` (e.g. a five-string bass was chosen).
    notice: VecDeque<String>,
    /// The setlist being played and the position in it (LH-BIB-04).
    active_setlist: Option<(String, usize)>,
    /// The input that went away, watched until it is back (LH-ALL-04).
    lost: Option<unplug::Lost>,
    /// What the input does, for the player (LH-EIN-15).
    input: input_state::InputStatus,
    /// Input used instead of the lost one; never saved.
    fallback_input: Option<String>,
    /// The player chose an input while one was lost: the next tick takes the offer away.
    offer_ended: bool,
    /// Dropouts of finished runs since the start (LH-NF-04).
    xruns_done: u64,
    /// Courses the player added (LH-LRN-14), read once and after each change.
    courses: Vec<kontra_session::lesson::CoursePack>,
    /// A course package being fetched from an address (LH-LRN-14).
    fetcher: course::Fetcher,
    /// Fretboard quiz in progress (LH-LRN-02).
    quiz: Option<quiz::Quiz>,
    /// Ear training (LH-LRN-03): the round and its audio.
    ear: Option<ear::EarQuiz>,
    live_ear: Option<LiveEar>,
    /// The prompt sounded at the last tick (the UI learns when it may answer).
    ear_prompting: bool,
    /// Level advice of the running input (LH-EIN-03/04) and what the UI was told last.
    level: level::LevelAdvisor,
    level_told: (level::LevelState, bool),
    started_at: std::time::Instant,
    /// Backing tracks without bass (LH-BIB-09).
    stems: stems::Stems,
    /// Backing track decoded in the background (QA 27.10., J2).
    prepare: prepare::Prepare,
    bluetooth: bluetooth::BluetoothCheck,
    device_lists: device_lists::DeviceLists,
    /// The linked folders' files (LH-BIB-17), walked in the background.
    folders: folders::Folders,
    /// Where the browser saves downloads (LH-BIB-11).
    downloads: Option<PathBuf>,
    /// Songsterr search (LH-BIB-11, round 12).
    finder: songsterr::Finder,
    updater: update::Updater,
    /// The blind probe running now (DIDAKTIK I1), if any.
    probe: Option<Probe>,
    /// Building blocks in the workshop (DIDAKTIK I6), if chosen, and the loop chosen before them.
    chain: Option<(Chain, Option<(usize, usize)>)>,
    /// The goal of the lesson block whose exercise is at this path (LH-LRN-20).
    lesson_goal: Option<(PathBuf, kontra_session::lesson::Goal)>,
    /// A course folder open live (LH-LRN-21).
    live_course: Option<live_course::LiveCourse>,
    /// Plugins of the effect chain: the scan, and why slots are bypassed (LH-EIN-20).
    plugins: fx_plugins::Plugins,
    /// A wished-for piece or a provider's model list, fetched in the background (LH-BIB-16).
    llm: llm::LlmJob,
}

/// One pass of the loop without notes on screen (DIDAKTIK I1), at the speed it was practised.
#[derive(Debug, Clone, Copy)]
struct Probe {
    speed: f64,
}

/// The UI language before the app starts ("de" or "en"): as set in the settings of `data_dir`,
/// else the environment's. Kirigami's own texts follow it (round 12: "Close Sidebar" in a German
/// Kontra on an English system).
pub fn startup_language(data_dir: &Path) -> &'static str {
    let settings: Settings = save::peek(&data_dir.join(save::SETTINGS_FILE));
    let language = settings
        .language
        .as_deref()
        .map_or_else(Language::from_environment, Language::from_code);
    match language {
        Language::German => "de",
        Language::English => "en",
    }
}

impl Core {
    pub fn new(data_dir: &Path, qa: QaOptions) -> Core {
        let (mut settings, mut library, damaged) = save::load_data(data_dir);
        settings.migrate();
        library.plain_paths();
        library.find_moved(&Self::exercises_dir());
        let moved_pieces = origin::move_old_pieces(data_dir, &mut library);
        let language = settings
            .language
            .as_deref()
            .map_or_else(Language::from_environment, Language::from_code);
        let mut core = Core {
            courses: kontra_session::CourseStore::new(data_dir).load(),
            fetcher: course::Fetcher::default(),
            quiz: None,
            ear: None,
            live_ear: None,
            ear_prompting: false,
            level: level::LevelAdvisor::default(),
            level_told: (level::LevelState::Waiting, false),
            started_at: std::time::Instant::now(),
            stems: stems::Stems::default(),
            prepare: prepare::Prepare::default(),
            bluetooth: bluetooth::BluetoothCheck::default(),
            device_lists: device_lists::DeviceLists::default(),
            folders: folders::Folders::default(),
            downloads: kontra_session::download_dir(),
            finder: songsterr::Finder::default(),
            updater: update::Updater::default(),
            probe: None,
            chain: None,
            lesson_goal: None,
            live_course: None,
            plugins: fx_plugins::Plugins::start(data_dir, &mut settings),
            llm: llm::LlmJob::default(),
            notice: VecDeque::new(),
            active_setlist: None,
            lost: None,
            input: input_state::InputStatus::new(),
            fallback_input: None,
            offer_ended: false,
            xruns_done: 0,
            catalog: Catalog::new(language),
            settings,
            library,
            data_dir: data_dir.to_path_buf(),
            qa,
            loaded: None,
            pending: None,
            live: None,
            finished_at_s: None,
            tuner: None,
            calibration: None,
            loopback: None,
            standby: standby::Standby::default(),
            run_started: None,
            quantum_checked: false,
            direct_checked: false,
            input_since: None,
            speed: 1.0,
            looping: None,
            call: call::CallState::default(),
        };
        core.save_after_start(moved_pieces, &damaged);
        // Measured latencies hold only for the devices they were measured on (LH-EIN-05).
        core.check_audio_path();
        core
    }

    pub fn tr(&self, key: &str, args: &[(&str, String)]) -> String {
        self.catalog.tr(key, args)
    }

    pub fn texts_json(&self) -> String {
        self.catalog.json()
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// A note name as the user's language writes it: German says H for B (B is B♭ there).
    pub fn note_label(&self, name: &str) -> String {
        let name = match (self.catalog.language(), name.strip_prefix('B')) {
            // B♭ (spelled "Bb") is German B: "Bb2" → "B2", "Bbm" → "Bm".
            (Language::German, Some(rest)) if rest.starts_with('b') => format!("B{}", &rest[1..]),
            (Language::German, Some(rest)) => format!("H{rest}"),
            _ => name.to_string(),
        };

        // Printed signs, as in the notation: "A#2" → "A♯2", "Ebm" → "E♭m" (QA 13.10.).
        name.chars()
            .enumerate()
            .map(|(i, c)| match (i, c) {
                (_, '#') => '♯',
                (1, 'b') => '♭',
                _ => c,
            })
            .collect()
    }

    /// A pitch spelled in a key, in the user's language: MIDI 34 in B♭ is "B♭1" ("B1" in German),
    /// as the notation writes it (QA 27.10., M4).
    fn pitch_label(&self, pitch: kontra_model::Pitch, key_fifths: i8) -> String {
        self.note_label(&pitch.spelled(key_fifths))
    }

    /// An import error in the user's language (QA P2: the English reason showed in German).
    fn import_error(&self, e: &kontra_session::ImportError) -> String {
        use kontra_session::ImportError as E;
        let reason = match e {
            E::Io(_) => self.tr("import.io", &[]),
            // Detection by content failed: also what a cut-off download looks like.
            E::UnknownFormat => self.tr("import.unknown", &[]),
            E::Unsupported(what) => self.tr("import.unsupported", &[("what", what.clone())]),
            E::Truncated(_) | E::Malformed(_) => self.tr("import.damaged", &[]),
            E::NoTrack(_) => self.tr("import.notrack", &[]),
            E::TooLarge(bytes) => {
                self.tr("import.large", &[("mb", (bytes / 1_000_000).to_string())])
            }
            E::Empty => self.tr("import.empty", &[]),
            E::TooLong(bars) => self.tr("import.long", &[("bars", bars.to_string())]),
        };
        self.tr("error.import", &[("reason", reason)])
    }

    /// Wait mode: what to play next, with the string by name; after a wrong note also what was
    /// played (J-B2: the player sees both, in words).
    pub fn wait_text(&self, note: usize, played_midi: Option<f64>) -> Option<String> {
        let a = self.arrangement()?;
        let n = a.notes.get(note)?;
        let string = n
            .string
            .and_then(|s| a.instrument.tuning.strings.get(usize::from(s)))
            .map_or_else(String::new, |p| self.note_label(&p.name().to_string()));
        let key = key_at(a, n.start_ticks);
        let args = [
            ("note", self.pitch_label(n.pitch, key)),
            ("string", string),
            ("fret", n.fret.unwrap_or(0).to_string()),
        ];
        let wanted = self.tr("waiting.for", &args);
        Some(match played_midi {
            Some(m) => {
                let played = kontra_model::Pitch(m.round().clamp(0.0, 127.0) as u8);
                let played = self.pitch_label(played, key);
                let played = self.tr("waiting.played", &[("played", played)]);
                format!("{played} {wanted}")
            }
            None => wanted,
        })
    }

    /// Scoring from the instrument, the input and the user's choices (LH-BEW-02, -05).
    pub fn score_settings(&self, kind: InstrumentKind) -> ScoreSettings {
        let mut settings = ScoreSettings::defaults(kind, self.source());
        settings.latency_ms = self.settings.latency_ms;
        settings.strictness = match self.settings.strictness {
            Window::Loose => kontra_session::Strictness::Loose,
            Window::Normal => kontra_session::Strictness::Normal,
            Window::Strict => kontra_session::Strictness::Strict,
            Window::Custom => kontra_session::Strictness::Custom {
                window_ms: self.settings.window_ms.clamp(WINDOW_MIN_MS, WINDOW_MAX_MS),
            },
            Window::Auto => settings.strictness,
        };
        if self.settings.pitch_cents > 0.0 {
            settings.pitch_cents = self.settings.pitch_cents;
        }
        settings
    }

    /// Open strings for guided tuning (LH-TUN-03), lowest first: the loaded piece's tuning, else
    /// the profile's. `[{"name": "E", "midi": 28}, …]`.
    pub fn tuning_json(&self) -> String {
        self.strings_json(&self.instrument().tuning)
    }

    /// The profile's open strings (LH-TUN-02), as `tuning_json`; for the settings.
    pub fn profile_tuning_json(&self) -> String {
        self.strings_json(&instrument_from(&self.settings).tuning)
    }

    /// Notes a string of a custom tuning can take, lowest first: `[{"name": "G0", "midi": 19}, …]`.
    pub fn note_choices_json(&self) -> String {
        let notes: Vec<serde_json::Value> = CUSTOM_NOTES
            .map(|m| json!({ "name": self.note_label(&kontra_model::Pitch(m).to_string()), "midi": m }))
            .collect();
        serde_json::Value::Array(notes).to_string()
    }

    /// `[{"name": "E", "octave": "E1", "midi": 28}, …]`: the name for the tuner, with octave
    /// where open strings share a name (drop D: D1 and D2).
    fn strings_json(&self, tuning: &kontra_model::Tuning) -> String {
        let strings: Vec<serde_json::Value> = tuning
            .strings
            .iter()
            .map(|p| {
                json!({
                    "name": self.note_label(&p.name().to_string()),
                    "full": self.note_label(&p.to_string()),
                    "midi": p.0,
                })
            })
            .collect();
        serde_json::Value::Array(strings).to_string()
    }

    /// The instrument to listen for: the loaded piece's, else the profile; with the reference
    /// pitch from the settings (LH-TUN-04).
    fn instrument(&self) -> kontra_model::Instrument {
        let mut instrument = self
            .arrangement()
            .map_or_else(|| instrument_from(&self.settings), |a| a.instrument.clone());
        instrument.a4_hz = self.a4_hz();
        instrument
    }

    fn a4_hz(&self) -> f64 {
        self.settings.a4_hz.clamp(A4_MIN_HZ, A4_MAX_HZ)
    }

    /// Writes settings, library and progress as one JSON file (LH-FOR-03); returns what was
    /// saved, as a text for the user.
    pub fn export_data(&self, path: &Path) -> Result<String, String> {
        let bundle = data::Bundle {
            format: data::FORMAT.into(),
            version: data::VERSION,
            exported_unix: unix_now(),
            settings: self.settings.clone(),
            library: self.library.clone(),
            progress: self.progress_store().all(),
            quiz: save::load(&self.data_dir.join(quiz_stats::FILE)),
        };
        save::save(path, &bundle)
            .map_err(|e| self.tr("data.error.write", &[("reason", e.to_string())]))?;
        let runs = bundle.progress.iter().map(|p| p.runs.len()).sum::<usize>();
        Ok(self.tr(
            "data.exported",
            &[
                ("pieces", bundle.library.recent.len().to_string()),
                ("setlists", bundle.library.setlists.len().to_string()),
                ("runs", runs.to_string()),
                ("name", file_name(path)),
            ],
        ))
    }

    /// Reads a file written by `export_data`: merges it into the own data or replaces them.
    /// Nothing changes if the file is not valid. Returns a report for the user.
    pub fn import_data(&mut self, path: &Path, how: DataImport) -> Result<String, String> {
        let text = std::fs::read_to_string(path).map_err(|_| self.tr("data.error.read", &[]))?;
        let bundle = data::Bundle::parse(&text).map_err(|e| match e {
            data::BundleError::Newer(v) => {
                self.tr("data.error.newer", &[("version", v.to_string())])
            }
            data::BundleError::Unreadable | data::BundleError::BadSongId => {
                self.tr("data.error.format", &[])
            }
        })?;
        self.stop_live();
        let store = self.progress_store();
        let mut merged = data::Merged::default();
        match how {
            DataImport::Replace => {
                merged.pieces = bundle.library.recent.len();
                merged.setlists = bundle.library.setlists.len();
                self.library = bundle.library;
                for old in store.all() {
                    let _ = std::fs::remove_file(
                        self.data_dir
                            .join("progress")
                            .join(format!("{}.json", old.song_id)),
                    );
                }
                // Volumes and effects of the backup reach what plays now.
                self.remix(|s| *s = bundle.settings);
            }
            DataImport::Merge => merged = data::merge_library(&mut self.library, bundle.library),
        }
        for incoming in bundle.progress {
            let mut song = match how {
                DataImport::Merge => store.load_id(&incoming.song_id),
                DataImport::Replace => None,
            }
            .unwrap_or_else(|| kontra_session::SongProgress {
                runs: Vec::new(),
                ..incoming.clone()
            });
            merged.runs += data::merge_runs(&mut song, incoming);
            store
                .save(&song)
                .map_err(|e| self.tr("data.error.write", &[("reason", e.to_string())]))?;
        }
        let quiz_file = self.data_dir.join(quiz_stats::FILE);
        let quiz = match how {
            DataImport::Replace => bundle.quiz,
            DataImport::Merge => {
                save::load::<quiz_stats::QuizStats>(&quiz_file).merged(bundle.quiz)
            }
        };
        save::save(&quiz_file, &quiz)
            .map_err(|e| self.tr("data.error.write", &[("reason", e.to_string())]))?;
        self.quiz = None;
        self.save_library();
        if how == DataImport::Replace {
            // Strings and reference act at once; a piece gone missing is a notice, not a failure.
            if let Err(e) = self.relayout() {
                self.notice.push_back(e);
            }
        }
        let key = match how {
            DataImport::Merge => "data.merged",
            DataImport::Replace => "data.replaced",
        };
        let report = self.tr(
            key,
            &[
                ("pieces", merged.pieces.to_string()),
                ("setlists", merged.setlists.to_string()),
                ("runs", merged.runs.to_string()),
                ("name", file_name(path)),
            ],
        );
        Ok(self.with_notices(report))
    }

    /// The About section's licence list (LH-NF-11): Kontra, the dynamically linked Qt and KDE
    /// Frameworks, then every crate: `[{"name", "version", "license"}, …]`.
    pub fn licences_json(&self) -> String {
        let crates: Vec<serde_json::Value> = serde_json::from_str(LICENCES).unwrap_or_default();
        let own = [
            json!({ "name": "Kontra", "version": env!("CARGO_PKG_VERSION"), "license": env!("CARGO_PKG_LICENSE") }),
            json!({ "name": "Qt 6", "version": "", "license": self.tr("about.lgpl", &[]) }),
            json!({ "name": "KDE Frameworks 6 (Kirigami)", "version": "", "license": self.tr("about.lgpl", &[]) }),
        ];
        serde_json::Value::Array(own.into_iter().chain(crates).collect()).to_string()
    }

    /// Tracks of the loaded file and the one played: `{"names": [...], "current": i}`.
    pub fn tracks_json(&self) -> String {
        let (names, current) = self.loaded.as_ref().map_or((Vec::new(), 0), |l| {
            let names: Vec<&String> = l.tracks.iter().map(|(_, n)| n).collect();
            let current = l.tracks.iter().position(|(i, _)| *i == l.track);
            (names, current.unwrap_or(0))
        });
        json!({ "names": names, "current": current }).to_string()
    }

    /// Plays another track of the loaded file (Kontra's guess may be the wrong one); `choice`
    /// counts the tracks of `tracks_json`.
    pub fn switch_track(&mut self, choice: usize) -> Result<(), String> {
        let track = self
            .loaded
            .as_ref()
            .and_then(|l| l.tracks.get(choice))
            .map(|(i, _)| *i)
            .ok_or("no such track")?;
        self.reload(track)
    }

    /// Imports the loaded file again and plays `track`; keeps the setlist position.
    fn reload(&mut self, track: usize) -> Result<(), String> {
        let path = self.loaded_path().ok_or("nothing loaded")?.to_path_buf();
        let imported = import_file(&path).map_err(|e| self.import_error(&e))?;
        let setlist = self.active_setlist.clone();
        self.load(&path, &imported, track)?;
        self.active_setlist = setlist;
        Ok(())
    }

    /// The view for the loaded piece: the one last used with it, else the default (LH-ANZ-04).
    pub fn view(&self) -> View {
        self.loaded_path()
            .and_then(|p| self.library.entry(p))
            .and_then(|e| e.view)
            .unwrap_or(self.settings.view)
    }

    /// Remembers `view` for the loaded piece and as the default for pieces opened next.
    pub fn set_view(&mut self, view: View) {
        self.update_settings(|s| s.view = view);
        if let Some(path) = self.loaded_path().map(Path::to_path_buf) {
            if let Some(entry) = self.library.entry_mut(&path) {
                entry.view = Some(view);
            }
            self.save_library();
        }
    }

    /// Changes settings that do not affect a running practice (view, style, language) and saves.
    pub fn update_settings(&mut self, change: impl FnOnce(&mut Settings)) {
        let before = self.settings.clone();
        change(&mut self.settings);
        // The MIDI inputs only when the choice changed: listing them asks the system.
        let ports = if before.midi_port == self.settings.midi_port {
            Vec::new()
        } else {
            kontra_session::list_midi_inputs()
        };
        calibrate::forget_on_switch(&before, &mut self.settings, &ports);
        self.switch_devices(&before);
        if let Some(code) = &self.settings.language {
            self.catalog = Catalog::new(Language::from_code(code));
        }
        self.save_settings();
    }

    /// Changes settings the audio depends on (mode, instrument, devices, latency, metronome):
    /// saves them and ends a running practice, the next `play` uses the new ones.
    pub fn reconfigure(&mut self, change: impl FnOnce(&mut Settings)) {
        self.update_settings(change);
        self.stop_live();
    }

    /// "Band reagiert" (LH-WIE-05) on or off, also for the run going on (audit 10.11.).
    pub fn set_reacting_band(&mut self, on: bool) {
        self.update_settings(|s| s.reacting_band = on);
        let options = self.practice_options();
        if let Some(live) = self.live.as_mut() {
            live.practice_mut().set_options(options);
        }
    }

    /// Changes the instrument profile (strings, tuning, reference pitch) so that it acts at once:
    /// a loaded piece without its own tuning is laid out on the new strings again (LH-TUN-02) and
    /// a running tuner listens with the new strings and reference.
    pub fn retune(&mut self, change: impl FnOnce(&mut Settings)) -> Result<(), String> {
        self.reconfigure(change);
        self.relayout()
    }

    /// The instrument as the UI offers it (LH-EIN-09): MIDI input, else the stored bass.
    pub fn instrument_choice(&self) -> InstrumentChoice {
        match (self.settings.input_kind, self.settings.instrument) {
            (InputKind::Midi, _) => InstrumentChoice::Midi,
            (_, Bass::Double) => InstrumentChoice::Double,
            (_, Bass::Fretted | Bass::Fretless) => InstrumentChoice::EBass,
        }
    }

    /// Chooses the instrument (LH-EIN-09) and suggests its input: a double bass the microphone
    /// (a pickup chosen after stays), an e-bass the cable, bass over MIDI the MIDI input.
    pub fn choose_instrument(&mut self, choice: InstrumentChoice) -> Result<(), String> {
        self.retune(|s| {
            let was_double = s.instrument == Bass::Double;
            match choice {
                InstrumentChoice::Midi => s.input_kind = InputKind::Midi,
                InstrumentChoice::Double => {
                    if !was_double || s.input_kind == InputKind::Midi {
                        s.input_kind = InputKind::Microphone;
                    }
                    s.instrument = Bass::Double;
                    s.strings = s.strings.min(Bass::Double.strings_max());
                }
                InstrumentChoice::EBass => {
                    let mic_of_double = was_double && s.input_kind == InputKind::Microphone;
                    if mic_of_double || s.input_kind == InputKind::Midi {
                        s.input_kind = InputKind::Line;
                    }
                    if was_double {
                        s.instrument = Bass::Fretted;
                    }
                }
            }
        })
    }

    /// Chooses a tuning by name (`TUNINGS`); "custom" starts from the strings in use.
    pub fn set_tuning(&mut self, preset: TuningPreset) -> Result<(), String> {
        let current = names_of(&tuning_from(&self.settings));
        self.retune(|s| {
            if preset == TuningPreset::Custom && s.custom_tuning.len() != current.len() {
                s.custom_tuning = current;
            }
            s.tuning = preset;
        })
    }

    /// Sets one open string (0 = lowest) to `midi` and makes the result the custom tuning.
    pub fn set_custom_string(&mut self, string: usize, midi: u8) -> Result<(), String> {
        let mut names = names_of(&tuning_from(&self.settings));
        let Some(slot) = names.get_mut(string) else {
            return Ok(());
        };
        if !CUSTOM_NOTES.contains(&midi) {
            return Ok(());
        }
        *slot = kontra_model::Pitch(midi).to_string();
        self.retune(|s| {
            s.custom_tuning = names;
            s.tuning = TuningPreset::Custom;
        })
    }

    /// Reference pitch of A4 (LH-TUN-04), clamped to 430–450 Hz; returns the value kept.
    pub fn set_reference(&mut self, a4_hz: f64) -> Result<f64, String> {
        let hz = a4_hz.clamp(A4_MIN_HZ, A4_MAX_HZ);
        self.retune(|s| s.a4_hz = hz)?;
        Ok(hz)
    }

    /// Free timing window (LH-BEW-02), clamped to 20–150 ms; returns the value kept.
    pub fn set_window(&mut self, window_ms: f64) -> f64 {
        let ms = window_ms.clamp(WINDOW_MIN_MS, WINDOW_MAX_MS).round();
        self.reconfigure(|s| {
            s.window_ms = ms;
            s.strictness = Window::Custom;
        });
        ms
    }

    /// Imports a score; picks the bass track or asks.
    pub fn open(&mut self, path: &Path) -> Result<Opened, String> {
        self.active_setlist = None;
        // A package is unpacked into the data folder: its own path is never kept.
        if package::is_package(path) {
            return self.open_package(path).map(|()| Opened::Ready);
        }
        self.keepable(path)?;
        let imported = import_file(path).map_err(|e| self.import_error(&e))?;
        // Round 12: plan exercises in a run-through, own pieces in the mode last chosen for them.
        let mode = if self.is_bundled(path) {
            Some(PLAN_MODE)
        } else {
            self.settings.piece_mode
        };
        if let Some(mode) = mode {
            self.update_settings(|s| s.mode = mode);
        }
        // Without a clear bass track, the one chosen last time (still in the file) counts.
        let chosen = self
            .library
            .entry(path)
            .map(|e| e.track)
            .filter(|t| *t < imported.tracks().len());
        if let Some(track) = imported.suggested().or(chosen) {
            return self.load(path, &imported, track).map(|()| Opened::Ready);
        }
        let names = playable(&imported).map(|(_, n)| n).collect();
        self.pending = Some((path.to_path_buf(), imported));
        Ok(Opened::ChooseTrack(names))
    }

    /// Loads the pending file's track `choice`, counted as in `Opened::ChooseTrack`.
    pub fn choose_track(&mut self, choice: usize) -> Result<(), String> {
        let (path, imported) = self.pending.take().ok_or("nothing to choose from")?;
        let track = playable(&imported)
            .nth(choice)
            .map(|(i, _)| i)
            .ok_or("no such track")?;
        self.load(&path, &imported, track)
    }

    fn load(&mut self, path: &Path, imported: &ImportResult, track: usize) -> Result<(), String> {
        let mut arrangement = imported
            .arrangement(track)
            .map_err(|e| self.import_error(&e))?;
        arrangement.title = piece_name(&arrangement.title, path);
        if !imported.has_tuning(track) {
            // No tuning in the file: the player's instrument, if it reaches the lowest note
            // (QA F13: a five-string profile was replaced by four strings).
            let profile = instrument_from(&self.settings);
            let lowest = arrangement.notes.iter().map(|n| n.pitch).min();
            let reaches = match (profile.tuning.lowest(), lowest) {
                (Some(open), Some(low)) => open <= low,
                _ => true,
            };
            if !reaches {
                // QA P2 (U1): say why the tab shows more strings than the profile.
                let note = lowest.map_or_else(String::new, |p| self.note_label(&p.to_string()));
                self.notice.push_back(self.tr(
                    "notice.more.strings",
                    &[
                        ("note", note),
                        (
                            "strings",
                            arrangement.instrument.tuning.strings.len().to_string(),
                        ),
                    ],
                ));
            }
            if reaches && profile.tuning != arrangement.instrument.tuning {
                arrangement.instrument = profile;
                for n in &mut arrangement.notes {
                    n.string = None;
                    n.fret = None;
                }
                kontra_session::assign_tab(&mut arrangement);
            }
        }
        // The profile's kind (fretless, double bass, MIDI) decides scoring and lessons.
        let kind = instrument_from(&self.settings).kind;
        if kind != InstrumentKind::Fretted {
            arrangement.instrument.kind = kind;
        }
        arrangement.instrument.a4_hz = self.a4_hz();
        self.stop_live();
        self.looping = None;
        self.chain = None;
        // Another piece starts at full tempo; the same one again keeps it (QA 27.10., J9).
        if self.loaded_path() != Some(path) {
            self.speed = 1.0;
        }
        let song_id = ProgressStore::song_id(&arrangement);
        self.adopt_progress(path, &song_id);
        self.library.touch(RecentEntry {
            path: path.to_path_buf(),
            title: arrangement.title.clone(),
            artist: arrangement.artist.clone(),
            track,
            last_unix: unix_now(),
            song_id,
            tempo_bpm: Some(arrangement.tempo.bpm_at(0)),
            ..Default::default()
        });
        // A backing track with a beat map: the bars follow the band (LH-BIB-06).
        if self
            .library
            .entry(path)
            .is_some_and(|e| e.backing.is_some())
        {
            if let Some(map) = self.library.beat_maps.get(path) {
                let times = map.bar_times(&quarters_per_bar(&arrangement));
                if times.len() >= 2 {
                    arrangement.tempo = tempo_from_bars(&arrangement, &times);
                    if let Some(entry) = self.library.entry_mut(path) {
                        entry.backing_offset_s = times[0];
                    }
                }
            }
        }
        self.save_library();
        // After the progress key: a transposed piece keeps its progress (LH-WIE-02).
        let semitones = self
            .library
            .entry(path)
            .map_or(0, |e| e.transpose_semitones);
        kontra_session::transpose(&mut arrangement, semitones);
        self.loaded = Some(Loaded {
            path: path.to_path_buf(),
            full: arrangement.clone(),
            arrangement,
            tracks: playable(imported).collect(),
            track,
        });
        self.finished_at_s = None;
        self.rethin();
        Ok(())
    }

    pub fn arrangement(&self) -> Option<&Arrangement> {
        self.loaded.as_ref().map(|l| &l.arrangement)
    }

    /// Title of the loaded piece: a bundled exercise in the UI's language, else as in the file.
    pub fn loaded_title(&self) -> String {
        self.loaded_path()
            .and_then(|p| self.plan_title(p))
            .or_else(|| self.arrangement().map(|a| a.title.clone()))
            .unwrap_or_default()
    }

    /// What the loaded bundled exercise trains, in the UI's language ("" for any other piece).
    pub fn loaded_goal(&self) -> String {
        let Some(path) = self.loaded_path() else {
            return String::new();
        };
        if !self.is_bundled(path) {
            return String::new();
        }
        let stem = path
            .file_stem()
            .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
        self.bundled_goal(&stem)
    }

    pub fn loaded_path(&self) -> Option<&Path> {
        self.loaded.as_ref().map(|l| l.path.as_path())
    }

    pub fn string_names_json(&self) -> String {
        let names: Vec<String> = self
            .arrangement()
            .map(|a| {
                a.instrument
                    .tuning
                    .strings
                    .iter()
                    .map(|p| p.name().to_string())
                    .collect()
            })
            .unwrap_or_default();
        serde_json::to_string(&names).unwrap_or_else(|_| "[]".into())
    }

    /// Keys of the keyboard view (J-E1): the instrument's range as MIDI numbers and the
    /// twelve note names in the user's language, spelled in the piece's first key, e.g.
    /// {"low": 28, "high": 67, "names": ["C", "C♯", …, "H"]} for a four-string bass with 24
    /// frets, in German.
    pub fn key_range_json(&self) -> String {
        let (low, high) = self
            .instrument()
            .range()
            .map_or(DEFAULT_KEYS, |(low, high)| (low.0, high.0));
        let key = self.arrangement().map_or(0, |a| key_at(a, 0));
        let names: Vec<String> = (0..12)
            .map(|pc| self.pitch_label(kontra_model::Pitch(MIDDLE_C + pc), key))
            .map(|name| {
                name.trim_end_matches(|c: char| c.is_ascii_digit())
                    .to_string()
            })
            .collect();
        json!({ "low": low, "high": high, "names": names }).to_string()
    }

    pub fn duration_s(&self) -> f64 {
        self.arrangement().map_or(0.0, Arrangement::duration_s)
    }

    /// True while a run exists, playing or paused (it goes on after a change of key).
    pub fn has_run(&self) -> bool {
        self.live.is_some()
    }

    pub fn is_playing(&self) -> bool {
        self.live
            .as_ref()
            .is_some_and(|l| l.practice().is_playing())
    }

    pub fn play(&mut self) -> Vec<UiEvent> {
        if self.backing_loading() {
            return vec![UiEvent::Message(self.tr("backing.preparing", &[]))];
        }
        let mut events = Vec::new();
        if let Some((path, _)) = self.backing().filter(|(p, _)| !p.exists()) {
            let name = path
                .file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into());
            events.push(UiEvent::Message(
                self.tr("hint.backing.missing", &[("name", name)]),
            ));
        }
        if self.live.is_none() {
            match self.start_live() {
                // The input's state shows it (LH-EIN-15), not a second bar.
                Err(Unstarted::Input) => return events,
                Ok(silent) if silent => events.push(UiEvent::Message(self.tr("hint.silent", &[]))),
                Ok(_) => {}
                Err(Unstarted::Other(e)) => return vec![UiEvent::Error(e)],
            }
        }
        self.ask_bluetooth();
        if let Some(hint) = self.monitor_hint() {
            events.push(UiEvent::Message(hint));
        }
        let aligned = self.qa.input_file.is_some();
        if let Some(live) = self.live.as_mut() {
            if aligned {
                live.play_aligned();
            } else {
                live.practice_mut().play();
            }
        }
        events
    }

    pub fn pause(&mut self) {
        if let Some(live) = self.live.as_mut() {
            live.practice_mut().pause();
        }
    }

    /// Plays the loop once without help (DIDAKTIK I1): no notes on screen, scored silently,
    /// compared afterwards with the workshop. Speed: as last practised in the workshop.
    pub fn start_probe(&mut self) -> Vec<UiEvent> {
        let Some(range) = self.loop_range() else {
            return vec![UiEvent::Error(self.tr("probe.noloop", &[]))];
        };
        self.stop_live();
        let practised = self
            .loaded_runs()
            .iter()
            .rev()
            .find(|r| r.mode == WORKSHOP_MODE && same_range(r.section, range))
            .map(|r| r.speed);
        self.probe = Some(Probe {
            speed: practised.unwrap_or(self.speed),
        });
        let events = self.play();

        // The input did not start: no probe hiding the notes of the next run.
        if self.live.is_none() {
            self.probe = None;
        }
        events
    }

    /// True while a blind probe runs: the UI hides the notes.
    pub fn is_probing(&self) -> bool {
        self.probe.is_some()
    }

    /// Runs of the loaded piece, oldest first.
    fn loaded_runs(&self) -> Vec<Run> {
        let Some(a) = self.loaded.as_ref().map(|l| &l.full) else {
            return Vec::new();
        };
        self.progress_store()
            .load(a)
            .map_or_else(|_| Vec::new(), |p| p.runs)
    }

    /// Ends the run; the next `play` starts from the beginning with current settings.
    pub fn stop_live(&mut self) {
        // The Bluetooth hint belongs to this run; nothing plays when it would come.
        self.forget_bluetooth();
        // A run stopped before the end still counts as practice (QA P1 J-C1); a probe stopped
        // before its pass is no probe and would end the start page's offer.
        let passed = self
            .live
            .as_ref()
            .is_some_and(|l| l.practice().session().is_finished());
        if self.probe.is_some() && !passed {
            self.run_started = None;
        }
        if self.run_started.is_some() {
            self.record_run();
        }
        if let Some(live) = self.live.take() {
            self.xruns_done += live.xruns();
            self.park(live.into_rig());
        }
        self.input.stopped(InputUse::Practice);
        self.probe = None;
        self.finished_at_s = None;
    }

    pub fn set_speed(&mut self, speed: f64) {
        self.speed = speed.clamp(0.25, 1.5);
        if let Some(live) = self.live.as_mut() {
            live.practice_mut().set_speed(self.speed);
        }
    }

    pub fn speed(&self) -> f64 {
        self.speed
    }

    /// Returns true if the notes changed: leaving the workshop ends the building blocks and
    /// loops the bars chosen before them (`stop_chain`).
    pub fn set_mode(&mut self, mode: PracticeMode) -> bool {
        let changed = mode != PracticeMode::Workshop && self.stop_chain();
        let own = self.loaded_path().is_some_and(|p| !self.is_bundled(p));
        self.reconfigure(|s| {
            s.mode = mode;
            if own {
                s.piece_mode = Some(mode);
            }
        });
        changed
    }

    /// Loops bars `from..=to` (indices into the arrangement's bars).
    /// The workshop's hit-rate band (low, high), for the UI.
    pub fn workshop_band() -> (f64, f64) {
        Workshop::default().band
    }

    /// Largest boost or cut of a monitor tone band in dB (LH-EIN-08), for the UI.
    pub fn shaping_range_db() -> f64 {
        f64::from(kontra_session::SHAPING_RANGE_DB)
    }

    /// The first unrolled bar of printed bar `printed` (from 1), as stored in runs and named on
    /// the start page's cards: bar 1 repeated gives bars 1 1 2, printed 2 is index 2.
    pub fn score_bar_index(&self, printed: u32) -> Option<usize> {
        let source = printed.checked_sub(1)?;
        self.arrangement()?
            .bars
            .iter()
            .position(|b| b.source_index == source)
    }

    pub fn seek(&mut self, position_s: f64) {
        if let Some(live) = self.live.as_mut() {
            live.practice_mut().seek(position_s);
        }
    }

    fn mode(&self) -> Mode {
        if self.probe.is_some() {
            return Mode::Probe;
        }
        match self.settings.mode {
            PracticeMode::Song => Mode::Song,
            PracticeMode::Wait => Mode::Wait,
            PracticeMode::Workshop => Mode::Workshop(Workshop {
                target_speed: self.target_speed(),
                ..Workshop::default()
            }),
            PracticeMode::Play => Mode::Play,
        }
    }

    fn source(&self) -> InputSource {
        let midi_take = self
            .qa
            .input_file
            .as_deref()
            .is_some_and(kontra_session::is_midi_file);
        if midi_take
            || (self.qa.input_file.is_none() && self.settings.input_kind == InputKind::Midi)
        {
            return InputSource::Midi;
        }
        match self.settings.input_kind {
            _ if self.qa.laptop_jack => InputSource::LaptopJack,
            InputKind::LaptopJack => InputSource::LaptopJack,
            InputKind::Microphone => InputSource::Microphone,
            InputKind::Line | InputKind::Midi => InputSource::Line,
        }
    }

    fn practice_options(&self) -> PracticeOptions {
        PracticeOptions {
            metronome: self.settings.metronome,
            // A probe from the start page begins at once: one bar to get ready at least.
            count_in_bars: if self.probe.is_some() {
                self.settings.count_in_bars.max(1)
            } else {
                self.settings.count_in_bars
            },
            // A probe is without help: no reference tones playing the notes.
            reference_tones: self.settings.reference_tones && self.probe.is_none(),
            backing_offset_s: self.backing().map_or(0.0, |(_, offset)| offset),
            transpose_semitones: self.transpose(),
            band: if self.settings.reacting_band {
                kontra_session::Band::Reacting
            } else {
                kontra_session::Band::AsWritten
            },
        }
    }

    fn input_choice(&self) -> InputChoice {
        match &self.qa.input_file {
            Some(path) => InputChoice::File {
                path: path.clone(),
                laptop_jack: self.qa.laptop_jack,
            },
            None if self.settings.input_kind == InputKind::Midi => InputChoice::Midi {
                port: self.midi_port().map(|p| p.id),
            },
            None => InputChoice::Device {
                id: self.fallback_input.clone().or_else(|| self.input_device()),
                channel: None,
            },
        }
    }

    /// Name of the MIDI input Kontra uses (the chosen one, else the first), or "" if none.
    pub fn midi_port_name(&self) -> String {
        self.midi_port().map_or_else(String::new, |p| p.name)
    }

    /// The MIDI input to use: the chosen device if connected, else the first (LH-EIN-12).
    fn midi_port(&self) -> Option<kontra_session::MidiPort> {
        let ports = kontra_session::list_midi_inputs();
        let (index, _) = pick_midi(&ports, self.settings.midi_port.as_deref());
        index.and_then(|i| ports.into_iter().nth(i))
    }

    /// MIDI devices for the connection settings (LH-EIN-12, round 9):
    /// `{"ports": [{"name", "chosen"}], "missingText"}`; `missingText` says when the chosen
    /// device is not connected and which one Kontra takes instead.
    pub fn midi_ports_json(&self) -> String {
        let ports = kontra_session::list_midi_inputs();
        let chosen = self.settings.midi_port.as_deref();
        let (index, missing) = pick_midi(&ports, chosen);
        let list: Vec<_> = ports
            .iter()
            .enumerate()
            .map(|(i, p)| json!({ "name": p.name, "chosen": Some(i) == index }))
            .collect();
        let missing_text = match (missing, chosen) {
            (MidiChoice::Missing, Some(name)) => {
                let instead = index.and_then(|i| ports.get(i)).map(|p| p.name.clone());
                match instead {
                    Some(instead) => self.tr(
                        "settings.midi.missing",
                        &[("name", name.to_string()), ("instead", instead)],
                    ),
                    None => self.tr("settings.midi.missing.none", &[("name", name.to_string())]),
                }
            }
            _ => String::new(),
        };
        json!({ "ports": list, "missingText": missing_text }).to_string()
    }

    /// Chooses the MIDI device by name; "" goes back to the first one. Ends a running practice.
    pub fn choose_midi(&mut self, name: &str) {
        let value = (!name.is_empty()).then(|| name.to_string());
        self.reconfigure(|s| s.midi_port = value);
    }

    /// The chosen output if it is there, else the system default.
    fn output_device(&self) -> Option<String> {
        let saved = self.settings.output_device.as_deref()?;
        if saved == kontra_session::NULL_DEVICE {
            return Some(saved.to_string());
        }
        chosen_output(Some(saved), &list_devices(Direction::Output))
    }

    /// Starts audio for the loaded arrangement; true if there is no sound output.
    fn start_live(&mut self) -> Result<bool, Unstarted> {
        self.check_audio_path();
        let Some(loaded) = self.loaded.as_ref() else {
            return Err(Unstarted::Other(self.tr("library.empty.hint", &[])));
        };
        let arrangement = loaded.arrangement.clone();
        self.end_tuner_and_ear();
        self.level.reset();
        let settings = self.score_settings(arrangement.instrument.kind);
        let options = self.practice_options();
        let setup = self.live_setup();
        self.release_standby();
        let mut live = match self.open_live(arrangement, settings, options, &setup) {
            Ok(live) => live,
            Err(LiveError::Input(e)) => {
                self.input.failed(InputUse::Practice, &e);
                return Err(Unstarted::Input);
            }
            Err(e @ LiveError::Setup(_)) => {
                let reason = e.to_string();
                return Err(Unstarted::Other(
                    self.tr("error.audio", &[("reason", reason)]),
                ));
            }
        };
        live.practice_mut()
            .set_speed(self.probe.map_or(self.speed, |p| p.speed));
        live.practice_mut().set_loop(self.loop_range());
        live.practice_mut()
            .set_gains(gains(&self.settings, &self.plugins));
        let silent = live.output_is_silent;
        self.live = Some(live);
        self.finished_at_s = None;
        self.input_opened(InputUse::Practice);
        self.run_started = Some(std::time::Instant::now());
        self.input_since = self.run_started;
        Ok(silent)
    }

    /// Devices and backing track for a run.
    fn live_setup(&self) -> LiveSetup {
        LiveSetup {
            input: self.input_choice(),
            output_device: self.output_device(),
            source: self.source(),
            // A backing file that is gone is skipped (play() says so), not an error.
            backing: self.backing().map(|(path, _)| path).filter(|p| p.exists()),
            original: self.backing_original(),
            backing_cache: Some(self.backing_cache_dir()),
            onset_gate_db: self.settings.onset_gate_db,
        }
    }

    /// Polls audio and session; call every ~16 ms.
    pub fn tick(&mut self) -> Vec<UiEvent> {
        self.tick_standby();
        self.take_walk();
        let mut raw = Vec::new();
        if let Some(live) = self.live.as_mut() {
            raw.extend(live.poll());
        }
        if let Some(tuner) = self.tuner.as_mut() {
            raw.extend(tuner.poll());
        }
        if let Some(live_ear) = self.live_ear.as_mut() {
            raw.extend(live_ear.poll());
        }
        self.input.observe(&raw);
        let ear_event = self.ear_heard(&mut raw);
        let mut events: Vec<UiEvent> = raw.into_iter().filter_map(|e| self.translate(e)).collect();
        events.extend(ear_event.into_iter().chain(self.notice_event()));
        events.extend(self.prepare_backing());
        events.extend(self.bluetooth_hint());
        events.extend(self.llm_poll());
        events.extend(self.poll_effects());
        events.extend(self.advise_level(&events));
        // The meter holds the peak the advice judges, so both say the same.
        let held = f64::from(self.level.held_peak_db());
        for event in &mut events {
            if let UiEvent::Level { peak_db, .. } = event {
                *peak_db = held;
            }
        }
        // Input before output when both failed in this tick.
        let lost = events
            .iter()
            .filter_map(|e| match e {
                UiEvent::DeviceLost(side, m) => Some((*side, m.clone())),
                _ => None,
            })
            .min_by_key(|(side, _)| *side != Direction::Input);
        if let Some((side, reason)) = lost {
            events.extend(self.device_lost(side, reason));
        }
        events.extend(self.watch_lost());
        // A probe's pass is no workshop pass: no pass count, no building-block step (QA
        // 27.10., P2 Jens: "pass 4: 10 %" after the probe).
        if self.probe.is_some() {
            events.retain(|e| !matches!(e, UiEvent::PassComplete { .. }));
        }
        let finished = events.iter().any(|e| matches!(e, UiEvent::Finished { .. }));
        let rate = events.iter().find_map(|e| match e {
            UiEvent::PassComplete { hit_rate, .. } => Some(*hit_rate),
            _ => None,
        });
        let rate = rate.or_else(|| {
            let live = self.live.as_ref().filter(|_| finished)?;
            Some(live.practice().session().summary().hit_rate)
        });
        if let Some(hint) = rate.and_then(|r| self.density_hint(r)) {
            events.push(hint);
        }
        let passed = events.iter().find_map(|e| match e {
            UiEvent::PassComplete { hit_rate, .. } => Some(*hit_rate),
            _ => None,
        });
        // Each pass or run of calls and answers is a try (LH-LRN-19).
        if let Some(rate) = rate {
            self.call_try(rate);
        }
        if let Some(rate) = passed {
            let steps = self.chain_pass(rate);
            events.extend(steps);
        }
        if finished {
            self.record_run();
            // A probe is one pass: the next play is the chosen mode again.
            if self.probe.is_some() {
                self.stop_live();
            }
        }
        if let Some(hint) = self.quantum_check() {
            events.push(UiEvent::Message(hint));
        }
        if let Some(hint) = self.direct_check() {
            events.push(UiEvent::Message(hint));
        }
        events.extend(self.poll_calibration());
        events.extend(self.poll_stems());
        events
    }

    /// LH-EIN-07: once per program run, a second after a live input started (tuner, setup or a
    /// run), a hint if the input's block is too long for the latency target. Before the first
    /// note where possible, not again on every run (QA 14.10.).
    fn quantum_check(&mut self) -> Option<String> {
        if self.quantum_checked {
            return None;
        }
        let since = self.input_since?;
        if since.elapsed() < std::time::Duration::from_secs(1) {
            return None;
        }
        let block = match (&self.live, &self.tuner) {
            (Some(live), _) => live.input_block(),
            (None, Some(tuner)) => tuner.input_block(),
            (None, None) => None,
        };
        let (frames, rate) = block?;
        self.quantum_checked = true;
        let ms = quantum_too_long_ms(frames, rate)?;
        Some(self.tr(
            "hint.quantum",
            &[("frames", frames.to_string()), ("ms", format!("{ms:.0}"))],
        ))
    }

    /// Once per program run, when a live input started: the interface has no PipeWire node and
    /// runs through ALSA directly; say so and how to get the node back (review #73).
    fn direct_check(&mut self) -> Option<String> {
        if self.direct_checked || self.input_since.is_none() {
            return None;
        }
        self.direct_checked = true;
        let InputChoice::Device { id: Some(id), .. } = self.input_choice() else {
            return None;
        };
        let devices = list_devices(Direction::Input);
        let device = devices.iter().find(|d| d.id == id && d.past_pipewire)?;
        Some(self.tr("hint.direct", &[("name", device.name.clone())]))
    }

    fn translate(&self, event: PracticeEvent) -> Option<UiEvent> {
        match event {
            PracticeEvent::Level {
                peak_db,
                rms_db,
                clipped,
            } => Some(UiEvent::Level {
                peak_db: f64::from(peak_db),
                rms_db: f64::from(rms_db),
                clipped,
            }),
            PracticeEvent::Tuner { note, cents } => Some(UiEvent::Tuner {
                midi: kontra_model::Pitch::parse(&note)
                    .map_or(0.0, |p| f64::from(p.0) + cents / 100.0),
                note: self.note_label(&note),
                cents,
            }),
            // A device error usually means the interface was unplugged (LH-ALL-04): the run ends
            // and the next play opens the device again.
            PracticeEvent::Error(side, m) => Some(UiEvent::DeviceLost(side, m)),
            // The device stopped delivering (LH-EIN-15): judging pauses, the player hears why.
            // Practice and tuner show it as the input's state; ear training only here.
            PracticeEvent::InputStalled | PracticeEvent::InputResumed
                if self.live_ear.is_none() =>
            {
                None
            }
            PracticeEvent::InputStalled => Some(UiEvent::Message(self.tr("input.stalled", &[]))),
            PracticeEvent::InputResumed => Some(UiEvent::Message(self.tr("input.resumed", &[]))),
            PracticeEvent::InputEnded | PracticeEvent::Heard { .. } => None,
            PracticeEvent::Session(e) => self.session_event(e),
        }
    }

    fn session_event(&self, event: SessionEvent) -> Option<UiEvent> {
        match event {
            SessionEvent::Result(r) => {
                let Some(index) = r.note else {
                    return Some(UiEvent::Extra);
                };
                let offset = r.offset_ms.unwrap_or(0.0);
                let state = match r.outcome {
                    // In wait mode the note stays open: show what was played instead of marking it.
                    Outcome::WrongPitch { played_midi }
                        if self.settings.mode == PracticeMode::Wait =>
                    {
                        let text = self.wait_text(index, Some(played_midi))?;
                        return Some(UiEvent::Holding { index, text });
                    }
                    Outcome::Extra { .. } => return Some(UiEvent::Extra),
                    outcome => state_of(outcome, offset),
                };
                Some(UiEvent::Judged {
                    index,
                    state,
                    offset_ms: offset,
                    cents: r.cents.unwrap_or(0.0),
                })
            }
            SessionEvent::Technique(t) => Some(UiEvent::Technique {
                index: t.sign,
                technique: technique_key(t.expected),
                result: technique_result_key(t.mark()),
            }),
            SessionEvent::Holding { note } => {
                let text = self.wait_text(note, None)?;
                Some(UiEvent::Holding { index: note, text })
            }
            SessionEvent::Released => Some(UiEvent::Released),
            SessionEvent::Reopened { note } => Some(UiEvent::Judged {
                index: note,
                state: "pending",
                offset_ms: 0.0,
                cents: 0.0,
            }),
            SessionEvent::PassComplete {
                clean, hit_rate, ..
            } => Some(UiEvent::PassComplete { clean, hit_rate }),
            SessionEvent::SpeedChanged(s) => Some(UiEvent::SpeedChanged(s)),
            SessionEvent::Finished => {
                let session = self.live.as_ref()?.practice().session();
                Some(UiEvent::Finished {
                    summary_json: self.summary_json(
                        &session.summary(),
                        session.results(),
                        session.technique_results(),
                    ),
                })
            }
        }
    }

    pub fn devices_json(&self, input: bool) -> String {
        let direction = if input {
            Direction::Input
        } else {
            Direction::Output
        };
        let devices = match direction {
            Direction::Input => self.answered(self.listed(direction)),
            Direction::Output => self.listed(direction),
        };
        // Input: the one Kontra takes when none is chosen (LH-EIN-02).
        let preferred = preferred_input(&devices, self.settings.input_kind)
            .filter(|_| input)
            .map(|d| d.id.clone());
        let list: Vec<_> = devices
            .iter()
            .map(|d| json!({ "id": d.id, "name": d.name, "default": d.is_default, "connection": format!("{:?}", d.connection), "preferred": preferred.as_ref() == Some(&d.id), "direct": d.past_pipewire, "usbRole": self.usb_role(d) }))
            .collect();
        serde_json::Value::Array(list).to_string()
    }
}

/// Plan exercises open in a run-through: evaluation and "next exercise" at the end (round 12).
const PLAN_MODE: PracticeMode = PracticeMode::Song;

/// Saved mode of workshop runs and of blind probes (DIDAKTIK I1).
const WORKSHOP_MODE: &str = "workshop";
const PROBE_MODE: &str = "probe";

/// A groove judged by time only sits when its spread is at most this (DIDAKTIK I4): then the
/// frets come in. Own choice (25.10.), not from a study; to be checked with real beginners.
const PULSE_SPREAD_MS: f64 = 25.0;

const SECONDS_PER_DAY: u64 = 24 * 60 * 60;

/// Sections are the same if their ends are this close (seconds).
const SECTION_TOLERANCE_S: f64 = 0.01;

/// A run's section is the loop `range`, as `same_section`.
fn same_range(section: Option<(f64, f64)>, range: LoopRange) -> bool {
    section.is_some_and(|(start_s, end_s)| {
        (start_s - range.start_s).abs() < SECTION_TOLERANCE_S
            && (end_s - range.end_s).abs() < SECTION_TOLERANCE_S
    })
}

/// True if `bars` names the section `start_s..end_s` (stored times, to the centisecond).
fn same_section(bars: &SectionBars, start_s: f64, end_s: f64) -> bool {
    (bars.start_s - start_s).abs() < SECTION_TOLERANCE_S
        && (bars.end_s - end_s).abs() < SECTION_TOLERANCE_S
}

/// Hits further off than this show as early or late.
const TIMING_MARK_MS: f64 = 25.0;

/// The view state of a judged note: "hit", "early", "late", "wrong", "offpitch", "missed".
fn state_of(outcome: Outcome, offset_ms: f64) -> &'static str {
    match outcome {
        Outcome::Hit if offset_ms < -TIMING_MARK_MS => "early",
        Outcome::Hit if offset_ms > TIMING_MARK_MS => "late",
        Outcome::Hit => "hit",
        Outcome::WrongPitch { .. } => "wrong",
        Outcome::OffPitch => "offpitch",
        Outcome::Missed | Outcome::Extra { .. } => "missed",
    }
}

/// Whether the chosen MIDI device is connected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MidiChoice {
    /// Chosen and connected, or none chosen.
    Found,
    /// Chosen, but not connected: the first device stands in.
    Missing,
}

/// Index of the MIDI port to use: the one named `chosen`, else the first. Settings from before
/// held the port id ("20:0"); it still finds its port.
fn pick_midi(
    ports: &[kontra_session::MidiPort],
    chosen: Option<&str>,
) -> (Option<usize>, MidiChoice) {
    let first = (!ports.is_empty()).then_some(0);
    let Some(name) = chosen else {
        return (first, MidiChoice::Found);
    };
    // A name first; ALSA hands old ids out again, so an id only when no name fits.
    let found = ports
        .iter()
        .position(|p| p.name == name)
        .or_else(|| ports.iter().position(|p| p.id == name));
    match found {
        Some(i) => (Some(i), MidiChoice::Found),
        None => (first, MidiChoice::Missing),
    }
}

/// Kante's key for a technique sign (KanteTechniqueMark); "" for those Kontra does not judge.
/// The tracks a bass can play, as (index in the file, name): drum parts are no choice.
fn playable(imported: &ImportResult) -> impl Iterator<Item = (usize, String)> + '_ {
    imported
        .tracks()
        .iter()
        .enumerate()
        .filter(|(_, t)| !t.is_percussion())
        .map(|(i, t)| (i, t.name().to_string()))
}

/// Speeds come in 5 % steps (LH-MOD-04).
const SPEED_STEPS_PER_UNIT: f64 = 20.0;
const SPEED_MIN: f64 = 0.25;
const SPEED_MAX: f64 = 1.5;

/// `speed` on the workshop's 5 % steps, 25 to 150 %: 0.72 → 0.7.
pub(crate) fn clamp_speed(speed: f64) -> f64 {
    (speed.clamp(SPEED_MIN, SPEED_MAX) * SPEED_STEPS_PER_UNIT).round() / SPEED_STEPS_PER_UNIT
}

fn technique_key(technique: Technique) -> &'static str {
    match technique {
        Technique::Slide => "slide",
        Technique::HammerOn => "hammer",
        Technique::PullOff => "pull",
        Technique::Slap => "slap",
        Technique::Pop => "pop",
        Technique::Ghost => "ghost",
        Technique::Muted => "dead",
        Technique::Harmonic
        | Technique::PalmMute
        | Technique::Vibrato
        | Technique::Bend
        | Technique::Staccato
        | Technique::Accent => "",
    }
}

/// Kante's key for a technique result; "" (the plain sign) for none, e.g. a hint not played.
fn technique_result_key(mark: Option<TechniqueOutcome>) -> &'static str {
    match mark {
        None => "",
        Some(TechniqueOutcome::Played) => "played",
        Some(TechniqueOutcome::Other(_)) => "other",
        Some(TechniqueOutcome::NotHeard) => "unheard",
    }
}

/// Quarter notes per bar: the beats the tracker follows.
fn quarters_per_bar(arrangement: &kontra_model::Arrangement) -> Vec<u32> {
    arrangement
        .bars
        .iter()
        .map(|b| {
            let quarters = b.signature.bar_ticks() as f64 / kontra_model::TICKS_PER_QUARTER as f64;
            quarters.round().max(1.0) as u32
        })
        .collect()
}

/// A tempo map that puts bar `i` at `times[i] - times[0]` seconds.
fn tempo_from_bars(
    arrangement: &kontra_model::Arrangement,
    times: &[f64],
) -> kontra_model::TempoMap {
    let mut changes = Vec::new();
    for (i, pair) in times.windows(2).enumerate() {
        let (Some(bar), seconds) = (arrangement.bars.get(i), pair[1] - pair[0]) else {
            continue;
        };
        if seconds <= 0.05 {
            continue;
        }
        let quarters = bar.signature.bar_ticks() as f64 / kontra_model::TICKS_PER_QUARTER as f64;
        changes.push(kontra_model::TempoChange {
            tick: bar.start_ticks,
            bpm: (quarters * 60.0 / seconds).clamp(20.0, 400.0),
        });
    }
    if changes.is_empty() {
        return arrangement.tempo.clone();
    }
    kontra_model::TempoMap::from_changes(changes)
}

/// The instrument choice of setup and settings (LH-EIN-09). Only a UI choice: `Midi` is the
/// stored bass with the MIDI input, the data keeps instrument and input apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstrumentChoice {
    EBass,
    Double,
    Midi,
}

fn instrument_from(settings: &Settings) -> kontra_model::Instrument {
    // Bass over MIDI (P5): the same strings for tab and range, scored as keys.
    let kind = match (settings.input_kind, settings.instrument) {
        (InputKind::Midi, _) => InstrumentKind::Keys,
        (_, Bass::Fretless) => InstrumentKind::Fretless,
        (_, Bass::Double) => InstrumentKind::Double,
        (_, Bass::Fretted) => InstrumentKind::Fretted,
    };
    let mut instrument = kontra_model::Instrument::new(kind, tuning_from(settings), 24)
        .unwrap_or_else(|_| kontra_model::Instrument::default_bass());
    instrument.a4_hz = settings.a4_hz.clamp(A4_MIN_HZ, A4_MAX_HZ);
    instrument
}

/// The profile's tuning (LH-TUN-02): standard for the string count, changed as chosen. A custom
/// tuning for another string count falls back to standard.
fn tuning_from(settings: &Settings) -> Tuning {
    let standard = match settings.strings {
        5 => Tuning::bass5_standard(),
        6 => Tuning::bass6_standard(),
        _ => Tuning::bass4_standard(),
    };
    let chosen = match settings.tuning {
        TuningPreset::Drop => drop_lowest(&standard),
        TuningPreset::HalfDown => standard.shifted("half-down", -1),
        TuningPreset::WholeDown => standard.shifted("whole-down", -2),
        TuningPreset::Custom if settings.custom_tuning.len() == standard.string_count() => {
            let names: Vec<&str> = settings.custom_tuning.iter().map(String::as_str).collect();
            Tuning::from_names("custom", &names).ok()
        }
        TuningPreset::Standard | TuningPreset::Custom => None,
    };
    chosen.unwrap_or(standard)
}

/// The lowest string a whole step down: E A D G → D A D G, B E A D G → A E A D G.
fn drop_lowest(standard: &Tuning) -> Option<Tuning> {
    let mut strings = standard.strings.clone();
    let lowest = strings.first_mut()?;
    *lowest = lowest.transposed(-2)?;
    Some(Tuning {
        name: "drop".into(),
        strings,
    })
}

/// Note names of a tuning's open strings, lowest first ("D1", "A1", …).
fn names_of(tuning: &Tuning) -> Vec<String> {
    tuning.strings.iter().map(ToString::to_string).collect()
}

#[cfg(test)]
mod tests;
