//! Qt-free core of the app: everything the QML bridge calls, testable without a GUI.
//!
//! ```text
//! QML ──invokables──► bridge (cxx-qt) ──► Core ──► kontra-session (Live, LiveTuner, service)
//!     ◄──properties/signals── bridge ◄── UiEvent
//! ```

mod i18n;
mod store;

pub use i18n::{Catalog, Language};
pub use store::{data_dir, InputKind, Library, RecentEntry, Settings};

use kontra_model::{Arrangement, BassKind, InputSource};
use kontra_session::{
    import_file, list_devices, run_from, CalibrationResult, Direction, ImportResult, InputChoice,
    Live, LiveCalibration, LiveSetup, LiveTuner, LoopRange, Mode, Outcome, PracticeEvent,
    PracticeOptions, ProgressStore, ScoreSettings, SessionEvent, Summary, Workshop,
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
    speed: f64,
    looping: Option<(usize, usize)>,
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
                json!({ "path": e.path, "title": e.title, "artist": e.artist, "best": best, "practiceMin": minutes })
            })
            .collect();
        serde_json::Value::Array(list).to_string()
    }

    /// Imports a score; picks the bass track or asks.
    pub fn open(&mut self, path: &Path) -> Result<Opened, String> {
        let imported =
            import_file(path).map_err(|e| self.tr("error.import", &[("reason", e.to_string())]))?;
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
            .map_err(|e| self.tr("error.import", &[("reason", e.to_string())]))?;
        if arrangement.title.trim().is_empty() {
            arrangement.title = path
                .file_stem()
                .map_or_else(String::new, |s| s.to_string_lossy().to_string());
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
        });
        let _ = store::save(&self.data_dir.join("library.json"), &self.library);
        self.loaded = Some(Loaded {
            path: path.to_path_buf(),
            arrangement,
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
        let mut settings = ScoreSettings::defaults(arrangement.instrument.kind, self.source());
        settings.latency_ms = self.settings.latency_ms;
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
            backing: self.backing().map(|(path, _)| path),
        };
        let mut live = Live::start(arrangement, self.mode(), settings, options, &setup)
            .map_err(|e| self.tr("error.audio", &[("reason", e.to_string())]))?;
        live.practice_mut().set_speed(self.speed);
        live.practice_mut().set_loop(self.loop_range());
        let silent = live.output_is_silent;
        self.live = Some(live);
        self.run_started = Some(std::time::Instant::now());
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
        if let Some(result) = self.calibration.as_mut().and_then(LiveCalibration::poll) {
            self.calibration = None;
            events.push(self.calibrated(result));
        }
        events
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
        let practice_s = self
            .run_started
            .take()
            .map_or(0.0, |t| t.elapsed().as_secs_f64());
        let section = session
            .transport()
            .loop_range()
            .map(|l| (l.start_s, l.end_s));
        let run = run_from(
            &summary,
            practice_s,
            self.speed,
            &self.settings.mode,
            section,
            unix_now(),
        );
        let arrangement = session.arrangement().clone();
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
            PracticeEvent::Tuner { note, cents } => Some(UiEvent::Tuner { note, cents }),
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
                    Outcome::WrongPitch { .. } => "wrong",
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
                let n = self.arrangement()?.notes.get(note)?;
                let text = self.tr(
                    "waiting.for",
                    &[
                        ("note", n.pitch.to_string()),
                        ("string", n.string.map_or(0, |s| s + 1).to_string()),
                        ("fret", n.fret.unwrap_or(0).to_string()),
                    ],
                );
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
            "meanOffsetMs": s.mean_offset_ms.unwrap_or(0.0),
            "counts": s.histogram.counts,
            "firstBinMs": s.histogram.first_bin_ms,
            "binMs": s.histogram.bin_ms,
            "hardestBars": hardest,
        })
        .to_string()
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

fn empty_staff() -> serde_json::Value {
    json!({ "notes": [], "rests": [], "keyFifths": 0 })
}

/// `staff_json` for an arrangement. The staff shows one key; it is the first bar's.
fn staff(a: &Arrangement) -> serde_json::Value {
    let notes: Vec<_> = a
        .notes
        .iter()
        .map(|n| {
            json!({
                "time": a.tempo.seconds_at(n.start_ticks),
                "beats": written_beats(n.duration_ticks),
                "midi": n.pitch.0,
                "tied": n.tied,
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
