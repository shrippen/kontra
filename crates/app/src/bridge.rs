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
        #[qproperty(QString, key_range_json, cxx_name = "keyRangeJson")]
        #[qproperty(f64, duration)]
        #[qproperty(f64, position)]
        #[qproperty(bool, playing)]
        #[qproperty(bool, probing)]
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
        /// Level advice (LH-EIN-03/04): "", "silent", "quiet", "good", "loud", "clip".
        #[qproperty(QString, level_state, cxx_name = "levelState")]
        #[qproperty(bool, suggest_laptop, cxx_name = "suggestLaptop")]
        /// Ear training state (LH-LRN-03), see `Core::ear_json`; "" when none runs.
        #[qproperty(QString, ear_state, cxx_name = "earState")]
        /// Open strings for guided tuning: `[{"name", "midi"}]`, lowest first.
        #[qproperty(QString, tuning_json, cxx_name = "tuningJson")]
        #[qproperty(f64, tuner_cents, cxx_name = "tunerCents")]
        #[qproperty(bool, tuner_active, cxx_name = "tunerActive")]
        #[qproperty(i32, holding_index, cxx_name = "holdingIndex")]
        #[qproperty(QString, holding_text, cxx_name = "holdingText")]
        #[qproperty(QString, summary_json, cxx_name = "summaryJson")]
        #[qproperty(QString, chain_json, cxx_name = "chainJson")]
        /// Loops kept with the piece: `[{"from", "to"}]`, bar indices (LH-BIB-07).
        #[qproperty(QString, saved_loops_json, cxx_name = "savedLoopsJson")]
        /// The workshop's target speed for the piece, 1.0 = as written.
        #[qproperty(f64, target_speed, cxx_name = "targetSpeed")]
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
        /// The instrument as setup and settings offer it: "ebass", "double" or "midi".
        #[qproperty(QString, instrument)]
        #[qproperty(bool, laptop_jack, cxx_name = "laptopJack")]
        /// How the bass arrives: "line", "laptop", "midi" (LH-EIN-12) or "mic" (LH-EIN-11).
        #[qproperty(QString, connection)]
        /// The MIDI input Kontra would use, "" if none is connected.
        #[qproperty(QString, midi_port_name, cxx_name = "midiPortName")]
        #[qproperty(QString, midi_ports_json, cxx_name = "midiPortsJson")]
        #[qproperty(bool, metronome)]
        #[qproperty(bool, count_in, cxx_name = "countIn")]
        #[qproperty(bool, reference)]
        #[qproperty(bool, low_string_on_top, cxx_name = "lowStringOnTop")]
        #[qproperty(bool, fretboard_shown, cxx_name = "fretboardShown")]
        #[qproperty(QString, play_marks, cxx_name = "playMarks")]
        #[qproperty(QString, motion)]
        #[qproperty(QString, strictness)]
        #[qproperty(QString, pocket)]
        #[qproperty(f64, pitch_cents, cxx_name = "pitchCents")]
        #[qproperty(f64, latency_ms, cxx_name = "latencyMs")]
        /// Tuning of the profile (LH-TUN-02): "standard", "drop", "half-down", "whole-down", "custom".
        #[qproperty(QString, tuning)]
        /// The profile's open strings: `[{"name", "full", "midi"}]`, lowest first.
        #[qproperty(QString, profile_tuning_json, cxx_name = "profileTuningJson")]
        /// Notes a string of a custom tuning can take: `[{"name", "midi"}]`.
        #[qproperty(QString, note_choices_json, cxx_name = "noteChoicesJson")]
        /// Reference pitch of A4 in Hz (LH-TUN-04).
        #[qproperty(f64, a4_hz, cxx_name = "a4Hz")]
        /// Free timing window in ms, used with strictness "custom" (LH-BEW-02).
        #[qproperty(f64, window_ms, cxx_name = "windowMs")]
        /// String colour preset (LH-ANZ-06): "kante", "mono" or "bright".
        #[qproperty(QString, string_colors, cxx_name = "stringColors")]
        /// Mirrored fretboard (LH-ANZ-08).
        #[qproperty(bool, left_handed, cxx_name = "leftHanded")]
        /// Mixer (LH-WIE-03, LH-MOD-05): volumes 0..1 and input monitoring.
        #[qproperty(f64, backing_volume, cxx_name = "backingVolume")]
        #[qproperty(f64, metronome_volume, cxx_name = "metronomeVolume")]
        #[qproperty(f64, tone_volume, cxx_name = "toneVolume")]
        #[qproperty(f64, drums_volume, cxx_name = "drumsVolume")]
        #[qproperty(f64, bass_volume, cxx_name = "bassVolume")]
        #[qproperty(f64, prompt_volume, cxx_name = "promptVolume")]
        #[qproperty(bool, monitor)]
        #[qproperty(f64, monitor_volume, cxx_name = "monitorVolume")]
        /// Tone shaping of the monitored bass (LH-EIN-08): bands in dB within
        /// ±`shapingRangeDb`, compressor and warmth 0..1.
        #[qproperty(f64, monitor_low_db, cxx_name = "monitorLowDb")]
        #[qproperty(f64, monitor_mid_db, cxx_name = "monitorMidDb")]
        #[qproperty(f64, monitor_high_db, cxx_name = "monitorHighDb")]
        #[qproperty(f64, monitor_compress, cxx_name = "monitorCompress")]
        #[qproperty(f64, monitor_warmth, cxx_name = "monitorWarmth")]
        /// The effect chain as JSON (`Core::effects_json`, LH-EIN-19); limiter on (LH-EIN-17).
        #[qproperty(QString, effects_json, cxx_name = "effectsJson")]
        #[qproperty(bool, monitor_limit, cxx_name = "monitorLimit")]
        #[qproperty(f64, shaping_range_db, cxx_name = "shapingRangeDb")]
        /// Share of the notes in the current section, 25..=100 % (LH-MOD-06), and whether it is fixed.
        #[qproperty(i32, density)]
        #[qproperty(bool, density_fixed, cxx_name = "densityFixed")]
        /// Suggested share after a run (0: none) and the question to show.
        #[qproperty(i32, density_suggestion, cxx_name = "densitySuggestion")]
        #[qproperty(QString, density_text, cxx_name = "densityText")]
        #[qproperty(bool, adaptive)]
        /// Generated drums answer the bass in free play (LH-WIE-05).
        #[qproperty(bool, reacting_band, cxx_name = "reactingBand")]
        /// Call and response on (LH-LRN-19), and the tab hidden until two failed tries.
        #[qproperty(bool, call)]
        #[qproperty(bool, call_hidden, cxx_name = "callHidden")]
        /// The practice page without its rare controls (LH-ANZ-11).
        #[qproperty(bool, simple_view, cxx_name = "simpleView")]
        #[qproperty(bool, streak)]
        #[qproperty(bool, calibrating)]
        /// The calibration's click: −4..−1 count-in, 0..7 measured, −99 none.
        #[qproperty(i32, calibration_beat, cxx_name = "calibrationBeat")]
        #[qproperty(QString, calibration_text, cxx_name = "calibrationText")]
        /// The running measurement is the one by cable (no clicks to count).
        #[qproperty(bool, cable_measuring, cxx_name = "cableMeasuring")]
        /// Where the latency comes from, with the player's lean: "mit Kabel gemessen · …".
        #[qproperty(QString, latency_note, cxx_name = "latencyNote")]
        #[qproperty(QString, backing_name, cxx_name = "backingName")]
        #[qproperty(QString, stems_json, cxx_name = "stemsJson")]
        /// The backing track is prepared in the background; play waits (QA 27.10., J2).
        #[qproperty(bool, backing_loading, cxx_name = "backingLoading")]
        /// Songsterr search: state, reason and hits as JSON (LH-BIB-11).
        #[qproperty(QString, song_search_json, cxx_name = "songSearchJson")]
        /// A course package fetched from an address, see `Core::course_fetch_poll`.
        #[qproperty(QString, course_fetch_json, cxx_name = "courseFetchJson")]
        /// A course folder open live, see `Core::course_folder_json` (LH-LRN-21).
        #[qproperty(QString, course_folder_json, cxx_name = "courseFolderJson")]
        /// While the arrangement is laid out again: the run goes on (a change of key), so the
        /// page keeps the marks.
        #[qproperty(bool, run_kept, cxx_name = "runKept")]
        /// Piece and backing moved by semitones (LH-WIE-02), −`transposeMax`..=`transposeMax`.
        #[qproperty(i32, transpose)]
        #[qproperty(i32, transpose_max, cxx_name = "transposeMax")]
        #[qproperty(QString, message)]
        #[qproperty(i32, xruns)]
        #[qproperty(QString, offer_input, cxx_name = "offerInput")]
        #[qproperty(QString, input_state, cxx_name = "inputState")]
        #[qproperty(QString, input_detail, cxx_name = "inputDetail")]
        #[qproperty(QString, offer_text, cxx_name = "offerText")]
        #[qproperty(QString, usb_ask, cxx_name = "usbAsk")]
        #[qproperty(QString, error_text, cxx_name = "errorText")]
        #[qproperty(QString, start_page, cxx_name = "startPage")]
        /// A course package given at start (`kontra kurs.kontrakurs`); the learn page shows it
        /// before adding and clears this.
        #[qproperty(QString, start_course, cxx_name = "startCourse")]
        #[qproperty(bool, setup_done, cxx_name = "setupDone")]
        #[qproperty(bool, autoplay)]
        /// QA: a blind probe of the start loop instead of autoplay (DIDAKTIK I1).
        #[qproperty(bool, autoprobe)]
        #[qproperty(bool, autocalibrate)]
        /// QA: practice and settings show the mixer from the start.
        #[qproperty(bool, show_mixer, cxx_name = "showMixer")]
        /// QA: practice opens in presentation mode.
        #[qproperty(bool, start_presenting, cxx_name = "startPresenting")]
        /// QA: the settings section to scroll to ("data", "about").
        #[qproperty(QString, start_section, cxx_name = "startSection")]
        #[qproperty(bool, show_align, cxx_name = "showAlign")]
        /// QA: bars to loop at start, 1-based "from-to" ("" = none).
        #[qproperty(QString, start_loop, cxx_name = "startLoop")]
        #[qproperty(QString, screenshot_path, cxx_name = "screenshotPath")]
        #[qproperty(i32, screenshot_after, cxx_name = "screenshotAfter")]
        #[qproperty(i32, quit_after, cxx_name = "quitAfter")]
        /// QA: a recording replaces the input (`--input-file`); no portal dialog, no browser
        /// (QA 27.10.: both opened on the owner's desktop).
        #[qproperty(bool, qa_run, cxx_name = "qaRun")]
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

        /// Building blocks (DIDAKTIK I6): `blocks` "2" or "4" bars or "phrases", "off" ends
        /// them; order "forward" or "backward".
        #[qinvokable]
        #[cxx_name = "startChain"]
        fn start_chain(self: Pin<&mut Self>, blocks: &QString, order: &QString);

        /// Plays the loop once without notes on screen (DIDAKTIK I1).
        #[qinvokable]
        #[cxx_name = "startProbe"]
        fn start_probe(self: Pin<&mut Self>);

        /// Loops the bars of a saved section, given by its times (start page).
        #[qinvokable]
        #[cxx_name = "loopAt"]
        fn loop_at(self: Pin<&mut Self>, start_s: f64, end_s: f64) -> bool;

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

        /// Loops printed bar `bar` (from 1), as the start page's cards name it.
        #[qinvokable]
        #[cxx_name = "loopScoreBar"]
        fn loop_score_bar(self: Pin<&mut Self>, bar: i32);

        #[qinvokable]
        #[cxx_name = "clearLoop"]
        fn clear_loop(self: Pin<&mut Self>);

        /// Keeps the current loop with the piece.
        #[qinvokable]
        #[cxx_name = "saveLoop"]
        fn save_loop(self: Pin<&mut Self>);

        /// Drops the kept loop at `index` (of `savedLoopsJson`).
        #[qinvokable]
        #[cxx_name = "removeLoop"]
        fn remove_loop(self: Pin<&mut Self>, index: i32);

        /// Sets the workshop's target speed for the piece (0.25..=1.5).
        #[qinvokable]
        #[cxx_name = "changeTargetSpeed"]
        fn change_target_speed(self: Pin<&mut Self>, speed: f64);

        /// Share of the notes in percent for the current section; `lock` "fixed" stops suggestions.
        #[qinvokable]
        #[cxx_name = "changeDensity"]
        fn change_density(self: Pin<&mut Self>, percent: i32, lock: &QString);

        /// Dismisses the density suggestion.
        #[qinvokable]
        #[cxx_name = "declineDensity"]
        fn decline_density(self: Pin<&mut Self>);

        /// Density suggestions on or off.
        #[qinvokable]
        #[cxx_name = "changeAdaptive"]
        fn change_adaptive(self: Pin<&mut Self>, on: bool);

        #[qinvokable]
        #[cxx_name = "changeReactingBand"]
        fn change_reacting_band(self: Pin<&mut Self>, on: bool);

        #[qinvokable]
        #[cxx_name = "changeCall"]
        fn change_call(self: Pin<&mut Self>, on: bool);

        /// Hits per bar over the last runs (LH-FOR-06), as JSON.
        #[qinvokable]
        #[cxx_name = "songMap"]
        fn song_map(self: &Self) -> QString;

        /// The simple practice page on or off (LH-ANZ-11).
        #[qinvokable]
        #[cxx_name = "changeSimpleView"]
        fn change_simple_view(self: Pin<&mut Self>, on: bool);

        #[qinvokable]
        #[cxx_name = "changeStreak"]
        fn change_streak(self: Pin<&mut Self>, on: bool);

        /// Update notice (LH-ALL-07): "poll", "dismiss", "on" or "off"; answers its JSON.
        #[qinvokable]
        #[cxx_name = "updateNotice"]
        fn update_notice(self: Pin<&mut Self>, action: &QString) -> QString;

        /// What the loaded bundled exercise trains ("" for other pieces).
        #[qinvokable]
        #[cxx_name = "loadedGoal"]
        fn loaded_goal(self: &Self) -> QString;

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

        /// The MIDI device by name ("" = the first one), LH-EIN-12.
        #[qinvokable]
        #[cxx_name = "chooseMidi"]
        fn choose_midi(self: Pin<&mut Self>, name: &QString);

        #[qinvokable]
        #[cxx_name = "chooseOutput"]
        fn choose_output(self: Pin<&mut Self>, id: &QString);

        /// Strings and frets of the bass; a double bass stays fretless.
        #[qinvokable]
        #[cxx_name = "changeInstrument"]
        fn change_instrument(self: Pin<&mut Self>, strings: i32, fretless: bool);

        /// Chooses the instrument (LH-EIN-09): "ebass", "double" or "midi".
        #[qinvokable]
        #[cxx_name = "chooseInstrument"]
        fn choose_instrument(self: Pin<&mut Self>, key: &QString);

        /// Chooses how the bass arrives: "line", "laptop" or "midi".
        #[qinvokable]
        #[cxx_name = "changeConnection"]
        fn change_connection(self: Pin<&mut Self>, key: &QString);

        #[qinvokable]
        #[cxx_name = "changeOptions"]
        fn change_options(
            self: Pin<&mut Self>,
            metronome: bool,
            reference: bool,
            low_string_on_top: bool,
        );

        /// The URL of the folder `path` lies in, for a file dialog ("" for a bare name).
        #[qinvokable]
        #[cxx_name = "folderUrl"]
        fn folder_url(self: &Self, path: &QString) -> QString;

        /// Adds all scores of a folder (URL) to the library, optionally as a setlist.
        #[qinvokable]
        #[cxx_name = "importFolder"]
        fn import_folder(self: Pin<&mut Self>, url: &QString, as_setlist: bool);

        /// Adds dropped files (JSON list of URLs) to the library (LH-BIB-11).
        #[qinvokable]
        #[cxx_name = "importFiles"]
        fn import_files(self: Pin<&mut Self>, urls_json: &QString);

        /// Search page of "songsterr" or "musescore" for `query`, for the browser (LH-BIB-11).
        #[qinvokable]
        #[cxx_name = "searchUrl"]
        fn search_url(self: &Self, site: &QString, query: &QString) -> QString;

        /// Scores downloaded lately and not yet in the library, as JSON (LH-BIB-11).
        #[qinvokable]
        #[cxx_name = "downloadsJson"]
        fn downloads_json(self: &Self) -> QString;

        /// Searches Songsterr from Kontra (LH-BIB-11, round 12); the result arrives in
        /// `songSearchJson`.
        #[qinvokable]
        #[cxx_name = "songSearch"]
        fn song_search(self: Pin<&mut Self>, query: &QString);

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
        #[cxx_name = "setlistAdd"]
        fn setlist_add(self: Pin<&mut Self>, name: &QString, path: &QString);

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

        /// Start page: last piece, next exercise, the week's practice (JSON).
        #[qinvokable]
        #[cxx_name = "homeJson"]
        fn home_json(self: &Self) -> QString;

        /// Timing window ("auto", "loose", "normal", "strict") and pitch limit (0 = auto).
        #[qinvokable]
        #[cxx_name = "changeScoring"]
        fn change_scoring(self: Pin<&mut Self>, strictness: &QString, pitch_cents: f64);

        /// Tuning by name (see the `tuning` property).
        #[qinvokable]
        #[cxx_name = "changeTuning"]
        fn change_tuning(self: Pin<&mut Self>, name: &QString);

        /// One open string of a custom tuning (0 = lowest) as MIDI note.
        #[qinvokable]
        #[cxx_name = "changeCustomString"]
        fn change_custom_string(self: Pin<&mut Self>, string: i32, midi: i32);

        /// Reference pitch of A4 in Hz (430–450).
        #[qinvokable]
        #[cxx_name = "changeReference"]
        fn change_reference(self: Pin<&mut Self>, a4_hz: f64);

        /// Free timing window in ms (20–150); switches the strictness to "custom".
        #[qinvokable]
        #[cxx_name = "changeWindow"]
        fn change_window(self: Pin<&mut Self>, window_ms: f64);

        /// String colour preset: "kante", "mono" or "bright".
        #[qinvokable]
        #[cxx_name = "changeStringColors"]
        fn change_string_colors(self: Pin<&mut Self>, preset: &QString);

        #[qinvokable]
        #[cxx_name = "changeLeftHanded"]
        fn change_left_handed(self: Pin<&mut Self>, on: bool);

        /// Aim against the beat (LH-BEW-08): "free", "onbeat", "laidback" or "pushed".
        #[qinvokable]
        #[cxx_name = "changePocket"]
        fn change_pocket(self: Pin<&mut Self>, pocket: &QString);

        /// Note marks in play mode: "quiet", "off" or "full".
        #[qinvokable]
        #[cxx_name = "changePlayMarks"]
        fn change_play_marks(self: Pin<&mut Self>, marks: &QString);

        /// Motion: "system" (follow the desktop), "reduce" or "full" (round 6).
        #[qinvokable]
        #[cxx_name = "changeMotion"]
        fn change_motion(self: Pin<&mut Self>, motion: &QString);

        /// Shows or hides the fretboard under the notes on the practice page.
        #[qinvokable]
        #[cxx_name = "changeFretboard"]
        fn change_fretboard(self: Pin<&mut Self>, shown: bool);

        #[qinvokable]
        #[cxx_name = "changeCountIn"]
        fn change_count_in(self: Pin<&mut Self>, on: bool);

        #[qinvokable]
        #[cxx_name = "changeLatency"]
        fn change_latency(self: Pin<&mut Self>, latency_ms: f64);

        /// Sets a mixer volume (0..1): `bus` is "backing", "metronome", "tone", "drums", "bass",
        /// "prompt" or "monitor".
        #[qinvokable]
        #[cxx_name = "changeVolume"]
        fn change_volume(self: Pin<&mut Self>, bus: &QString, value: f64);

        /// Plays the bass input through the output (off by default).
        #[qinvokable]
        #[cxx_name = "changeMonitor"]
        fn change_monitor(self: Pin<&mut Self>, on: bool);

        /// Sets a tone band of the monitored bass in dB (`band` "low", "mid", "high") or the
        /// compressor or warmth 0..1 (`band` "compress", "warmth").
        #[qinvokable]
        #[cxx_name = "changeShaping"]
        fn change_shaping(self: Pin<&mut Self>, band: &QString, value: f64);

        /// Turns the limiter of the monitored bass on or off.
        #[qinvokable]
        #[cxx_name = "changeLimit"]
        fn change_limit(self: Pin<&mut Self>, on: bool);

        /// Edits the effect chain, or starts the plugin search (`Core::edit_effects`: `op`).
        #[qinvokable]
        #[cxx_name = "editEffects"]
        fn edit_effects(self: Pin<&mut Self>, op: &QString, index: i32, name: &QString, value: f64);

        /// Back to the unshaped bass.
        #[qinvokable]
        #[cxx_name = "resetShaping"]
        fn reset_shaping(self: Pin<&mut Self>);

        /// Latency calibration "play on the click" (LH-EIN-05).
        #[qinvokable]
        #[cxx_name = "startCalibration"]
        fn start_calibration(self: Pin<&mut Self>);

        /// Latency by cable, output into input (LH-EIN-05).
        #[qinvokable]
        #[cxx_name = "startLoopback"]
        fn start_loopback(self: Pin<&mut Self>);

        /// Backing track for the loaded piece (LH-WIE-01); an empty URL removes it.
        #[qinvokable]
        #[cxx_name = "chooseBacking"]
        fn choose_backing(self: Pin<&mut Self>, url: &QString);

        /// Backing track without (true) or with its bass (LH-BIB-09); separates it when needed.
        #[qinvokable]
        #[cxx_name = "removeBass"]
        fn remove_bass(self: Pin<&mut Self>, on: bool);

        #[qinvokable]
        #[cxx_name = "cancelStems"]
        fn cancel_stems(self: Pin<&mut Self>);

        /// Deletes the backing track's separated file; it plays with its bass again.
        #[qinvokable]
        #[cxx_name = "forgetStems"]
        fn forget_stems(self: Pin<&mut Self>);

        /// Looks for the separator again (after installing it) and shows the state.
        #[qinvokable]
        #[cxx_name = "refreshStems"]
        fn refresh_stems(self: Pin<&mut Self>);

        /// Moves the piece and its backing by `semitones` and lays the notes out again.
        #[qinvokable]
        #[cxx_name = "changeTranspose"]
        fn change_transpose(self: Pin<&mut Self>, semitones: i32);

        /// Marks the first-start assistant as done (or skipped).
        #[qinvokable]
        #[cxx_name = "finishSetup"]
        fn finish_setup(self: Pin<&mut Self>);

        /// Bundled exercises by level (LH-BIB-08); see `Core::bundled_json`.
        #[qinvokable]
        #[cxx_name = "exercisesJson"]
        fn exercises_json(self: &Self) -> QString;

        /// Generates an exercise (LH-LRN-04) and opens it; see `Core::exercise_spec`.
        #[qinvokable]
        #[cxx_name = "generateExercise"]
        fn generate_exercise(
            self: Pin<&mut Self>,
            kind: &QString,
            variant: &QString,
            rhythm: &QString,
            key: i32,
            bpm: f64,
            judging: &QString,
        ) -> bool;

        /// Lernen (LH-LRN-07): the course, see `Core::learn_json`.
        #[qinvokable]
        #[cxx_name = "learnJson"]
        fn learn_json(self: &Self) -> QString;

        /// One lesson, see `Core::lesson_json`.
        #[qinvokable]
        #[cxx_name = "lessonJson"]
        fn lesson_json(self: &Self, id: &QString) -> QString;

        /// Page `page` of lesson `id` was shown (the last finishes it).
        #[qinvokable]
        #[cxx_name = "lessonSeen"]
        fn lesson_seen(self: Pin<&mut Self>, id: &QString, page: i32);

        /// "Listen" or "try it" of a lesson block; what was heard comes as `earState`. False when
        /// it did not start (the round before goes on).
        #[qinvokable]
        #[cxx_name = "lessonEar"]
        fn lesson_ear(self: Pin<&mut Self>, id: &QString, page: i32, block: i32) -> bool;

        /// "Listen" or "try it" in a glossary entry, as `lessonEar`.
        #[qinvokable]
        #[cxx_name = "termEar"]
        fn term_ear(self: Pin<&mut Self>, id: &QString, block: i32);

        /// An answer to a lesson question; true if right. It comes back for review (LH-LRN-10).
        #[qinvokable]
        #[cxx_name = "lessonAnswer"]
        fn lesson_answer(
            self: Pin<&mut Self>,
            id: &QString,
            page: i32,
            block: i32,
            option: i32,
        ) -> bool;

        /// Questions due for review, see `Core::review_json`.
        #[qinvokable]
        #[cxx_name = "reviewJson"]
        fn review_json(self: &Self) -> QString;

        /// "Kann ich schon": a chapter's questions, see `Core::chapter_check_json`.
        #[qinvokable]
        #[cxx_name = "chapterCheckJson"]
        fn chapter_check_json(self: &Self, chapter: &QString) -> QString;

        /// Ticks the chapter off if all its questions were right since `since` (seconds).
        #[qinvokable]
        #[cxx_name = "chapterPass"]
        fn chapter_pass(self: Pin<&mut Self>, chapter: &QString, since: f64) -> bool;

        /// A course package before adding it, see `Core::course_preview`.
        #[qinvokable]
        #[cxx_name = "coursePreview"]
        fn course_preview(self: &Self, url: &QString) -> QString;

        /// Adds a course package; "" when done, else what went wrong.
        #[qinvokable]
        #[cxx_name = "courseAdd"]
        fn course_add(self: Pin<&mut Self>, url: &QString) -> QString;

        /// Fetches a course package from an https address; the outcome comes as
        /// `courseFetchJson`.
        #[qinvokable]
        #[cxx_name = "courseFetch"]
        fn course_fetch(self: Pin<&mut Self>, url: &QString);

        /// Opens a course folder live; "" closes it.
        #[qinvokable]
        #[cxx_name = "openCourseFolder"]
        fn open_course_folder(self: Pin<&mut Self>, url: &QString);

        /// What to play for a lesson's video block, see `Core::lesson_video`.
        #[qinvokable]
        #[cxx_name = "lessonVideo"]
        fn lesson_video(self: &Self, id: &QString, page: i32, block: i32) -> QString;

        /// Removes added course `id`; "" when done, else what went wrong.
        #[qinvokable]
        #[cxx_name = "courseRemove"]
        fn course_remove(self: Pin<&mut Self>, id: &QString) -> QString;

        /// Learns course `id` from now on.
        #[qinvokable]
        #[cxx_name = "chooseCourse"]
        fn choose_course(self: Pin<&mut Self>, id: &QString);

        /// Opens the exercise of a lesson's practice block; true when it is ready to play.
        #[qinvokable]
        #[cxx_name = "lessonExercise"]
        fn lesson_exercise(self: Pin<&mut Self>, id: &QString, page: i32, block: i32) -> bool;

        /// Opens stage 0–4 of a practice block's ladder (LH-LRN-17).
        #[qinvokable]
        #[cxx_name = "lessonStage"]
        fn lesson_stage(
            self: Pin<&mut Self>,
            id: &QString,
            page: i32,
            block: i32,
            stage: i32,
        ) -> bool;

        /// Opens the exercise that rounds off chapter `chapter`.
        #[qinvokable]
        #[cxx_name = "finaleExercise"]
        fn finale_exercise(self: Pin<&mut Self>, chapter: &QString) -> bool;

        /// Glossary (LH-LRN-09): one term, see `Core::glossary_json`.
        #[qinvokable]
        #[cxx_name = "glossaryJson"]
        fn glossary_json(self: &Self, id: &QString) -> QString;

        /// All terms for the instrument: `[{"id", "title"}]`.
        #[qinvokable]
        #[cxx_name = "glossaryListJson"]
        fn glossary_list_json(self: &Self) -> QString;

        /// The term behind a "?" outside the lessons, see `Core::term_json`.
        #[qinvokable]
        #[cxx_name = "termJson"]
        fn term_json(self: &Self, topic: &QString) -> QString;

        /// Fretboard quiz (LH-LRN-02, -18): "notes" or "intervals" on frets `low`–`high`.
        #[qinvokable]
        #[cxx_name = "quizStart"]
        fn quiz_start(self: Pin<&mut Self>, kind: &QString, low: i32, high: i32) -> QString;

        #[qinvokable]
        #[cxx_name = "quizNext"]
        fn quiz_next(self: Pin<&mut Self>) -> QString;

        /// True when the played pitch answers the question (first time).
        #[qinvokable]
        #[cxx_name = "quizAnswer"]
        fn quiz_answer(self: Pin<&mut Self>, midi: f64) -> bool;

        #[qinvokable]
        #[cxx_name = "quizJson"]
        fn quiz_json(self: &Self) -> QString;

        /// Ear training (LH-LRN-03): "echo", "interval" or "chord"; plays the first prompt.
        /// Answers come from the bass and update `earState`.
        #[qinvokable]
        #[cxx_name = "earStart"]
        fn ear_start(self: Pin<&mut Self>, kind: &QString);

        /// Plays the prompt again.
        #[qinvokable]
        #[cxx_name = "earPlay"]
        fn ear_play(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "earNext"]
        fn ear_next(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "earStop"]
        fn ear_stop(self: Pin<&mut Self>);

        /// An answer chosen by click instead of played (no input, LH-LRN-08); updates `earState`.
        #[qinvokable]
        #[cxx_name = "earAnswer"]
        fn ear_answer(self: Pin<&mut Self>, midi: i32);

        /// Backing-track beat map (LH-BIB-06); see `Core::backing_json`.
        #[qinvokable]
        #[cxx_name = "backingJson"]
        fn backing_json(self: &Self) -> QString;

        #[qinvokable]
        #[cxx_name = "backingAnalyse"]
        fn backing_analyse(self: Pin<&mut Self>);

        /// Bar 1 by `downbeat` beats, all bars by `shift_ms`, bar `bar` (>= 0) by `bar_ms`.
        #[qinvokable]
        #[cxx_name = "backingAdjust"]
        fn backing_adjust(
            self: Pin<&mut Self>,
            downbeat: i32,
            shift_ms: f64,
            bar: i32,
            bar_ms: f64,
        );

        /// Every bar half a beat later, or back (QA 27.10., J3).
        #[qinvokable]
        #[cxx_name = "backingHalfBeat"]
        fn backing_half_beat(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "backingClear"]
        fn backing_clear(self: Pin<&mut Self>);

        /// Opens the bundled first exercise.
        #[qinvokable]
        #[cxx_name = "openDemo"]
        fn open_demo(self: Pin<&mut Self>) -> bool;

        /// All user data as one JSON file (LH-FOR-03).
        #[qinvokable]
        #[cxx_name = "exportData"]
        fn export_data(self: Pin<&mut Self>, url: &QString);

        /// Loads a data file; `how` is "merge" or "replace".
        #[qinvokable]
        #[cxx_name = "importData"]
        fn import_data(self: Pin<&mut Self>, url: &QString, how: &QString);

        /// Progress report of one piece as HTML (LH-FOR-04).
        #[qinvokable]
        #[cxx_name = "exportReport"]
        fn export_report(self: Pin<&mut Self>, song_id: &QString, url: &QString);

        /// Shares a library piece as `.kontra` (LH-BIB-07); `audio` is "embed" or "link".
        #[qinvokable]
        #[cxx_name = "exportPackage"]
        fn export_package(self: Pin<&mut Self>, piece: &QString, url: &QString, audio: &QString);

        /// Licences for the About section (LH-NF-11).
        #[qinvokable]
        #[cxx_name = "licencesJson"]
        fn licences_json(self: &Self) -> QString;

        /// Prepared backing tracks and separated songs for Settings → Data (round 16).
        #[qinvokable]
        #[cxx_name = "cachesJson"]
        fn caches_json(self: &Self) -> QString;

        /// Deletes one entry of `cache` ("backing" or "stems").
        #[qinvokable]
        #[cxx_name = "forgetCached"]
        fn forget_cached(self: Pin<&mut Self>, cache: &QString, key: &QString);

        /// Empties `cache` ("backing" or "stems").
        #[qinvokable]
        #[cxx_name = "clearCache"]
        fn clear_cache(self: Pin<&mut Self>, cache: &QString);

        #[qinvokable]
        #[cxx_name = "dismissMessage"]
        fn dismiss_message(self: Pin<&mut Self>);

        /// Takes the input offered after a device was lost (LH-ALL-04).
        #[qinvokable]
        #[cxx_name = "useOfferedInput"]
        fn use_offered_input(self: Pin<&mut Self>);

        /// Answers the question about a new USB input: "instrument" or "microphone" (LH-EIN-02).
        #[qinvokable]
        #[cxx_name = "answerUsbAsk"]
        fn answer_usb_ask(self: Pin<&mut Self>, answer: &QString);

        /// The role of a USB input without output, "instrument" or "microphone" (LH-EIN-02).
        #[qinvokable]
        #[cxx_name = "setUsbRole"]
        fn set_usb_role(self: Pin<&mut Self>, id: &QString, role: &QString);

        /// Opens the failed or silent input again (LH-EIN-15).
        #[qinvokable]
        #[cxx_name = "retryInput"]
        fn retry_input(self: Pin<&mut Self>);

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

        /// How a note's technique was played (LH-BEW-06): Kante's keys, e.g. "slide", "played".
        #[qsignal]
        #[cxx_name = "noteTechnique"]
        fn note_technique(self: Pin<&mut Self>, index: i32, technique: QString, result: QString);

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

