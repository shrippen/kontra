//! Start-up of the controller: `Default` builds the core and the placeholders, `initialize` fills
//! the properties through the refresh functions used later.

use super::{q, qobject, AppArgs, ControllerRust, NO_BEAT};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use kontra_appcore::{data_dir, Choice, Core, PracticeMode, QaOptions, Style, View};

impl Default for ControllerRust {
    fn default() -> Self {
        let args = AppArgs::from_env();
        let qa = QaOptions {
            input_file: args.input_file.clone(),
            laptop_jack: args.laptop_jack,
        };
        let mut core = Core::new(&data_dir(), qa);
        // Ready before the settings page asks for them.
        core.list_devices_early();
        // Devices stay open while Kontra is open (LH-EIN-18); a screenshot run holds none.
        if args.screenshot.is_none() {
            core.keep_devices_open();
        }
        if let Some(answer) = &args.songsterr_answer {
            core.fake_song_search(answer);
        }
        if let Some(answer) = &args.course_answer {
            core.fake_course_web(answer);
        }
        core.start_update_check(args.update_answer.as_deref(), args.screenshot.as_deref());
        if let Some(style) = core.pick_opt::<Style>(args.style.as_deref()) {
            core.update_settings(|s| s.style = style);
        }
        if let Some(language) = &args.language {
            core.update_settings(|s| s.language = Some(language.clone()));
        }
        if let Some(mode) = core.pick_opt::<PracticeMode>(args.mode.as_deref()) {
            core.reconfigure(|s| s.mode = mode);
        }
        if let Some(view) = core.pick_opt::<View>(args.view.as_deref()) {
            core.update_settings(|s| s.view = view);
        }
        ControllerRust {
            texts: QString::default(),
            loaded: false,
            title: QString::default(),
            artist: QString::default(),
            notes_json: q("[]"),
            bars_json: q("[]"),
            staff_json: QString::default(),
            string_names_json: q("[]"),
            key_range_json: QString::default(),
            duration: 0.0,
            position: 0.0,
            playing: false,
            probing: false,
            speed: 1.0,
            loop_start: -1.0,
            pass_rate: -1.0,
            passes: 0,
            band_low: kontra_appcore::Core::workshop_band().0,
            band_high: kontra_appcore::Core::workshop_band().1,
            loop_end: -1.0,
            mode: QString::default(),
            view: QString::default(),
            style: QString::default(),
            language: QString::default(),
            peak_db: -120.0,
            rms_db: -120.0,
            clipped: false,
            tuner_note: QString::default(),
            ear_state: QString::default(),
            level_state: QString::default(),
            suggest_laptop: false,
            tuner_midi: 0.0,
            tuning_json: QString::default(),
            tuner_cents: 0.0,
            tuner_active: false,
            holding_index: -1,
            holding_text: QString::default(),
            summary_json: QString::default(),
            chain_json: q("null"),
            saved_loops_json: q("[]"),
            target_speed: 1.0,
            recent_json: QString::default(),
            setlists_json: QString::default(),
            library_revision: 0,
            next_title: QString::default(),
            tracks_json: q("{\"names\": [], \"current\": 0}"),
            track_choices_json: q("[]"),
            input_devices_json: q("[]"),
            output_devices_json: q("[]"),
            input_device: QString::default(),
            output_device: QString::default(),
            strings: 0,
            fretless: false,
            instrument: QString::default(),
            laptop_jack: false,
            connection: QString::default(),
            midi_port_name: QString::default(),
            midi_ports_json: QString::default(),
            metronome: false,
            count_in: false,
            reference: false,
            low_string_on_top: false,
            fretboard_shown: false,
            play_marks: QString::default(),
            motion: QString::default(),
            strictness: QString::default(),
            pocket: QString::default(),
            pitch_cents: 0.0,
            latency_ms: 0.0,
            tuning: QString::default(),
            profile_tuning_json: QString::default(),
            note_choices_json: QString::default(),
            a4_hz: 0.0,
            window_ms: 0.0,
            string_colors: QString::default(),
            left_handed: false,
            backing_volume: 0.0,
            metronome_volume: 0.0,
            tone_volume: 0.0,
            drums_volume: 0.0,
            bass_volume: 0.0,
            prompt_volume: 0.0,
            monitor: false,
            monitor_volume: 0.0,
            monitor_low_db: 0.0,
            monitor_mid_db: 0.0,
            monitor_high_db: 0.0,
            monitor_compress: 0.0,
            monitor_warmth: 0.0,
            effects_json: QString::default(),
            monitor_limit: false,
            shaping_range_db: kontra_appcore::Core::shaping_range_db(),
            density: 100,
            density_fixed: false,
            density_suggestion: 0,
            density_text: QString::default(),
            adaptive: false,
            reacting_band: false,
            call: false,
            call_hidden: false,
            simple_view: false,
            streak: false,
            calibrating: false,
            calibration_beat: NO_BEAT,
            calibration_text: QString::default(),
            cable_measuring: false,
            latency_note: QString::default(),
            backing_name: QString::default(),
            stems_json: QString::default(),
            backing_loading: false,
            song_search_json: QString::default(),
            course_fetch_json: QString::default(),
            course_folder_json: QString::default(),
            run_kept: false,
            transpose: 0,
            transpose_max: kontra_appcore::Core::transpose_max(),
            message: QString::default(),
            xruns: 0,
            offer_input: QString::default(),
            input_state: QString::default(),
            input_detail: QString::default(),
            offer_text: QString::default(),
            usb_ask: QString::default(),
            error_text: QString::default(),
            start_page: q(args.page.as_deref().unwrap_or("")),
            start_course: QString::default(),
            setup_done: false,
            llm_provider: QString::default(),
            llm_base_url: QString::default(),
            llm_model: QString::default(),
            llm_models_json: q("[]"),
            llm_busy: false,
            llm_key_busy: false,
            autoplay: args.autoplay,
            autoprobe: args.probe,
            autocalibrate: args.calibrate,
            show_mixer: args.mixer,
            start_presenting: args.present,
            start_section: q(args.section.as_deref().unwrap_or("")),
            show_align: args.align,
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
            qa_run: args.input_file.is_some(),
            args,
            core,
        }
    }
}

