//! The QML-facing controller. Thin: every call goes to `kontra_appcore::Core`, every result
//! comes back as a property or signal. Property names are camelCase in QML.
//!
//! cxx-qt generates C++ glue with `unsafe` blocks; the workspace lint is relaxed for this file
//! only (agent.md allows `unsafe` in the cxx-qt bridge).
#![allow(unsafe_code)]

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, texts)]
        #[qproperty(bool, loaded)]
        #[qproperty(QString, title)]
        #[qproperty(QString, artist)]
        #[qproperty(QString, notes_json, cxx_name = "notesJson")]
        #[qproperty(QString, bars_json, cxx_name = "barsJson")]
        #[qproperty(QString, staff_json, cxx_name = "staffJson")]
        #[qproperty(QString, string_names_json, cxx_name = "stringNamesJson")]
        #[qproperty(f64, duration)]
        #[qproperty(f64, position)]
        #[qproperty(bool, playing)]
        #[qproperty(f64, speed)]
        #[qproperty(f64, loop_start, cxx_name = "loopStart")]
        #[qproperty(f64, pass_rate, cxx_name = "passRate")]
        #[qproperty(i32, passes, cxx_name = "passes")]
        #[qproperty(f64, band_low, cxx_name = "bandLow")]
        #[qproperty(f64, band_high, cxx_name = "bandHigh")]
        #[qproperty(f64, loop_end, cxx_name = "loopEnd")]
        #[qproperty(QString, mode)]
        #[qproperty(QString, view)]
        #[qproperty(QString, style)]
        #[qproperty(QString, language)]
        #[qproperty(f64, peak_db, cxx_name = "peakDb")]
        #[qproperty(f64, rms_db, cxx_name = "rmsDb")]
        #[qproperty(bool, clipped)]
        #[qproperty(QString, tuner_note, cxx_name = "tunerNote")]
        #[qproperty(f64, tuner_midi, cxx_name = "tunerMidi")]
        /// Open strings for guided tuning: `[{"name", "midi"}]`, lowest first.
        #[qproperty(QString, tuning_json, cxx_name = "tuningJson")]
        #[qproperty(f64, tuner_cents, cxx_name = "tunerCents")]
        #[qproperty(bool, tuner_active, cxx_name = "tunerActive")]
        #[qproperty(i32, holding_index, cxx_name = "holdingIndex")]
        #[qproperty(QString, holding_text, cxx_name = "holdingText")]
        #[qproperty(QString, summary_json, cxx_name = "summaryJson")]
        #[qproperty(QString, recent_json, cxx_name = "recentJson")]
        #[qproperty(QString, setlists_json, cxx_name = "setlistsJson")]
        /// Tracks of the loaded file: `{"names": [...], "current": i}`.
        #[qproperty(QString, tracks_json, cxx_name = "tracksJson")]
        /// Title of the next piece of the setlist being played ("" = none).
        #[qproperty(QString, next_title, cxx_name = "nextTitle")]
        #[qproperty(QString, track_choices_json, cxx_name = "trackChoicesJson")]
        #[qproperty(QString, input_devices_json, cxx_name = "inputDevicesJson")]
        #[qproperty(QString, output_devices_json, cxx_name = "outputDevicesJson")]
        #[qproperty(QString, input_device, cxx_name = "inputDevice")]
        #[qproperty(QString, output_device, cxx_name = "outputDevice")]
        #[qproperty(i32, strings)]
        #[qproperty(bool, fretless)]
        #[qproperty(bool, laptop_jack, cxx_name = "laptopJack")]
        #[qproperty(bool, metronome)]
        #[qproperty(bool, count_in, cxx_name = "countIn")]
        #[qproperty(bool, reference)]
        #[qproperty(bool, low_string_on_top, cxx_name = "lowStringOnTop")]
        #[qproperty(QString, play_marks, cxx_name = "playMarks")]
        #[qproperty(QString, motion)]
        #[qproperty(QString, strictness)]
        #[qproperty(f64, pitch_cents, cxx_name = "pitchCents")]
        #[qproperty(f64, latency_ms, cxx_name = "latencyMs")]
        /// Mixer (LH-WIE-03, LH-MOD-05): volumes 0..1 and input monitoring.
        #[qproperty(f64, backing_volume, cxx_name = "backingVolume")]
        #[qproperty(f64, metronome_volume, cxx_name = "metronomeVolume")]
        #[qproperty(f64, tone_volume, cxx_name = "toneVolume")]
        #[qproperty(bool, monitor)]
        #[qproperty(f64, monitor_volume, cxx_name = "monitorVolume")]
        #[qproperty(bool, calibrating)]
        #[qproperty(QString, calibration_text, cxx_name = "calibrationText")]
        #[qproperty(QString, backing_name, cxx_name = "backingName")]
        #[qproperty(QString, message)]
        #[qproperty(QString, error_text, cxx_name = "errorText")]
        #[qproperty(QString, start_page, cxx_name = "startPage")]
        #[qproperty(bool, setup_done, cxx_name = "setupDone")]
        #[qproperty(bool, autoplay)]
        #[qproperty(bool, autocalibrate)]
        /// QA: practice and settings show the mixer from the start.
        #[qproperty(bool, show_mixer, cxx_name = "showMixer")]
        /// QA: bars to loop at start, 1-based "from-to" ("" = none).
        #[qproperty(QString, start_loop, cxx_name = "startLoop")]
        #[qproperty(QString, screenshot_path, cxx_name = "screenshotPath")]
        #[qproperty(i32, screenshot_after, cxx_name = "screenshotAfter")]
        #[qproperty(i32, quit_after, cxx_name = "quitAfter")]
        type Controller = super::ControllerRust;

        /// Opens a score from a `file://` URL or a path.
        #[qinvokable]
        #[cxx_name = "openFile"]
        fn open_file(self: Pin<&mut Self>, url: &QString) -> bool;

        #[qinvokable]
        #[cxx_name = "chooseTrack"]
        fn choose_track(self: Pin<&mut Self>, track: i32) -> bool;

        #[qinvokable]
        #[cxx_name = "togglePlay"]
        fn toggle_play(self: Pin<&mut Self>);

        #[qinvokable]
        fn pause(self: Pin<&mut Self>);

        #[qinvokable]
        fn rewind(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "changeSpeed"]
        fn change_speed(self: Pin<&mut Self>, speed: f64);

        #[qinvokable]
        #[cxx_name = "changeMode"]
        fn change_mode(self: Pin<&mut Self>, mode: &QString);

        #[qinvokable]
        #[cxx_name = "changeView"]
        fn change_view(self: Pin<&mut Self>, view: &QString);

        #[qinvokable]
        #[cxx_name = "changeStyle"]
        fn change_style(self: Pin<&mut Self>, style: &QString);

        #[qinvokable]
        #[cxx_name = "changeLanguage"]
        fn change_language(self: Pin<&mut Self>, code: &QString);

        /// Loops bars `from..=to` (0-based indices into the timeline's bars).
        #[qinvokable]
        #[cxx_name = "loopBars"]
        fn loop_bars(self: Pin<&mut Self>, from: i32, to: i32);

        #[qinvokable]
        #[cxx_name = "clearLoop"]
        fn clear_loop(self: Pin<&mut Self>);

        /// Sets the loop's "start" or "end" to the bar at the current position (LH-MOD-07).
        #[qinvokable]
        #[cxx_name = "loopEdge"]
        fn loop_edge(self: Pin<&mut Self>, edge: &QString);

        /// Polls audio and session; QML calls it from a 16 ms timer.
        #[qinvokable]
        fn tick(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "startTuner"]
        fn start_tuner(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "stopTuner"]
        fn stop_tuner(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "refreshDevices"]
        fn refresh_devices(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "chooseInput"]
        fn choose_input(self: Pin<&mut Self>, id: &QString);

        #[qinvokable]
        #[cxx_name = "chooseOutput"]
        fn choose_output(self: Pin<&mut Self>, id: &QString);

        #[qinvokable]
        #[cxx_name = "changeInstrument"]
        fn change_instrument(self: Pin<&mut Self>, strings: i32, fretless: bool, laptop_jack: bool);

        #[qinvokable]
        #[cxx_name = "changeOptions"]
        fn change_options(
            self: Pin<&mut Self>,
            metronome: bool,
            reference: bool,
            low_string_on_top: bool,
        );

        /// Adds all scores of a folder (URL) to the library, optionally as a setlist.
        #[qinvokable]
        #[cxx_name = "importFolder"]
        fn import_folder(self: Pin<&mut Self>, url: &QString, as_setlist: bool);

        #[qinvokable]
        #[cxx_name = "setlistCreate"]
        fn setlist_create(self: Pin<&mut Self>, name: &QString);

        #[qinvokable]
        #[cxx_name = "setlistDelete"]
        fn setlist_delete(self: Pin<&mut Self>, name: &QString);

        #[qinvokable]
        #[cxx_name = "setlistAddLoaded"]
        fn setlist_add_loaded(self: Pin<&mut Self>, name: &QString);

        #[qinvokable]
        #[cxx_name = "setlistRemove"]
        fn setlist_remove(self: Pin<&mut Self>, name: &QString, index: i32);

        #[qinvokable]
        #[cxx_name = "setlistMove"]
        fn setlist_move(self: Pin<&mut Self>, name: &QString, index: i32, step: i32);

        /// Opens piece `index` of a setlist; true when it is ready to practise.
        #[qinvokable]
        #[cxx_name = "openFromSetlist"]
        fn open_from_setlist(self: Pin<&mut Self>, name: &QString, index: i32) -> bool;

        /// Plays another track of the loaded file.
        #[qinvokable]
        #[cxx_name = "switchTrack"]
        fn switch_track(self: Pin<&mut Self>, track: i32);

        /// Opens the next piece of the setlist being played.
        #[qinvokable]
        #[cxx_name = "openNext"]
        fn open_next(self: Pin<&mut Self>) -> bool;

        /// Sets a library piece's tags from a JSON list of strings (LH-BIB-04).
        #[qinvokable]
        #[cxx_name = "setTags"]
        fn set_tags(self: Pin<&mut Self>, path: &QString, tags_json: &QString);

        /// Every tag in the library as a JSON list, for suggestions.
        #[qinvokable]
        #[cxx_name = "tagsJson"]
        fn tags_json(self: &Self) -> QString;

        /// Progress over all pieces as JSON (LH-FOR-01), read when the page opens.
        #[qinvokable]
        #[cxx_name = "progressJson"]
        fn progress_json(self: &Self) -> QString;

        /// Timing window ("auto", "loose", "normal", "strict") and pitch limit (0 = auto).
        #[qinvokable]
        #[cxx_name = "changeScoring"]
        fn change_scoring(self: Pin<&mut Self>, strictness: &QString, pitch_cents: f64);

        /// Note marks in play mode: "quiet", "off" or "full".
        #[qinvokable]
        #[cxx_name = "changePlayMarks"]
        fn change_play_marks(self: Pin<&mut Self>, marks: &QString);

        /// Motion: "system" (follow the desktop), "reduce" or "full" (round 6).
        #[qinvokable]
        #[cxx_name = "changeMotion"]
        fn change_motion(self: Pin<&mut Self>, motion: &QString);

        #[qinvokable]
        #[cxx_name = "changeCountIn"]
        fn change_count_in(self: Pin<&mut Self>, on: bool);

        #[qinvokable]
        #[cxx_name = "changeLatency"]
        fn change_latency(self: Pin<&mut Self>, latency_ms: f64);

        /// Sets a mixer volume (0..1): `bus` is "backing", "metronome", "tone" or "monitor".
        #[qinvokable]
        #[cxx_name = "changeVolume"]
        fn change_volume(self: Pin<&mut Self>, bus: &QString, value: f64);

        /// Plays the bass input through the output (off by default).
        #[qinvokable]
        #[cxx_name = "changeMonitor"]
        fn change_monitor(self: Pin<&mut Self>, on: bool);

        /// Latency calibration "play on the click" (LH-EIN-05).
        #[qinvokable]
        #[cxx_name = "startCalibration"]
        fn start_calibration(self: Pin<&mut Self>);

        /// Backing track for the loaded piece (LH-WIE-01); an empty URL removes it.
        #[qinvokable]
        #[cxx_name = "chooseBacking"]
        fn choose_backing(self: Pin<&mut Self>, url: &QString);

        /// Marks the first-start assistant as done (or skipped).
        #[qinvokable]
        #[cxx_name = "finishSetup"]
        fn finish_setup(self: Pin<&mut Self>);

        /// Opens the bundled first exercise.
        #[qinvokable]
        #[cxx_name = "openDemo"]
        fn open_demo(self: Pin<&mut Self>) -> bool;

        #[qinvokable]
        #[cxx_name = "dismissMessage"]
        fn dismiss_message(self: Pin<&mut Self>);

        #[qsignal]
        #[cxx_name = "noteJudged"]
        fn note_judged(
            self: Pin<&mut Self>,
            index: i32,
            state: QString,
            offset_ms: f64,
            cents: f64,
        );

        #[qsignal]
        #[cxx_name = "extraNote"]
        fn extra_note(self: Pin<&mut Self>);

        #[qsignal]
        #[cxx_name = "passComplete"]
        fn pass_complete(self: Pin<&mut Self>, clean: bool, hit_rate: f64);

        #[qsignal]
        fn finished(self: Pin<&mut Self>);

        /// The arrangement changed: views reset all note states.
        #[qsignal]
        #[cxx_name = "arrangementLoaded"]
        fn arrangement_loaded(self: Pin<&mut Self>);
    }
}