mod backing;
mod learn;
mod library;
mod loops;
mod settings;

use crate::args::AppArgs;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use kontra_appcore::{
    data_dir, file_url, local_path, Core, DensityLock, InstrumentChoice, LoopEdge, Opened,
    QaOptions, UiEvent,
};

/// `calibrationBeat` while no calibration runs.
const NO_BEAT: i32 = -99;

pub struct ControllerRust {
    texts: QString,
    loaded: bool,
    title: QString,
    artist: QString,
    notes_json: QString,
    bars_json: QString,
    staff_json: QString,
    string_names_json: QString,
    key_range_json: QString,
    duration: f64,
    position: f64,
    playing: bool,
    probing: bool,
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
    level_state: QString,
    suggest_laptop: bool,
    ear_state: QString,
    tuning_json: QString,
    tuner_cents: f64,
    tuner_active: bool,
    holding_index: i32,
    holding_text: QString,
    summary_json: QString,
    chain_json: QString,
    saved_loops_json: QString,
    target_speed: f64,
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
    instrument: QString,
    laptop_jack: bool,
    connection: QString,
    midi_port_name: QString,
    midi_ports_json: QString,
    metronome: bool,
    count_in: bool,
    reference: bool,
    low_string_on_top: bool,
    fretboard_shown: bool,
    play_marks: QString,
    motion: QString,
    strictness: QString,
    pocket: QString,
    pitch_cents: f64,
    latency_ms: f64,
    tuning: QString,
    profile_tuning_json: QString,
    note_choices_json: QString,
    a4_hz: f64,
    window_ms: f64,
    string_colors: QString,
    left_handed: bool,
    backing_volume: f64,
    metronome_volume: f64,
    tone_volume: f64,
    drums_volume: f64,
    bass_volume: f64,
    prompt_volume: f64,
    monitor: bool,
    monitor_volume: f64,
    monitor_low_db: f64,
    monitor_mid_db: f64,
    monitor_high_db: f64,
    monitor_compress: f64,
    monitor_warmth: f64,
    effects_json: QString,
    monitor_limit: bool,
    shaping_range_db: f64,
    density: i32,
    density_fixed: bool,
    density_suggestion: i32,
    density_text: QString,
    adaptive: bool,
    reacting_band: bool,
    call: bool,
    call_hidden: bool,
    simple_view: bool,
    streak: bool,
    calibrating: bool,
    calibration_beat: i32,
    calibration_text: QString,
    cable_measuring: bool,
    latency_note: QString,
    backing_name: QString,
    stems_json: QString,
    backing_loading: bool,
    song_search_json: QString,
    course_fetch_json: QString,
    course_folder_json: QString,
    run_kept: bool,
    transpose: i32,
    transpose_max: i32,
    message: QString,
    xruns: i32,
    offer_input: QString,
    input_state: QString,
    input_detail: QString,
    offer_text: QString,
    usb_ask: QString,
    error_text: QString,
    start_page: QString,
    start_course: QString,
    setup_done: bool,
    autoplay: bool,
    autoprobe: bool,
    autocalibrate: bool,
    show_mixer: bool,
    start_presenting: bool,
    start_section: QString,
    show_align: bool,
    start_loop: QString,
    screenshot_path: QString,
    screenshot_after: i32,
    quit_after: i32,
    qa_run: bool,
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
        // Ready before the settings page asks for them.
        core.relist_devices();
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
            key_range_json: q(&core.key_range_json()),
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
            mode: q(&s.mode),
            view: q(&s.view),
            style: q(&s.style),
            language: q(s.language.as_deref().unwrap_or("")),
            peak_db: -120.0,
            rms_db: -120.0,
            clipped: false,
            tuner_note: QString::default(),
            ear_state: QString::default(),
            level_state: QString::default(),
            suggest_laptop: false,
            tuner_midi: 0.0,
            tuning_json: q(&core.tuning_json()),
            tuner_cents: 0.0,
            tuner_active: false,
            holding_index: -1,
            holding_text: QString::default(),
            summary_json: QString::default(),
            chain_json: q("null"),
            saved_loops_json: q("[]"),
            target_speed: 1.0,
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
            fretless: s.instrument == kontra_appcore::Bass::Fretless,
            instrument: q(instrument_key(core.instrument_choice())),
            laptop_jack: s.input_kind == kontra_appcore::InputKind::LaptopJack || args.laptop_jack,
            connection: q(connection_key(s.input_kind)),
            midi_port_name: QString::default(),
            midi_ports_json: QString::default(),
            metronome: s.metronome,
            count_in: s.count_in_bars > 0,
            reference: s.reference_tones,
            low_string_on_top: s.low_string_on_top,
            fretboard_shown: s.fretboard_shown,
            play_marks: q(&s.play_marks),
            motion: q(&s.motion),
            strictness: q(&s.strictness),
            pocket: q(&s.pocket),
            pitch_cents: s.pitch_cents,
            latency_ms: s.latency_ms,
            tuning: q(&s.tuning),
            profile_tuning_json: q(&core.profile_tuning_json()),
            note_choices_json: q(&core.note_choices_json()),
            a4_hz: s.a4_hz,
            window_ms: s.window_ms,
            string_colors: q(&s.string_colors),
            left_handed: s.left_handed,
            backing_volume: s.backing_volume,
            metronome_volume: s.metronome_volume,
            tone_volume: s.tone_volume,
            drums_volume: s.drums_volume,
            bass_volume: s.bass_volume,
            prompt_volume: s.prompt_volume,
            monitor: s.monitor,
            monitor_volume: s.monitor_volume,
            monitor_low_db: s.monitor_low_db,
            monitor_mid_db: s.monitor_mid_db,
            monitor_high_db: s.monitor_high_db,
            monitor_compress: s.monitor_compress,
            monitor_warmth: s.monitor_warmth,
            effects_json: q(&core.effects_json()),
            monitor_limit: s.monitor_limit,
            shaping_range_db: kontra_appcore::Core::shaping_range_db(),
            density: 100,
            density_fixed: false,
            density_suggestion: 0,
            density_text: QString::default(),
            adaptive: s.adaptive,
            reacting_band: s.reacting_band,
            call: false,
            call_hidden: false,
            simple_view: s.simple_view,
            streak: s.streak,
            calibrating: false,
            calibration_beat: NO_BEAT,
            calibration_text: QString::default(),
            cable_measuring: false,
            latency_note: q(&core.latency_note()),
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
            setup_done: s.setup_done,
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
            core,
        };
        controller.song_search_json = q(&match &args.song_search {
            Some(query) => controller.core.song_search(query),
            None => controller.core.song_search_json(),
        });
        // A course is added, not played: the learn page shows what it is first (LH-LRN-14).
        if let Some(path) = args.open.as_ref().filter(|p| Core::is_course(p)) {
            controller.start_course = q(&path.to_string_lossy());
            controller.start_page = q("learn");
        } else if let Some(path) = args.open {
            controller.open_now(&path);
            // Opening may set the mode (plan exercises: run-through); an explicit --mode wins.
            if let Some(mode) = &args.mode {
                controller.core.set_mode(mode);
            }
            controller.mode = q(&controller.core.settings().mode);
            // An explicit --view wins over the view remembered for the piece (QA runs).
            if let Some(view) = &args.view {
                controller.core.set_view(view);
                controller.view = q(view);
            }
            if args.remove_bass {
                controller.core.remove_bass(true);
                controller.stems_json = q(&controller.core.stems_json());
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
                self.title = q(&self.core.loaded_title());
                self.artist = q(&self
                    .core
                    .arrangement()
                    .map_or(String::new(), |a| a.artist.clone()));
                self.notes_json = q(&self.core.notes_json());
                self.bars_json = q(&self.core.bars_json());
                self.staff_json = q(&self.core.staff_json());
                self.string_names_json = q(&self.core.string_names_json());
                self.key_range_json = q(&self.core.key_range_json());
                self.duration = self.core.duration_s();
                self.recent_json = q(&self.core.recent_json());
                self.backing_name = q(&self.core.backing_name());
                self.stems_json = q(&self.core.stems_json());
                self.transpose = self.core.transpose();
                self.saved_loops_json = q(&self.core.saved_loops_json());
                self.target_speed = self.core.target_speed();
                self.view = q(&self.core.view());
                self.tracks_json = q(&self.core.tracks_json());
                let (density, lock) = self.core.density();
                self.density = i32::from(density.percent());
                self.density_fixed = lock == DensityLock::Fixed;
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

/// The connection's key for QML: "line", "laptop" or "midi".
fn connection_key(kind: kontra_appcore::InputKind) -> &'static str {
    match kind {
        kontra_appcore::InputKind::Line => "line",
        kontra_appcore::InputKind::LaptopJack => "laptop",
        kontra_appcore::InputKind::Midi => "midi",
        kontra_appcore::InputKind::Microphone => "mic",
    }
}

/// The instrument choice's key for QML.
fn instrument_key(choice: InstrumentChoice) -> &'static str {
    match choice {
        InstrumentChoice::EBass => "ebass",
        InstrumentChoice::Double => "double",
        InstrumentChoice::Midi => "midi",
    }
}

