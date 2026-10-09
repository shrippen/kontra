//! Qt-free core of the app: everything the QML bridge calls, testable without a GUI.
//!
//! ```text
//! QML ──invokables──► bridge (cxx-qt) ──► Core ──► kontra-session (Live, LiveTuner, service)
//!     ◄──properties/signals── bridge ◄── UiEvent
//! ```

mod data;
mod i18n;
mod package;
mod report;
mod store;

pub use data::DataImport;
pub use i18n::{Catalog, Language};
pub use package::PackageAudio;
pub use store::{data_dir, InputKind, Library, RecentEntry, Setlist, Settings};

use kontra_model::{Arrangement, BassKind, InputSource};
use kontra_session::{
    import_file, list_devices, run_from, CalibrationResult, Direction, Gains, ImportResult,
    InputChoice, Live, LiveCalibration, LiveSetup, LiveTuner, LoopRange, Mode, Outcome,
    PracticeEvent, PracticeOptions, ProgressStore, ScoreSettings, SessionEvent, Summary, Workshop,
};
use serde_json::json;
use std::path::{Path, PathBuf};

/// Seconds from the input's start to the first calibration click.
const CALIBRATION_LEAD_S: f64 = 1.0;

/// Options for QA runs (QA.md): a recording instead of the audio input.
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
    /// The audio device failed (unplugged); the run was stopped.
    DeviceLost(String),
    /// Calibration finished: new latency (if measured) and a text for the user.
    Calibrated {
        latency_ms: Option<f64>,
        text: String,
    },
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
    arrangement: Arrangement,
    /// All tracks of the file and the one played, so the player can switch (QA P2 R9).
    tracks: Vec<String>,
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
    /// Engine-independent start of the current run, for practice time.
    run_started: Option<std::time::Instant>,
    /// The audio server's block size was checked for this run (LH-EIN-07).
    quantum_checked: bool,
    speed: f64,
    looping: Option<(usize, usize)>,
    /// A note for the user from the last `open` (e.g. a five-string bass was chosen).
    notice: Option<String>,
    /// The setlist being played and the position in it (LH-BIB-04).
    active_setlist: Option<(String, usize)>,
}

impl Core {
    pub fn new(data_dir: &Path, qa: QaOptions) -> Core {
        let settings: Settings = store::load(&data_dir.join("settings.json"));
        let library: Library = store::load(&data_dir.join("library.json"));
        let language = settings
            .language
            .as_deref()
            .map_or_else(Language::from_environment, Language::from_code);
        Core {
            notice: None,
            active_setlist: None,
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
            run_started: None,
            quantum_checked: false,
            speed: 1.0,
            looping: None,
        }
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
        match (self.catalog.language(), name.strip_prefix('B')) {
            (Language::German, Some(rest)) if !rest.starts_with('#') => format!("H{rest}"),
            _ => name.to_string(),
        }
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
        let args = [
            ("note", self.note_label(&n.pitch.to_string())),
            ("string", string),
            ("fret", n.fret.unwrap_or(0).to_string()),
        ];
        let wanted = self.tr("waiting.for", &args);
        Some(match played_midi {
            Some(m) => {
                let played = kontra_model::Pitch(m.round().clamp(0.0, 127.0) as u8).to_string();
                let played = self.tr("waiting.played", &[("played", self.note_label(&played))]);
                format!("{played} {wanted}")
            }
            None => wanted,
        })
    }