use crate::args::AppArgs;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use kontra_appcore::{data_dir, Core, LoopEdge, Opened, QaOptions, UiEvent};
use std::path::PathBuf;

pub struct ControllerRust {
    texts: QString,
    loaded: bool,
    title: QString,
    artist: QString,
    notes_json: QString,
    bars_json: QString,
    staff_json: QString,
    string_names_json: QString,
    duration: f64,
    position: f64,
    playing: bool,
    speed: f64,
    loop_start: f64,
    /// Hit rate of the last loop pass (-1: none yet).
    pass_rate: f64,
    /// Loop passes completed since the loop or mode was set.
    passes: i32,
    band_low: f64,
    band_high: f64,
    loop_end: f64,
    mode: QString,
    view: QString,
    style: QString,
    language: QString,
    peak_db: f64,
    rms_db: f64,
    clipped: bool,
    tuner_note: QString,
    tuner_midi: f64,
    tuning_json: QString,
    tuner_cents: f64,
    tuner_active: bool,
    holding_index: i32,
    holding_text: QString,
    summary_json: QString,
    recent_json: QString,
    setlists_json: QString,
    next_title: QString,
    tracks_json: QString,
    track_choices_json: QString,
    input_devices_json: QString,
    output_devices_json: QString,
    input_device: QString,
    output_device: QString,
    strings: i32,
    fretless: bool,
    laptop_jack: bool,
    metronome: bool,
    count_in: bool,
    reference: bool,
    low_string_on_top: bool,
    play_marks: QString,
    motion: QString,
    strictness: QString,
    pitch_cents: f64,
    latency_ms: f64,
    backing_volume: f64,
    metronome_volume: f64,
    tone_volume: f64,
    monitor: bool,
    monitor_volume: f64,
    calibrating: bool,
    calibration_text: QString,
    backing_name: QString,
    message: QString,
    error_text: QString,
    start_page: QString,
    setup_done: bool,
    autoplay: bool,
    autocalibrate: bool,
    show_mixer: bool,
    start_loop: QString,
    screenshot_path: QString,
    screenshot_after: i32,
    quit_after: i32,
    core: Core,
}