impl qobject::Controller {
    fn core(self: Pin<&mut Self>) -> &mut Core {
        &mut self.rust_mut().get_mut().core
    }

    fn refresh_arrangement(mut self: Pin<&mut Self>) {
        let this = self.as_ref();
        let core = &this.rust().core;
        let title = core.loaded_title();
        let artist = core
            .arrangement()
            .map_or(String::new(), |a| a.artist.clone());
        let notes = core.notes_json();
        let bars = core.bars_json();
        let staff = core.staff_json();
        let names = core.string_names_json();
        let keys = core.key_range_json();
        let duration = core.duration_s();
        let recent = core.recent_json();
        let loaded = core.arrangement().is_some();
        let backing = core.backing_name();
        let transpose = core.transpose();
        let view = core.view();
        let tracks = core.tracks_json();
        // Loop and tempo as the core has them now (QA 27.10., J9: the last piece's showed on).
        let (loop_start, loop_end) = core.loop_seconds();
        let speed = core.speed();
        self.as_mut().set_loop_start(loop_start);
        self.as_mut().set_loop_end(loop_end);
        self.as_mut().set_speed(speed);
        self.as_mut().set_view(q(&view));
        self.as_mut().set_tracks_json(q(&tracks));
        self.as_mut().set_title(q(&title));
        self.as_mut().set_artist(q(&artist));
        self.as_mut().set_notes_json(q(&notes));
        self.as_mut().set_bars_json(q(&bars));
        self.as_mut().set_staff_json(q(&staff));
        self.as_mut().set_string_names_json(q(&names));
        self.as_mut().set_key_range_json(q(&keys));
        self.as_mut().set_duration(duration);
        self.as_mut().set_recent_json(q(&recent));
        self.as_mut().set_loaded(loaded);
        self.as_mut().set_backing_name(q(&backing));
        self.as_mut().show_stems();
        self.as_mut().set_transpose(transpose);
        self.as_mut().set_position(0.0);
        self.as_mut().set_summary_json(QString::default());
        self.as_mut().set_holding_index(-1);
        self.as_mut().refresh_density();
        self.as_mut().refresh_chain();
        self.as_mut().refresh_practice();
        self.as_mut().refresh_call();
        self.as_mut().arrangement_loaded();
    }