    /// Scoring from the instrument, the input and the user's choices (LH-BEW-02, -05).
    pub fn score_settings(&self, kind: BassKind) -> ScoreSettings {
        let mut settings = ScoreSettings::defaults(kind, self.source());
        settings.latency_ms = self.settings.latency_ms;
        settings.strictness = match self.settings.strictness.as_str() {
            "loose" => kontra_session::Strictness::Loose,
            "normal" => kontra_session::Strictness::Normal,
            "strict" => kontra_session::Strictness::Strict,
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
        let instrument = self
            .arrangement()
            .map_or_else(|| instrument_from(&self.settings), |a| a.instrument.clone());
        let strings: Vec<serde_json::Value> = instrument
            .tuning
            .strings
            .iter()
            .map(|p| json!({ "name": self.note_label(&p.name().to_string()), "midi": p.0 }))
            .collect();
        serde_json::Value::Array(strings).to_string()
    }

    /// Progress over all pieces (LH-FOR-01): per piece best result, best tempo and time, and
    /// every run's time stamp and length so the view can sum days in local time.
    pub fn progress_json(&self) -> String {
        let mut songs = self.progress_store().all();
        songs.sort_by_key(|p| std::cmp::Reverse(p.runs.iter().map(|r| r.finished_unix).max()));
        let pieces: Vec<serde_json::Value> = songs
            .iter()
            .map(|p| {
                json!({
                    "songId": p.song_id,
                    "title": p.title,
                    "artist": p.artist,
                    "best": p.best_hit_rate(),
                    "bestSpeed": p.best_speed(0.9),
                    "practiceMin": p.practice_s() / 60.0,
                    "runs": p.runs.len(),
                    "lastUnix": p.runs.iter().map(|r| r.finished_unix).max(),
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

    /// Tracks of the loaded file and the one played: `{"names": [...], "current": i}`.
    pub fn tracks_json(&self) -> String {
        let (names, current) = self
            .loaded
            .as_ref()
            .map_or((Vec::new(), 0), |l| (l.tracks.clone(), l.track));
        json!({ "names": names, "current": current }).to_string()
    }

    /// Plays another track of the loaded file (Kontra's guess may be the wrong one).
    pub fn switch_track(&mut self, track: usize) -> Result<(), String> {
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
        change(&mut self.settings);
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

    /// Changes the mixer (volumes, monitoring) and saves; a running practice hears it at once.
    /// Returns a hint when monitoring is on but cannot work.
    pub fn remix(&mut self, change: impl FnOnce(&mut Settings)) -> Option<String> {
        self.update_settings(change);
        let gains = gains(&self.settings);
        if let Some(live) = self.live.as_mut() {
            live.practice_mut().set_gains(gains);
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

    pub fn recent_json(&self) -> String {
        let store = self.progress_store();
        let list: Vec<_> = self
            .library
            .recent
            .iter()
            .map(|e| {
                let progress = store.load_id(&e.song_id);
                let best = progress.as_ref().and_then(|p| p.best_hit_rate());
                let minutes = progress.as_ref().map_or(0.0, |p| p.practice_s() / 60.0);
                json!({ "path": e.path, "title": e.title, "artist": e.artist, "best": best, "practiceMin": minutes, "lastUnix": e.last_unix, "hasBacking": e.backing.is_some() })
            })
            .collect();
        serde_json::Value::Array(list).to_string()
    }

    /// Adds every score in `dir` to the library without playing it (LH-BIB-04, J-B1); with
    /// `as_setlist` also a setlist named after the folder, in file name order. Returns how many
    /// were added and, for each file that failed, its name and the reason in the user's language.
    pub fn import_folder(
        &mut self,
        dir: &Path,
        as_setlist: bool,
    ) -> (usize, Vec<(String, String)>) {
        const EXTENSIONS: [&str; 9] = [
            "gp", "gp3", "gp4", "gp5", "gpx", "mid", "midi", "musicxml", "mxl",
        ];
        let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
            .map(|entries| entries.filter_map(Result::ok).map(|e| e.path()).collect())
            .unwrap_or_default();
        files.retain(|p| {
            p.extension().and_then(|e| e.to_str()).is_some_and(|e| {
                EXTENSIONS.contains(&e.to_ascii_lowercase().as_str())
                    || e.eq_ignore_ascii_case("xml")
            })
        });
        files.sort();
        let mut added = Vec::new();
        let mut failed = Vec::new();
        for path in files {
            let name = path
                .file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into());
            let arrangement = import_file(&path)
                .map_err(|e| self.import_error(&e))
                .and_then(|i| {
                    i.arrangement(i.suggested().unwrap_or(0))
                        .map_err(|e| self.import_error(&e))
                });
            match arrangement {
                Ok(a) => {
                    let title = if a.title.trim().is_empty() {
                        path.file_stem()
                            .map_or_else(String::new, |s| s.to_string_lossy().into())
                    } else {
                        a.title.clone()
                    };
                    self.library.add_known(RecentEntry {
                        path: path.clone(),
                        title,
                        artist: a.artist.clone(),
                        track: 0,
                        last_unix: 0,
                        song_id: ProgressStore::song_id(&a),
                        backing: None,
                        backing_offset_s: 0.0,
                        view: None,
                    });
                    added.push(path);
                }
                Err(reason) => failed.push((name, reason)),
            }
        }
        if as_setlist && !added.is_empty() {
            let name = dir
                .file_name()
                .map_or_else(|| "Setlist".into(), |n| n.to_string_lossy().into());
            self.library.setlists.retain(|s| s.name != name);
            self.library.setlists.push(Setlist {
                name,
                pieces: added.clone(),
            });
        }
        self.save_library();
        (added.len(), failed)
    }

    pub fn setlist_create(&mut self, name: &str) {
        let name = name.trim();
        if !name.is_empty() && self.library.setlist_mut(name).is_none() {
            self.library.setlists.push(Setlist {
                name: name.to_string(),
                pieces: Vec::new(),
            });
            self.save_library();
        }
    }

    pub fn setlist_delete(&mut self, name: &str) {
        self.library.setlists.retain(|s| s.name != name);
        self.save_library();
    }

    /// Adds the loaded piece to the end of `name`.
    pub fn setlist_add_loaded(&mut self, name: &str) {
        let Some(path) = self.loaded_path().map(Path::to_path_buf) else {
            return;
        };
        if let Some(list) = self.library.setlist_mut(name) {
            list.pieces.push(path);
            self.save_library();
        }
    }

    pub fn setlist_remove(&mut self, name: &str, index: usize) {
        if let Some(list) = self.library.setlist_mut(name) {
            if index < list.pieces.len() {
                list.pieces.remove(index);
                self.save_library();
            }
        }
    }

    /// Moves the piece at `from` one place up (`-1`) or down (`1`).
    pub fn setlist_move(&mut self, name: &str, from: usize, step: i32) {
        if let Some(list) = self.library.setlist_mut(name) {
            let to = from as i64 + i64::from(step);
            if from < list.pieces.len() && (0..list.pieces.len() as i64).contains(&to) {
                list.pieces.swap(from, to as usize);
                self.save_library();
            }
        }
    }

    /// Opens piece `index` of setlist `name` and remembers the position for `next_in_setlist`.
    pub fn open_from_setlist(&mut self, name: &str, index: usize) -> Result<Opened, String> {
        let path = self
            .library
            .setlists
            .iter()
            .find(|s| s.name == name)
            .and_then(|s| s.pieces.get(index))
            .cloned()
            .ok_or("no such piece")?;
        let opened = self.open(&path)?;
        self.active_setlist = Some((name.to_string(), index));
        Ok(opened)
    }

    /// Title of the next piece in the active setlist, if any.
    pub fn next_title(&self) -> Option<String> {
        let (name, index) = self.active_setlist.as_ref()?;
        let list = self.library.setlists.iter().find(|s| &s.name == name)?;
        let path = list.pieces.get(index + 1)?;
        Some(self.library.entry(path).map_or_else(
            || {
                path.file_stem()
                    .map_or_else(String::new, |s| s.to_string_lossy().into())
            },
            |e| e.title.clone(),
        ))
    }

    /// Opens the next piece of the active setlist.
    pub fn open_next(&mut self) -> Result<Opened, String> {
        let (name, index) = self.active_setlist.clone().ok_or("no setlist")?;
        self.open_from_setlist(&name, index + 1)
    }

    pub fn setlists_json(&self) -> String {
        let lists: Vec<serde_json::Value> = self
            .library
            .setlists
            .iter()
            .map(|l| {
                let pieces: Vec<serde_json::Value> = l
                    .pieces
                    .iter()
                    .map(|p| {
                        let e = self.library.entry(p);
                        json!({
                            "path": p,
                            "title": e.map_or_else(|| p.file_stem().map_or_else(String::new, |s| s.to_string_lossy().into()), |e| e.title.clone()),
                            "artist": e.map_or_else(String::new, |e| e.artist.clone()),
                            "missing": !p.exists(),
                        })
                    })
                    .collect();
                json!({ "name": l.name, "pieces": pieces })
            })
            .collect();
        serde_json::Value::Array(lists).to_string()
    }

    fn save_library(&self) {
        let _ = store::save(&self.data_dir.join("library.json"), &self.library);
    }

    /// Imports a score; picks the bass track or asks.
    pub fn open(&mut self, path: &Path) -> Result<Opened, String> {
        self.active_setlist = None;
        if package::is_package(path) {
            return self.open_package(path).map(|()| Opened::Ready);
        }
        let imported = import_file(path).map_err(|e| self.import_error(&e))?;
        if let Some(track) = imported.suggested() {
            return self.load(path, &imported, track).map(|()| Opened::Ready);
        }
        let names = imported
            .tracks()
            .iter()
            .map(|t| t.name().to_string())
            .collect();
        self.pending = Some((path.to_path_buf(), imported));
        Ok(Opened::ChooseTrack(names))
    }

    pub fn choose_track(&mut self, track: usize) -> Result<(), String> {
        let (path, imported) = self.pending.take().ok_or("nothing to choose from")?;
        self.load(&path, &imported, track)
    }

    fn load(&mut self, path: &Path, imported: &ImportResult, track: usize) -> Result<(), String> {
        let mut arrangement = imported
            .arrangement(track)
            .map_err(|e| self.import_error(&e))?;
        if arrangement.title.trim().is_empty() {
            arrangement.title = path
                .file_stem()
                .map_or_else(String::new, |s| s.to_string_lossy().to_string());
        }
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
        if self.settings.fretless {
            arrangement.instrument.kind = BassKind::Fretless;
        }
        self.stop_live();
        self.looping = None;
        self.library.touch(RecentEntry {
            path: path.to_path_buf(),
            title: arrangement.title.clone(),
            artist: arrangement.artist.clone(),
            track,
            last_unix: unix_now(),
            song_id: ProgressStore::song_id(&arrangement),
            backing: None,
            backing_offset_s: 0.0,
            view: None,
        });
        let _ = store::save(&self.data_dir.join("library.json"), &self.library);
        self.loaded = Some(Loaded {
            path: path.to_path_buf(),
            arrangement,
            tracks: imported
                .tracks()
                .iter()
                .map(|t| t.name().to_string())
                .collect(),
            track,
        });
        Ok(())
    }

    pub fn arrangement(&self) -> Option<&Arrangement> {
        self.loaded.as_ref().map(|l| &l.arrangement)
    }

    pub fn loaded_path(&self) -> Option<&Path> {
        self.loaded.as_ref().map(|l| l.path.as_path())
    }

    /// Notes for the views: time and duration in seconds, string/fret, beats, midi.
    pub fn notes_json(&self) -> String {
        let Some(a) = self.arrangement() else {
            return "[]".into();
        };
        let notes: Vec<_> = a
            .notes
            .iter()
            .map(|n| {
                let time = a.tempo.seconds_at(n.start_ticks);
                json!({
                    "time": time,
                    "duration": a.tempo.seconds_at(n.end_ticks()) - time,
                    "string": n.string.unwrap_or(0),
                    "fret": n.fret.unwrap_or(0),
                    "midi": n.pitch.0,
                    "beats": written_beats(n.duration_ticks),
                    "tied": n.tied,
                    "state": "pending",
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
            .map(|b| {
                json!({
                    "time": a.tempo.seconds_at(b.start_ticks),
                    "number": b.source_index + 1,
                    "numerator": b.signature.numerator,
                    "denominator": b.signature.denominator,
                })
            })
            .collect();
        serde_json::Value::Array(bars).to_string()
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

    pub fn duration_s(&self) -> f64 {
        self.arrangement().map_or(0.0, Arrangement::duration_s)
    }

    pub fn position_s(&self) -> f64 {
        self.live
            .as_ref()
            .map_or(0.0, |l| l.practice().position_s())
    }

    pub fn is_playing(&self) -> bool {
        self.live
            .as_ref()
            .is_some_and(|l| l.practice().is_playing())
    }

    pub fn play(&mut self) -> Vec<UiEvent> {
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
                Ok(silent) if silent => events.push(UiEvent::Message(self.tr("hint.silent", &[]))),
                Ok(_) => {}
                Err(e) => return vec![UiEvent::Error(e)],
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

    /// Ends the run; the next `play` starts from the beginning with current settings.
    pub fn stop_live(&mut self) {
        // A run stopped before the end still counts as practice (QA P1 J-C1).
        if self.run_started.is_some() {
            self.record_run();
        }
        self.live = None;
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

    pub fn set_mode(&mut self, mode: &str) {
        self.reconfigure(|s| s.mode = mode.to_string());
    }

    /// Loops bars `from..=to` (indices into the arrangement's bars).
    /// The workshop's hit-rate band (low, high), for the UI.
    pub fn workshop_band() -> (f64, f64) {
        Workshop::default().band
    }

    pub fn set_loop(&mut self, from: usize, to: usize) {
        self.looping = Some((from.min(to), from.max(to)));
        let range = self.loop_range();
        if let Some(live) = self.live.as_mut() {
            live.practice_mut().set_loop(range);
        }
    }

    pub fn clear_loop(&mut self) {
        self.looping = None;
        if let Some(live) = self.live.as_mut() {
            live.practice_mut().set_loop(None);
        }
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
        match self.settings.mode.as_str() {
            "song" => Mode::Song,
            "wait" => Mode::Wait,
            "workshop" => Mode::Workshop(Workshop::default()),
            _ => Mode::Play,
        }
    }

    fn source(&self) -> InputSource {
        if self.qa.laptop_jack || self.settings.input_kind == InputKind::LaptopJack {
            InputSource::LaptopJack
        } else {
            InputSource::Line
        }
    }

    fn input_choice(&self) -> InputChoice {
        match &self.qa.input_file {
            Some(path) => InputChoice::File {
                path: path.clone(),
                laptop_jack: self.qa.laptop_jack,
            },
            None => InputChoice::Device {
                id: self.settings.input_device.clone(),
                channel: None,
            },
        }
    }

    /// Starts audio for the loaded arrangement; true if there is no sound output.
    fn start_live(&mut self) -> Result<bool, String> {
        let Some(loaded) = self.loaded.as_ref() else {
            return Err(self.tr("library.empty.hint", &[]));
        };
        self.tuner = None;
        let arrangement = loaded.arrangement.clone();
        let settings = self.score_settings(arrangement.instrument.kind);
        let options = PracticeOptions {
            metronome: self.settings.metronome,
            count_in_bars: self.settings.count_in_bars,
            reference_tones: self.settings.reference_tones,
            backing_offset_s: self.backing().map_or(0.0, |(_, offset)| offset),
        };
        let setup = LiveSetup {
            input: self.input_choice(),
            output_device: self.settings.output_device.clone(),
            source: self.source(),
            // A backing file that is gone is skipped (play() says so), not an error.
            backing: self.backing().map(|(path, _)| path).filter(|p| p.exists()),
        };
        let mut live = Live::start(arrangement, self.mode(), settings, options, &setup)
            .map_err(|e| self.tr("error.audio", &[("reason", e.to_string())]))?;
        live.practice_mut().set_speed(self.speed);
        live.practice_mut().set_loop(self.loop_range());
        live.practice_mut().set_gains(gains(&self.settings));
        let silent = live.output_is_silent;
        self.live = Some(live);
        self.run_started = Some(std::time::Instant::now());
        self.quantum_checked = false;
        Ok(silent)
    }

    pub fn start_tuner(&mut self) -> Result<(), String> {
        self.stop_live();
        let instrument = match self.arrangement() {
            Some(a) => a.instrument.clone(),
            None => instrument_from(&self.settings),
        };
        let tuner = LiveTuner::start(&self.input_choice(), &instrument, self.source())
            .map_err(|e| self.tr("error.audio", &[("reason", e.to_string())]))?;
        self.tuner = Some(tuner);
        Ok(())
    }

    pub fn stop_tuner(&mut self) {
        self.tuner = None;
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
        let mut events: Vec<UiEvent> = raw.into_iter().filter_map(|e| self.translate(e)).collect();
        if events.iter().any(|e| matches!(e, UiEvent::DeviceLost(_))) {
            self.stop_live();
            self.tuner = None;
        }
        if events.iter().any(|e| matches!(e, UiEvent::Finished { .. })) {
            self.record_run();
        }
        if let Some(hint) = self.quantum_check() {
            events.push(UiEvent::Message(hint));
        }
        if let Some(result) = self.calibration.as_mut().and_then(LiveCalibration::poll) {
            self.calibration = None;
            events.push(self.calibrated(result));
        }
        events
    }

    /// LH-EIN-07: once per run, a second after the start, a hint if the audio server's block is
    /// too long for the latency target.
    fn quantum_check(&mut self) -> Option<String> {
        if self.quantum_checked {
            return None;
        }
        let started = self.run_started?;
        if started.elapsed() < std::time::Duration::from_secs(1) {
            return None;
        }
        self.quantum_checked = true;
        let (frames, rate) = self.live.as_ref()?.input_block()?;
        let ms = quantum_too_long_ms(frames, rate)?;
        Some(self.tr(
            "hint.quantum",
            &[("frames", frames.to_string()), ("ms", format!("{ms:.0}"))],
        ))
    }

    /// Starts "play on the click" (LH-EIN-05): 8 clicks from one second after the input starts.
    pub fn start_calibration(&mut self) -> Result<(), String> {
        self.stop_live();
        self.tuner = None;
        let instrument = match self.arrangement() {
            Some(a) => a.instrument.clone(),
            None => instrument_from(&self.settings),
        };
        let setup = LiveSetup {
            input: self.input_choice(),
            output_device: self.settings.output_device.clone(),
            source: self.source(),
            backing: None,
        };
        let calibration = LiveCalibration::start(&setup, &instrument, CALIBRATION_LEAD_S)
            .map_err(|e| self.tr("error.audio", &[("reason", e.to_string())]))?;
        self.calibration = Some(calibration);
        Ok(())
    }

    pub fn is_calibrating(&self) -> bool {
        self.calibration.is_some()
    }

    fn calibrated(&mut self, result: CalibrationResult) -> UiEvent {
        match result {
            CalibrationResult::Measured {
                latency_ms,
                spread_ms,
            } => {
                self.reconfigure(|s| s.latency_ms = latency_ms.round());
                let text = self.tr(
                    "calibration.done",
                    &[
                        ("ms", format!("{latency_ms:.0}")),
                        ("spread", format!("{spread_ms:.0}")),
                    ],
                );
                UiEvent::Calibrated {
                    latency_ms: Some(latency_ms.round()),
                    text,
                }
            }
            CalibrationResult::TooFew { matched } => UiEvent::Calibrated {
                latency_ms: None,
                text: self.tr("calibration.few", &[("matched", matched.to_string())]),
            },
            CalibrationResult::Unsteady { .. } => UiEvent::Calibrated {
                latency_ms: None,
                text: self.tr("calibration.unsteady", &[]),
            },
        }
    }

    fn progress_store(&self) -> ProgressStore {
        ProgressStore::new(&self.data_dir.join("progress"))
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
        let mut run = run_from(
            &summary,
            practice_s,
            self.speed,
            &self.settings.mode,
            section,
            unix_now(),
        );
        let arrangement = session.arrangement().clone();
        // Bar numbers as printed in the score (repeats are unrolled into more bars).
        run.hardest_bars = summary
            .hardest_bars
            .iter()
            .filter_map(|b| arrangement.bars.get(*b))
            .map(|bar| bar.source_index + 1)
            .collect();
        let _ = self.progress_store().record(&arrangement, run);
    }

    /// Backing track of the loaded piece and its offset.
    fn backing(&self) -> Option<(PathBuf, f64)> {
        let path = self.loaded_path()?;
        let entry = self.library.entry(path)?;
        entry.backing.clone().map(|b| (b, entry.backing_offset_s))
    }

    pub fn backing_name(&self) -> String {
        self.backing()
            .and_then(|(p, _)| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_default()
    }

    /// Sets (or with `None` removes) the backing track of the loaded piece.
    pub fn set_backing(&mut self, path: Option<PathBuf>, offset_s: f64) {
        let Some(loaded) = self.loaded_path().map(Path::to_path_buf) else {
            return;
        };
        if let Some(entry) = self.library.entry_mut(&loaded) {
            entry.backing = path;
            entry.backing_offset_s = offset_s;
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
            PracticeEvent::Error(m) => Some(UiEvent::DeviceLost(
                self.tr("error.device.lost", &[("reason", m)]),
            )),
            PracticeEvent::InputEnded => None,
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
                let window = 25.0;
                let state = match r.outcome {
                    Outcome::Hit if offset < -window => "early",
                    Outcome::Hit if offset > window => "late",
                    Outcome::Hit => "hit",
                    // In wait mode the note stays open: show what was played instead of marking it.
                    Outcome::WrongPitch { played_midi } if self.settings.mode == "wait" => {
                        let text = self.wait_text(index, Some(played_midi))?;
                        return Some(UiEvent::Holding { index, text });
                    }
                    Outcome::WrongPitch { .. } => "wrong",
                    Outcome::OffPitch => "offpitch",
                    Outcome::Missed => "missed",
                    Outcome::Extra { .. } => return Some(UiEvent::Extra),
                };
                Some(UiEvent::Judged {
                    index,
                    state,
                    offset_ms: offset,
                    cents: r.cents.unwrap_or(0.0),
                })
            }
            SessionEvent::Holding { note } => {
                let text = self.wait_text(note, None)?;
                Some(UiEvent::Holding { index: note, text })
            }
            SessionEvent::Released => Some(UiEvent::Released),
            SessionEvent::PassComplete {
                clean, hit_rate, ..
            } => Some(UiEvent::PassComplete { clean, hit_rate }),
            SessionEvent::SpeedChanged(s) => Some(UiEvent::SpeedChanged(s)),
            SessionEvent::Finished => {
                let summary = self.live.as_ref()?.practice().session().summary();
                Some(UiEvent::Finished {
                    summary_json: self.summary_json(&summary),
                })
            }
        }
    }

    /// Summary with ready-made texts for the evaluation sheet.
    pub fn summary_json(&self, s: &Summary) -> String {
        let arrangement = self.arrangement();
        let tendency = match s.mean_offset_ms {
            Some(m) if m < -5.0 => self.tr("summary.early", &[("ms", format!("{:.0}", -m))]),
            Some(m) if m > 5.0 => self.tr("summary.late", &[("ms", format!("{m:.0}"))]),
            Some(_) => self.tr("summary.ontime", &[]),
            None => String::new(),
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
        json!({
            "hitRate": s.hit_rate,
            "hitRateText": self.tr("summary.hitrate", &[("percent", format!("{:.0}", s.hit_rate * 100.0))]),
            "countsText": self.tr("summary.counts", &[
                ("hits", s.hits.to_string()),
                ("judged", s.judged.to_string()),
                ("wrong", s.wrong.to_string()),
                ("missed", s.missed.to_string()),
            ]),
            "tendencyText": tendency,
            "spreadText": s.spread_ms.map_or(String::new(), |v| self.tr("summary.spread", &[("ms", format!("{v:.0}"))])),
            // Intonation only where it is scored: a fretless or double bass, or off-pitch notes.
            "intonationText": match s.mean_abs_cents {
                Some(c) if s.off_pitch > 0 || self.scored_intonation() => self.tr("summary.intonation", &[
                    ("offpitch", s.off_pitch.to_string()),
                    ("cents", format!("{c:.0}")),
                ]),
                _ => String::new(),
            },
            "meanOffsetMs": s.mean_offset_ms.unwrap_or(0.0),
            "counts": s.histogram.counts,
            "firstBinMs": s.histogram.first_bin_ms,
            "binMs": s.histogram.bin_ms,
            "hardestBars": hardest,
        })
        .to_string()
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

    /// Path of the bundled first exercise: next to the binary when installed, else in the repo.
    pub fn demo_path() -> PathBuf {
        let installed = std::env::current_exe().ok().and_then(|e| {
            e.parent()
                .map(|p| p.join("../share/kontra/demo/etude-1.mid"))
        });
        match installed {
            Some(path) if path.exists() => path,
            _ => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/demo/etude-1.mid"),
        }
    }

    pub fn devices_json(&self, input: bool) -> String {
        let direction = if input {
            Direction::Input
        } else {
            Direction::Output
        };
        let list: Vec<_> = list_devices(direction)
            .into_iter()
            .map(|d| json!({ "id": d.id, "name": d.name, "default": d.is_default, "connection": format!("{:?}", d.connection) }))
            .collect();
        serde_json::Value::Array(list).to_string()
    }
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
    json!({ "notes": [], "rests": [], "keyFifths": 0 })
}

/// `staff_json` for an arrangement. The staff shows one key; it is the first bar's.
fn staff(a: &Arrangement) -> serde_json::Value {
    let notes: Vec<_> = a
        .notes
        .iter()
        .zip(ties_forward(&a.notes))
        .map(|(n, tied)| {
            json!({
                "time": a.tempo.seconds_at(n.start_ticks),
                "beats": written_beats(n.duration_ticks),
                "midi": n.pitch.0.saturating_add(WRITTEN_OCTAVE_UP),
                "tied": tied,
                "state": "pending",
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
    json!({ "notes": notes, "rests": rests, "keyFifths": key })
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

fn instrument_from(settings: &Settings) -> kontra_model::Instrument {
    let tuning = match settings.strings {
        5 => kontra_model::Tuning::bass5_standard(),
        6 => kontra_model::Tuning::bass6_standard(),
        _ => kontra_model::Tuning::bass4_standard(),
    };
    let kind = if settings.fretless {
        BassKind::Fretless
    } else {
        BassKind::Fretted
    };
    kontra_model::Instrument::new(kind, tuning, 24)
        .unwrap_or_else(|_| kontra_model::Instrument::default_bass())
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
        monitor: if settings.monitor {
            volume(settings.monitor_volume)
        } else {
            0.0
        },
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn demo() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/demo/etude-1.mid")
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("kontra-core-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn written_beats_round_to_sixteenths() {
        let q = kontra_model::TICKS_PER_QUARTER;
        assert_eq!(written_beats(q / 2 - 40), 0.5);
        assert_eq!(written_beats(q), 1.0);
        assert_eq!(written_beats(10), 0.25);
    }

    const Q: u64 = kontra_model::TICKS_PER_QUARTER;

    /// One bar of 4/4 or more, notes as (start, duration, MIDI) in ticks.
    fn arrangement(notes: &[(u64, u64, u8)]) -> Arrangement {
        let mut a = Arrangement::new("Staff", kontra_model::Instrument::default_bass());
        for &(start, duration, midi) in notes {
            a.notes.push(kontra_model::Note::new(
                start,
                duration,
                kontra_model::Pitch(midi),
            ));
        }
        a.normalize();
        a
    }

    #[test]
    fn staff_shows_the_key_of_the_first_bar() {
        let mut a = arrangement(&[(0, 4 * Q, 34), (4 * Q, 4 * Q, 34)]);
        a.bars[0].key_fifths = -2;
        a.bars[1].key_fifths = 1;
        assert_eq!(staff(&a)["keyFifths"], -2);
    }

    /// Rests as (start in quarters, length in quarters); 120 BPM: a quarter is 0.5 s.
    fn rests_of(a: &Arrangement) -> Vec<(f64, f64)> {
        let value = staff(a);
        let rests = value["rests"].as_array().unwrap();
        rests
            .iter()
            .map(|r| {
                let time = r["time"].as_f64().unwrap();
                (time * 2.0, r["beats"].as_f64().unwrap())
            })
            .collect()
    }

    #[test]
    fn staff_rests_fill_gaps_on_their_beat() {
        // | q q q - | q - - - | (empty) | q |
        let a = arrangement(&[
            (0, Q, 28),
            (Q, Q, 30),
            (2 * Q, Q, 31),
            (4 * Q, Q, 33),
            (12 * Q, Q, 35),
        ]);
        assert_eq!(
            rests_of(&a),
            [
                (3.0, 1.0),
                (5.0, 1.0),
                (6.0, 2.0),
                (8.0, 4.0),
                (13.0, 1.0),
                (14.0, 2.0)
            ]
        );
    }

    #[test]
    fn staff_rests_ignore_short_played_notes_and_triplets() {
        // Staccato eighths (played 400 of 480 ticks), quarter triplets, a rest on beat 4.
        let t = Q * 2 / 3;
        let a = arrangement(&[
            (0, 400, 28),
            (Q / 2, 400, 30),
            (Q, t, 31),
            (Q + t, t, 33),
            (Q + 2 * t, t, 35),
        ]);
        assert_eq!(rests_of(&a), [(3.0, 1.0)]);
    }

    #[test]
    fn staff_ties_start_at_the_note_before_the_continuation() {
        // G1 on beat 4 tied over the bar line; Kontra marks the second G1, Kante the first.
        let mut a = arrangement(&[(0, 3 * Q, 28), (3 * Q, Q, 31), (4 * Q, Q, 31)]);
        a.notes[2].tied = true;
        let value = staff(&a);
        let ties: Vec<bool> = value["notes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n["tied"].as_bool().unwrap())
            .collect();
        assert_eq!(ties, [false, true, false]);
    }

    #[test]
    fn staff_writes_bass_an_octave_up() {
        // B0 (MIDI 23) is written as B1 (35).
        let a = arrangement(&[(0, 4 * Q, 23)]);
        assert_eq!(staff(&a)["notes"][0]["midi"], 35);
    }

    #[test]
    fn note_names_follow_the_language() {
        let dir = temp_dir("names");
        let mut core = Core::new(&dir, QaOptions::default());
        core.update_settings(|s| s.language = Some("de".into()));
        assert_eq!(core.note_label("B1"), "H1");
        assert_eq!(core.note_label("A#1"), "A#1");
        assert_eq!(core.note_label("E2"), "E2");
        core.update_settings(|s| s.language = Some("en".into()));
        assert_eq!(core.note_label("B1"), "B1");
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Wait mode (J-B2): the string by its name, not a number counted from the low string, and
    /// after a wrong note what was played.
    #[test]
    fn wait_text_names_string_and_played_note() {
        let dir = temp_dir("wait");
        let mut core = Core::new(&dir, QaOptions::default());
        core.update_settings(|s| s.language = Some("de".into()));
        core.open(&demo()).unwrap();
        let a = core.arrangement().unwrap().clone();
        let (index, n) = a
            .notes
            .iter()
            .enumerate()
            .find(|(_, n)| n.string.is_some())
            .unwrap();
        let open = a.instrument.tuning.strings[usize::from(n.string.unwrap())]
            .name()
            .to_string();
        let text = core.wait_text(index, None).unwrap();
        assert!(text.contains(&format!("{open}-Saite")), "{text}");
        let played = f64::from(n.pitch.0) + 1.0;
        let text = core.wait_text(index, Some(played)).unwrap();
        let played_name = core.note_label(&kontra_model::Pitch(n.pitch.0 + 1).to_string());
        assert!(text.contains(&played_name), "{text}");
        let _ = std::fs::remove_dir_all(dir);
    }

    /// QA P3 (F13): a file without tuning (MIDI, MusicXML) takes the instrument profile; it used
    /// to fall back to four strings and overwrite a five-string profile.
    #[test]
    fn file_without_tuning_uses_the_profile() {
        let dir = temp_dir("profile");
        let mut core = Core::new(&dir, QaOptions::default());
        core.reconfigure(|s| s.strings = 5);
        core.open(&demo()).unwrap();
        let names: Vec<String> = serde_json::from_str(&core.string_names_json()).unwrap();
        assert_eq!(names.len(), 5, "{names:?}");
        let a = core.arrangement().unwrap();
        assert!(a.notes.iter().all(|n| n.string.is_some_and(|s| s < 5)));
        let _ = std::fs::remove_dir_all(dir);
    }

    /// QA P2 (F9): a backing file that is gone must not stop the piece; it plays without, and
    /// says so in the user's language.
    #[test]
    fn missing_backing_plays_without_it() {
        let dir = temp_dir("backing");
        let take = dir.join("take.wav");
        std::fs::create_dir_all(&dir).unwrap();
        kontra_session::write_mono_wav(&take, &vec![0.0; 48_000], 48_000).unwrap();
        let mut core = Core::new(
            &dir,
            QaOptions {
                input_file: Some(take),
                laptop_jack: false,
            },
        );
        core.update_settings(|s| s.language = Some("de".into()));
        core.open(&demo()).unwrap();
        core.set_backing(Some(dir.join("weg.wav")), 0.0);
        let events = core.play();
        assert!(
            !events.iter().any(|e| matches!(e, UiEvent::Error(_))),
            "{events:?}"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, UiEvent::Message(m) if m.contains("weg.wav"))),
            "{events:?}"
        );
        core.stop_live();
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn gains_follow_the_mixer() {
        let mut settings = Settings {
            backing_volume: 0.25,
            metronome_volume: 1.5,
            tone_volume: -1.0,
            monitor_volume: 0.5,
            ..Settings::default()
        };
        let off = gains(&settings);
        assert_eq!((off.backing, off.click, off.tone), (0.25, 1.0, 0.0));
        assert_eq!(off.monitor, 0.0);
        settings.monitor = true;
        assert_eq!(gains(&settings).monitor, 0.5);
    }

    /// LH-WIE-03: mixer changes reach a running practice without restarting it and are saved.
    #[test]
    fn mixer_changes_apply_while_playing() {
        let dir = temp_dir("mixer");
        let take = dir.join("take.wav");
        std::fs::create_dir_all(&dir).unwrap();
        kontra_session::write_mono_wav(&take, &vec![0.0; 48_000], 48_000).unwrap();
        let mut core = Core::new(
            &dir,
            QaOptions {
                input_file: Some(take),
                laptop_jack: false,
            },
        );
        core.open(&demo()).unwrap();
        core.play();
        assert!(core.live.is_some());
        let hint = core.remix(|s| {
            s.monitor = true;
            s.backing_volume = 0.3;
        });
        assert!(core.live.is_some());
        let silent = core.live.as_ref().is_some_and(|l| l.output_is_silent);
        assert_eq!(
            hint.is_some(),
            !silent && !core.live.as_ref().unwrap().can_monitor()
        );
        let saved: Settings = store::load(&dir.join("settings.json"));
        assert!(saved.monitor);
        assert_eq!(saved.backing_volume, 0.3);
        core.stop_live();
        let _ = std::fs::remove_dir_all(dir);
    }

    /// QA P2 (R1): a broken file gets a German reason in German, not "unknown file format".
    #[test]
    fn import_errors_are_translated() {
        let dir = temp_dir("broken");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("kaputt.gp5");
        std::fs::write(&file, b"FICHIER GUITAR PRO v5.00 but cut off").unwrap();
        let mut core = Core::new(&dir, QaOptions::default());
        core.update_settings(|s| s.language = Some("de".into()));
        let err = core.open(&file).unwrap_err();
        assert!(
            err.starts_with("Die Datei lässt sich nicht öffnen"),
            "{err}"
        );
        assert!(!err.contains("format") && !err.contains("file"), "{err}");
        let _ = std::fs::remove_dir_all(dir);
    }

    /// LH-ANZ-04, QA P2 (R7): each piece keeps its own view.
    #[test]
    fn view_is_remembered_per_piece() {
        let dir = temp_dir("view");
        let mut core = Core::new(&dir, QaOptions::default());
        core.open(&demo()).unwrap();
        core.set_view("tab");
        core.update_settings(|s| s.view = "lane".into());
        let mut again = Core::new(&dir, QaOptions::default());
        again.open(&demo()).unwrap();
        assert_eq!(again.view(), "tab");
        let _ = std::fs::remove_dir_all(dir);
    }

    /// LH-EIN-07: quantum 2048 at 48 kHz gives the hint, 256 and 512 do not.
    #[test]
    fn long_quantum_is_reported() {
        assert!(quantum_too_long_ms(256, 48_000).is_none());
        assert!(quantum_too_long_ms(512, 48_000).is_none());
        let ms = quantum_too_long_ms(2048, 48_000).unwrap();
        assert!((ms - 42.7).abs() < 0.1, "{ms}");
        assert!(quantum_too_long_ms(1024, 44_100).is_some());
    }

    /// QA P3 (F6): window and pitch limit are the user's choice; "auto" keeps the defaults.
    #[test]
    fn scoring_follows_the_settings() {
        let dir = temp_dir("scoring");
        let mut core = Core::new(&dir, QaOptions::default());
        let auto = core.score_settings(BassKind::Fretless);
        assert_eq!(auto.pitch_cents, 30.0);
        assert_eq!(auto.strictness.window_ms(), 60.0);
        core.reconfigure(|s| {
            s.strictness = "strict".into();
            s.pitch_cents = 20.0;
        });
        let chosen = core.score_settings(BassKind::Fretless);
        assert_eq!(chosen.strictness.window_ms(), 25.0);
        assert_eq!(chosen.pitch_cents, 20.0);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// LH-TUN-03: guided tuning walks the profile's strings, lowest first.
    #[test]
    fn tuning_lists_the_profile_strings() {
        let dir = temp_dir("tuning");
        let mut core = Core::new(&dir, QaOptions::default());
        core.update_settings(|s| s.language = Some("de".into()));
        core.reconfigure(|s| s.strings = 5);
        let strings: serde_json::Value = serde_json::from_str(&core.tuning_json()).unwrap();
        let names: Vec<&str> = strings
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["H", "E", "A", "D", "G"]);
        assert_eq!(strings[0]["midi"], 23);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// LH-FOR-01 / J-C1: recorded runs show up per piece and with their time for the day view.
    #[test]
    fn progress_lists_pieces_and_runs() {
        let dir = temp_dir("progress");
        let mut core = Core::new(&dir, QaOptions::default());
        core.open(&demo()).unwrap();
        let a = core.arrangement().unwrap().clone();
        for (unix, rate) in [(1_000, 0.7), (90_000, 0.92)] {
            let run = kontra_session::Run {
                finished_unix: unix,
                practice_s: 600.0,
                speed: 0.8,
                hit_rate: rate,
                mode: "song".into(),
                section: None,
                hardest_bars: Vec::new(),
            };
            core.progress_store().record(&a, run).unwrap();
        }
        let p: serde_json::Value = serde_json::from_str(&core.progress_json()).unwrap();
        assert_eq!(p["pieces"][0]["title"], "Etude 1");
        assert_eq!(p["pieces"][0]["best"], 0.92);
        assert_eq!(p["pieces"][0]["practiceMin"], 20.0);
        assert_eq!(p["runs"].as_array().unwrap().len(), 2);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// LH-BIB-04 / J-B1: a folder becomes library pieces and a setlist; a broken file is named
    /// with a translated reason; the setlist plays in order.
    #[test]
    fn folder_import_makes_a_setlist() {
        let dir = temp_dir("folder");
        let songs = dir.join("Freitag");
        std::fs::create_dir_all(&songs).unwrap();
        std::fs::copy(demo(), songs.join("1-etude.mid")).unwrap();
        std::fs::copy(
            demo().with_file_name("clicks.mid"),
            songs.join("2-clicks.mid"),
        )
        .unwrap();
        std::fs::write(songs.join("3-kaputt.gp5"), b"FICHIER GUITAR PRO v5.00 cut").unwrap();
        std::fs::write(songs.join("notes.txt"), b"not a score").unwrap();
        let mut core = Core::new(&dir, QaOptions::default());
        core.update_settings(|s| s.language = Some("de".into()));
        let (added, failed) = core.import_folder(&songs, true);
        assert_eq!(added, 2);
        assert_eq!(failed.len(), 1, "{failed:?}");
        assert_eq!(failed[0].0, "3-kaputt.gp5");
        assert!(failed[0].1.starts_with("Die Datei"), "{failed:?}");
        let lists: serde_json::Value = serde_json::from_str(&core.setlists_json()).unwrap();
        assert_eq!(lists[0]["name"], "Freitag");
        assert_eq!(lists[0]["pieces"].as_array().unwrap().len(), 2);

        assert_eq!(core.open_from_setlist("Freitag", 0).unwrap(), Opened::Ready);
        assert_eq!(
            core.next_title().as_deref(),
            Some(lists[0]["pieces"][1]["title"].as_str().unwrap())
        );
        core.open_next().unwrap();
        assert_eq!(core.next_title(), None);

        core.setlist_move("Freitag", 1, -1);
        let lists: serde_json::Value = serde_json::from_str(&core.setlists_json()).unwrap();
        assert!(lists[0]["pieces"][0]["path"]
            .as_str()
            .unwrap()
            .ends_with("2-clicks.mid"));
        let again = Core::new(&dir, QaOptions::default());
        assert!(again.setlists_json().contains("Freitag"));
        let _ = std::fs::remove_dir_all(dir);
    }

    /// LH-FOR-03: an export loaded on a second profile gives the same state (replace), or adds
    /// to an existing one without doubling runs (merge); a foreign file changes nothing.
    #[test]
    fn data_export_and_import() {
        let dir = temp_dir("data");
        let mut first = Core::new(&dir.join("a"), QaOptions::default());
        first.update_settings(|s| s.language = Some("de".into()));
        first.reconfigure(|s| s.strings = 5);
        first.open(&demo()).unwrap();
        first.setlist_create("Freitag");
        first.setlist_add_loaded("Freitag");
        let a = first.arrangement().unwrap().clone();
        let run = kontra_session::Run {
            finished_unix: 7,
            practice_s: 300.0,
            speed: 0.8,
            hit_rate: 0.75,
            mode: "song".into(),
            section: None,
            hardest_bars: Vec::new(),
        };
        first.progress_store().record(&a, run).unwrap();
        let file = dir.join("sicherung.json");
        let text = first.export_data(&file).unwrap();
        assert!(
            text.contains("1 Durchläufe") || text.contains("1 Durchl"),
            "{text}"
        );

        let mut second = Core::new(&dir.join("b"), QaOptions::default());
        let report = second.import_data(&file, DataImport::Replace).unwrap();
        assert!(report.contains("sicherung.json"), "{report}");
        assert_eq!(second.settings(), first.settings());
        assert_eq!(second.setlists_json(), first.setlists_json());
        assert_eq!(second.recent_json(), first.recent_json());
        assert_eq!(second.progress_json(), first.progress_json());

        // Merging the same file again adds nothing and keeps the own settings.
        second.update_settings(|s| s.strings = 4);
        let report = second.import_data(&file, DataImport::Merge).unwrap();
        assert!(
            report.contains("0 neue Stücke, 0 neue Setlisten, 0 neue Durchläufe"),
            "{report}"
        );
        assert_eq!(second.settings().strings, 4);
        assert_eq!(second.progress_json(), first.progress_json());

        std::fs::write(&file, r#"{"format": "something else"}"#).unwrap();
        assert!(second.import_data(&file, DataImport::Replace).is_err());
        assert_eq!(second.progress_json(), first.progress_json());
        let _ = std::fs::remove_dir_all(dir);
    }

    /// LH-FOR-04: the report is one HTML file with Kante's styles inside, the piece's figures,
    /// a chart and every run with its hardest bars.
    #[test]
    fn progress_report_is_self_contained() {
        let dir = temp_dir("report");
        let mut core = Core::new(&dir, QaOptions::default());
        core.update_settings(|s| s.language = Some("de".into()));
        core.open(&demo()).unwrap();
        let a = core.arrangement().unwrap().clone();
        for (unix, rate) in [(1_791_504_000, 0.7), (1_791_590_400, 0.95)] {
            let run = kontra_session::Run {
                finished_unix: unix,
                practice_s: 600.0,
                speed: 0.8,
                hit_rate: rate,
                mode: "workshop".into(),
                section: None,
                hardest_bars: vec![3, 7],
            };
            core.progress_store().record(&a, run).unwrap();
        }
        let p: serde_json::Value = serde_json::from_str(&core.progress_json()).unwrap();
        let id = p["pieces"][0]["songId"].as_str().unwrap().to_string();
        let file = dir.join("bericht.html");
        let text = core.export_report(&id, &file).unwrap();
        assert!(text.contains("bericht.html"), "{text}");
        let html = std::fs::read_to_string(&file).unwrap();
        assert!(html.starts_with("<!doctype html>"));
        assert!(html.contains("Übungsbericht: Etude 1"));
        assert!(html.contains("<svg class=\"chart\""));
        assert!(html.contains("2026-10-10") && html.contains("Werkstatt") && html.contains("3, 7"));
        assert!(html.contains("<b>20</b>"), "20 minutes");
        assert!(
            !html.contains("<link") && !html.contains("<script"),
            "no outside files"
        );
        assert!(core.export_report("0000000000000000", &file).is_err());
        let _ = std::fs::remove_dir_all(dir);
    }

    /// LH-BIB-07 / J-D2: a piece shared as `.kontra` opens on a second profile with the same
    /// notes, track, backing (only if embedded) and progress; opening it twice adds no runs.
    #[test]
    fn package_moves_a_piece_to_another_profile() {
        let dir = temp_dir("package");
        std::fs::create_dir_all(&dir).unwrap();
        let backing = dir.join("song.wav");
        kontra_session::write_mono_wav(&backing, &vec![0.0; 4_800], 48_000).unwrap();
        let mut teacher = Core::new(&dir.join("thomas"), QaOptions::default());
        teacher.open(&demo()).unwrap();
        teacher.set_backing(Some(backing.clone()), 0.5);
        let a = teacher.arrangement().unwrap().clone();
        let run = kontra_session::Run {
            finished_unix: 9,
            practice_s: 120.0,
            speed: 0.7,
            hit_rate: 0.85,
            mode: "workshop".into(),
            section: None,
            hardest_bars: vec![2],
        };
        teacher.progress_store().record(&a, run).unwrap();
        let linked = dir.join("etude-link.kontra");
        let embedded = dir.join("etude.kontra");
        teacher
            .export_package(&demo(), &linked, PackageAudio::Link)
            .unwrap();
        teacher
            .export_package(&demo(), &embedded, PackageAudio::Embed)
            .unwrap();
        assert!(
            std::fs::metadata(&embedded).unwrap().len() > std::fs::metadata(&linked).unwrap().len()
        );

        let mut student = Core::new(&dir.join("schueler"), QaOptions::default());
        assert_eq!(student.open(&embedded).unwrap(), Opened::Ready);
        assert_eq!(student.notes_json(), teacher.notes_json());
        assert_eq!(student.bars_json(), teacher.bars_json());
        assert_eq!(student.backing_name(), "song.wav");
        assert!(student.take_notice().is_some());
        let p: serde_json::Value = serde_json::from_str(&student.progress_json()).unwrap();
        assert_eq!(p["pieces"][0]["runs"], 1);
        student.open(&embedded).unwrap();
        let p: serde_json::Value = serde_json::from_str(&student.progress_json()).unwrap();
        assert_eq!(p["pieces"][0]["runs"], 1, "no doubled runs");

        std::fs::write(&linked, b"PK\x03\x04 broken").unwrap();
        assert!(student.open(&linked).is_err());
        let _ = std::fs::remove_dir_all(dir);
    }

    /// QA P2 (R9): another track of the file can be chosen after Kontra's own choice.
    #[test]
    fn switches_to_another_track() {
        let dir = temp_dir("tracks");
        let mut core = Core::new(&dir, QaOptions::default());
        core.open(&demo()).unwrap();
        let t: serde_json::Value = serde_json::from_str(&core.tracks_json()).unwrap();
        assert!(!t["names"].as_array().unwrap().is_empty());
        assert!(core.switch_track(0).is_ok());
        assert!(core.switch_track(99).is_err());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn open_lists_notes_and_remembers_recent() {
        let dir = temp_dir("open");
        let mut core = Core::new(&dir, QaOptions::default());
        assert_eq!(core.open(&demo()).unwrap(), Opened::Ready);
        let notes: serde_json::Value = serde_json::from_str(&core.notes_json()).unwrap();
        assert_eq!(notes.as_array().unwrap().len(), 64);
        let bars: serde_json::Value = serde_json::from_str(&core.bars_json()).unwrap();
        assert_eq!(bars[1]["number"], 2);
        let again = Core::new(&dir, QaOptions::default());
        assert!(again.recent_json().contains("Etude 1"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn calibration_measures_recorded_latency() {
        let dir = temp_dir("calibrate");
        std::fs::create_dir_all(&dir).unwrap();
        let take = dir.join("clicks.wav");
        // Eight quarter notes at 100 BPM (0.6 s), plucked 45 ms after each click.
        let mut a = Arrangement::new("clicks", kontra_model::Instrument::default_bass());
        a.tempo = kontra_model::TempoMap::constant(100.0);
        for i in 0..8 {
            let q = kontra_model::TICKS_PER_QUARTER;
            a.notes.push(kontra_model::Note::new(
                i * q,
                q / 2,
                kontra_model::Pitch(33),
            ));
        }
        a.normalize();
        let mistakes = kontra_session::Mistakes {
            offset_ms: 45.0,
            ..Default::default()
        };
        let samples =
            kontra_session::render_performance(&a, 1.0, CALIBRATION_LEAD_S, mistakes, 48_000);
        kontra_session::write_mono_wav(&take, &samples, 48_000).unwrap();
        let mut core = Core::new(
            &dir,
            QaOptions {
                input_file: Some(take),
                laptop_jack: false,
            },
        );
        core.start_calibration().unwrap();
        let end = std::time::Instant::now() + std::time::Duration::from_secs(8);
        let mut result = None;
        while std::time::Instant::now() < end && result.is_none() {
            result = core.tick().into_iter().find_map(|e| match e {
                UiEvent::Calibrated { latency_ms, .. } => Some(latency_ms),
                _ => None,
            });
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let latency = result.flatten().expect("measured");
        assert!((latency - 45.0).abs() <= 3.0, "{latency}");
        assert_eq!(core.settings().latency_ms, latency);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn plays_a_recording_and_judges_notes() {
        let dir = temp_dir("play");
        let take = dir.join("take.wav");
        std::fs::create_dir_all(&dir).unwrap();
        let mut core = Core::new(
            &dir,
            QaOptions {
                input_file: Some(take.clone()),
                laptop_jack: false,
            },
        );
        core.open(&demo()).unwrap();
        // One count-in bar at 100 BPM = 2.4 s lead-in; render 4 bars at double speed for a short test.
        let arrangement = core.arrangement().unwrap().clone();
        let samples = kontra_session::render_performance(
            &arrangement,
            1.0,
            2.4,
            kontra_session::Mistakes::default(),
            48_000,
        );
        kontra_session::write_mono_wav(&take, &samples[..(48_000.0 * 6.0) as usize], 48_000)
            .unwrap();
        let events = core.play();
        assert!(
            !events.iter().any(|e| matches!(e, UiEvent::Error(_))),
            "{events:?}"
        );
        let mut hits = 0;
        let end = std::time::Instant::now() + std::time::Duration::from_secs(7);
        while std::time::Instant::now() < end {
            for e in core.tick() {
                if let UiEvent::Judged { state: "hit", .. } = e {
                    hits += 1;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        // 3.6 s of playing at 0.3 s per note: about 12 hits.
        assert!(hits >= 10, "hits {hits}");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