impl cxx_qt::Initialize for qobject::Controller {
    /// Fills everything the core knows through the refresh functions the page uses later, so the
    /// start state and a refresh after a backup restore or an opened piece cannot differ.
    fn initialize(mut self: Pin<&mut Self>) {
        let args = self.as_ref().rust().args.clone();
        self.as_mut().sync_settings();
        // The command line may force the laptop jack without changing the stored input.
        if args.laptop_jack {
            self.as_mut().set_laptop_jack(true);
        }
        let songs = match &args.song_search {
            Some(query) => self.as_mut().core().song_search(query),
            None => self.as_mut().core().song_search_json(),
        };
        self.as_mut().set_song_search_json(q(&songs));
        // A course is added, not played: the learn page shows what it is first (LH-LRN-14).
        if let Some(path) = args.open.as_ref().filter(|p| Core::is_course(p)) {
            self.as_mut().set_start_course(q(&path.to_string_lossy()));
            self.as_mut().set_start_page(q("learn"));
            return;
        }
        let Some(path) = args.open else {
            return;
        };
        let result = self.as_mut().core().open(&path);
        self.as_mut().after_open(result);
        // Opening may set the mode (plan exercises: run-through); an explicit --mode wins.
        // `Default` told an unknown key already; here it is read silently.
        let mode = args.mode.as_deref().and_then(PracticeMode::from_key);
        if let Some(mode) = mode {
            self.as_mut().core().set_mode(mode);
        }
        let mode = self.as_ref().rust().core.settings().mode.key();
        self.as_mut().set_mode(q(&mode));
        // An explicit --view wins over the view remembered for the piece (QA runs).
        let view = args.view.as_deref().and_then(View::from_key);
        if let Some(view) = view {
            self.as_mut().core().set_view(view);
            self.as_mut().set_view(q(&view.key()));
        }
        if args.remove_bass {
            self.as_mut().core().remove_bass(true);
            self.as_mut().show_stems();
        }
    }
}