    /// Kept loops and target speed of the loaded piece.
    fn refresh_practice(mut self: Pin<&mut Self>) {
        let (loops, target) = {
            let this = self.as_ref();
            let core = &this.rust().core;
            (core.saved_loops_json(), core.target_speed())
        };
        self.as_mut().set_saved_loops_json(q(&loops));
        self.as_mut().set_target_speed(target);
    }

    fn loaded_goal(&self) -> QString {
        q(&self.rust().core.loaded_goal())
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
        // A start that resolves other devices forgets the measurements (review #202).
        self.as_mut().refresh_latency_note();
        let playing = self.as_ref().rust().core.is_playing();
        self.as_mut().set_playing(playing);
    }

    fn pause(mut self: Pin<&mut Self>) {
        self.as_mut().core().pause();
        self.as_mut().set_playing(false);
    }

    fn start_probe(mut self: Pin<&mut Self>) {
        self.as_mut().set_summary_json(QString::default());
        self.as_mut().set_holding_index(-1);
        self.as_mut().arrangement_loaded();
        let events = self.as_mut().core().start_probe();
        self.as_mut().apply(events);
        let (playing, probing) = {
            let this = self.as_ref();
            let core = &this.rust().core;
            (core.is_playing(), core.is_probing())
        };
        self.as_mut().set_probing(probing);
        self.as_mut().set_playing(playing);
    }

