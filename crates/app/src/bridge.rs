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
        #[qproperty(QString, string_names_json, cxx_name = "stringNamesJson")]
        #[qproperty(f64, duration)]
        #[qproperty(f64, position)]
        #[qproperty(bool, playing)]
        #[qproperty(f64, speed)]
        #[qproperty(f64, loop_start, cxx_name = "loopStart")]
        #[qproperty(f64, loop_end, cxx_name = "loopEnd")]
        #[qproperty(QString, mode)]
        #[qproperty(QString, view)]
        #[qproperty(QString, style)]
        #[qproperty(QString, language)]
        #[qproperty(f64, peak_db, cxx_name = "peakDb")]
        #[qproperty(f64, rms_db, cxx_name = "rmsDb")]
        #[qproperty(bool, clipped)]
        #[qproperty(QString, tuner_note, cxx_name = "tunerNote")]
        #[qproperty(f64, tuner_cents, cxx_name = "tunerCents")]
        #[qproperty(bool, tuner_active, cxx_name = "tunerActive")]
        #[qproperty(i32, holding_index, cxx_name = "holdingIndex")]
        #[qproperty(QString, holding_text, cxx_name = "holdingText")]
        #[qproperty(QString, summary_json, cxx_name = "summaryJson")]
        #[qproperty(QString, recent_json, cxx_name = "recentJson")]
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
        #[qproperty(f64, latency_ms, cxx_name = "latencyMs")]
        #[qproperty(bool, calibrating)]
        #[qproperty(QString, calibration_text, cxx_name = "calibrationText")]
        #[qproperty(QString, backing_name, cxx_name = "backingName")]
        #[qproperty(QString, message)]
        #[qproperty(QString, error_text, cxx_name = "errorText")]
        #[qproperty(QString, start_page, cxx_name = "startPage")]
        #[qproperty(bool, autoplay)]
        #[qproperty(bool, autocalibrate)]
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

        #[qinvokable]
        #[cxx_name = "changeCountIn"]
        fn change_count_in(self: Pin<&mut Self>, on: bool);

        #[qinvokable]
        #[cxx_name = "changeLatency"]
        fn change_latency(self: Pin<&mut Self>, latency_ms: f64);

        /// Latency calibration "play on the click" (LH-EIN-05).
        #[qinvokable]
        #[cxx_name = "startCalibration"]
        fn start_calibration(self: Pin<&mut Self>);

        /// Backing track for the loaded piece (LH-WIE-01); an empty URL removes it.
        #[qinvokable]
        #[cxx_name = "chooseBacking"]
        fn choose_backing(self: Pin<&mut Self>, url: &QString);

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
        fn pass_complete(self: Pin<&mut Self>, clean: bool);

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
use kontra_appcore::{data_dir, Core, Opened, QaOptions, UiEvent};
use std::path::PathBuf;

pub struct ControllerRust {
    texts: QString,
    loaded: bool,
    title: QString,
    artist: QString,
    notes_json: QString,
    bars_json: QString,
    string_names_json: QString,
    duration: f64,
    position: f64,
    playing: bool,
    speed: f64,
    loop_start: f64,
    loop_end: f64,
    mode: QString,
    view: QString,
    style: QString,
    language: QString,
    peak_db: f64,
    rms_db: f64,
    clipped: bool,
    tuner_note: QString,
    tuner_cents: f64,
    tuner_active: bool,
    holding_index: i32,
    holding_text: QString,
    summary_json: QString,
    recent_json: QString,
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
    latency_ms: f64,
    calibrating: bool,
    calibration_text: QString,
    backing_name: QString,
    message: QString,
    error_text: QString,
    start_page: QString,
    autoplay: bool,
    autocalibrate: bool,
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
            string_names_json: q("[]"),
            duration: 0.0,
            position: 0.0,
            playing: false,
            speed: 1.0,
            loop_start: -1.0,
            loop_end: -1.0,
            mode: q(&s.mode),
            view: q(&s.view),
            style: q(&s.style),
            language: q(s.language.as_deref().unwrap_or("")),
            peak_db: -120.0,
            rms_db: -120.0,
            clipped: false,
            tuner_note: QString::default(),
            tuner_cents: 0.0,
            tuner_active: false,
            holding_index: -1,
            holding_text: QString::default(),
            summary_json: QString::default(),
            recent_json: q(&core.recent_json()),
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
            latency_ms: s.latency_ms,
            calibrating: false,
            calibration_text: QString::default(),
            backing_name: QString::default(),
            message: QString::default(),
            error_text: QString::default(),
            start_page: q(args.page.as_deref().unwrap_or("library")),
            autoplay: args.autoplay,
            autocalibrate: args.calibrate,
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
                self.string_names_json = q(&self.core.string_names_json());
                self.duration = self.core.duration_s();
                self.recent_json = q(&self.core.recent_json());
                self.backing_name = q(&self.core.backing_name());
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
        let names = core.string_names_json();
        let duration = core.duration_s();
        let recent = core.recent_json();
        let loaded = core.arrangement().is_some();
        let backing = core.backing_name();
        self.as_mut().set_title(q(&title));
        self.as_mut().set_artist(q(&artist));
        self.as_mut().set_notes_json(q(&notes));
        self.as_mut().set_bars_json(q(&bars));
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
        match self.as_mut().core().open(&path) {
            Ok(Opened::Ready) => {
                self.as_mut().set_track_choices_json(q("[]"));
                self.as_mut().refresh_arrangement();
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
        }
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
        self.as_mut().core().stop_live();
        self.as_mut().set_playing(false);
        self.as_mut().set_position(0.0);
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
        self.as_mut().rewind();
    }

    fn change_view(mut self: Pin<&mut Self>, view: &QString) {
        let view = view.to_string();
        self.as_mut()
            .core()
            .update_settings(|s| s.view = view.clone());
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
        let (start, end) = self.as_ref().rust().core.loop_seconds();
        self.as_mut().set_loop_start(start);
        self.as_mut().set_loop_end(end);
    }

    fn clear_loop(mut self: Pin<&mut Self>) {
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
                UiEvent::PassComplete { clean } => self.as_mut().pass_complete(clean),
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
                UiEvent::Tuner { note, cents } => {
                    self.as_mut().set_tuner_note(q(&note));
                    self.as_mut().set_tuner_cents(cents);
                }
                UiEvent::Message(text) => self.as_mut().set_message(q(&text)),
                UiEvent::Error(text) => self.as_mut().show_error(&text),
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