fn q(text: &str) -> QString {
    QString::from(text)
}

impl Default for ControllerRust {
    fn default() -> Self {
        let args = AppArgs::from_env();
        let qa = QaOptions {
            input_file: args.input_file.clone(),
            laptop_jack: args.laptop_jack,
        };
        let mut core = Core::new(&data_dir(), qa);
        if let Some(style) = &args.style {
            core.update_settings(|s| s.style = style.clone());
        }
        if let Some(language) = &args.language {
            core.update_settings(|s| s.language = Some(language.clone()));
        }
        if let Some(mode) = &args.mode {
            core.reconfigure(|s| s.mode = mode.clone());
        }
        if let Some(view) = &args.view {
            core.update_settings(|s| s.view = view.clone());
        }
        let s = core.settings().clone();
        let mut controller = ControllerRust {
            texts: q(&core.texts_json()),
            loaded: false,
            title: QString::default(),
            artist: QString::default(),
            notes_json: q("[]"),
            bars_json: q("[]"),
            staff_json: q(&core.staff_json()),
            string_names_json: q("[]"),
            duration: 0.0,
            position: 0.0,
            playing: false,
            speed: 1.0,
            loop_start: -1.0,
            pass_rate: -1.0,
            passes: 0,
            band_low: kontra_appcore::Core::workshop_band().0,
            band_high: kontra_appcore::Core::workshop_band().1,
            loop_end: -1.0,
            mode: q(&s.mode),
            view: q(&s.view),
            style: q(&s.style),
            language: q(s.language.as_deref().unwrap_or("")),
            peak_db: -120.0,
            rms_db: -120.0,
            clipped: false,
            tuner_note: QString::default(),
            tuner_midi: 0.0,
            tuning_json: q(&core.tuning_json()),
            tuner_cents: 0.0,
            tuner_active: false,
            holding_index: -1,
            holding_text: QString::default(),
            summary_json: QString::default(),
            recent_json: q(&core.recent_json()),
            setlists_json: q(&core.setlists_json()),
            next_title: QString::default(),
            tracks_json: q("{\"names\": [], \"current\": 0}"),
            track_choices_json: q("[]"),
            input_devices_json: q("[]"),
            output_devices_json: q("[]"),
            input_device: q(s.input_device.as_deref().unwrap_or("")),
            output_device: q(s.output_device.as_deref().unwrap_or("")),
            strings: i32::from(s.strings),
            fretless: s.fretless,
            laptop_jack: s.input_kind == kontra_appcore::InputKind::LaptopJack || args.laptop_jack,
            metronome: s.metronome,
            count_in: s.count_in_bars > 0,
            reference: s.reference_tones,
            low_string_on_top: s.low_string_on_top,
            play_marks: q(&s.play_marks),
            motion: q(&s.motion),
            strictness: q(&s.strictness),
            pitch_cents: s.pitch_cents,
            latency_ms: s.latency_ms,
            backing_volume: s.backing_volume,
            metronome_volume: s.metronome_volume,
            tone_volume: s.tone_volume,
            monitor: s.monitor,
            monitor_volume: s.monitor_volume,
            calibrating: false,
            calibration_text: QString::default(),
            backing_name: QString::default(),
            message: QString::default(),
            error_text: QString::default(),
            start_page: q(args.page.as_deref().unwrap_or("")),
            setup_done: s.setup_done,
            autoplay: args.autoplay,
            autocalibrate: args.calibrate,
            show_mixer: args.mixer,
            start_loop: q(&args
                .loop_bars
                .map(|(a, b)| format!("{a}-{b}"))
                .unwrap_or_default()),
            screenshot_path: q(&args
                .screenshot
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default()),
            screenshot_after: args.screenshot_after_ms,
            quit_after: args.quit_after_ms,
            core,
        };
        if let Some(path) = args.open {
            controller.open_now(&path);
            // An explicit --view wins over the view remembered for the piece (QA runs).
            if let Some(view) = &args.view {
                controller.core.set_view(view);
                controller.view = q(view);
            }
        }
        controller
    }
}