    fn loop_at(mut self: Pin<&mut Self>, start_s: f64, end_s: f64) -> bool {
        let Some((from, to)) = self.as_ref().rust().core.bars_at(start_s, end_s) else {
            return false;
        };
        let from = i32::try_from(from).unwrap_or(0);
        let to = i32::try_from(to).unwrap_or(0);
        self.as_mut().loop_bars(from, to);
        true
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
        // Leaving the workshop ends the building blocks: back to the bars chosen before them.
        if self.as_mut().core().set_mode(&mode) {
            self.as_mut().refresh_arrangement();
        }
        self.as_mut().after_loop();
        self.as_mut().set_mode(q(&mode));
        self.as_mut().reset_passes();
        self.as_mut().rewind();
    }

    fn change_view(mut self: Pin<&mut Self>, view: &QString) {
        let view = view.to_string();
        self.as_mut().core().set_view(&view);
        self.as_mut().set_view(q(&view));
    }

    fn loop_bars(mut self: Pin<&mut Self>, from: i32, to: i32) {
        let from = usize::try_from(from).unwrap_or(0);
        let to = usize::try_from(to).unwrap_or(0);
        let changed = self.as_mut().core().set_loop(from, to);
        if changed {
            self.as_mut().set_playing(false);
            self.as_mut().refresh_arrangement();
        }
        self.as_mut().refresh_density();
        self.as_mut().refresh_chain();
        self.as_mut().reset_passes();
        let (start, end) = self.as_ref().rust().core.loop_seconds();
        self.as_mut().set_loop_start(start);
        self.as_mut().set_loop_end(end);
    }

