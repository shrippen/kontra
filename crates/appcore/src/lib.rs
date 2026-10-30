//! Qt-free core of the app: everything the QML bridge calls, testable without a GUI.
//!
//! ```text
//! QML ──invokables──► bridge (cxx-qt) ──► Core ──► kontra-session (Live, LiveTuner, service)
//!     ◄──properties/signals── bridge ◄── UiEvent
//! ```

mod caches;
mod calibrate;
mod course;
mod data;
pub mod ear;
mod i18n;
mod input_state;
mod learn;
pub mod level;
mod library;
mod package;
mod prepare;
pub mod quiz;
pub mod quiz_stats;
mod report;
mod songsterr;
mod sources;
mod stems;
mod store;
mod unplug;

pub use data::DataImport;
pub use i18n::{Catalog, Language};
pub use input_state::InputState;
pub use kontra_session::{exercise, Blocks, ChainOrder, Density};
pub use package::PackageAudio;
pub use store::{
    data_dir, AudioPath, Bass, InputKind, LatencyFrom, Library, PracticeLoop, RecentEntry,
    SectionBars, SectionDensity, Setlist, Settings,
};

use input_state::{InputUse, Unstarted};
use kontra_model::{Arrangement, InputSource, InstrumentKind, Judging, Technique, Tuning};
use kontra_session::{
    import_file, list_devices, run_from, technique_signs, Chain, Connection, DeviceInfo, Direction,
    Gains, ImportResult, InputChoice, Intonation, Leaning, Live, LiveCalibration, LiveEar,
    LiveError, LiveLoopback, LiveSetup, LiveTuner, LoopRange, Mode, NoteResult, Observation,
    Outcome, OutsideNote, Pocket, PocketVerdict, PracticeEvent, PracticeOptions, ProgressStore,
    Run, ScoreSettings, SessionEvent, Shaping, Side, Summary, TechniqueOutcome, TechniqueResult,
    Workshop,
};
use serde_json::json;
use std::path::{Path, PathBuf};

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
/// Tones of a double stop (R8): only its missed tone says "nur ein Ton des Griffs gehört".
const DOUBLE_STOP_TONES: usize = 2;
/// Tunings the settings offer (LH-TUN-02); "drop" lowers the lowest string a whole step.
const TUNINGS: [&str; 5] = ["standard", "drop", "half-down", "whole-down", "custom"];