impl ControllerRust {
    /// Opening at start-up, before Qt owns the object (fields set directly).
    fn open_now(&mut self, path: &std::path::Path) {
        match self.core.open(path) {
            Ok(Opened::Ready) => {
                self.loaded = true;
                self.title = q(&self
                    .core
                    .arrangement()
                    .map_or(String::new(), |a| a.title.clone()));
                self.artist = q(&self
                    .core
                    .arrangement()
                    .map_or(String::new(), |a| a.artist.clone()));
                self.notes_json = q(&self.core.notes_json());
                self.bars_json = q(&self.core.bars_json());
                self.staff_json = q(&self.core.staff_json());
                self.string_names_json = q(&self.core.string_names_json());
                self.duration = self.core.duration_s();
                self.recent_json = q(&self.core.recent_json());
                self.backing_name = q(&self.core.backing_name());
                self.view = q(&self.core.view());
                self.tracks_json = q(&self.core.tracks_json());
                if let Some(notice) = self.core.take_notice() {
                    self.message = q(&notice);
                }
            }
            Ok(Opened::ChooseTrack(names)) => {
                self.track_choices_json = q(&serde_json::to_string(&names).unwrap_or_default());
            }
            Err(e) => self.error_text = q(&e),
        }
    }
}

/// `file:///a%20b.gp5` → `/a b.gp5`; plain paths stay.
fn local_path(url: &str) -> PathBuf {
    let Some(rest) = url.strip_prefix("file://") else {
        return PathBuf::from(url);
    };
    let bytes = rest.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(value) = u8::from_str_radix(&rest[i + 1..i + 3], 16) {
                out.push(value);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    PathBuf::from(String::from_utf8_lossy(&out).to_string())
}

impl qobject::Controller {
    fn core(self: Pin<&mut Self>) -> &mut Core {
        &mut self.rust_mut().get_mut().core
    }

    fn refresh_arrangement(mut self: Pin<&mut Self>) {
        let this = self.as_ref();
        let core = &this.rust().core;
        let (title, artist) = core
            .arrangement()
            .map_or((String::new(), String::new()), |a| {
                (a.title.clone(), a.artist.clone())
            });
        let notes = core.notes_json();
        let bars = core.bars_json();
        let staff = core.staff_json();
        let names = core.string_names_json();
        let duration = core.duration_s();
        let recent = core.recent_json();
        let loaded = core.arrangement().is_some();
        let backing = core.backing_name();
        let view = core.view();
        let tracks = core.tracks_json();
        self.as_mut().set_view(q(&view));
        self.as_mut().set_tracks_json(q(&tracks));
        self.as_mut().set_title(q(&title));
        self.as_mut().set_artist(q(&artist));
        self.as_mut().set_notes_json(q(&notes));
        self.as_mut().set_bars_json(q(&bars));
        self.as_mut().set_staff_json(q(&staff));
        self.as_mut().set_string_names_json(q(&names));
        self.as_mut().set_duration(duration);
        self.as_mut().set_recent_json(q(&recent));
        self.as_mut().set_loaded(loaded);
        self.as_mut().set_backing_name(q(&backing));
        self.as_mut().set_position(0.0);
        self.as_mut().set_summary_json(QString::default());
        self.as_mut().set_holding_index(-1);
        self.as_mut().arrangement_loaded();
    }

    fn show_error(mut self: Pin<&mut Self>, text: &str) {
        self.as_mut().set_error_text(q(text));
    }

    fn open_file(mut self: Pin<&mut Self>, url: &QString) -> bool {
        let path = local_path(&url.to_string());
        let result = self.as_mut().core().open(&path);
        self.as_mut().after_open(result)
    }

    fn choose_track(mut self: Pin<&mut Self>, track: i32) -> bool {
        let result = self
            .as_mut()
            .core()
            .choose_track(usize::try_from(track).unwrap_or(0));
        self.as_mut().set_track_choices_json(q("[]"));
        match result {
            Ok(()) => {
                self.as_mut().refresh_arrangement();
                if let Some(notice) = self.as_mut().core().take_notice() {
                    self.as_mut().set_message(q(&notice));
                }
                true
            }
            Err(e) => {
                self.as_mut().show_error(&e);
                false
            }
        }
    }

    fn toggle_play(mut self: Pin<&mut Self>) {
        if self.as_ref().rust().core.is_playing() {
            self.as_mut().pause();
            return;
        }
        let finished_before = !self.as_ref().summary_json().is_empty();
        if finished_before {
            // A finished run starts again from the beginning.
            self.as_mut().core().stop_live();
            self.as_mut().set_summary_json(QString::default());
            self.as_mut().arrangement_loaded();
        }
        let events = self.as_mut().core().play();
        self.as_mut().apply(events);
        let playing = self.as_ref().rust().core.is_playing();
        self.as_mut().set_playing(playing);
    }

    fn pause(mut self: Pin<&mut Self>) {
        self.as_mut().core().pause();
        self.as_mut().set_playing(false);
    }

    fn rewind(mut self: Pin<&mut Self>) {
        let start = self.as_mut().core().rewind();
        self.as_mut().set_playing(false);
        self.as_mut().set_position(start);
        self.as_mut().set_summary_json(QString::default());
        self.as_mut().set_holding_index(-1);
        self.as_mut().arrangement_loaded();
    }

    fn change_speed(mut self: Pin<&mut Self>, speed: f64) {
        self.as_mut().core().set_speed(speed);
        let speed = self.as_ref().rust().core.speed();
        self.as_mut().set_speed(speed);
    }

    fn change_mode(mut self: Pin<&mut Self>, mode: &QString) {
        let mode = mode.to_string();
        self.as_mut().core().set_mode(&mode);
        self.as_mut().set_mode(q(&mode));
        self.as_mut().reset_passes();
        self.as_mut().rewind();
    }

    fn change_view(mut self: Pin<&mut Self>, view: &QString) {
        let view = view.to_string();
        self.as_mut().core().set_view(&view);
        self.as_mut().set_view(q(&view));
    }

    fn change_style(mut self: Pin<&mut Self>, style: &QString) {
        let style = style.to_string();
        self.as_mut()
            .core()
            .update_settings(|s| s.style = style.clone());
        self.as_mut().set_style(q(&style));
    }

    fn change_language(mut self: Pin<&mut Self>, code: &QString) {
        let code = code.to_string();
        self.as_mut()
            .core()
            .update_settings(|s| s.language = Some(code.clone()));
        let texts = self.as_ref().rust().core.texts_json();
        self.as_mut().set_texts(q(&texts));
        self.as_mut().set_language(q(&code));
    }

    fn loop_bars(mut self: Pin<&mut Self>, from: i32, to: i32) {
        let from = usize::try_from(from).unwrap_or(0);
        let to = usize::try_from(to).unwrap_or(0);
        self.as_mut().core().set_loop(from, to);
        self.as_mut().reset_passes();
        let (start, end) = self.as_ref().rust().core.loop_seconds();
        self.as_mut().set_loop_start(start);
        self.as_mut().set_loop_end(end);
    }

    fn loop_edge(mut self: Pin<&mut Self>, edge: &QString) {
        let edge = match edge.to_string().as_str() {
            "start" => LoopEdge::Start,
            "end" => LoopEdge::End,
            _ => return,
        };
        let position = *self.as_ref().position();
        self.as_mut().core().set_loop_edge(edge, position);
        self.as_mut().reset_passes();
        let (start, end) = self.as_ref().rust().core.loop_seconds();
        self.as_mut().set_loop_start(start);
        self.as_mut().set_loop_end(end);
    }

    fn reset_passes(mut self: Pin<&mut Self>) {
        self.as_mut().set_passes(0);
        self.as_mut().set_pass_rate(-1.0);
    }

    fn clear_loop(mut self: Pin<&mut Self>) {
        self.as_mut().reset_passes();
        self.as_mut().core().clear_loop();
        self.as_mut().set_loop_start(-1.0);
        self.as_mut().set_loop_end(-1.0);
    }

    fn tick(mut self: Pin<&mut Self>) {
        let events = self.as_mut().core().tick();
        self.as_mut().apply(events);
        let (position, playing) = {
            let this = self.as_ref();
            let core = &this.rust().core;
            (core.position_s(), core.is_playing())
        };
        if (position - *self.as_ref().position()).abs() > 1e-6 {
            self.as_mut().set_position(position);
        }
        if playing != *self.as_ref().playing() {
            self.as_mut().set_playing(playing);
        }
    }

    fn apply(mut self: Pin<&mut Self>, events: Vec<UiEvent>) {
        for event in events {
            match event {
                UiEvent::Judged {
                    index,
                    state,
                    offset_ms,
                    cents,
                } => {
                    self.as_mut().note_judged(
                        i32::try_from(index).unwrap_or(-1),
                        q(state),
                        offset_ms,
                        cents,
                    );
                }
                UiEvent::Extra => self.as_mut().extra_note(),
                UiEvent::Holding { index, text } => {
                    self.as_mut()
                        .set_holding_index(i32::try_from(index).unwrap_or(-1));
                    self.as_mut().set_holding_text(q(&text));
                }
                UiEvent::Released => self.as_mut().set_holding_index(-1),
                UiEvent::PassComplete { clean, hit_rate } => {
                    let passes = *self.as_ref().passes() + 1;
                    self.as_mut().set_passes(passes);
                    self.as_mut().set_pass_rate(hit_rate);
                    self.as_mut().pass_complete(clean, hit_rate);
                }
                UiEvent::SpeedChanged(speed) => self.as_mut().set_speed(speed),
                UiEvent::Finished { summary_json } => {
                    self.as_mut().set_summary_json(q(&summary_json));
                    self.as_mut().set_playing(false);
                    self.as_mut().finished();
                }
                UiEvent::Level {
                    peak_db,
                    rms_db,
                    clipped,
                } => {
                    self.as_mut().set_peak_db(peak_db);
                    self.as_mut().set_rms_db(rms_db);
                    self.as_mut().set_clipped(clipped);
                }
                UiEvent::Tuner { note, cents, midi } => {
                    self.as_mut().set_tuner_note(q(&note));
                    self.as_mut().set_tuner_cents(cents);
                    self.as_mut().set_tuner_midi(midi);
                }
                UiEvent::Message(text) => self.as_mut().set_message(q(&text)),
                UiEvent::Error(text) => self.as_mut().show_error(&text),
                UiEvent::DeviceLost(text) => {
                    self.as_mut().set_playing(false);
                    self.as_mut().set_tuner_active(false);
                    self.as_mut().show_error(&text);
                }
                UiEvent::Calibrated { latency_ms, text } => {
                    if let Some(latency) = latency_ms {
                        self.as_mut().set_latency_ms(latency);
                    }
                    self.as_mut().set_calibration_text(q(&text));
                    self.as_mut().set_calibrating(false);
                }
            }
        }
    }

    fn start_tuner(mut self: Pin<&mut Self>) {
        self.as_mut().set_playing(false);
        let tuning = self.as_ref().rust().core.tuning_json();
        self.as_mut().set_tuning_json(q(&tuning));
        match self.as_mut().core().start_tuner() {
            Ok(()) => self.as_mut().set_tuner_active(true),
            Err(e) => self.as_mut().show_error(&e),
        }
    }

    fn stop_tuner(mut self: Pin<&mut Self>) {
        self.as_mut().core().stop_tuner();
        self.as_mut().set_tuner_active(false);
        self.as_mut().set_tuner_note(QString::default());
    }

    fn refresh_devices(mut self: Pin<&mut Self>) {
        let inputs = self.as_ref().rust().core.devices_json(true);
        let outputs = self.as_ref().rust().core.devices_json(false);
        self.as_mut().set_input_devices_json(q(&inputs));
        self.as_mut().set_output_devices_json(q(&outputs));
    }

    fn choose_input(mut self: Pin<&mut Self>, id: &QString) {
        let id = id.to_string();
        let value = (!id.is_empty()).then(|| id.clone());
        self.as_mut().core().reconfigure(|s| s.input_device = value);
        self.as_mut().set_input_device(q(&id));
    }

    fn choose_output(mut self: Pin<&mut Self>, id: &QString) {
        let id = id.to_string();
        let value = (!id.is_empty()).then(|| id.clone());
        self.as_mut()
            .core()
            .reconfigure(|s| s.output_device = value);
        self.as_mut().set_output_device(q(&id));
    }

    fn change_instrument(
        mut self: Pin<&mut Self>,
        strings: i32,
        fretless: bool,
        laptop_jack: bool,
    ) {
        let count = u8::try_from(strings.clamp(4, 6)).unwrap_or(4);
        self.as_mut().core().reconfigure(|s| {
            s.strings = count;
            s.fretless = fretless;
            s.input_kind = if laptop_jack {
                kontra_appcore::InputKind::LaptopJack
            } else {
                kontra_appcore::InputKind::Line
            };
        });
        self.as_mut().set_strings(i32::from(count));
        self.as_mut().set_fretless(fretless);
        self.as_mut().set_laptop_jack(laptop_jack);
        self.as_mut().set_playing(false);
    }

    fn change_options(
        mut self: Pin<&mut Self>,
        metronome: bool,
        reference: bool,
        low_string_on_top: bool,
    ) {
        self.as_mut().core().reconfigure(|s| {
            s.metronome = metronome;
            s.reference_tones = reference;
            s.low_string_on_top = low_string_on_top;
        });
        self.as_mut().set_metronome(metronome);
        self.as_mut().set_reference(reference);
        self.as_mut().set_low_string_on_top(low_string_on_top);
        self.as_mut().set_playing(false);
    }

    fn import_folder(mut self: Pin<&mut Self>, url: &QString, as_setlist: bool) {
        let dir = local_path(&url.to_string());
        let (added, failed) = self.as_mut().core().import_folder(&dir, as_setlist);
        let (text, recent, lists) = {
            let this = self.as_ref();
            let core = &this.rust().core;
            let mut text = core.tr("library.imported", &[("n", added.to_string())]);
            for (name, reason) in &failed {
                text.push_str(&format!("\n{name}: {reason}"));
            }
            (text, core.recent_json(), core.setlists_json())
        };
        self.as_mut().set_recent_json(q(&recent));
        self.as_mut().set_setlists_json(q(&lists));
        self.as_mut().set_message(q(&text));
    }

    fn refresh_setlists(mut self: Pin<&mut Self>) {
        let (lists, next) = {
            let this = self.as_ref();
            let core = &this.rust().core;
            (core.setlists_json(), core.next_title().unwrap_or_default())
        };
        self.as_mut().set_setlists_json(q(&lists));
        self.as_mut().set_next_title(q(&next));
    }

    fn setlist_create(mut self: Pin<&mut Self>, name: &QString) {
        self.as_mut().core().setlist_create(&name.to_string());
        self.as_mut().refresh_setlists();
    }

    fn setlist_delete(mut self: Pin<&mut Self>, name: &QString) {
        self.as_mut().core().setlist_delete(&name.to_string());
        self.as_mut().refresh_setlists();
    }

    fn setlist_add_loaded(mut self: Pin<&mut Self>, name: &QString) {
        self.as_mut().core().setlist_add_loaded(&name.to_string());
        self.as_mut().refresh_setlists();
    }

    fn setlist_remove(mut self: Pin<&mut Self>, name: &QString, index: i32) {
        let index = usize::try_from(index).unwrap_or(usize::MAX);
        self.as_mut()
            .core()
            .setlist_remove(&name.to_string(), index);
        self.as_mut().refresh_setlists();
    }

    fn setlist_move(mut self: Pin<&mut Self>, name: &QString, index: i32, step: i32) {
        let index = usize::try_from(index).unwrap_or(usize::MAX);
        self.as_mut()
            .core()
            .setlist_move(&name.to_string(), index, step.signum());
        self.as_mut().refresh_setlists();
    }

    fn open_from_setlist(mut self: Pin<&mut Self>, name: &QString, index: i32) -> bool {
        let index = usize::try_from(index).unwrap_or(usize::MAX);
        let result = self
            .as_mut()
            .core()
            .open_from_setlist(&name.to_string(), index);
        self.as_mut().after_open(result)
    }

    fn switch_track(mut self: Pin<&mut Self>, track: i32) {
        let track = usize::try_from(track).unwrap_or(usize::MAX);
        match self.as_mut().core().switch_track(track) {
            Ok(()) => self.as_mut().refresh_arrangement(),
            Err(e) => self.as_mut().show_error(&e),
        }
    }

    fn open_next(mut self: Pin<&mut Self>) -> bool {
        let result = self.as_mut().core().open_next();
        self.as_mut().after_open(result)
    }

    /// Shared tail of opening: refresh views, ask for the track, or show the error.
    fn after_open(mut self: Pin<&mut Self>, result: Result<Opened, String>) -> bool {
        let ready = match result {
            Ok(Opened::Ready) => {
                self.as_mut().set_track_choices_json(q("[]"));
                self.as_mut().refresh_arrangement();
                if let Some(notice) = self.as_mut().core().take_notice() {
                    self.as_mut().set_message(q(&notice));
                }
                true
            }
            Ok(Opened::ChooseTrack(names)) => {
                self.as_mut()
                    .set_track_choices_json(q(&serde_json::to_string(&names).unwrap_or_default()));
                false
            }
            Err(e) => {
                self.as_mut().show_error(&e);
                false
            }
        };
        self.as_mut().refresh_setlists();
        ready
    }

    fn set_tags(mut self: Pin<&mut Self>, path: &QString, tags_json: &QString) {
        let tags: Vec<String> = serde_json::from_str(&tags_json.to_string()).unwrap_or_default();
        self.as_mut()
            .core()
            .set_tags(&local_path(&path.to_string()), &tags);
        let recent = self.as_ref().rust().core.recent_json();
        self.as_mut().set_recent_json(q(&recent));
    }

    fn tags_json(&self) -> QString {
        q(&self.rust().core.tags_json())
    }

    fn progress_json(&self) -> QString {
        q(&self.rust().core.progress_json())
    }

    fn change_scoring(mut self: Pin<&mut Self>, strictness: &QString, pitch_cents: f64) {
        let strictness = strictness.to_string();
        if !["auto", "loose", "normal", "strict"].contains(&strictness.as_str()) {
            return;
        }
        let cents = pitch_cents.clamp(0.0, 100.0);
        let value = q(&strictness);
        self.as_mut().core().reconfigure(|s| {
            s.strictness = strictness;
            s.pitch_cents = cents;
        });
        self.as_mut().set_strictness(value);
        self.as_mut().set_pitch_cents(cents);
        self.as_mut().set_playing(false);
    }

    fn change_play_marks(mut self: Pin<&mut Self>, marks: &QString) {
        let marks = marks.to_string();
        if !["quiet", "off", "full"].contains(&marks.as_str()) {
            return;
        }
        let value = q(&marks);
        // Display only: no need to stop a running take.
        self.as_mut()
            .core()
            .update_settings(|s| s.play_marks = marks);
        self.as_mut().set_play_marks(value);
    }

    fn change_motion(mut self: Pin<&mut Self>, motion: &QString) {
        let motion = motion.to_string();
        if !["system", "reduce", "full"].contains(&motion.as_str()) {
            return;
        }
        let value = q(&motion);
        self.as_mut()
            .core()
            .update_settings(|s| s.motion = motion.clone());
        self.as_mut().set_motion(value);
        // Less motion means the page-wise tab instead of the scrolling lane.
        if motion == "reduce" && self.as_ref().view().to_string() == "lane" {
            self.as_mut().change_view(&q("tab"));
        }
    }

    fn change_count_in(mut self: Pin<&mut Self>, on: bool) {
        self.as_mut()
            .core()
            .reconfigure(|s| s.count_in_bars = u32::from(on));
        self.as_mut().set_count_in(on);
        self.as_mut().set_playing(false);
    }

    fn change_latency(mut self: Pin<&mut Self>, latency_ms: f64) {
        let value = latency_ms.clamp(0.0, 500.0);
        self.as_mut().core().reconfigure(|s| s.latency_ms = value);
        self.as_mut().set_latency_ms(value);
        self.as_mut().set_playing(false);
    }

    fn change_volume(mut self: Pin<&mut Self>, bus: &QString, value: f64) {
        let value = value.clamp(0.0, 1.0);
        let hint = match bus.to_string().as_str() {
            "backing" => {
                self.as_mut().set_backing_volume(value);
                self.as_mut().core().remix(|s| s.backing_volume = value)
            }
            "metronome" => {
                self.as_mut().set_metronome_volume(value);
                self.as_mut().core().remix(|s| s.metronome_volume = value)
            }
            "tone" => {
                self.as_mut().set_tone_volume(value);
                self.as_mut().core().remix(|s| s.tone_volume = value)
            }
            "monitor" => {
                self.as_mut().set_monitor_volume(value);
                self.as_mut().core().remix(|s| s.monitor_volume = value)
            }
            _ => None,
        };
        if let Some(hint) = hint {
            self.as_mut().set_message(q(&hint));
        }
    }

    fn change_monitor(mut self: Pin<&mut Self>, on: bool) {
        self.as_mut().set_monitor(on);
        if let Some(hint) = self.as_mut().core().remix(|s| s.monitor = on) {
            self.as_mut().set_message(q(&hint));
        }
    }

    fn start_calibration(mut self: Pin<&mut Self>) {
        self.as_mut().set_playing(false);
        self.as_mut().set_tuner_active(false);
        match self.as_mut().core().start_calibration() {
            Ok(()) => {
                let text = self.as_ref().rust().core.tr("calibration.running", &[]);
                self.as_mut().set_calibration_text(q(&text));
                self.as_mut().set_calibrating(true);
            }
            Err(e) => self.as_mut().show_error(&e),
        }
    }

    fn choose_backing(mut self: Pin<&mut Self>, url: &QString) {
        let url = url.to_string();
        let path = (!url.is_empty()).then(|| local_path(&url));
        self.as_mut().core().set_backing(path, 0.0);
        let name = self.as_ref().rust().core.backing_name();
        self.as_mut().set_backing_name(q(&name));
        self.as_mut().set_playing(false);
    }

    fn finish_setup(mut self: Pin<&mut Self>) {
        self.as_mut().core().stop_tuner();
        self.as_mut()
            .core()
            .update_settings(|s| s.setup_done = true);
        self.as_mut().set_tuner_active(false);
        self.as_mut().set_setup_done(true);
    }

    fn open_demo(mut self: Pin<&mut Self>) -> bool {
        let path = Core::demo_path();
        self.as_mut().open_file(&q(&path.to_string_lossy()))
    }

    fn dismiss_message(mut self: Pin<&mut Self>) {
        self.as_mut().set_message(QString::default());
        self.as_mut().set_error_text(QString::default());
    }
}

#[cfg(test)]
mod tests {
    use super::local_path;
    use std::path::PathBuf;

    #[test]
    fn decodes_file_urls() {
        assert_eq!(
            local_path("file:///home/a%20b/x.gp5"),
            PathBuf::from("/home/a b/x.gp5")
        );
        assert_eq!(
            local_path("/plain/path.mid"),
            PathBuf::from("/plain/path.mid")
        );
    }
}