    fn loop_score_bar(mut self: Pin<&mut Self>, bar: i32) {
        let printed = u32::try_from(bar).unwrap_or(0);
        let Some(index) = self.as_ref().rust().core.score_bar_index(printed) else {
            return;
        };
        let index = i32::try_from(index).unwrap_or(0);
        self.as_mut().loop_bars(index, index);
    }

    fn loop_edge(mut self: Pin<&mut Self>, edge: &QString) {
        let edge = match edge.to_string().as_str() {
            "start" => LoopEdge::Start,
            "end" => LoopEdge::End,
            _ => return,
        };
        let position = *self.as_ref().position();
        if self.as_mut().core().set_loop_edge(edge, position) {
            self.as_mut().set_playing(false);
            self.as_mut().refresh_arrangement();
        }
        self.as_mut().refresh_density();
        self.as_mut().refresh_chain();
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
        if self.as_mut().core().clear_loop() {
            self.as_mut().set_playing(false);
            self.as_mut().refresh_arrangement();
        }
        self.as_mut().refresh_density();
        self.as_mut().refresh_chain();
        self.as_mut().set_loop_start(-1.0);
        self.as_mut().set_loop_end(-1.0);
    }

    fn tick(mut self: Pin<&mut Self>) {
        let events = self.as_mut().core().tick();
        self.as_mut().apply(events);
        if self.as_mut().core().devices_poll() {
            self.as_mut().show_devices();
        }
        if let Some(json) = self.as_mut().core().song_search_poll() {
            self.as_mut().set_song_search_json(q(&json));
        }
        if let Some(json) = self.as_mut().core().course_fetch_poll() {
            self.as_mut().set_course_fetch_json(q(&json));
        }
        if let Some(json) = self.as_mut().core().course_folder_poll() {
            self.as_mut().set_course_folder_json(q(&json));
        }
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
        let probing = self.as_ref().rust().core.is_probing();
        if probing != *self.as_ref().probing() {
            self.as_mut().set_probing(probing);
        }
        let beat = self
            .as_ref()
            .rust()
            .core
            .calibration_beat()
            .unwrap_or(NO_BEAT);
        if beat != *self.as_ref().calibration_beat() {
            self.as_mut().set_calibration_beat(beat);
        }
        let input = q(self.as_ref().rust().core.input_state().key());
        if input != *self.as_ref().input_state() {
            let detail = self.as_ref().rust().core.input_detail();
            self.as_mut().set_input_detail(q(&detail));
            self.as_mut().set_input_state(input);
        }
        let xruns = i32::try_from(self.as_ref().rust().core.xruns()).unwrap_or(i32::MAX);
        if xruns != *self.as_ref().xruns() {
            self.as_mut().set_xruns(xruns);
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
                UiEvent::Technique {
                    index,
                    technique,
                    result,
                } => {
                    self.as_mut().note_technique(
                        i32::try_from(index).unwrap_or(-1),
                        q(technique),
                        q(result),
                    );
                }
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
                    self.as_mut().refresh_call();
                    self.as_mut().pass_complete(clean, hit_rate);
                }
                UiEvent::SpeedChanged(speed) => self.as_mut().set_speed(speed),
                UiEvent::Finished { summary_json } => {
                    // The run's result shows in the library at once (QA 27.10., P7: "not played
                    // yet" until a restart).
                    let recent = self.as_ref().rust().core.recent_json();
                    self.as_mut().set_recent_json(q(&recent));
                    self.as_mut().set_summary_json(q(&summary_json));
                    self.as_mut().set_playing(false);
                    self.as_mut().refresh_call();
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
                UiEvent::Ear(state) => self.as_mut().set_ear_state(q(&state)),
                UiEvent::LevelAdvice {
                    state,
                    suggest_laptop,
                } => {
                    self.as_mut().set_level_state(q(state));
                    self.as_mut().set_suggest_laptop(suggest_laptop);
                }
                UiEvent::Message(text) => self.as_mut().set_message(q(&text)),
                UiEvent::Error(text) => self.as_mut().show_error(&text),
                UiEvent::InputOffer { id, text } => {
                    self.as_mut().set_offer_input(q(&id));
                    self.as_mut().set_offer_text(q(&text));
                }
                UiEvent::InputOfferEnded => {
                    self.as_mut().set_offer_input(QString::default());
                    self.as_mut().set_offer_text(QString::default());
                }
                UiEvent::UsbAsk(json) => self.as_mut().set_usb_ask(q(&json)),
                UiEvent::InputBack(text) => {
                    self.as_mut().set_offer_input(QString::default());
                    self.as_mut().set_offer_text(QString::default());
                    self.as_mut().set_error_text(QString::default());
                    self.as_mut().set_playing(false);
                    self.as_mut().set_message(q(&text));
                }
                // The input's state shows it (LH-EIN-15).
                UiEvent::DeviceLost(..) => {
                    self.as_mut().set_playing(false);
                    self.as_mut().set_tuner_active(false);
                }
                UiEvent::DensitySuggestion { percent, text } => {
                    self.as_mut().set_density_suggestion(i32::from(percent));
                    self.as_mut().set_density_text(q(&text));
                }
                UiEvent::EffectsChanged => self.as_mut().refresh_effects(),
                UiEvent::Stems(state) => self.as_mut().set_stems_json(q(&state)),
                UiEvent::BackingLoading(loading) => self.as_mut().set_backing_loading(loading),
                UiEvent::ChainStep { chain_json } => {
                    // A new run: the marks of the last step go.
                    self.as_mut().refresh_arrangement();
                    self.as_mut().after_loop();
                    self.as_mut().set_chain_json(q(&chain_json));
                    let playing = self.as_ref().rust().core.is_playing();
                    self.as_mut().set_playing(playing);
                }
                UiEvent::Calibrated { latency_ms, text } => {
                    if let Some(latency) = latency_ms {
                        self.as_mut().set_latency_ms(latency);
                    }
                    self.as_mut().set_calibration_text(q(&text));
                    self.as_mut().set_calibrating(false);
                    self.as_mut().set_cable_measuring(false);
                    let note = self.as_ref().rust().core.latency_note();
                    self.as_mut().set_latency_note(q(&note));
                }
            }
        }
    }

    fn start_tuner(mut self: Pin<&mut Self>) {
        self.as_mut().set_playing(false);
        let tuning = self.as_ref().rust().core.tuning_json();
        self.as_mut().set_tuning_json(q(&tuning));
        // A failed input shows as the input's state (LH-EIN-15), not as a second bar.
        let started = self.as_mut().core().start_tuner().is_ok();
        self.as_mut().set_tuner_active(started);
    }

    fn stop_tuner(mut self: Pin<&mut Self>) {
        self.as_mut().core().stop_tuner();
        self.as_mut().set_tuner_active(false);
        self.as_mut().set_tuner_note(QString::default());
    }

    /// Shared tail of opening: refresh views, ask for the track, or show the error.
    fn after_open(mut self: Pin<&mut Self>, result: Result<Opened, String>) -> bool {
        let ready = match result {
            Ok(Opened::Ready) => {
                self.as_mut().set_track_choices_json(q("[]"));
                self.as_mut().refresh_arrangement();
                // Opening may change the mode (plan exercises: run-through, round 12).
                let mode = self.as_ref().rust().core.settings().mode.clone();
                self.as_mut().set_mode(q(&mode));
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

    fn finish_setup(mut self: Pin<&mut Self>) {
        self.as_mut().core().stop_tuner();
        self.as_mut()
            .core()
            .update_settings(|s| s.setup_done = true);
        self.as_mut().set_tuner_active(false);
        self.as_mut().set_setup_done(true);
    }
}