/// Options for QA runs (planung/QA.md): a recording instead of the audio input.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct QaOptions {
    pub input_file: Option<PathBuf>,
    pub laptop_jack: bool,
}

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
    /// Between runs: offer another note density (LH-MOD-06), 1..=3, with the question.
    DensitySuggestion {
        level: u8,
        text: String,
    },
    /// Another input could be used meanwhile (LH-ALL-04): its id and a question for the user.
    InputOffer {
        id: String,
        text: String,
    },
    /// The lost input is back and used again.
    InputBack(String),
    /// Bass removal of the backing track changed (`Core::stems_json`).
    Stems(String),
    /// The backing track is being prepared (true) or ready (false); play waits for it.
    BackingLoading(bool),
    /// Building blocks moved on (DIDAKTIK I6): the new step (`Core::chain_json`) is the loop,
    /// its run starts with a count-in.
    ChainStep {
        chain_json: String,
    },
    /// Calibration finished: new latency (if measured) and a text for the user.
    Calibrated {
        latency_ms: Option<f64>,
        text: String,
    },
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
    tuner: Option<LiveTuner>,
    calibration: Option<LiveCalibration>,
    loopback: Option<LiveLoopback>,
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
    /// A note for the user from the last `open` (e.g. a five-string bass was chosen).
    notice: Option<String>,
    /// The setlist being played and the position in it (LH-BIB-04).
    active_setlist: Option<(String, usize)>,
    /// The input that went away, watched until it is back (LH-ALL-04).
    lost: Option<unplug::Lost>,
    /// What the input does, for the player (LH-EIN-15).
    input: input_state::InputStatus,
    /// Input used instead of the lost one; never saved.
    fallback_input: Option<String>,
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
    /// Where the browser saves downloads (LH-BIB-11).
    downloads: Option<PathBuf>,
    /// Songsterr search (LH-BIB-11, round 12).
    finder: songsterr::Finder,
    /// The blind probe running now (DIDAKTIK I1), if any.
    probe: Option<Probe>,
    /// Building blocks in the workshop (DIDAKTIK I6), if chosen, and the loop chosen before them.
    chain: Option<(Chain, Option<(usize, usize)>)>,
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
    let settings: Settings = store::load(&data_dir.join("settings.json"));
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
        let settings: Settings = store::load(&data_dir.join("settings.json"));
        let mut library: Library = store::load(&data_dir.join("library.json"));
        library.plain_paths();
        library.find_moved(&Self::exercises_dir());
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
            downloads: kontra_session::download_dir(),
            finder: songsterr::Finder::default(),
            probe: None,
            chain: None,
            notice: None,
            active_setlist: None,
            lost: None,
            input: input_state::InputStatus::new(),
            fallback_input: None,
            xruns_done: 0,
            catalog: Catalog::new(language),
            settings,
            library,
            data_dir: data_dir.to_path_buf(),
            qa,
            loaded: None,
            pending: None,
            live: None,
            tuner: None,
            calibration: None,
            loopback: None,
            run_started: None,
            quantum_checked: false,
            direct_checked: false,
            input_since: None,
            speed: 1.0,
            looping: None,
        };
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
        settings.strictness = match self.settings.strictness.as_str() {
            "loose" => kontra_session::Strictness::Loose,
            "normal" => kontra_session::Strictness::Normal,
            "strict" => kontra_session::Strictness::Strict,
            "custom" => kontra_session::Strictness::Custom {
                window_ms: self.settings.window_ms.clamp(WINDOW_MIN_MS, WINDOW_MAX_MS),
            },
            _ => settings.strictness,
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

    /// Progress over all pieces (LH-FOR-01): per piece best result, best tempo and time, and
    /// every run's time stamp and length so the view can sum days in local time.
    pub fn progress_json(&self) -> String {
        self.progress_json_at(unix_now())
    }

    /// In level order, the first bundled exercise that does not sit yet (see `sits`),
    /// apart from `skip` (the one just played): `{title, path, best, mastered, level}`.
    fn next_exercise(&self, skip: Option<&Path>) -> Option<serde_json::Value> {
        self.bundled_levels().into_iter().find_map(|level| {
            let mut item = level["items"]
                .as_array()?
                .iter()
                .filter(|i| skip.is_none_or(|p| i["path"].as_str() != p.to_str()))
                .find(|i| i["mastered"] != true)?
                .clone();
            item["level"] = level["level"].clone();
            Some(item)
        })
    }

    /// Start page: the last piece played, the first bundled exercise that does not sit yet,
    /// the next lesson (LH-LRN-07) and the practice of the last seven days.
    /// `{"last": {path, title, artist, best, practiceMin} | null,
    ///   "next": {title, path, best, level} | null, "lesson": {id, title} | null, "review": due,
    ///   "week": {minutes, runs}}`
    pub fn home_json(&self) -> String {
        self.home_json_at(unix_now())
    }

    fn home_json_at(&self, now: u64) -> String {
        let store = self.progress_store();
        let last = self
            .library
            .recent
            .iter()
            .max_by_key(|e| e.last_unix)
            .map(|e| {
                let progress = store.load_id(&e.song_id);
                json!({
                    "path": e.path,
                    "title": self.library_title(e),
                    "artist": e.artist,
                    "best": progress.as_ref().and_then(|p| p.best_hit_rate()),
                    "practiceMin": progress.as_ref().map_or(0.0, |p| p.practice_s() / 60.0),
                })
            });

        let next = self.next_exercise(None);

        let since = now.saturating_sub(WEEK_DAYS * SECONDS_PER_DAY);
        let week: Vec<_> = store
            .all()
            .into_iter()
            .flat_map(|p| p.runs)
            .filter(|r| r.finished_unix > since)
            .collect();
        let minutes = week.iter().map(|r| r.practice_s).sum::<f64>() / 60.0;
        json!({
            "last": last,
            "next": next,
            "spots": self.hard_spots(&store, now),
            "probe": self.probe_offer(&store, now),
            "lesson": self.next_lesson(),
            "review": self.review_count(now),
            "week": { "minutes": minutes, "runs": week.len() },
        })
        .to_string()
    }

    /// "Spots from yesterday" (round 11, DIDAKTIK: places come back over days): from the piece
    /// practised last in the recent days, the bars that were among the hardest most often (ties:
    /// the later one). Runs of the last hours wait; a place comes back after a night.
    ///
    /// ```text
    /// now − 3 days ─────────── runs that count ─────────── now − 8 h ── today's runs ── now
    /// ```
    fn hard_spots(&self, store: &ProgressStore, now: u64) -> Option<serde_json::Value> {
        let from = now.saturating_sub(SPOT_DAYS * SECONDS_PER_DAY);
        let until = now.saturating_sub(SPOT_REST_S);
        let counts = |r: &Run| r.finished_unix > from && r.finished_unix <= until;
        let (song, runs) = store
            .all()
            .into_iter()
            .filter_map(|p| {
                let runs: Vec<Run> = p
                    .runs
                    .into_iter()
                    .filter(|r| counts(r) && !r.hardest_bars.is_empty())
                    .collect();
                let latest = runs.iter().map(|r| r.finished_unix).max()?;
                Some((latest, p.song_id, runs))
            })
            .max_by_key(|(latest, _, _)| *latest)
            .map(|(_, song, runs)| (song, runs))?;
        let entry = self.library.recent.iter().find(|e| e.song_id == song)?;
        if !entry.path.exists() {
            return None;
        }

        // Per bar: how often it was among the hardest, and when last.
        let mut bars: Vec<(u32, usize, u64)> = Vec::new();
        for run in &runs {
            for bar in &run.hardest_bars {
                match bars.iter_mut().find(|(b, _, _)| b == bar) {
                    Some(seen) => {
                        seen.1 += 1;
                        seen.2 = seen.2.max(run.finished_unix);
                    }
                    None => bars.push((*bar, 1, run.finished_unix)),
                }
            }
        }
        bars.sort_by_key(|(_, n, last)| std::cmp::Reverse((*n, *last)));
        let picked: Vec<u32> = bars.iter().take(SPOTS).map(|(b, _, _)| *b).collect();
        Some(json!({
            "path": entry.path,
            "title": self.library_title(entry),
            "bars": picked,
        }))
    }

    /// "Check without help" (DIDAKTIK I1): the section practised last in the workshop in the
    /// recent days, rested a night like the spots, and not probed since.
    fn probe_offer(&self, store: &ProgressStore, now: u64) -> Option<serde_json::Value> {
        let from = now.saturating_sub(SPOT_DAYS * SECONDS_PER_DAY);
        let until = now.saturating_sub(SPOT_REST_S);
        let mut latest: Option<(u64, &RecentEntry, &SectionBars)> = None;
        for progress in store.all() {
            // Only pieces the library can open, with sections it can name (a newer one it
            // cannot must not hide an older one).
            let Some(entry) = self
                .library
                .recent
                .iter()
                .find(|e| e.song_id == progress.song_id && e.path.exists())
            else {
                continue;
            };
            for run in &progress.runs {
                let Some((start_s, end_s)) = run.section else {
                    continue;
                };
                if run.mode != WORKSHOP_MODE
                    || run.finished_unix <= from
                    || run.finished_unix > until
                    || latest.is_some_and(|(t, _, _)| t >= run.finished_unix)
                {
                    continue;
                }
                let range = LoopRange { start_s, end_s };
                let probed = progress.runs.iter().any(|r| {
                    r.mode == PROBE_MODE
                        && r.finished_unix > run.finished_unix
                        && same_range(r.section, range)
                });
                if probed {
                    continue;
                }
                let Some(bars) = entry
                    .sections
                    .iter()
                    .find(|b| same_section(b, start_s, end_s))
                else {
                    continue;
                };
                latest = Some((run.finished_unix, entry, bars));
            }
        }
        let (_, entry, bars) = latest?;
        Some(json!({
            "path": entry.path,
            "title": self.library_title(entry),
            "from": bars.from_bar,
            "to": bars.to_bar,
            "startS": bars.start_s,
            "endS": bars.end_s,
        }))
    }

    /// Bars of the sections practised in the piece with `song_id`, as the library knows them.
    fn section_names(&self, song_id: &str) -> &[SectionBars] {
        self.library
            .recent
            .iter()
            .find(|e| e.song_id == song_id)
            .map_or(&[], |e| e.sections.as_slice())
    }

    /// A piece's title for progress and report: a bundled exercise in the UI's language (the
    /// stored `title` is the German progress key), else as stored.
    fn song_title(&self, song_id: &str, title: &str) -> String {
        self.library
            .recent
            .iter()
            .find(|e| e.song_id == song_id)
            .map_or_else(|| title.to_string(), |e| self.library_title(e))
    }

    fn progress_json_at(&self, now: u64) -> String {
        let mut songs = self.progress_store().all();
        songs.sort_by_key(|p| std::cmp::Reverse(p.runs.iter().map(|r| r.finished_unix).max()));
        let pieces: Vec<serde_json::Value> = songs
            .iter()
            .map(|p| {
                json!({
                    "songId": p.song_id,
                    "title": self.song_title(&p.song_id, &p.title),
                    "artist": p.artist,
                    "best": p.best_hit_rate(),
                    "bestSpeed": p.best_speed(0.9),
                    "practiceMin": p.practice_s() / 60.0,
                    "runs": p.runs.len(),
                    "lastUnix": p.runs.iter().map(|r| r.finished_unix).max(),
                    "sections": self.sections_json(&p.runs, self.section_names(&p.song_id), now),
                })
            })
            .collect();
        let runs: Vec<serde_json::Value> = songs
            .iter()
            .flat_map(|p| p.runs.iter())
            .map(|r| json!({ "unix": r.finished_unix, "s": r.practice_s }))
            .collect();
        json!({ "pieces": pieces, "runs": runs }).to_string()
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
            quiz: store::load(&self.data_dir.join(quiz_stats::FILE)),
        };
        store::save(path, &bundle)
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
                self.update_settings(|s| *s = bundle.settings);
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
                store::load::<quiz_stats::QuizStats>(&quiz_file).merged(bundle.quiz)
            }
        };
        store::save(&quiz_file, &quiz)
            .map_err(|e| self.tr("data.error.write", &[("reason", e.to_string())]))?;
        self.quiz = None;
        self.save_library();
        let key = match how {
            DataImport::Merge => "data.merged",
            DataImport::Replace => "data.replaced",
        };
        Ok(self.tr(
            key,
            &[
                ("pieces", merged.pieces.to_string()),
                ("setlists", merged.setlists.to_string()),
                ("runs", merged.runs.to_string()),
                ("name", file_name(path)),
            ],
        ))
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

    /// The note left by the last `open`, once.
    pub fn take_notice(&mut self) -> Option<String> {
        self.notice.take()
    }

    /// The view for the loaded piece: the one last used with it, else the default (LH-ANZ-04).
    pub fn view(&self) -> String {
        self.loaded_path()
            .and_then(|p| self.library.entry(p))
            .and_then(|e| e.view.clone())
            .unwrap_or_else(|| self.settings.view.clone())
    }

    /// Remembers `view` for the loaded piece and as the default for pieces opened next.
    pub fn set_view(&mut self, view: &str) {
        self.update_settings(|s| s.view = view.to_string());
        if let Some(path) = self.loaded_path().map(Path::to_path_buf) {
            if let Some(entry) = self.library.entry_mut(&path) {
                entry.view = Some(view.to_string());
            }
            let _ = store::save(&self.data_dir.join("library.json"), &self.library);
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
        if let Some(code) = &self.settings.language {
            self.catalog = Catalog::new(Language::from_code(code));
        }
        let _ = store::save(&self.data_dir.join("settings.json"), &self.settings);
    }

    /// Changes settings the audio depends on (mode, instrument, devices, latency, metronome):
    /// saves them and ends a running practice, the next `play` uses the new ones.
    pub fn reconfigure(&mut self, change: impl FnOnce(&mut Settings)) {
        self.update_settings(change);
        self.stop_live();
    }

    /// Changes the instrument profile (strings, tuning, reference pitch) so that it acts at once:
    /// a loaded piece without its own tuning is laid out on the new strings again (LH-TUN-02) and
    /// a running tuner listens with the new strings and reference.
    pub fn retune(&mut self, change: impl FnOnce(&mut Settings)) -> Result<(), String> {
        let tuner_running = self.tuner.is_some();
        self.reconfigure(change);
        if let Some(track) = self.loaded.as_ref().map(|l| l.track) {
            self.reload(track)?;
        }
        if tuner_running {
            self.start_tuner()?;
        }
        Ok(())
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
    pub fn set_tuning(&mut self, name: &str) -> Result<(), String> {
        if !TUNINGS.contains(&name) {
            return Ok(());
        }
        let current = names_of(&tuning_from(&self.settings));
        self.retune(|s| {
            if name == "custom" && s.custom_tuning.len() != current.len() {
                s.custom_tuning = current;
            }
            s.tuning = name.to_string();
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
            s.tuning = "custom".into();
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
            s.strictness = "custom".into();
        });
        ms
    }

    /// Changes the mixer (volumes, monitoring) and saves; a running practice hears it at once.
    /// Returns a hint when monitoring is on but cannot work.
    pub fn remix(&mut self, change: impl FnOnce(&mut Settings)) -> Option<String> {
        self.update_settings(change);
        let gains = gains(&self.settings);
        if let Some(live) = self.live.as_mut() {
            live.practice_mut().set_gains(gains);
        }
        if let Some(live_ear) = self.live_ear.as_mut() {
            live_ear.set_gains(ear_gains(&self.settings));
        }
        self.monitor_hint()
    }

    /// Monitoring is on, but the running practice has no output or devices at different rates.
    fn monitor_hint(&self) -> Option<String> {
        let live = self.live.as_ref()?;
        if !self.settings.monitor || live.can_monitor() || live.output_is_silent {
            return None;
        }
        Some(self.tr("hint.monitor.rate", &[]))
    }

    /// Imports a score; picks the bass track or asks.
    pub fn open(&mut self, path: &Path) -> Result<Opened, String> {
        self.active_setlist = None;
        if package::is_package(path) {
            return self.open_package(path).map(|()| Opened::Ready);
        }
        let imported = import_file(path).map_err(|e| self.import_error(&e))?;
        // Round 12: plan exercises in a run-through, own pieces in the mode last chosen for them.
        let mode = if self.is_bundled(path) {
            Some(PLAN_MODE.to_string())
        } else {
            self.settings.piece_mode.clone()
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
                self.notice = Some(self.tr(
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
            backing: None,
            backing_offset_s: 0.0,
            view: None,
            tags: Vec::new(),
            sections: Vec::new(),
            densities: Vec::new(),
            transpose_semitones: 0,
            backing_without_bass: false,
            loops: Vec::new(),
            target_speed: None,
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
        let _ = store::save(&self.data_dir.join("library.json"), &self.library);
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

    /// Notes for the views: time and duration in seconds, string/fret, beats, midi.
    pub fn notes_json(&self) -> String {
        let Some(a) = self.arrangement() else {
            return "[]".into();
        };
        let positions = hand_positions(&a.notes);
        let techniques = technique_signs(a);
        let notes: Vec<_> = a
            .notes
            .iter()
            .zip(positions)
            .enumerate()
            .map(|(i, (n, position))| {
                let time = a.tempo.seconds_at(n.start_ticks);
                let fret = n.fret.unwrap_or(0);
                json!({
                    "time": time,
                    "duration": a.tempo.seconds_at(n.end_ticks()) - time,
                    "string": n.string.unwrap_or(0),
                    "fret": n.fret.unwrap_or(0),
                    "midi": n.pitch.0,
                    "beats": written_beats(n.duration_ticks),
                    "tied": n.tied,
                    "state": "pending",
                    // Hand position (LH-ANZ-05) and the finger it suggests (0 = open string).
                    "position": position,
                    "finger": if fret == 0 { 0 } else { fret + 1 - position.min(fret) },
                    // Kante's technique sign (LH-BEW-06); a slide shows where it starts.
                    "technique": techniques.get(&i).map_or("", |t| technique_key(*t)),
                    "techniqueResult": "",
                })
            })
            .collect();
        serde_json::Value::Array(notes).to_string()
    }

    pub fn bars_json(&self) -> String {
        let Some(a) = self.arrangement() else {
            return "[]".into();
        };
        let bars: Vec<_> = a
            .bars
            .iter()
            .enumerate()
            .map(|(i, b)| {
                json!({
                    "time": a.tempo.seconds_at(b.start_ticks),
                    "number": b.source_index + 1,
                    "numerator": b.signature.numerator,
                    "denominator": b.signature.denominator,
                    // Written repeat signs and volta numbers (LH-ANZ-02), e.g. "endings": [1, 2].
                    "repeatStart": b.repeat_open,
                    "repeatEnd": b.repeat_close,
                    "endings": (0..32).filter(|i| b.endings & (1 << i) != 0).map(|i| i + 1).collect::<Vec<u32>>(),
                    // Key per bar and chord symbols (Kante 1.27 KanteBassStaff).
                    "keyFifths": b.key_fifths,
                    "chord": a.markers.iter().find(|m| m.tick == b.start_ticks && is_chord(m)).map(|m| m.name.clone()),
                    // Rehearsal marks (Kante 1.50): the parts beginning in this bar.
                    "section": bar_sections(a, i)
                        .map(|names| names.iter().map(|n| self.part_name(n)).collect::<Vec<_>>()),
                })
            })
            .collect();
        serde_json::Value::Array(bars).to_string()
    }

    /// A part name in the UI's language where Kontra's generator wrote it ("Strophe" → "Verse");
    /// any other name as the file has it. Intro, Bridge, Outro and Chorus read alike.
    fn part_name(&self, name: &str) -> String {
        GENERATED_PARTS
            .iter()
            .find(|(written, _)| *written == name)
            .map_or_else(|| name.to_string(), |(_, key)| self.tr(key, &[]))
    }

    /// The notation view (Kante `KanteBassStaff`): notes in the order of `notes_json`, rests,
    /// key signature.
    pub fn staff_json(&self) -> String {
        self.arrangement()
            .map_or_else(empty_staff, staff)
            .to_string()
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

    pub fn position_s(&self) -> f64 {
        self.live
            .as_ref()
            .map_or(0.0, |l| l.practice().position_s())
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
        if self.output_is_bluetooth() {
            events.push(UiEvent::Message(self.tr("hint.bluetooth", &[])));
        }
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
        }
        self.input.stopped(InputUse::Practice);
        self.probe = None;
    }

    /// Audio dropouts since Kontra started, over all runs (LH-NF-04).
    pub fn xruns(&self) -> u64 {
        self.xruns_done + self.live.as_ref().map_or(0, Live::xruns)
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
    pub fn set_mode(&mut self, mode: &str) -> bool {
        let changed = mode != "workshop" && self.stop_chain();
        let own = self.loaded_path().is_some_and(|p| !self.is_bundled(p));
        self.reconfigure(|s| {
            s.mode = mode.to_string();
            if own {
                s.piece_mode = Some(mode.to_string());
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

    /// Returns true if the notes changed: the section has another note density (LH-MOD-06).
    pub fn set_loop(&mut self, from: usize, to: usize) -> bool {
        // Bars chosen by hand end the building blocks.
        self.chain = None;
        self.loop_to(from, to)
    }

    fn loop_to(&mut self, from: usize, to: usize) -> bool {
        self.looping = Some((from.min(to), from.max(to)));
        let range = self.loop_range();
        if let Some(live) = self.live.as_mut() {
            live.practice_mut().set_loop(range);
        }
        self.rethin()
    }

    /// Sets one end of the loop to the bar at `position_s` (keys `[` and `]`, LH-MOD-07); the
    /// other end stays where it is unless it would cross, e.g. loop 3–6, `]` in bar 2 → 2–2.
    /// Returns true if the notes changed, as `set_loop`.
    pub fn set_loop_edge(&mut self, edge: LoopEdge, position_s: f64) -> bool {
        let Some(a) = self.arrangement() else {
            return false;
        };
        let Some(bar) = a.bar_at(a.tempo.tick_at(position_s.max(0.0))) else {
            return false;
        };
        let (from, to) = match (edge, self.looping) {
            (LoopEdge::Start, Some((_, to))) if to >= bar => (bar, to),
            (LoopEdge::End, Some((from, _))) if from <= bar => (from, bar),
            _ => (bar, bar),
        };
        self.set_loop(from, to)
    }

    /// Back to the start: the loop's first bar if a loop is set, else the piece's (LH-MOD-07).
    /// Ends the run; returns the position the next `play` starts from.
    pub fn rewind(&mut self) -> f64 {
        self.stop_live();
        self.loop_range().map_or(0.0, |r| r.start_s)
    }

    /// Returns true if the notes changed, as `set_loop`.
    pub fn clear_loop(&mut self) -> bool {
        self.looping = None;
        self.chain = None;
        if let Some(live) = self.live.as_mut() {
            live.practice_mut().set_loop(None);
        }
        self.rethin()
    }

    /// Keeps the current loop with the piece (LH-BIB-07: a package hands it on); a loop kept
    /// already is not kept twice. Returns true if one was added.
    pub fn save_loop(&mut self) -> bool {
        let Some((from, to)) = self.looping else {
            return false;
        };
        let Some(entry) = self.loaded_entry_mut() else {
            return false;
        };
        let practice = PracticeLoop { from, to };
        if entry.loops.contains(&practice) {
            return false;
        }
        entry.loops.push(practice);
        self.save_library();
        true
    }

    /// Drops the kept loop at `index` (as `saved_loops` lists them).
    pub fn remove_loop(&mut self, index: usize) {
        let Some(entry) = self.loaded_entry_mut() else {
            return;
        };
        if index >= entry.loops.len() {
            return;
        }
        entry.loops.remove(index);
        self.save_library();
    }

    /// Loops kept with the loaded piece, as bar indices `(from, to)`.
    pub fn saved_loops(&self) -> Vec<(usize, usize)> {
        self.loaded_path()
            .and_then(|p| self.library.entry(p))
            .map_or_else(Vec::new, |e| {
                e.loops.iter().map(|l| (l.from, l.to)).collect()
            })
    }

    /// Kept loops for the UI: `[{"from", "to"}]`, bar indices from 0.
    pub fn saved_loops_json(&self) -> String {
        let loops: Vec<_> = self
            .saved_loops()
            .into_iter()
            .map(|(from, to)| json!({ "from": from, "to": to }))
            .collect();
        serde_json::Value::from(loops).to_string()
    }

    /// The workshop's target speed for the loaded piece (1.0 = as written).
    pub fn target_speed(&self) -> f64 {
        self.loaded_path()
            .and_then(|p| self.library.entry(p))
            .and_then(|e| e.target_speed)
            .unwrap_or(1.0)
    }

    /// Sets the target speed of the loaded piece, in the workshop's 5 % steps.
    pub fn set_target_speed(&mut self, speed: f64) {
        let Some(entry) = self.loaded_entry_mut() else {
            return;
        };
        entry.target_speed = Some(clamp_speed(speed));
        self.save_library();
    }

    fn loaded_entry_mut(&mut self) -> Option<&mut RecentEntry> {
        let path = self.loaded_path()?.to_path_buf();
        self.library.entry_mut(&path)
    }

    /// Indices of the bars from `start_s` to `end_s` (a saved section), if bars begin and end
    /// there.
    pub fn bars_at(&self, start_s: f64, end_s: f64) -> Option<(usize, usize)> {
        let a = self.arrangement()?;
        let at = |s: f64, t: u64| (a.tempo.seconds_at(t) - s).abs() < SECTION_TOLERANCE_S;
        let from = a.bars.iter().position(|b| at(start_s, b.start_ticks))?;
        let to = a.bars.iter().position(|b| at(end_s, b.end_ticks()))?;
        Some((from, to))
    }

    /// Note density of the current section (the loop, else the whole piece) and its lock.
    pub fn density(&self) -> (Density, DensityLock) {
        self.stored_density()
            .map_or((Density::Full, DensityLock::Free), |d| {
                let lock = if d.fixed {
                    DensityLock::Fixed
                } else {
                    DensityLock::Free
                };
                (Density::from_level(d.level), lock)
            })
    }

    /// Sets the note density of the current section (LH-MOD-06); the next run plays it.
    pub fn set_density(&mut self, density: Density, lock: DensityLock) {
        let section = self.section_key();
        let Some(path) = self.loaded_path().map(Path::to_path_buf) else {
            return;
        };
        let Some(entry) = self.library.entry_mut(&path) else {
            return;
        };
        entry.densities.retain(|d| d.section != section);
        entry.densities.push(SectionDensity {
            section,
            level: density.level(),
            fixed: lock == DensityLock::Fixed,
        });
        self.save_library();
        self.rethin();
    }

    /// The loop as stored with a section's density.
    fn section_key(&self) -> Option<(f64, f64)> {
        self.loop_range().map(|r| (r.start_s, r.end_s))
    }

    /// The density stored for the current section, else the whole piece's.
    fn stored_density(&self) -> Option<&SectionDensity> {
        let section = self.section_key();
        let entry = self.library.entry(self.loaded_path()?)?;
        let own = entry.densities.iter().find(|d| match (d.section, section) {
            (Some((a, b)), Some((c, e))) => (a - c).abs() < 0.01 && (b - e).abs() < 0.01,
            _ => false,
        });
        own.or_else(|| entry.densities.iter().find(|d| d.section.is_none()))
    }

    /// Thins the loaded notes to the current density; true if they changed, which ends a
    /// running practice (densities change only between runs).
    fn rethin(&mut self) -> bool {
        let (density, _) = self.density();
        let Some(loaded) = self.loaded.as_mut() else {
            return false;
        };
        let thinned = density.thin(&loaded.full);
        if thinned.notes == loaded.arrangement.notes {
            return false;
        }
        loaded.arrangement = thinned;
        self.stop_live();
        true
    }

    /// The suggestion after a pass or run with `hit_rate` (LH-MOD-06): song and workshop only,
    /// not for a fixed section, not when switched off, not after a probe (it only judges).
    fn density_hint(&self, hit_rate: f64) -> Option<UiEvent> {
        if !self.settings.adaptive || !matches!(self.settings.mode.as_str(), "song" | "workshop") {
            return None;
        }
        if self.probe.is_some() {
            return None;
        }
        let (density, lock) = self.density();
        if lock == DensityLock::Fixed {
            return None;
        }
        let next = density.suggest(hit_rate)?;
        let key = if next > density {
            "density.more"
        } else {
            "density.fewer"
        };
        Some(UiEvent::DensitySuggestion {
            level: next.level(),
            text: self.tr(key, &[("percent", format!("{:.0}", hit_rate * 100.0))]),
        })
    }

    /// Starts building blocks (DIDAKTIK I6), a few bars each or phrases, over the chosen bars
    /// (with blocks running: the bars they started from), or over the whole piece when they
    /// make only one block; the loop becomes the first step. Returns true if the notes changed,
    /// as `set_loop`.
    pub fn start_chain(&mut self, blocks: Blocks, order: ChainOrder) -> bool {
        let Some(bars) = self.arrangement().map(|a| a.bars.len()).filter(|n| *n > 0) else {
            return false;
        };
        // Running blocks loop one step: a new order or size cuts the bars chosen before them.
        let chosen = match self.chain.as_ref() {
            Some((_, chosen)) => *chosen,
            None => self.looping,
        };
        let Some(a) = self.arrangement() else {
            return false;
        };
        let hesitant = self.hesitant_bars();
        let cut = |bars| Chain::new(a, bars, blocks, &hesitant, order, &Workshop::default());
        // Chosen bars that make one block leave nothing to join: the whole piece instead.
        let chain = match chosen.map(cut) {
            Some(chain) if chain.count() > 1 => chain,
            _ => cut((0, bars - 1)),
        };
        let (from, to) = chain.step();
        self.chain = Some((chain, chosen));
        self.loop_to(from, to)
    }

    /// Bars (indices) the player keeps hesitating at, from the piece's last runs; phrases start
    /// there.
    fn hesitant_bars(&self) -> Vec<usize> {
        let (Some(loaded), Some(a)) = (self.loaded.as_ref(), self.arrangement()) else {
            return Vec::new();
        };
        let Ok(progress) = self.progress_store().load(&loaded.full) else {
            return Vec::new();
        };
        let mut counts = std::collections::BTreeMap::new();
        for run in progress.runs.iter().rev().take(HESITATION_RUNS) {
            for bar in &run.hesitant_bars {
                *counts.entry(*bar).or_insert(0) += 1;
            }
        }

        // Stored as printed numbers; with repeats a number names several bars.
        a.bars
            .iter()
            .enumerate()
            .filter(|(_, bar)| {
                counts
                    .get(&(bar.source_index + 1))
                    .is_some_and(|n| *n >= HESITATION_MIN)
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Ends the building blocks; the loop goes back to the one chosen before them (none, if
    /// none was). Returns true if the notes changed, as `set_loop`.
    pub fn stop_chain(&mut self) -> bool {
        let Some((_, chosen)) = self.chain.take() else {
            return false;
        };
        match chosen {
            Some((from, to)) => self.loop_to(from, to),
            None => self.clear_loop(),
        }
    }

    /// The building blocks for the UI, or `null`: `{step, steps, last, text}`, `text` like
    /// "Baustein: Takte 3–4 · Schritt 2 von 5".
    pub fn chain_json(&self) -> String {
        let Some((chain, _)) = self.chain.as_ref() else {
            return "null".into();
        };
        let (from, to) = chain.step();
        let number = |bar: usize| {
            self.arrangement()
                .and_then(|a| a.bars.get(bar))
                .map_or(bar + 1, |b| b.source_index as usize + 1)
        };
        let key = if chain.is_last() {
            "chain.whole"
        } else if from == to {
            "chain.step.one"
        } else {
            "chain.step"
        };
        let text = self.tr(
            key,
            &[
                ("from", number(from).to_string()),
                ("to", number(to).to_string()),
                ("step", (chain.index() + 1).to_string()),
                ("steps", chain.count().to_string()),
            ],
        );
        json!({
            "step": chain.index(),
            "steps": chain.count(),
            "last": chain.is_last(),
            "text": text,
        })
        .to_string()
    }

    /// After a workshop pass: when the step sits, its run ends (and is saved) and the next step
    /// starts with a count-in. Not within the run: clicks and backing are planned ahead.
    fn chain_pass(&mut self, hit_rate: f64) -> Vec<UiEvent> {
        if self.settings.mode != "workshop" || self.probe.is_some() {
            return Vec::new();
        }
        let Some((from, to)) = self.chain.as_mut().and_then(|(c, _)| c.on_pass(hit_rate)) else {
            return Vec::new();
        };
        self.stop_live();
        self.loop_to(from, to);
        let mut events = vec![UiEvent::ChainStep {
            chain_json: self.chain_json(),
        }];
        events.extend(self.play());
        events
    }

    /// Loop range in seconds, or (-1, -1).
    pub fn loop_seconds(&self) -> (f64, f64) {
        self.loop_range()
            .map_or((-1.0, -1.0), |r| (r.start_s, r.end_s))
    }

    fn loop_range(&self) -> Option<LoopRange> {
        let (from, to) = self.looping?;
        let a = self.arrangement()?;
        let first = a.bars.get(from)?;
        let last = a.bars.get(to)?;
        Some(LoopRange {
            start_s: a.tempo.seconds_at(first.start_ticks),
            end_s: a.tempo.seconds_at(last.end_ticks()),
        })
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
        match self.settings.mode.as_str() {
            "song" => Mode::Song,
            "wait" => Mode::Wait,
            "workshop" => Mode::Workshop(Workshop {
                target_speed: self.target_speed(),
                ..Workshop::default()
            }),
            _ => Mode::Play,
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

    /// The chosen input, else an interface before the built-in card (LH-EIN-02).
    fn input_device(&self) -> Option<String> {
        chosen_input(
            self.settings.input_device.as_deref(),
            &list_devices(Direction::Input),
        )
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
        self.tuner = None;
        self.live_ear = None;
        self.level.reset();
        let arrangement = loaded.arrangement.clone();
        let settings = self.score_settings(arrangement.instrument.kind);
        let options = self.practice_options();
        let setup = self.live_setup();
        let mut live = match Live::start(arrangement, self.mode(), settings, options, &setup) {
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
        live.practice_mut().set_gains(gains(&self.settings));
        let silent = live.output_is_silent;
        self.live = Some(live);
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

    /// Starts the tuner; a failed input shows as the input's state (the text is for callers
    /// without one).
    pub fn start_tuner(&mut self) -> Result<(), String> {
        self.stop_live();
        self.live_ear = None;
        self.level.reset();
        let instrument = self.instrument();
        let tuner = match LiveTuner::start(&self.input_choice(), &instrument, self.source()) {
            Ok(tuner) => tuner,
            Err(e) => {
                self.input.failed(InputUse::Tuner, &e);
                return Err(self.tr("error.audio", &[("reason", e.to_string())]));
            }
        };
        self.tuner = Some(tuner);
        self.input_opened(InputUse::Tuner);
        self.input_since = Some(std::time::Instant::now());
        Ok(())
    }

    /// True while the tuner listens.
    pub fn is_tuning(&self) -> bool {
        self.tuner.is_some()
    }

    pub fn stop_tuner(&mut self) {
        self.tuner = None;
        self.input.stopped(InputUse::Tuner);
    }

    /// Polls audio and session; call every ~16 ms.
    pub fn tick(&mut self) -> Vec<UiEvent> {
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
        events.extend(ear_event);
        events.extend(self.prepare_backing());
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

    fn progress_store(&self) -> ProgressStore {
        ProgressStore::new(&self.data_dir.join("progress"))
    }

    /// The file at `path` changed (a regenerated exercise, an edited score): its runs move from
    /// the id the library knew to `song_id`, unless another entry still uses the old one.
    fn adopt_progress(&self, path: &Path, song_id: &str) {
        let Some((known, old)) = self
            .library
            .entry(path)
            .map(|e| (e.path.clone(), e.song_id.clone()))
        else {
            return;
        };
        let shared = self
            .library
            .recent
            .iter()
            .any(|e| e.path != known && e.song_id == old);
        if shared {
            return;
        }
        // Progress is a convenience here: a failed move leaves the old runs where they were.
        let _ = self.progress_store().adopt(&old, song_id);
    }

    /// Saves the finished run (LH-FOR-01).
    fn record_run(&mut self) {
        let Some(live) = self.live.as_ref() else {
            return;
        };
        let session = live.practice().session();
        let summary = session.summary();
        if summary.judged == 0 {
            self.run_started = None;
            return;
        }
        let practice_s = self
            .run_started
            .take()
            .map_or(0.0, |t| t.elapsed().as_secs_f64());
        let section = session
            .transport()
            .loop_range()
            .map(|l| (l.start_s, l.end_s));
        // The speed played: the workshop changes it on its own after a pass (LH-FOR-02 charts
        // it, the probe takes it over).
        let mut run = run_from(
            &summary,
            practice_s,
            session.played_speed(),
            self.run_mode(),
            section,
            unix_now(),
        );
        // Progress belongs to the piece, not to a thinned copy (LH-MOD-06).
        let arrangement = self
            .loaded
            .as_ref()
            .map_or_else(|| session.arrangement().clone(), |l| l.full.clone());
        // Bar numbers as printed in the score (repeats are unrolled into more bars).
        run.hardest_bars = summary
            .hardest_bars
            .iter()
            .filter_map(|b| arrangement.bars.get(*b))
            .map(|bar| bar.source_index + 1)
            .collect();
        run.hesitant_bars = summary
            .hesitant_bars
            .iter()
            .filter_map(|b| arrangement.bars.get(*b))
            .map(|bar| bar.source_index + 1)
            .collect();
        let _ = self.progress_store().record(&arrangement, run);
        if let (Some((start_s, end_s)), Some((from, to))) = (section, self.looping) {
            self.remember_section(start_s, end_s, from, to);
        }
    }

    /// The mode a run is saved under: a probe apart from the mode chosen.
    fn run_mode(&self) -> &str {
        if self.probe.is_some() {
            return PROBE_MODE;
        }
        &self.settings.mode
    }

    /// Keeps the written bar numbers of a practised section with the piece (LH-FOR-01).
    fn remember_section(&mut self, start_s: f64, end_s: f64, from: usize, to: usize) {
        let Some(a) = self.arrangement() else {
            return;
        };
        let number = |i: usize| a.bars.get(i).map_or(0, |b| b.source_index + 1);
        let section = SectionBars {
            start_s,
            end_s,
            from_bar: number(from),
            to_bar: number(to),
        };
        let Some(path) = self.loaded_path().map(Path::to_path_buf) else {
            return;
        };
        let Some(entry) = self.library.entry_mut(&path) else {
            return;
        };
        if entry
            .sections
            .iter()
            .any(|s| same_section(s, start_s, end_s))
        {
            return;
        }
        entry.sections.push(section);
        self.save_library();
    }

    /// Per section of a piece (LH-FOR-01): the best run with its tempo and date, and the
    /// workshop runs of the last 30 days for the tempo chart (LH-FOR-02). The whole piece
    /// comes first, then sections by start.
    fn sections_json(
        &self,
        runs: &[Run],
        names: &[SectionBars],
        now_unix: u64,
    ) -> Vec<serde_json::Value> {
        // Sections as centiseconds, so float noise does not split them.
        let key = |r: &Run| {
            r.section
                .map(|(a, b)| ((a * 100.0).round() as i64, (b * 100.0).round() as i64))
        };
        let mut groups: std::collections::BTreeMap<Option<(i64, i64)>, Vec<&Run>> =
            std::collections::BTreeMap::new();
        for run in runs {
            groups.entry(key(run)).or_default().push(run);
        }
        let since = now_unix.saturating_sub(HISTORY_DAYS * SECONDS_PER_DAY);
        groups
            .into_values()
            .filter_map(|runs| {
                let best = runs.iter().max_by(|a, b| {
                    a.hit_rate.total_cmp(&b.hit_rate).then(a.speed.total_cmp(&b.speed))
                })?;
                let mut history: Vec<&&Run> = runs
                    .iter()
                    .filter(|r| r.mode == "workshop" && r.finished_unix >= since)
                    .collect();
                history.sort_by_key(|r| r.finished_unix);
                let history: Vec<serde_json::Value> = history
                    .iter()
                    .map(|r| json!({ "unix": r.finished_unix, "speed": r.speed, "hitRate": r.hit_rate }))
                    .collect();
                Some(json!({
                    "label": self.section_label(best.section, names),
                    "best": best.hit_rate,
                    "speed": best.speed,
                    "unix": best.finished_unix,
                    "runs": runs.len(),
                    "history": history,
                }))
            })
            .collect()
    }

    /// "Ganzes Stück", "Takt 5–8" or, for a section without known bars, "0:12–0:24".
    fn section_label(&self, section: Option<(f64, f64)>, names: &[SectionBars]) -> String {
        let Some((start_s, end_s)) = section else {
            return self.tr("progress.section.whole", &[]);
        };
        let Some(bars) = names.iter().find(|s| same_section(s, start_s, end_s)) else {
            let clock = |s: f64| format!("{}:{:02}", (s / 60.0) as u32, (s % 60.0) as u32);
            return self.tr(
                "progress.section.time",
                &[("from", clock(start_s)), ("to", clock(end_s))],
            );
        };
        if bars.from_bar == bars.to_bar {
            return self.tr(
                "progress.section.bar",
                &[("bar", bars.from_bar.to_string())],
            );
        }
        self.tr(
            "progress.section.bars",
            &[
                ("from", bars.from_bar.to_string()),
                ("to", bars.to_bar.to_string()),
            ],
        )
    }

    /// Detects the beats of the backing track and lines the bars up with them (LH-BIB-06).
    pub fn backing_analyse(&mut self) -> Result<(), String> {
        let (backing, _) = self
            .backing()
            .ok_or_else(|| self.tr("backing.nobacking", &[]))?;
        let audio =
            kontra_session::BackingTrack::load(&backing, 44_100).map_err(|e| e.to_string())?;
        let mono: Vec<f32> = audio
            .left
            .iter()
            .zip(&audio.right)
            .map(|(l, r)| 0.5 * (l + r))
            .collect();
        let grid = kontra_session::track_beats(&mono, audio.sample_rate);
        if grid.beats.len() < 4 {
            return Err(self.tr("backing.nobeats", &[]));
        }
        let path = self.loaded_path().ok_or("nothing loaded")?.to_path_buf();
        self.library.beat_maps.insert(
            path,
            store::BeatMap {
                beats: grid.beats,
                bpm: grid.bpm,
                on_beat_share: grid.on_beat_share,
                ..store::BeatMap::default()
            },
        );
        self.apply_beat_map()
    }

    /// Changes the beat map of the loaded piece: `downbeat` beats later (negative: earlier),
    /// every bar by `shift_ms`, or one bar by `bar_ms`.
    pub fn backing_adjust(
        &mut self,
        downbeat: i32,
        shift_ms: f64,
        bar: Option<(usize, f64)>,
    ) -> Result<(), String> {
        let path = self.loaded_path().ok_or("nothing loaded")?.to_path_buf();
        let Some(map) = self.library.beat_maps.get_mut(&path) else {
            return Ok(());
        };
        let last = map.beats.len().saturating_sub(2) as i64;
        map.first = (map.first as i64 + i64::from(downbeat)).clamp(0, last.max(0)) as usize;
        map.shift_ms += shift_ms;
        if let Some((bar, ms)) = bar {
            *map.nudges.entry(bar).or_insert(0.0) += ms;
        }
        self.apply_beat_map()
    }

    /// Moves every bar half a beat later, or back (QA 27.10., J3: the grid sat on the offbeat).
    pub fn backing_half_beat(&mut self) -> Result<(), String> {
        let path = self.loaded_path().ok_or("nothing loaded")?.to_path_buf();
        let Some(map) = self.library.beat_maps.get_mut(&path) else {
            return Ok(());
        };
        map.half_beat = !map.half_beat;
        self.apply_beat_map()
    }

    /// Back to the piece's own tempo, the backing starting at its old offset.
    pub fn backing_clear(&mut self) -> Result<(), String> {
        let path = self.loaded_path().ok_or("nothing loaded")?.to_path_buf();
        self.library.beat_maps.remove(&path);
        if let Some(entry) = self.library.entry_mut(&path) {
            entry.backing_offset_s = 0.0;
        }
        self.apply_beat_map()
    }

    fn apply_beat_map(&mut self) -> Result<(), String> {
        let _ = store::save(&self.data_dir.join("library.json"), &self.library);
        let track = self.loaded.as_ref().map_or(0, |l| l.track);
        self.reload(track)
    }

    /// The beat map in words for the alignment sheet:
    /// `{"analysed", "bpm", "firstS", "mapped", "bars", "sure", "halfBeat", "text"}`. Not `sure`: kick and
    /// bass do not show where the beat is, the grid may sit on the offbeat.
    pub fn backing_json(&self) -> String {
        let Some(path) = self.loaded_path() else {
            return "{}".into();
        };
        let bars = self.arrangement().map_or(0, |a| a.bars.len());
        let Some(map) = self.library.beat_maps.get(path) else {
            return json!({ "analysed": false, "bars": bars }).to_string();
        };
        let times = self
            .arrangement()
            .map(|a| map.bar_times(&quarters_per_bar(a)))
            .unwrap_or_default();
        let first = times.first().copied().unwrap_or(0.0);
        // Moved by hand, the player has listened: no warning, but the text says so.
        let sure = map.on_beat_share >= SURE_ON_BEAT_SHARE || map.half_beat;
        let key = match (map.half_beat, sure) {
            (true, _) => "backing.aligned.half",
            (false, true) => "backing.aligned",
            (false, false) => "backing.unsure",
        };
        json!({
            "analysed": true,
            "bpm": map.bpm,
            "firstS": first,
            "mapped": times.len(),
            "bars": bars,
            "sure": sure,
            "halfBeat": map.half_beat,
            "text": self.tr(key, &[
                ("bpm", format!("{:.0}", map.bpm)),
                ("first", format!("{first:.2}")),
                ("mapped", times.len().to_string()),
                ("bars", bars.to_string()),
            ]),
        })
        .to_string()
    }

    /// Backing track of the loaded piece and its offset; without its bass when chosen and
    /// separated (LH-BIB-09).
    fn backing(&self) -> Option<(PathBuf, f64)> {
        let path = self.loaded_path()?;
        let entry = self.library.entry(path)?;
        let song = entry.backing.clone()?;
        let separated = entry
            .backing_without_bass
            .then(|| self.without_bass(&song))
            .flatten();
        let played = separated.unwrap_or(song);
        Some((played, entry.backing_offset_s))
    }

    /// The backing track with its bass, when it plays without it: the mixer brings the bass
    /// back from it (LH-BIB-09).
    fn backing_original(&self) -> Option<PathBuf> {
        let entry = self.library.entry(self.loaded_path()?)?;
        let song = entry.backing.clone()?;
        let separated = entry.backing_without_bass && self.without_bass(&song).is_some();
        separated.then_some(song).filter(|p| p.exists())
    }

    /// File name of the chosen backing track (also when it plays without its bass).
    pub fn backing_name(&self) -> String {
        let path = self.loaded_path();
        path.and_then(|p| self.library.entry(p))
            .and_then(|e| e.backing.as_ref())
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_default()
    }

    /// Largest transposition offered, for the UI.
    pub fn transpose_max() -> i32 {
        kontra_session::TRANSPOSE_MAX
    }

    /// Semitones the loaded piece and its backing track are moved by (LH-WIE-02).
    pub fn transpose(&self) -> i32 {
        self.loaded_path()
            .and_then(|p| self.library.entry(p))
            .map_or(0, |e| e.transpose_semitones)
    }

    /// Moves the loaded piece and its backing by `semitones` (±6) and lays out the notes again.
    /// A running practice goes on with the moved notes: what was judged stays, the backing
    /// shifts in the mixer. Only if the notes changed in more than pitch does the run end.
    pub fn set_transpose(&mut self, semitones: i32) -> Result<(), String> {
        let Some(loaded) = self.loaded.as_ref().map(|l| (l.path.clone(), l.track)) else {
            return Ok(());
        };
        let max = kontra_session::TRANSPOSE_MAX;
        if let Some(entry) = self.library.entry_mut(&loaded.0) {
            entry.transpose_semitones = semitones.clamp(-max, max);
        }
        let _ = store::save(&self.data_dir.join("library.json"), &self.library);
        // Kept out of `reload`, which would end it.
        let live = self.live.take();
        let looping = self.looping;
        let reloaded = self.reload(loaded.1);
        // The loop stays, and its own note density (LH-MOD-06) needs it back before thinning.
        self.looping = looping;
        self.rethin();
        let Some(mut live) = live else {
            return reloaded;
        };
        let moved = self.arrangement().cloned();
        let goes_on = reloaded.is_ok()
            && moved.is_some_and(|a| live.practice_mut().retarget(a, self.transpose()));
        self.live = Some(live);
        if !goes_on {
            self.stop_live();
        }
        reloaded
    }

    /// Sets (or with `None` removes) the backing track of the loaded piece.
    pub fn set_backing(&mut self, path: Option<PathBuf>, offset_s: f64) {
        let Some(loaded) = self.loaded_path().map(Path::to_path_buf) else {
            return;
        };
        self.cancel_stems_of(&loaded);
        if let Some(entry) = self.library.entry_mut(&loaded) {
            entry.backing = path;
            entry.backing_offset_s = offset_s;
            entry.backing_without_bass = false;
        }
        let _ = store::save(&self.data_dir.join("library.json"), &self.library);
        self.stop_live();
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
                    Outcome::WrongPitch { played_midi } if self.settings.mode == "wait" => {
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

    /// Every judged note of the run (LH-BEW-01), the last attempt per pass and note, in order:
    /// `{index, pass, bar, state, offsetMs, cents, severity, technique, techniqueResult, text}`;
    /// `text` reads like "Takt 3 · G1 · 42 ms zu spät · +12 Cent · Slap: andere Technik"; a
    /// missed note in `outside` tells where it fell beside the window.
    fn note_details(
        &self,
        results: &[NoteResult],
        techniques: &[TechniqueResult],
        outside: &[OutsideNote],
    ) -> Vec<serde_json::Value> {
        let Some(a) = self.arrangement() else {
            return Vec::new();
        };
        let mut last = std::collections::BTreeMap::new();
        for r in results {
            if let Some(note) = r.note {
                last.insert((r.pass, note), *r);
            }
        }
        let mut technique_of = std::collections::BTreeMap::new();
        for t in techniques {
            technique_of.insert((t.pass, t.sign), *t);
        }
        // Double stop tones sounded per pass and start: a missed tone beside one is "nur ein Ton"
        // (R8). Not for larger chords: MIDI judges each tone alone, two of three are not "one".
        let double_stop = |start: u64| {
            a.notes.iter().filter(|n| n.start_ticks == start).count() == DOUBLE_STOP_TONES
        };
        let sounded: Vec<(u32, u64)> = last
            .iter()
            .filter(|(_, r)| r.outcome != Outcome::Missed)
            .filter_map(|((pass, index), _)| Some((*pass, a.notes.get(*index)?.start_ticks)))
            .filter(|(_, start)| double_stop(*start))
            .collect();
        last.into_iter()
            .filter_map(|((pass, index), r)| {
                let note = a.notes.get(index)?;
                let state = state_of(r.outcome, r.offset_ms.unwrap_or(0.0));
                let bar = r
                    .bar
                    .and_then(|b| a.bars.get(b))
                    .map_or(0, |b| b.source_index + 1);
                let key = key_at(a, note.start_ticks);
                let mut parts = vec![
                    self.tr("summary.note.bar", &[("bar", bar.to_string())]),
                    self.pitch_label(note.pitch, key),
                ];
                // Early and late say it with the milliseconds below.
                if !matches!(state, "early" | "late") {
                    parts.push(self.tr(&format!("result.{state}"), &[]));
                }
                match r.offset_ms {
                    Some(ms) if ms <= -1.0 => {
                        parts.push(self.tr("summary.note.early", &[("ms", format!("{:.0}", -ms))]))
                    }
                    Some(ms) if ms >= 1.0 => {
                        parts.push(self.tr("summary.note.late", &[("ms", format!("{ms:.0}"))]))
                    }
                    Some(_) => parts.push(self.tr("summary.note.ontime", &[])),
                    None => {}
                }
                if r.outcome == Outcome::Missed && sounded.contains(&(pass, note.start_ticks)) {
                    parts.push(self.tr("summary.note.one.tone", &[]));
                }
                // A missed note played beside the window: where it fell (M6).
                let beside = outside
                    .iter()
                    .find(|o| o.pass == pass && o.note == index)
                    .map(|o| o.offset_ms);
                match beside {
                    Some(ms) if ms < 0.0 => parts.push(self.tr(
                        "summary.note.outside.early",
                        &[("ms", format!("{:.0}", -ms))],
                    )),
                    Some(ms) => parts
                        .push(self.tr("summary.note.outside.late", &[("ms", format!("{ms:.0}"))])),
                    None => {}
                }
                // A wrong note names what was played; cents only for the right note's intonation.
                match (r.outcome, r.cents) {
                    (Outcome::WrongPitch { played_midi }, _) => {
                        let played =
                            kontra_model::Pitch(played_midi.round().clamp(0.0, 127.0) as u8);
                        let played = self.pitch_label(played, key);
                        parts.push(self.tr("summary.note.played", &[("played", played)]));
                    }
                    (_, Some(c)) => {
                        parts.push(self.tr("summary.note.cents", &[("cents", format!("{c:+.0}"))]));
                    }
                    _ => {}
                }
                let technique = technique_of.get(&(pass, index));
                let mark = technique.and_then(|t| t.mark());
                if let (Some(t), Some(_)) = (technique, mark) {
                    parts.push(self.tr(
                        "summary.note.technique",
                        &[
                            ("technique", self.technique_name(t.expected)),
                            (
                                "result",
                                self.tr(
                                    &format!("technique.result.{}", technique_result_key(mark)),
                                    &[],
                                ),
                            ),
                        ],
                    ));
                }
                Some(json!({
                    "index": index,
                    "pass": pass,
                    "bar": bar,
                    "state": state,
                    "offsetMs": r.offset_ms,
                    "cents": r.cents,
                    "severity": severity(state),
                    "technique": technique.map_or("", |t| technique_key(t.expected)),
                    "techniqueResult": technique_result_key(mark),
                    "text": parts.join(" · "),
                }))
            })
            .collect()
    }

    /// Notes on another string than the tab's (R7); a profile that cannot tell says so, also
    /// for some notes only (a five-string's low B, review #207). One line each.
    fn strings_text(&self, strings: kontra_session::StringTally) -> String {
        let mut parts = Vec::new();
        if strings.other > 0 {
            parts.push(self.tr(
                "summary.strings.other",
                &[
                    ("other", strings.other.to_string()),
                    ("told", strings.told.to_string()),
                ],
            ));
        }
        if strings.unsupported > 0 && strings.told == 0 {
            parts.push(self.tr("summary.strings.unsupported", &[]));
        }
        if strings.unsupported > 0 && strings.told > 0 {
            parts.push(self.tr(
                "summary.strings.unsupported.some",
                &[("count", strings.unsupported.to_string())],
            ));
        }
        parts.join("\n")
    }

    /// What this input did not judge: techniques, hints (slap and pop over a cable) and double
    /// stops at a microphone (Runde 17); one line each.
    fn untold_text(&self, techniques: &[String], hints: &[String], double_stops: u32) -> String {
        let mut parts = Vec::new();
        if !techniques.is_empty() {
            parts.push(self.tr(
                "summary.techniques.untold",
                &[("techniques", techniques.join(", "))],
            ));
        }
        if !hints.is_empty() {
            parts.push(self.tr(
                "summary.techniques.hint",
                &[("techniques", hints.join(", "))],
            ));
        }
        if double_stops > 0 {
            parts.push(self.tr("summary.doublestops.untold", &[]));
        }
        parts.join("\n")
    }

    /// Summary with ready-made texts for the evaluation sheet, and every note's result.
    pub fn summary_json(
        &self,
        s: &Summary,
        results: &[NoteResult],
        techniques: &[TechniqueResult],
    ) -> String {
        let notes = self.note_details(results, techniques, &s.outside_notes);
        // Techniques apart from the hit rate (round 9): "Slide: 8 von 10".
        let technique_list: Vec<_> = s
            .techniques
            .iter()
            .map(|t| {
                json!({
                    "technique": technique_key(t.technique),
                    "played": t.played,
                    "judged": t.judged,
                    "text": self.tr("summary.technique", &[
                        ("technique", self.technique_name(t.technique)),
                        ("played", t.played.to_string()),
                        ("judged", t.judged.to_string()),
                    ]),
                })
            })
            .collect();
        let names = |list: &[Technique]| -> Vec<String> {
            list.iter().map(|t| self.technique_name(*t)).collect()
        };
        let untold = names(&s.untold_techniques);
        let hints = names(&s.hint_techniques);
        let hardest_notes = hardest_notes(&notes);
        let arrangement = self.arrangement();
        let judging = arrangement.map_or(Judging::PitchAndTime, |a| a.judging);
        // Most notes missed the window but were played beside it: their offset is the tendency
        // (QA 27.10., M2: strict window, all 40 ms early read "0 % hit" without a word on time).
        let outside = s.outside_offset_ms.filter(|_| s.missed > s.hits);
        // Intonation only where it is scored: a fretless or double bass, or off-pitch notes.
        let intonation = s
            .intonation
            .filter(|_| s.off_pitch > 0 || self.scored_intonation());
        let tendency = match (outside, s.mean_offset_ms) {
            (Some(m), _) if m < 0.0 => {
                self.tr("summary.outside.early", &[("ms", format!("{:.0}", -m))])
            }
            (Some(m), _) => self.tr("summary.outside.late", &[("ms", format!("{m:.0}"))]),
            (None, Some(m)) if m < -5.0 => {
                self.tr("summary.early", &[("ms", format!("{:.0}", -m))])
            }
            (None, Some(m)) if m > 5.0 => self.tr("summary.late", &[("ms", format!("{m:.0}"))]),
            (None, Some(_)) => self.tr("summary.ontime", &[]),
            (None, None) => String::new(),
        };
        let hardest: Vec<_> = s
            .hardest_bars
            .iter()
            .map(|b| {
                let number = arrangement
                    .and_then(|a| a.bars.get(*b))
                    .map_or(b + 1, |bar| bar.source_index as usize + 1);
                json!({ "index": b, "number": number })
            })
            .collect();
        // A bundled exercise that now sits: the next one is one click away.
        let next = self
            .loaded_path()
            .filter(|p| p.parent().and_then(Path::parent) == Some(Self::exercises_dir().as_path()))
            .filter(|_| Self::run_sits(judging, s.hit_rate, s.spread_ms))
            .and_then(|p| self.next_exercise(Some(p)));
        json!({
            "next": next,
            "probe": self.probe_json(s),
            "hitRate": s.hit_rate,
            "hitRateText": self.tr("summary.hitrate", &[("percent", format!("{:.0}", s.hit_rate * 100.0))]),
            "countsText": self.tr("summary.counts", &[
                ("hits", s.hits.to_string()),
                ("judged", s.judged.to_string()),
                ("wrong", s.wrong.to_string()),
                ("missed", s.missed.to_string()),
            ]),
            "observation": self.observation_json(s),
            "recalibrateText": self.recalibrate_text(s),
            "tendencyText": tendency,
            // The few hits beside the missed notes tell no steadiness.
            "spreadText": s
                .spread_ms
                .filter(|_| outside.is_none())
                .map_or(String::new(), |v| self.spread_text(v, judging)),
            "pocketText": self.pocket_text(s),
            "intonationText": intonation.map_or(String::new(), |i| self.tr("summary.intonation", &[
                ("offpitch", s.off_pitch.to_string()),
                ("text", self.intonation_text(i)),
            ])),
            "intonationParts": intonation.map_or(Vec::new(), |_| self.intonation_parts(s)),
            // Feedback beside the score (R7): notes on another string than the tab's.
            "stringsText": self.strings_text(s.strings),
            "meanOffsetMs": s.mean_offset_ms.unwrap_or(0.0),
            "counts": s.histogram.counts,
            "firstBinMs": s.histogram.first_bin_ms,
            "binMs": s.histogram.bin_ms,
            "hardestBars": hardest,
            "notes": notes,
            "hardestNotes": hardest_notes,
            "techniques": technique_list,
            "untoldText": self.untold_text(&untold, &hints, s.untold_double_stops),
        })
        .to_string()
    }

    /// The one observation of the run (LH-BEW-09), or null below the minimum of notes (or of
    /// hits, for timing):
    /// `{ solid: false, text: "Am meisten bringt jetzt: Slide, 2 von 6. …" }`.
    fn observation_json(&self, s: &Summary) -> serde_json::Value {
        let pocket = Pocket::from_key(&self.settings.pocket).unwrap_or_default();
        let Some(observation) = s.observation(pocket) else {
            return serde_json::Value::Null;
        };
        // "Solid" would contradict the latency hint next to it (review #195): that hint alone.
        if observation == Observation::Solid && self.latency_hint(s, pocket).is_some() {
            return serde_json::Value::Null;
        }
        let percent = |share: f64| format!("{:.0}", share * 100.0);
        let text = match observation {
            Observation::Technique {
                technique,
                played,
                judged,
            } => self.tr(
                "summary.observe.technique",
                &[
                    ("technique", self.technique_name(technique)),
                    ("played", played.to_string()),
                    ("judged", judged.to_string()),
                ],
            ),
            Observation::Beside { offset_ms } if offset_ms < 0.0 => self.tr(
                "summary.observe.beside.early",
                &[("ms", format!("{:.0}", -offset_ms))],
            ),
            Observation::Beside { offset_ms } => self.tr(
                "summary.observe.beside.late",
                &[("ms", format!("{offset_ms:.0}"))],
            ),
            Observation::Missed { notes, judged } => self.tr(
                "summary.observe.missed",
                &[("notes", notes.to_string()), ("judged", judged.to_string())],
            ),
            Observation::Lean {
                side: Side::Early,
                share,
            } => self.tr("summary.observe.early", &[("percent", percent(share))]),
            Observation::Lean {
                side: Side::Late,
                share,
            } => self.tr("summary.observe.late", &[("percent", percent(share))]),
            Observation::Aim {
                side: Side::Early,
                off_ms,
            } => self.tr(
                "summary.observe.aim.early",
                &[("ms", format!("{off_ms:.0}"))],
            ),
            Observation::Aim {
                side: Side::Late,
                off_ms,
            } => self.tr(
                "summary.observe.aim.late",
                &[("ms", format!("{off_ms:.0}"))],
            ),
            Observation::Solid => self.tr("summary.observe.solid", &[]),
        };
        json!({ "solid": observation == Observation::Solid, "text": text })
    }

    /// The latency hint (LH-EIN-14), else "". After an observation of the same lean it follows
    /// on ("Liegen die Treffer so beständig hinter dem Schlag (im Mittel 25 ms), kann auch …"),
    /// otherwise it stands alone ("Die Treffer liegen beständig hinter dem Schlag …").
    fn recalibrate_text(&self, s: &Summary) -> String {
        let pocket = Pocket::from_key(&self.settings.pocket).unwrap_or_default();
        let Some(side) = self.latency_hint(s, pocket) else {
            return String::new();
        };
        let follows = matches!(
            s.observation(pocket),
            Some(Observation::Lean { side: lean, .. } | Observation::Aim { side: lean, .. })
                if lean == side
        );
        let key = match (side, follows) {
            (Side::Early, true) => "summary.recalibrate.early",
            (Side::Late, true) => "summary.recalibrate.late",
            (Side::Early, false) => "summary.recalibrate.early.alone",
            (Side::Late, false) => "summary.recalibrate.late.alone",
        };
        let ms = format!("{:.0}", s.mean_offset_ms.unwrap_or_default().abs());
        self.tr(key, &[("ms", ms)])
    }

    /// A blind probe next to the workshop before it (DIDAKTIK I1), or null:
    /// `{ with: "88 % · ±11 ms" | "", without: "71 % · ±19 ms", text: "17 Punkte weniger …" }`.
    fn probe_json(&self, s: &Summary) -> serde_json::Value {
        let (Some(_), Some(range)) = (self.probe, self.loop_range()) else {
            return serde_json::Value::Null;
        };
        let row = |rate: f64, spread: Option<f64>| {
            let percent = format!("{:.0}", rate * 100.0);
            match spread {
                Some(ms) => self.tr(
                    "probe.row.spread",
                    &[("percent", percent), ("ms", format!("{ms:.0}"))],
                ),
                None => self.tr("probe.row", &[("percent", percent)]),
            }
        };
        let without = row(s.hit_rate, s.spread_ms);
        let runs = self.loaded_runs();
        let practised = runs
            .iter()
            .rev()
            .find(|r| r.mode == WORKSHOP_MODE && same_range(r.section, range));
        let Some(practised) = practised else {
            return json!({ "with": "", "without": without, "text": self.tr("probe.first", &[]) });
        };
        let points = (practised.hit_rate - s.hit_rate) * 100.0;
        let text = if points > PROBE_SAME_POINTS {
            self.tr("probe.less", &[("points", format!("{points:.0}"))])
        } else {
            self.tr("probe.holds", &[])
        };
        json!({
            "with": row(practised.hit_rate, practised.spread_ms),
            "without": without,
            "text": text,
        })
    }

    /// A technique's name in the UI's language ("Hammer-on").
    fn technique_name(&self, technique: Technique) -> String {
        self.tr(&format!("technique.{}", technique_key(technique)), &[])
    }

    /// Intonation is part of the score for fretless and double basses (pitch limit below 50 ct).
    fn scored_intonation(&self) -> bool {
        let kind = self.arrangement().map_or_else(
            || instrument_from(&self.settings).kind,
            |a| a.instrument.kind,
        );
        self.score_settings(kind).pitch_cents < 50.0
    }

    /// True if the chosen (or default) output goes over Bluetooth (LH-EIN-06).
    fn output_is_bluetooth(&self) -> bool {
        list_devices(Direction::Output).iter().any(|d| {
            let chosen = match &self.settings.output_device {
                Some(id) => &d.id == id,
                None => d.is_default,
            };
            chosen && d.connection == kontra_session::Connection::Bluetooth
        })
    }

    /// Folder of the bundled exercises (LH-BIB-08): installed share dir, else the repo's.
    pub fn exercises_dir() -> PathBuf {
        let installed = std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(|p| p.join("../share/kontra/exercises")));
        let dir = match installed {
            Some(path) if path.is_dir() => path,
            _ => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/exercises"),
        };
        // Without "..": progress is keyed by the path, and the same file opened from the file
        // manager has its plain one.
        std::fs::canonicalize(&dir).unwrap_or(dir)
    }

    /// Bundled exercises by level: `[{"level", "mastered": 3, "items": [{"title", "path", "best",
    /// "mastered"}]}]`; mastered: see `sits`.
    pub fn bundled_json(&self) -> String {
        json!(self.bundled_levels()).to_string()
    }

    fn bundled_levels(&self) -> Vec<serde_json::Value> {
        let mut levels: Vec<PathBuf> = std::fs::read_dir(Self::exercises_dir())
            .map(|d| {
                d.flatten()
                    .map(|e| e.path())
                    .filter(|p| p.is_dir())
                    .collect()
            })
            .unwrap_or_default();
        levels.sort();
        let list: Vec<_> = levels
            .iter()
            .map(|dir| {
                let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
                    .map(|d| {
                        d.flatten()
                            .map(|e| e.path())
                            .filter(|p| p.extension().is_some_and(|x| x == "mid"))
                            .collect()
                    })
                    .unwrap_or_default();
                // By plan place: "100 …" comes after "99 …" (a plain sort puts it first), level 1
                // in learning order.
                files.sort_by_key(|f| {
                    let (number, path) = plan_number(f);
                    (exercise::plan_place(number), path)
                });
                let items: Vec<_> = files
                    .iter()
                    .map(|f| {
                        let stem = f
                            .file_stem()
                            .map_or(String::new(), |s| s.to_string_lossy().into_owned());
                        // "07 Dur-Tonleiter in G (60 BPM)": the number picks the plan entry,
                        // whose title is told in the UI's language.
                        let title = self.bundled_title(&stem).unwrap_or_else(|| {
                            stem.split_once(' ')
                                .map_or(stem.clone(), |(_, t)| t.to_string())
                        });
                        let best = self.best_rate(f);
                        json!({
                            "title": title,
                            "goal": self.bundled_goal(&stem),
                            "goalTopic": bundled_goal_key(&stem)
                                .map_or(String::new(), |g| format!("goal.{g}")),
                            "path": f.to_string_lossy(),
                            "best": best,
                            "mastered": self.sits(f, self.bundled_judging(&stem)),
                        })
                    })
                    .collect();
                let mastered = items.iter().filter(|i| i["mastered"] == true).count();
                let name = dir
                    .file_name()
                    .map_or(String::new(), |n| n.to_string_lossy().into_owned());
                // "1 Einstieg": the number names the level in the UI's language.
                let level = name.split_once(' ').map_or(name.clone(), |(n, _)| {
                    self.tr(&format!("exercise.level.{n}"), &[])
                });
                json!({ "level": level, "items": items, "mastered": mastered })
            })
            .collect();
        list
    }

    /// A library entry's title; bundled exercises in the UI's language.
    fn library_title(&self, entry: &RecentEntry) -> String {
        self.plan_title(&entry.path)
            .unwrap_or_else(|| entry.title.clone())
    }

    /// A file of the bundled plan (exercises dir / level / file).
    fn is_bundled(&self, path: &Path) -> bool {
        path.parent()
            .and_then(Path::parent)
            .is_some_and(|dir| dir == Self::exercises_dir())
    }

    /// The title of a bundled exercise in the UI's language; `None` for any other file.
    fn plan_title(&self, path: &Path) -> Option<String> {
        let bundled = self.is_bundled(path);
        let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned());
        bundled
            .then_some(stem)
            .flatten()
            .and_then(|s| self.bundled_title(&s))
    }

    /// The title of bundled exercise "NN …" in the UI's language, from its plan entry.
    fn bundled_title(&self, stem: &str) -> Option<String> {
        let number: usize = stem.split_once(' ')?.0.parse().ok()?;
        let (_, spec) = exercise::bundled_specs()
            .into_iter()
            .nth(number.checked_sub(1)?)?;
        Some(self.exercise_title(&exercise::describe(&spec)))
    }

    /// How bundled exercise "NN …" is judged; pitch and time when unknown.
    fn bundled_judging(&self, stem: &str) -> Judging {
        stem.split_once(' ')
            .and_then(|(n, _)| n.parse::<usize>().ok())
            .and_then(|n| exercise::bundled_specs().into_iter().nth(n.checked_sub(1)?))
            .map_or(Judging::PitchAndTime, |(_, spec)| spec.judging)
    }

    /// Whether a run sits: hits above the workshop band (`MASTERED_RATE`); judged by time only,
    /// also a spread of at most `PULSE_SPREAD_MS` (DIDAKTIK I4: then the frets come in), in
    /// whole ms as the summary shows it.
    fn run_sits(judging: Judging, hit_rate: f64, spread_ms: Option<f64>) -> bool {
        let pulse = match judging {
            Judging::TimeOnly => spread_ms.is_some_and(|s| s.round() <= PULSE_SPREAD_MS),
            Judging::PitchAndTime => true,
        };
        hit_rate >= MASTERED_RATE && pulse
    }

    /// Whether any run of the piece at `path` sits (`run_sits`).
    fn sits(&self, path: &Path, judging: Judging) -> bool {
        self.library
            .entry(path)
            .and_then(|e| self.progress_store().load_id(&e.song_id))
            .is_some_and(|p| {
                p.runs
                    .iter()
                    .any(|r| Self::run_sits(judging, r.hit_rate, r.spread_ms))
            })
    }

    /// What bundled exercise "NN …" trains, in the UI's language ("" for none).
    fn bundled_goal(&self, stem: &str) -> String {
        bundled_goal_key(stem)
            .map(|goal| self.tr(&format!("exercise.goal.{goal}"), &[]))
            .unwrap_or_default()
    }

    /// "Dur-Tonleiter in G (60 BPM)" / "Major scale in G (60 BPM)"; finger exercises
    /// "Fingerübung 1-3-2-4 ab Bund 5 (60 BPM)"; a tempo ramp "(80–110 BPM)".
    pub fn exercise_title(&self, d: &exercise::Description) -> String {
        let key = self.note_label(&format!("{}{}", d.key, if d.minor { "m" } else { "" }));
        let bpm = match d.end_bpm {
            Some(end) => format!("{:.0}-{end:.0}", d.bpm),
            None => format!("{:.0}", d.bpm),
        };
        let part = |group: &str, key: &str| self.tr(&format!("exercise.name.{group}.{key}"), &[]);
        let name = match d.kind {
            "scale" => part("scale", d.variant),
            "sequence" => self.tr(
                "exercise.name.sequence",
                &[
                    ("scale", part("scale", d.variant)),
                    (
                        "sequence",
                        d.rhythm.map(|r| part("sequence", r)).unwrap_or_default(),
                    ),
                ],
            ),
            "song" => match d.rhythm {
                Some(level) => self.tr(
                    "exercise.name.song.level",
                    &[
                        ("style", part("style", d.variant)),
                        ("level", part("level", level)),
                    ],
                ),
                None => self.tr("exercise.name.song", &[("style", part("style", d.variant))]),
            },
            "chromatic" => self.tr(
                "exercise.name.chromatic",
                &[(
                    "fingers",
                    d.variant
                        .chars()
                        .map(String::from)
                        .collect::<Vec<_>>()
                        .join("-"),
                )],
            ),
            _ => self.tr(
                &format!("exercise.name.{}", d.kind),
                &[
                    ("progression", part("progression", d.variant)),
                    (
                        "rhythm",
                        d.rhythm.map(|r| part("rhythm", r)).unwrap_or_default(),
                    ),
                ],
            ),
        };
        let title = match d.fret {
            Some(fret) => self.tr(
                "exercise.title.fret",
                &[("name", name), ("fret", fret.to_string()), ("bpm", bpm)],
            ),
            None => self.tr(
                "exercise.title",
                &[("name", name), ("key", key), ("bpm", bpm)],
            ),
        };
        // Judged by time only: another exercise with its own file and progress (DIDAKTIK I4).
        match d.judging {
            Judging::TimeOnly => self.tr("exercise.title.timing", &[("title", title)]),
            Judging::PitchAndTime => title,
        }
    }

    /// Best hit rate (0..1) of a piece, or -1 when never played.
    fn best_rate(&self, path: &Path) -> f64 {
        self.library
            .entry(path)
            .and_then(|e| self.progress_store().load_id(&e.song_id))
            .and_then(|p| p.best_hit_rate())
            .unwrap_or(-1.0)
    }

    /// An exercise from the UI's choice: `kind` "scale" | "arpeggio" | "triads" | "walking" |
    /// "groove" | "chromatic"; `variant` the scale ("major", "dorian" …), the progression ("tonic",
    /// "251", "blues", "minor", "145", "pop", "1625", "minor251") or the finger order ("1324");
    /// `rhythm` the groove's rhythm or a scale's pattern ("thirds", "fours", "" for up and down).
    /// Finger exercises take `key` as the start fret. `judging` "time" judges only the attack
    /// (grooves, DIDAKTIK I4); "pitch" or "" pitch and time.
    pub fn exercise_spec(
        kind: &str,
        variant: &str,
        rhythm: &str,
        key: u8,
        bpm: f64,
        judging: &str,
    ) -> Option<exercise::ExerciseSpec> {
        let judging = match judging {
            "" | JUDGING_PITCH => Judging::PitchAndTime,
            JUDGING_TIME => Judging::TimeOnly,
            _ => return None,
        };
        use exercise::{ExerciseKind, Fingers, Progression, Rhythm, ScaleKind, Sequence};
        let progression = || Progression::from_key(variant);
        let kind = match kind {
            "scale" if rhythm.is_empty() => ExerciseKind::Scale(ScaleKind::from_key(variant)?),
            "scale" => {
                ExerciseKind::Sequence(ScaleKind::from_key(variant)?, Sequence::from_key(rhythm)?)
            }
            "arpeggio" => ExerciseKind::Arpeggio(progression()?),
            "triads" => ExerciseKind::Triads(progression()?),
            "walking" => ExerciseKind::WalkingBass(progression()?),
            "groove" => ExerciseKind::Groove(progression()?, Rhythm::from_key(rhythm)?),
            "chromatic" => ExerciseKind::Chromatic(Fingers::from_key(variant)?),
            // A piece takes its level where grooves take the rhythm; none is medium.
            "song" => ExerciseKind::Song(
                exercise::Style::from_key(variant)?,
                match rhythm {
                    "" => exercise::Level::default(),
                    level => exercise::Level::from_key(level)?,
                },
            ),
            _ => return None,
        };
        // A piece's form repeats its parts itself; a 12-bar blues is long enough once.
        let once = matches!(kind, ExerciseKind::Song(..)) || variant == "blues";
        Some(exercise::ExerciseSpec {
            kind,
            key: key % 12,
            bpm: bpm.clamp(40.0, 200.0),
            slow: false,
            // Short patterns (scales, two-bar grooves) get more passes.
            repeats: if once { 1 } else { 2 },
            ramp_bpm: 0.0,
            judging,
        })
    }

    /// Direction and spread, e.g. "im Schnitt 9 Cent zu tief, Streuung ±6"; without a direction
    /// only the spread (LH-BEW-12).
    fn intonation_text(&self, i: Intonation) -> String {
        let key = match i.leaning() {
            Some(Leaning::Flat) => "summary.intonation.flat",
            Some(Leaning::Sharp) => "summary.intonation.sharp",
            None => "summary.intonation.centred",
        };
        self.tr(
            key,
            &[
                ("cents", format!("{:.0}", i.mean_cent.abs())),
                ("spread", format!("{:.0}", i.spread_cent)),
            ],
        )
    }

    /// Per string and position, e.g. "E-Saite, Bund 5 bis 9: …" (LH-BEW-12); none for a single
    /// part, it would repeat the line above.
    fn intonation_parts(&self, s: &Summary) -> Vec<String> {
        if s.intonation_parts.len() < 2 {
            return Vec::new();
        }

        let strings = self
            .arrangement()
            .map(|a| a.instrument.tuning.strings.clone())
            .unwrap_or_default();
        s.intonation_parts
            .iter()
            .map(|p| {
                let string = strings
                    .get(usize::from(p.string))
                    .map_or_else(|| (p.string + 1).to_string(), |s| s.name().to_string());
                self.tr(
                    "summary.intonation.part",
                    &[
                        ("string", string),
                        ("from", p.first_fret.to_string()),
                        ("to", p.last_fret.to_string()),
                        ("text", self.intonation_text(p.intonation)),
                    ],
                )
            })
            .collect()
    }

    /// The spread, e.g. "Streuung ±18 ms"; judged by time only with the aim for the frets
    /// (DIDAKTIK I4), e.g. "Streuung ±31 ms · Bünde kommen unter ±25 ms dazu".
    fn spread_text(&self, spread_ms: f64, judging: Judging) -> String {
        let spread = self.tr("summary.spread", &[("ms", format!("{spread_ms:.0}"))]);
        match judging {
            Judging::TimeOnly => self.tr(
                "summary.spread.pulse",
                &[("spread", spread), ("ms", format!("{PULSE_SPREAD_MS:.0}"))],
            ),
            Judging::PitchAndTime => spread,
        }
    }

    /// Whether the run held the chosen aim against the beat (LH-BEW-08), e.g. "Ziel laid-back
    /// (10 bis 30 ms dahinter): verfehlt, 6 ms vor dem Zielbereich"; empty without an aim or
    /// timed notes.
    fn pocket_text(&self, s: &Summary) -> String {
        let pocket = Pocket::from_key(&self.settings.pocket).unwrap_or_default();
        // Whole ms, as the text shows them: +9.6 is laid-back, not "missed by 0 ms".
        let Some(verdict) = s.mean_offset_ms.and_then(|m| pocket.judge(m.round())) else {
            return String::new();
        };
        let Some((a, b)) = pocket.band_ms() else {
            return String::new();
        };
        let (low, high) = (a.abs().min(b.abs()), a.abs().max(b.abs()));
        let aim = self.tr(
            &format!("pocket.{}.aim", pocket.key()),
            &[("low", format!("{low:.0}")), ("high", format!("{high:.0}"))],
        );
        let (key, ms) = match verdict {
            PocketVerdict::Held => ("summary.pocket.held", 0.0),
            PocketVerdict::Early(ms) => ("summary.pocket.early", ms),
            PocketVerdict::Late(ms) => ("summary.pocket.late", ms),
        };
        self.tr(key, &[("aim", aim), ("ms", format!("{ms:.0}"))])
    }

    /// Starts the fretboard quiz ("notes" or "intervals") on frets `low`–`high` (kept for the
    /// next start, LH-LRN-18) and returns `quiz_json`.
    pub fn quiz_start(&mut self, kind: &str, low: i32, high: i32) -> String {
        let kind = match kind {
            "intervals" => quiz::QuizKind::Intervals,
            _ => quiz::QuizKind::Notes,
        };
        let mut stats: quiz_stats::QuizStats = store::load(&self.data_dir.join(quiz_stats::FILE));
        stats.range = quiz_stats::FretRange::new(low, high);
        let instrument = instrument_from(&self.settings);
        let quiz = quiz::Quiz::new(kind, &instrument, stats, time_seed(), unix_now());
        let _ = store::save(&self.data_dir.join(quiz_stats::FILE), quiz.stats());
        self.quiz = Some(quiz);
        self.quiz_json()
    }

    /// Next question.
    pub fn quiz_next(&mut self) -> String {
        let instrument = instrument_from(&self.settings);
        if let Some(q) = &mut self.quiz {
            q.next(&instrument, unix_now());
        }
        self.quiz_json()
    }

    /// Answers with a played pitch; true when it was the first right answer. Found and missed
    /// places are stored at once.
    pub fn quiz_answer(&mut self, midi: f64) -> bool {
        let instrument = instrument_from(&self.settings);
        let Some(q) = self.quiz.as_mut() else {
            return false;
        };
        let right = q.answer(midi, &instrument, unix_now());
        let _ = store::save(&self.data_dir.join(quiz_stats::FILE), q.stats());
        right
    }

    /// "3 of 4 found" over finished questions only: the open one is not lost yet (QA 13.10.: it
    /// read "0 of 1 found" before the first answer). Nothing before the first one is finished.
    /// "3 of 4 found" (quiz: solved at all) or "3 of 4 without a mistake" (ear training: clean
    /// answers only; QA 14.10.: "0 of 1 found" right after a correct answer on the fifth try).
    /// Quiz: found of found and lost; skipped questions are no miss (LH-LRN-18).
    fn quiz_score(&self, right: u32, lost: u32) -> String {
        self.score_text("quiz.score", right, right + lost, true)
    }

    fn score_text(&self, key: &str, right: u32, asked: u32, solved: bool) -> String {
        let done = asked - u32::from(!solved && asked > 0);
        if done == 0 {
            return String::new();
        }
        self.tr(
            key,
            &[("right", right.to_string()), ("asked", done.to_string())],
        )
    }

    /// The question in words, the score, the range, and once solved where the answer lies:
    /// `{"prompt", "right", "asked", "solved", "range": {low, high}, "positions": [{string,
    /// fret, role}]}`. Before a start only the stored range.
    pub fn quiz_json(&self) -> String {
        let Some(q) = &self.quiz else {
            let stats: quiz_stats::QuizStats = store::load(&self.data_dir.join(quiz_stats::FILE));
            return json!({ "range": stats.range }).to_string();
        };
        let instrument = instrument_from(&self.settings);
        let string_name = |s: usize| {
            instrument
                .tuning
                .strings
                .get(s)
                .map_or(String::new(), |p| self.note_label(&p.name().to_string()))
        };
        let question = &q.question;
        let prompt = match q.kind {
            quiz::QuizKind::Notes => self.tr(
                "quiz.notes.prompt",
                &[
                    ("note", self.note_label(&quiz::note_name(question.target))),
                    ("string", string_name(question.string)),
                ],
            ),
            quiz::QuizKind::Intervals => {
                let (_, fret) = q.root_position(&instrument).unwrap_or((0, 0));
                self.tr(
                    "quiz.intervals.prompt",
                    &[
                        (
                            "interval",
                            self.tr(
                                &format!("quiz.interval.{}", question.interval.unwrap_or("p5")),
                                &[],
                            ),
                        ),
                        (
                            "note",
                            self.note_label(&quiz::note_name(question.root.unwrap_or(0))),
                        ),
                        ("string", string_name(question.string)),
                        ("fret", fret.to_string()),
                    ],
                )
            }
        };
        let mut positions: Vec<serde_json::Value> = Vec::new();
        if let Some((string, fret)) = q.root_position(&instrument) {
            positions.push(json!({ "string": string, "fret": fret, "role": "root" }));
        }
        if q.solved {
            positions.extend(q.positions(&instrument).into_iter().map(
                |(string, fret)| json!({ "string": string, "fret": fret, "role": "current" }),
            ));
        }
        json!({
            "prompt": prompt,
            "right": q.right,
            "asked": q.asked,
            "solved": q.solved,
            "score": self.quiz_score(q.right, q.lost),
            "range": q.stats().range,
            "positions": positions,
        })
        .to_string()
    }

    /// Starts ear training ("echo", "interval" or "chord"), plays the first prompt and returns
    /// `ear_json`.
    pub fn ear_start(&mut self, kind: &str) -> Result<String, String> {
        let kind = match kind {
            "interval" => ear::EarKind::Interval,
            "chord" => ear::EarKind::ChordRoot,
            _ => ear::EarKind::Echo,
        };
        self.stop_live();
        self.tuner = None;
        self.live_ear = None;
        self.level.reset();
        let instrument = self.instrument();
        let setup = LiveSetup {
            input: self.input_choice(),
            output_device: self.output_device(),
            source: self.source(),
            backing: None,
            original: None,
            backing_cache: None,
            onset_gate_db: self.settings.onset_gate_db,
        };
        let mut live_ear = LiveEar::start(&setup, &instrument)
            .map_err(|e| self.tr("error.audio", &[("reason", e.to_string())]))?;
        live_ear.set_gains(ear_gains(&self.settings));
        self.live_ear = Some(live_ear);
        self.input.remember(InputUse::Ear, setup.input);
        self.ear = Some(ear::EarQuiz::new(kind, &instrument, time_seed()));
        self.ear_play();
        Ok(self.ear_json())
    }

    pub fn ear_stop(&mut self) {
        self.live_ear = None;
        self.input.stopped(InputUse::Ear);
        self.ear = None;
        self.ear_prompting = false;
    }

    /// Plays the current prompt (again).
    pub fn ear_play(&mut self) -> String {
        let (Some(quiz), Some(live_ear)) = (&self.ear, self.live_ear.as_mut()) else {
            return self.ear_json();
        };
        live_ear.play(&quiz.question.tones);
        self.ear_prompting = live_ear.is_prompting();
        self.ear_json()
    }

    /// Next question, played at once.
    pub fn ear_next(&mut self) -> String {
        let instrument = self.instrument();
        if let Some(quiz) = self.ear.as_mut() {
            quiz.next(&instrument);
        }
        self.ear_play()
    }

    /// Judges a played note (fractional MIDI); the UI gets the new state.
    pub fn ear_answer(&mut self, midi: f64) -> String {
        if let Some(quiz) = self.ear.as_mut() {
            quiz.answer(midi);
        }
        self.ear_json()
    }

    /// Feeds the level readings of `events` to the advisor; returns the advice when it changed.
    fn advise_level(&mut self, events: &[UiEvent]) -> Option<UiEvent> {
        let now = self.started_at.elapsed().as_secs_f64();
        for event in events {
            if let UiEvent::Level {
                peak_db, clipped, ..
            } = event
            {
                self.level.push(now, *peak_db as f32, *clipped);
            }
        }
        let laptop = self.source() == InputSource::Line && self.level.suggests_laptop();
        let told = (self.level.state(), laptop);
        if told == self.level_told {
            return None;
        }
        self.level_told = told;
        Some(UiEvent::LevelAdvice {
            state: told.0.key(),
            suggest_laptop: laptop,
        })
    }

    /// Takes the answers out of `raw` and judges them; returns the new state when it changed
    /// (a note was judged or the prompt has ended).
    fn ear_heard(&mut self, raw: &mut Vec<PracticeEvent>) -> Option<UiEvent> {
        let mut changed = false;
        raw.retain(|event| {
            let PracticeEvent::Heard { midi } = event else {
                return true;
            };
            if let Some(quiz) = self.ear.as_mut() {
                changed |= quiz.answer(*midi) != ear::Verdict::Ignored;
            }
            false
        });
        let prompting = self.live_ear.as_ref().is_some_and(LiveEar::is_prompting);
        changed |= prompting != self.ear_prompting;
        self.ear_prompting = prompting;
        changed.then(|| UiEvent::Ear(self.ear_json()))
    }

    /// The question in words, the notes played so far and, once solved, what it was and where
    /// it lies: `{"prompt", "prompting", "solved", "progress", "total", "played": [{note,
    /// right, direction ("high", "low" or "")}], "reveal", "score", "positions": [{string, fret,
    /// role}], "silent", "listening" (false without an input: answers come by click)}`.
    pub fn ear_json(&self) -> String {
        let Some(q) = &self.ear else {
            return String::new();
        };
        let name = |p: kontra_model::Pitch| self.note_label(&p.to_string());
        let question = &q.question;
        let prompt = match q.kind {
            ear::EarKind::Echo if question.answer.len() == 1 => self.tr("ear.echo.prompt.one", &[]),
            ear::EarKind::Echo => self.tr(
                "ear.echo.prompt.many",
                &[("count", question.answer.len().to_string())],
            ),
            ear::EarKind::Interval => self.tr(
                "ear.interval.prompt",
                &[("note", question.root.map(name).unwrap_or_default())],
            ),
            ear::EarKind::ChordRoot => self.tr("ear.chord.prompt", &[]),
        };
        let reveal = match q.kind {
            _ if !q.solved => String::new(),
            ear::EarKind::Echo => {
                let notes: Vec<String> = question.answer.iter().map(|&p| name(p)).collect();
                self.tr("ear.reveal.echo", &[("notes", notes.join(" – "))])
            }
            ear::EarKind::Interval => self.tr(
                "ear.reveal.interval",
                &[
                    (
                        "interval",
                        self.tr(
                            &format!("quiz.interval.{}", question.interval.unwrap_or("p5")),
                            &[],
                        ),
                    ),
                    ("note", question.root.map(name).unwrap_or_default()),
                ],
            ),
            ear::EarKind::ChordRoot => self.tr(
                "ear.reveal.chord",
                &[
                    (
                        "note",
                        question
                            .answer
                            .first()
                            .map(|&p| name(p))
                            .unwrap_or_default(),
                    ),
                    (
                        "quality",
                        self.tr(
                            &format!("ear.chord.{}", question.chord.unwrap_or("major")),
                            &[],
                        ),
                    ),
                ],
            ),
        };
        let played: Vec<serde_json::Value> = q
            .played
            .iter()
            .map(|p| {
                let direction = match p.offset {
                    o if o > 0 => "high",
                    o if o < 0 => "low",
                    _ => "",
                };
                json!({ "note": name(p.pitch), "right": p.right, "direction": direction })
            })
            .collect();

        // Once solved, where the answer lies; any octave for a chord's root.
        let instrument = self.instrument();
        let mut positions: Vec<serde_json::Value> = Vec::new();
        // An interval's first note is named in the prompt: shown at once where it lies (QA
        // 14.10., P1 found the start only by trying).
        if let (ear::EarKind::Interval, Some(root)) = (q.kind, question.root) {
            for (string, fret) in instrument.positions(root) {
                positions.push(json!({ "string": string, "fret": fret, "role": "root" }));
            }
        }
        if q.solved {
            let wanted: Vec<kontra_model::Pitch> = match question.matching {
                ear::Match::Exact => question.answer.clone(),
                ear::Match::AnyOctave => question
                    .answer
                    .iter()
                    .flat_map(|p| (0..4).filter_map(move |o| p.transposed(12 * o - 12)))
                    .collect(),
            };
            for pitch in wanted {
                for (string, fret) in instrument.positions(pitch) {
                    positions.push(json!({ "string": string, "fret": fret, "role": "current" }));
                }
            }
        }
        json!({
            "prompt": prompt,
            "prompting": self.ear_prompting,
            "solved": q.solved,
            "progress": q.progress,
            "total": question.answer.len(),
            "played": played,
            "reveal": reveal,
            "score": self.score_text("ear.score", q.right, q.asked, q.solved),
            "positions": positions,
            "silent": self.live_ear.as_ref().is_some_and(LiveEar::output_is_silent),
            "listening": self.live_ear.is_some(),
        })
        .to_string()
    }

    /// Writes a generated exercise (LH-LRN-04) into the data folder and returns its path.
    pub fn generate_exercise(&mut self, spec: &exercise::ExerciseSpec) -> Result<PathBuf, String> {
        let generated = exercise::generate(spec);
        let dir = self.data_dir.join("exercises");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        // The file is named in the UI's language; the title inside stays German (progress key).
        let name: String = self
            .exercise_title(&exercise::describe(spec))
            .chars()
            // ♯ and ♭ as before the printed signs, so a file made again keeps its name.
            .map(|c| match c {
                '/' | ':' | '#' | '♯' => '-',
                '♭' => 'b',
                _ => c,
            })
            .collect();
        let path = dir.join(format!("{name}.mid"));
        std::fs::write(&path, &generated.midi).map_err(|e| e.to_string())?;
        Ok(path)
    }

    /// Path of the bundled first exercise: next to the binary when installed, else in the repo.
    pub fn demo_path() -> PathBuf {
        let installed = std::env::current_exe().ok().and_then(|e| {
            e.parent()
                .map(|p| p.join("../share/kontra/demo/etude-1.mid"))
        });
        let path = match installed {
            Some(path) if path.exists() => path,
            _ => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/demo/etude-1.mid"),
        };
        // Without "..", as `exercises_dir`: progress is keyed by the path.
        std::fs::canonicalize(&path).unwrap_or(path)
    }

    pub fn devices_json(&self, input: bool) -> String {
        let direction = if input {
            Direction::Input
        } else {
            Direction::Output
        };
        let devices = list_devices(direction);
        // Input: the one Kontra takes when none is chosen (LH-EIN-02).
        let preferred = preferred_input(&devices)
            .filter(|_| input)
            .map(|d| d.id.clone());
        let list: Vec<_> = devices
            .iter()
            .map(|d| json!({ "id": d.id, "name": d.name, "default": d.is_default, "connection": format!("{:?}", d.connection), "preferred": preferred.as_ref() == Some(&d.id), "direct": d.past_pipewire }))
            .collect();
        serde_json::Value::Array(list).to_string()
    }
}

/// Days of workshop history in the tempo chart (LH-FOR-02).
const HISTORY_DAYS: u64 = 30;
/// Days the start page sums up as "this week".
const WEEK_DAYS: u64 = 7;
/// Plan exercises open in a run-through: evaluation and "next exercise" at the end (round 12).
const PLAN_MODE: &str = "song";
/// Generator choices of `Core::exercise_spec`: judged by pitch and time, or by time only.
const JUDGING_PITCH: &str = "pitch";
const JUDGING_TIME: &str = "time";

/// Saved mode of workshop runs and of blind probes (DIDAKTIK I1).
const WORKSHOP_MODE: &str = "workshop";
const PROBE_MODE: &str = "probe";

/// A probe this many points below the workshop or closer still counts as holding (I1).
const PROBE_SAME_POINTS: f64 = 5.0;
/// "Spots from yesterday": runs of the last days count, the last hours wait (one night).
const SPOT_DAYS: u64 = 3;
const SPOT_REST_S: u64 = 8 * 60 * 60;
/// Bars on the card.
const SPOTS: usize = 2;
/// Phrases from hesitating (DIDAKTIK I6): a bar counts when the player hesitated there in at
/// least `HESITATION_MIN` of the piece's last `HESITATION_RUNS` runs; once may be a slip.
const HESITATION_RUNS: usize = 5;
const HESITATION_MIN: usize = 2;

/// A bundled exercise sits above the workshop's target band (80–90 %, round 5).
const MASTERED_RATE: f64 = 0.9;
/// A groove judged by time only sits when its spread is at most this (DIDAKTIK I4): then the
/// frets come in. Own choice (25.10.), not from a study; to be checked with real beginners.
const PULSE_SPREAD_MS: f64 = 25.0;

/// Place of a bundled exercise in the plan, from its file name "NN …" (unnumbered last).
fn plan_number(path: &Path) -> (usize, PathBuf) {
    let number = path
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.split_once(' '))
        .and_then(|(n, _)| n.parse().ok())
        .unwrap_or(usize::MAX);
    (number, path.to_path_buf())
}

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

/// How many notes the evaluation lists as hardest.
const HARDEST_NOTES: usize = 5;

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

/// The goal key of a bundled exercise ("07 …" is plan entry 7), e.g. "scale".
fn bundled_goal_key(stem: &str) -> Option<&'static str> {
    let number = stem
        .split_once(' ')
        .and_then(|(n, _)| n.parse::<usize>().ok())?;
    exercise::bundled_specs()
        .into_iter()
        .nth(number.saturating_sub(1))
        .map(|(_, spec)| exercise::describe(&spec).goal)
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

/// Weight of a state for "hardest notes": a missed or wrong note before an off-pitch one,
/// before one off in time; a clean hit is 0.
fn severity(state: &str) -> u8 {
    match state {
        "missed" | "wrong" => 3,
        "offpitch" => 2,
        "early" | "late" => 1,
        _ => 0,
    }
}

/// The hardest notes of `notes_details`: worst state first, then the largest timing offset.
fn hardest_notes(notes: &[serde_json::Value]) -> Vec<serde_json::Value> {
    let key = |n: &serde_json::Value| {
        let severity = n["severity"].as_u64().unwrap_or(0);
        let offset = n["offsetMs"].as_f64().map_or(0.0, f64::abs);
        (severity, offset)
    };
    let mut hard: Vec<&serde_json::Value> = notes.iter().filter(|n| key(n).0 > 0).collect();
    hard.sort_by(|a, b| {
        let (a, b) = (key(a), key(b));
        b.0.cmp(&a.0).then(b.1.total_cmp(&a.1))
    });
    hard.into_iter().take(HARDEST_NOTES).cloned().collect()
}

/// Frets one hand covers without shifting: index finger on the position's fret, pinky three up.
const HAND_SPAN: u8 = 3;

/// Hand position (Lage: the index finger's fret, at least 1) for every note, from the tab.
/// The hand stays while notes fit; a note outside moves it to the position, among those
/// holding that note, that also holds the most notes after it. Open strings fit anywhere.
///
/// ```text
/// frets 3 5 7 9 7      positions 3 3 6 6 6    (7 needs a shift; 6–9 holds 7, 9 and 7)
/// ```
fn hand_positions(notes: &[kontra_model::Note]) -> Vec<u8> {
    let frets: Vec<u8> = notes.iter().map(|n| n.fret.unwrap_or(0)).collect();
    let fits =
        |position: u8, fret: u8| fret == 0 || (position..=position + HAND_SPAN).contains(&fret);
    let mut out = Vec::with_capacity(frets.len());
    let mut position = 0u8;
    for (index, &fret) in frets.iter().enumerate() {
        if position == 0 || !fits(position, fret) {
            position = best_position(&frets[index..], position, fits);
        }
        out.push(position.max(1));
    }
    out
}

/// The position for `frets[0]` that holds the longest run of the following notes; ties keep
/// the hand nearer to `from`.
fn best_position(frets: &[u8], from: u8, fits: impl Fn(u8, u8) -> bool) -> u8 {
    let Some(&first) = frets.iter().find(|f| **f > 0) else {
        return from;
    };
    // The first position puts the index finger on the first note when nothing else decides.
    let from = if from == 0 { first } else { from };
    let candidates = first.saturating_sub(HAND_SPAN).max(1)..=first;
    let reach = |p: u8| frets.iter().take_while(|f| fits(p, **f)).count();
    candidates
        .map(|p| (reach(p), std::cmp::Reverse(p.abs_diff(from)), p))
        .max()
        .map_or(first, |(_, _, p)| p)
}

/// Written length in quarter notes, rounded to the nearest sixteenth: played (MIDI) notes are
/// often a little shorter than written (staccato), and notation must not turn eighths into
/// sixteenths.
fn written_beats(duration_ticks: u64) -> f64 {
    written_ticks(duration_ticks) as f64 / kontra_model::TICKS_PER_QUARTER as f64
}

/// Written length in ticks, rounded as `written_beats`.
fn written_ticks(duration_ticks: u64) -> u64 {
    let sixteenth = kontra_model::TICKS_PER_QUARTER / 4;
    ((duration_ticks as f64 / sixteenth as f64).round() as u64).max(1) * sixteenth
}

/// Bass is written an octave above its sound (bass clef, as in MuseScore and Guitar Pro):
/// low B (B0, 6 ledger lines) reads as B1 (2 ledger lines). Only the staff is shifted; the
/// importers already turned any written octave (<transpose>) into sounding pitch, and tab,
/// scoring and tuner stay sounding.
const WRITTEN_OCTAVE_UP: u8 = 12;

fn empty_staff() -> serde_json::Value {
    json!({ "notes": [], "rests": [], "keyFifths": 0, "chords": [] })
}

/// `staff_json` for an arrangement. The staff shows one key; it is the first bar's.
fn staff(a: &Arrangement) -> serde_json::Value {
    let techniques = technique_signs(a);
    let notes: Vec<_> = a
        .notes
        .iter()
        .zip(ties_forward(&a.notes))
        .enumerate()
        .map(|(i, (n, tied))| {
            json!({
                "time": a.tempo.seconds_at(n.start_ticks),
                "beats": written_beats(n.duration_ticks),
                "midi": n.pitch.0.saturating_add(WRITTEN_OCTAVE_UP),
                "tied": tied,
                "state": "pending",
                "technique": techniques.get(&i).map_or("", |t| technique_key(*t)),
                "techniqueResult": "",
            })
        })
        .collect();
    let quarter = kontra_model::TICKS_PER_QUARTER as f64;
    let rests: Vec<_> = rests(a)
        .into_iter()
        .map(|(tick, ticks)| {
            json!({ "time": a.tempo.seconds_at(tick), "beats": ticks as f64 / quarter })
        })
        .collect();
    let key = a.bars.first().map_or(0, |b| b.key_fifths);
    // Chord symbols within a bar (MusicXML: B♭m7 on beat 3); those on a bar start come with
    // `bars_json`.
    let chords: Vec<_> = a
        .markers
        .iter()
        .filter(|m| is_chord(m) && !a.bars.iter().any(|b| b.start_ticks == m.tick))
        .map(|m| json!({ "time": a.tempo.seconds_at(m.tick), "text": m.name }))
        .collect();
    json!({ "notes": notes, "rests": rests, "keyFifths": key, "chords": chords })
}

/// Key signature of the bar holding `tick` (0 = C major without bars).
fn key_at(a: &Arrangement, tick: u64) -> i8 {
    a.bar_at(tick)
        .and_then(|b| a.bars.get(b))
        .map_or(0, |b| b.key_fifths)
}

/// Kontra marks the note that continues a tie (`tied`: no new attack); Kante's staff marks
/// the note tied to the next one. The last earlier note of the same pitch gets the tie.
fn ties_forward(notes: &[kontra_model::Note]) -> Vec<bool> {
    let mut out = vec![false; notes.len()];
    for (index, note) in notes.iter().enumerate() {
        if !note.tied {
            continue;
        }
        let before = notes[..index].iter().rposition(|n| n.pitch == note.pitch);
        if let Some(before) = before {
            out[before] = true;
        }
    }
    out
}

/// Rests as (start, length) in ticks: the gaps between written notes in each bar, on the
/// sixteenth grid. A gap fills the bar → one bar rest; else it is split into values that
/// start on their own beat, e.g. a gap from beat 2 to the end of 4/4 → quarter, half.
fn rests(a: &Arrangement) -> Vec<(u64, u64)> {
    let mut out = Vec::new();
    let mut covered = 0u64;
    let mut notes = a.notes.iter().peekable();
    for bar in &a.bars {
        let (start, end) = (bar.start_ticks, bar.end_ticks());

        // Gaps between the notes starting in this bar; a note may reach into the next bar.
        let mut gaps = Vec::new();
        while let Some(note) = notes.next_if(|n| n.start_ticks < end) {
            let from = covered.max(start);
            if note.start_ticks > from {
                gaps.push((from, note.start_ticks));
            }
            let written = note.start_ticks + written_ticks(note.duration_ticks);
            covered = covered.max(note.end_ticks()).max(written);
        }
        if covered < end {
            gaps.push((covered.max(start), end));
        }

        for (from, to) in gaps {
            let (from, to) = (snap(from, start).min(end), snap(to, start).min(end));
            if from == start && to == end {
                out.push((start, end - start));
                continue;
            }
            split_rest(start, from, to, &mut out);
        }
    }
    out
}

/// `tick` on the sixteenth grid of the bar starting at `bar`.
fn snap(tick: u64, bar: u64) -> u64 {
    let sixteenth = kontra_model::TICKS_PER_QUARTER / 4;
    let offset = tick.saturating_sub(bar);
    bar + (offset + sixteenth / 2) / sixteenth * sixteenth
}

/// Rest values from whole to sixteenth, in ticks.
const REST_TICKS: [u64; 5] = [
    kontra_model::TICKS_PER_QUARTER * 4,
    kontra_model::TICKS_PER_QUARTER * 2,
    kontra_model::TICKS_PER_QUARTER,
    kontra_model::TICKS_PER_QUARTER / 2,
    kontra_model::TICKS_PER_QUARTER / 4,
];

/// Splits the rest `from..to` into the largest values that fit and start on a multiple
/// of their own length within the bar.
fn split_rest(bar: u64, from: u64, to: u64, out: &mut Vec<(u64, u64)>) {
    let mut at = from;
    while at < to {
        let offset = at - bar;
        let value = REST_TICKS
            .into_iter()
            .find(|v| *v <= to - at && offset.is_multiple_of(*v))
            .unwrap_or(to - at);
        out.push((at, value));
        at += value;
    }
}

/// Part names the exercise generator writes in German (`exercise/song.rs`), with their keys.
const GENERATED_PARTS: [(&str, &str); 2] = [("Strophe", "part.verse"), ("Refrain", "part.chorus")];

/// Names of the parts that begin in bar `index`, e.g. `["Chorus", "B"]`; `None` without one.
fn bar_sections(a: &Arrangement, index: usize) -> Option<Vec<String>> {
    let start = a.bars.get(index)?.start_ticks;
    let end = a.bars.get(index + 1).map_or(u64::MAX, |b| b.start_ticks);
    let names: Vec<String> = a
        .markers
        .iter()
        .filter(|m| (start..end).contains(&m.tick) && !is_chord(m))
        .map(|m| m.name.clone())
        .collect();
    (!names.is_empty()).then_some(names)
}

/// A marker that names a chord ("Am7", "F#m7b5", "Bb/D") rather than a section ("Chorus").
/// A bare letter ("A", "Bb") counts as a rehearsal mark: Guitar Pro files use those.
/// A chord symbol: marked as one by the file, or a marker named like one (MIDI).
fn is_chord(marker: &kontra_model::Marker) -> bool {
    marker.kind == kontra_model::MarkerKind::Chord || is_chord_name(&marker.name)
}

fn is_chord_name(name: &str) -> bool {
    let mut chars = name.chars();
    if !matches!(chars.next(), Some('A'..='G')) {
        return false;
    }
    let rest: String = chars.collect();
    let rest = rest.strip_prefix(['#', 'b']).unwrap_or(&rest);
    if rest.is_empty() {
        return false;
    }
    const WORDS: [&str; 7] = ["maj", "min", "dim", "aug", "sus", "add", "m"];
    let mut s = rest;
    while !s.is_empty() {
        if let Some(w) = WORDS.iter().find(|w| s.starts_with(**w)) {
            s = &s[w.len()..];
        } else if s.starts_with(|c: char| {
            c.is_ascii_digit() || "#b/+-()°ø".contains(c) || ('A'..='G').contains(&c)
        }) {
            s = &s[s.chars().next().map_or(1, char::len_utf8)..];
        } else {
            return false;
        }
    }
    true
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
    let chosen = match settings.tuning.as_str() {
        "drop" => drop_lowest(&standard),
        "half-down" => standard.shifted("half-down", -1),
        "whole-down" => standard.shifted("whole-down", -2),
        "custom" if settings.custom_tuning.len() == standard.string_count() => {
            let names: Vec<&str> = settings.custom_tuning.iter().map(String::as_str).collect();
            Tuning::from_names("custom", &names).ok()
        }
        _ => None,
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

/// LH-EIN-02: with no input chosen, an interface (USB) before the built-in card, the system's
/// default first if it is one. `None`: no interface, the system decides.
/// The saved input if it is listed, else an interface (LH-EIN-02): an unplugged device or an
/// ALSA id saved before PipeWire means "Automatic", not the system default (review #73).
fn chosen_input(saved: Option<&str>, devices: &[DeviceInfo]) -> Option<String> {
    if let Some(id) = saved.filter(|id| devices.iter().any(|d| d.id == *id)) {
        return Some(id.to_string());
    }
    preferred_input(devices).map(|d| d.id.clone())
}

/// The saved output if it is listed (or the engine's timer), else the system default.
fn chosen_output(saved: Option<&str>, devices: &[DeviceInfo]) -> Option<String> {
    let id = saved?;
    if id == kontra_session::NULL_DEVICE || devices.iter().any(|d| d.id == id) {
        return Some(id.to_string());
    }
    None
}

fn preferred_input(devices: &[DeviceInfo]) -> Option<&DeviceInfo> {
    let usb = |d: &&DeviceInfo| d.connection == Connection::Usb;
    devices
        .iter()
        .filter(usb)
        .find(|d| d.is_default)
        .or_else(|| devices.iter().find(usb))
}

/// Block length in ms if it is longer than 512 frames at 48 kHz (10.7 ms): beyond that the
/// latency target (LH-NF-01) cannot be met.
/// The mixer's volumes for the player; monitoring off mutes the input.
fn gains(settings: &Settings) -> Gains {
    let volume = |v: f64| v.clamp(0.0, 1.0) as f32;
    Gains {
        click: volume(settings.metronome_volume),
        tone: volume(settings.tone_volume),
        backing: volume(settings.backing_volume),
        drums: volume(settings.drums_volume),
        bass: volume(settings.bass_volume),
        monitor: if settings.monitor {
            volume(settings.monitor_volume)
        } else {
            0.0
        },
        // The shaper clamps the ranges itself.
        shaping: Shaping {
            low_db: settings.monitor_low_db as f32,
            mid_db: settings.monitor_mid_db as f32,
            high_db: settings.monitor_high_db as f32,
            compress: settings.monitor_compress as f32,
            warmth: settings.monitor_warmth as f32,
            bands: instrument_from(settings).tone_bands(),
        },
    }
}

/// Ear training plays its prompts as tones at the "Vorspielen" volume (round 8).
fn ear_gains(settings: &Settings) -> Gains {
    Gains {
        tone: settings.prompt_volume.clamp(0.0, 1.0) as f32,
        ..gains(settings)
    }
}

/// A seed that differs between rounds (quiz and ear training questions).
fn time_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |d| d.as_nanos() as u64)
}

fn quantum_too_long_ms(frames: u32, rate: u32) -> Option<f64> {
    let ms = f64::from(frames) / f64::from(rate.max(1)) * 1000.0;
    (ms > 512.0 / 48.0 + 0.05).then_some(ms)
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into())
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// The name a piece is shown by: its title, or the file name when the title names nothing
/// ("", "001", "1." from Guitar Pro templates; QA 27.10., J4).
fn piece_name(title: &str, path: &Path) -> String {
    if title.chars().any(char::is_alphabetic) {
        return title.to_string();
    }
    path.file_stem()
        .map_or_else(|| title.to_string(), |s| s.to_string_lossy().into())
}

#[cfg(test)]
mod tests;
