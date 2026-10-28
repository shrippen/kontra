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
        #[qproperty(f64, shaping_range_db, cxx_name = "shapingRangeDb")]
        /// Note density of the current section, 1..=3 (LH-MOD-06), and whether it is fixed.
        #[qproperty(i32, density)]
        #[qproperty(bool, density_fixed, cxx_name = "densityFixed")]
        /// Suggested density after a run (0: none) and the question to show.
        #[qproperty(i32, density_suggestion, cxx_name = "densitySuggestion")]
        #[qproperty(QString, density_text, cxx_name = "densityText")]
        #[qproperty(bool, adaptive)]
        /// The practice page without its rare controls (LH-ANZ-11).
        #[qproperty(bool, simple_view, cxx_name = "simpleView")]
        #[qproperty(bool, calibrating)]
        /// The calibration's click: −4..−1 count-in, 0..7 measured, −99 none.
        #[qproperty(i32, calibration_beat, cxx_name = "calibrationBeat")]
        #[qproperty(QString, calibration_text, cxx_name = "calibrationText")]
        #[qproperty(QString, backing_name, cxx_name = "backingName")]
        #[qproperty(QString, stems_json, cxx_name = "stemsJson")]
        /// Songsterr search: state, reason and hits as JSON (LH-BIB-11).
        #[qproperty(QString, song_search_json, cxx_name = "songSearchJson")]
        /// A course package fetched from an address, see `Core::course_fetch_poll`.
        #[qproperty(QString, course_fetch_json, cxx_name = "courseFetchJson")]
        /// While the arrangement is laid out again: the run goes on (a change of key), so the
        /// page keeps the marks.
        #[qproperty(bool, run_kept, cxx_name = "runKept")]
        /// Piece and backing moved by semitones (LH-WIE-02), −`transposeMax`..=`transposeMax`.
        #[qproperty(i32, transpose)]
        #[qproperty(i32, transpose_max, cxx_name = "transposeMax")]
        #[qproperty(QString, message)]
        #[qproperty(i32, xruns)]
        #[qproperty(QString, offer_input, cxx_name = "offerInput")]
        #[qproperty(QString, offer_text, cxx_name = "offerText")]
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

        #[qinvokable]
        #[cxx_name = "clearLoop"]
        fn clear_loop(self: Pin<&mut Self>);

        /// Note density 1..=3 for the current section; `lock` "fixed" stops suggestions.
        #[qinvokable]
        #[cxx_name = "changeDensity"]
        fn change_density(self: Pin<&mut Self>, level: i32, lock: &QString);

        /// Dismisses the density suggestion.
        #[qinvokable]
        #[cxx_name = "declineDensity"]
        fn decline_density(self: Pin<&mut Self>);

        /// Density suggestions on or off.
        #[qinvokable]
        #[cxx_name = "changeAdaptive"]
        fn change_adaptive(self: Pin<&mut Self>, on: bool);

        /// The simple practice page on or off (LH-ANZ-11).
        #[qinvokable]
        #[cxx_name = "changeSimpleView"]
        fn change_simple_view(self: Pin<&mut Self>, on: bool);

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

        /// Back to the unshaped bass.
        #[qinvokable]
        #[cxx_name = "resetShaping"]
        fn reset_shaping(self: Pin<&mut Self>);

        /// Latency calibration "play on the click" (LH-EIN-05).
        #[qinvokable]
        #[cxx_name = "startCalibration"]
        fn start_calibration(self: Pin<&mut Self>);

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

        /// Fretboard quiz (LH-LRN-02): "notes" or "intervals"; returns the question JSON.
        #[qinvokable]
        #[cxx_name = "quizStart"]
        fn quiz_start(self: Pin<&mut Self>, kind: &QString) -> QString;

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

        #[qinvokable]
        #[cxx_name = "dismissMessage"]
        fn dismiss_message(self: Pin<&mut Self>);

        /// Takes the input offered after a device was lost (LH-ALL-04).
        #[qinvokable]
        #[cxx_name = "useOfferedInput"]
        fn use_offered_input(self: Pin<&mut Self>);

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

use crate::args::AppArgs;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use kontra_appcore::{
    data_dir, Bass, Blocks, ChainOrder, Core, DataImport, Density, DensityLock, InstrumentChoice,
    LoopEdge, Opened, PackageAudio, QaOptions, UiEvent,
};
use std::path::PathBuf;

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
    shaping_range_db: f64,
    density: i32,
    density_fixed: bool,
    density_suggestion: i32,
    density_text: QString,
    adaptive: bool,
    simple_view: bool,
    calibrating: bool,
    calibration_beat: i32,
    calibration_text: QString,
    backing_name: QString,
    stems_json: QString,
    song_search_json: QString,
    course_fetch_json: QString,
    run_kept: bool,
    transpose: i32,
    transpose_max: i32,
    message: QString,
    xruns: i32,
    offer_input: QString,
    offer_text: QString,
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
        if let Some(answer) = &args.songsterr_answer {
            core.fake_song_search(answer);
        }
        if let Some(answer) = &args.course_answer {
            core.fake_course_web(answer);
        }
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
            shaping_range_db: kontra_appcore::Core::shaping_range_db(),
            density: 3,
            density_fixed: false,
            density_suggestion: 0,
            density_text: QString::default(),
            adaptive: s.adaptive,
            simple_view: s.simple_view,
            calibrating: false,
            calibration_beat: NO_BEAT,
            calibration_text: QString::default(),
            backing_name: QString::default(),
            stems_json: QString::default(),
            song_search_json: QString::default(),
            course_fetch_json: QString::default(),
            run_kept: false,
            transpose: 0,
            transpose_max: kontra_appcore::Core::transpose_max(),
            message: QString::default(),
            xruns: 0,
            offer_input: QString::default(),
            offer_text: QString::default(),
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
                self.view = q(&self.core.view());
                self.tracks_json = q(&self.core.tracks_json());
                let (density, lock) = self.core.density();
                self.density = i32::from(density.level());
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
        self.as_mut().arrangement_loaded();
    }

    /// Density of the current section; a pending suggestion goes (it was for another one).
    fn refresh_density(mut self: Pin<&mut Self>) {
        let (density, lock) = self.as_ref().rust().core.density();
        self.as_mut().set_density(i32::from(density.level()));
        self.as_mut().set_density_fixed(lock == DensityLock::Fixed);
        self.as_mut().set_density_suggestion(0);
    }

    fn refresh_chain(mut self: Pin<&mut Self>) {
        let chain = self.as_ref().rust().core.chain_json();
        self.as_mut().set_chain_json(q(&chain));
    }

    fn start_chain(mut self: Pin<&mut Self>, blocks: &QString, order: &QString) {
        let order = match order.to_string().as_str() {
            "backward" => ChainOrder::Backward,
            _ => ChainOrder::Forward,
        };
        // "off", "2", "4" (bars per block) or "phrases".
        let blocks = match blocks.to_string().as_str() {
            "phrases" => Some(Blocks::Phrases),
            size => size.parse().ok().filter(|n| *n > 0).map(Blocks::Bars),
        };
        let changed = match blocks {
            Some(blocks) => self.as_mut().core().start_chain(blocks, order),
            None => self.as_mut().core().stop_chain(),
        };
        if changed {
            self.as_mut().set_playing(false);
            self.as_mut().refresh_arrangement();
        }
        self.as_mut().after_loop();
    }

    /// The loop moved: density, building blocks, pass count and loop times follow.
    fn after_loop(mut self: Pin<&mut Self>) {
        self.as_mut().refresh_density();
        self.as_mut().refresh_chain();
        self.as_mut().reset_passes();
        let (start, end) = self.as_ref().rust().core.loop_seconds();
        self.as_mut().set_loop_start(start);
        self.as_mut().set_loop_end(end);
    }

    fn change_density(mut self: Pin<&mut Self>, level: i32, lock: &QString) {
        let density = Density::from_level(u8::try_from(level.clamp(1, 3)).unwrap_or(3));
        let lock = if lock.to_string() == "fixed" {
            DensityLock::Fixed
        } else {
            DensityLock::Free
        };
        self.as_mut().core().set_density(density, lock);
        self.as_mut().set_playing(false);
        self.as_mut().refresh_arrangement();
    }

    fn decline_density(mut self: Pin<&mut Self>) {
        self.as_mut().set_density_suggestion(0);
    }

    fn change_simple_view(mut self: Pin<&mut Self>, on: bool) {
        self.as_mut().core().update_settings(|s| s.simple_view = on);
        self.as_mut().set_simple_view(on);
    }

    fn loaded_goal(&self) -> QString {
        q(&self.rust().core.loaded_goal())
    }

    fn change_adaptive(mut self: Pin<&mut Self>, on: bool) {
        self.as_mut().core().update_settings(|s| s.adaptive = on);
        self.as_mut().set_adaptive(on);
        if !on {
            self.as_mut().set_density_suggestion(0);
        }
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
        // Note names follow the language (H in German).
        self.as_mut().refresh_tuning();
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
        if let Some(json) = self.as_mut().core().song_search_poll() {
            self.as_mut().set_song_search_json(q(&json));
        }
        if let Some(json) = self.as_mut().core().course_fetch_poll() {
            self.as_mut().set_course_fetch_json(q(&json));
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
                UiEvent::InputBack(text) => {
                    self.as_mut().set_offer_input(QString::default());
                    self.as_mut().set_offer_text(QString::default());
                    self.as_mut().set_error_text(QString::default());
                    self.as_mut().set_playing(false);
                    self.as_mut().set_message(q(&text));
                }
                UiEvent::DeviceLost(text) => {
                    self.as_mut().set_playing(false);
                    self.as_mut().set_tuner_active(false);
                    self.as_mut().show_error(&text);
                }
                UiEvent::DensitySuggestion { level, text } => {
                    self.as_mut().set_density_suggestion(i32::from(level));
                    self.as_mut().set_density_text(q(&text));
                }
                UiEvent::Stems(state) => self.as_mut().set_stems_json(q(&state)),
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

    fn change_connection(mut self: Pin<&mut Self>, key: &QString) {
        let kind = match key.to_string().as_str() {
            "laptop" => kontra_appcore::InputKind::LaptopJack,
            "midi" => kontra_appcore::InputKind::Midi,
            "mic" => kontra_appcore::InputKind::Microphone,
            _ => kontra_appcore::InputKind::Line,
        };
        let result = self.as_mut().core().retune(|s| s.input_kind = kind);
        self.as_mut().refresh_instrument();
        self.as_mut().refresh_devices();
        self.as_mut().after_retune(result);
    }

    fn refresh_devices(mut self: Pin<&mut Self>) {
        let midi = self.as_ref().rust().core.midi_port_name();
        self.as_mut().set_midi_port_name(q(&midi));
        let ports = self.as_ref().rust().core.midi_ports_json();
        self.as_mut().set_midi_ports_json(q(&ports));
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

    fn choose_midi(mut self: Pin<&mut Self>, name: &QString) {
        self.as_mut().core().choose_midi(&name.to_string());
        self.as_mut().refresh_devices();
    }

    fn choose_output(mut self: Pin<&mut Self>, id: &QString) {
        let id = id.to_string();
        let value = (!id.is_empty()).then(|| id.clone());
        self.as_mut()
            .core()
            .reconfigure(|s| s.output_device = value);
        self.as_mut().set_output_device(q(&id));
    }

    fn change_instrument(mut self: Pin<&mut Self>, strings: i32, fretless: bool) {
        let result = self.as_mut().core().retune(|s| {
            let max = i32::from(s.instrument.strings_max());
            s.strings = u8::try_from(strings.clamp(4, max)).unwrap_or(4);
            if s.instrument != Bass::Double {
                s.instrument = if fretless {
                    Bass::Fretless
                } else {
                    Bass::Fretted
                };
            }
        });
        self.as_mut().refresh_instrument();
        self.as_mut().after_retune(result);
    }

    fn choose_instrument(mut self: Pin<&mut Self>, key: &QString) {
        let choice = match key.to_string().as_str() {
            "double" => InstrumentChoice::Double,
            "midi" => InstrumentChoice::Midi,
            _ => InstrumentChoice::EBass,
        };
        let result = self.as_mut().core().choose_instrument(choice);
        self.as_mut().refresh_instrument();
        self.as_mut().refresh_devices();
        self.as_mut().after_retune(result);
    }

    /// Instrument and input as stored, for setup and settings.
    fn refresh_instrument(mut self: Pin<&mut Self>) {
        let (s, choice) = {
            let this = self.as_ref();
            let core = &this.rust().core;
            (core.settings().clone(), core.instrument_choice())
        };
        self.as_mut().set_strings(i32::from(s.strings));
        self.as_mut().set_fretless(s.instrument == Bass::Fretless);
        self.as_mut().set_instrument(q(instrument_key(choice)));
        self.as_mut()
            .set_laptop_jack(s.input_kind == kontra_appcore::InputKind::LaptopJack);
        self.as_mut()
            .set_connection(q(connection_key(s.input_kind)));
    }

    /// After an instrument change: new strings everywhere, or the error.
    fn after_retune(mut self: Pin<&mut Self>, result: Result<(), String>) {
        self.as_mut().set_playing(false);
        self.as_mut().refresh_tuning();
        if self.as_ref().rust().core.arrangement().is_some() {
            self.as_mut().refresh_arrangement();
        }
        if let Err(e) = result {
            self.as_mut().show_error(&e);
        }
    }

    fn refresh_tuning(mut self: Pin<&mut Self>) {
        let (tuning, profile, guided, notes) = {
            let this = self.as_ref();
            let core = &this.rust().core;
            (
                core.settings().tuning.clone(),
                core.profile_tuning_json(),
                core.tuning_json(),
                core.note_choices_json(),
            )
        };
        self.as_mut().set_tuning(q(&tuning));
        self.as_mut().set_profile_tuning_json(q(&profile));
        self.as_mut().set_tuning_json(q(&guided));
        self.as_mut().set_note_choices_json(q(&notes));
    }

    fn change_tuning(mut self: Pin<&mut Self>, name: &QString) {
        let result = self.as_mut().core().set_tuning(&name.to_string());
        self.as_mut().after_retune(result);
    }

    fn change_custom_string(mut self: Pin<&mut Self>, string: i32, midi: i32) {
        let (Ok(string), Ok(midi)) = (usize::try_from(string), u8::try_from(midi)) else {
            return;
        };
        let result = self.as_mut().core().set_custom_string(string, midi);
        self.as_mut().after_retune(result);
    }

    fn change_reference(mut self: Pin<&mut Self>, a4_hz: f64) {
        let result = self.as_mut().core().set_reference(a4_hz);
        let kept = self.as_ref().rust().core.settings().a4_hz;
        self.as_mut().set_a4_hz(kept);
        self.as_mut().after_retune(result.map(|_| ()));
    }

    fn change_window(mut self: Pin<&mut Self>, window_ms: f64) {
        let kept = self.as_mut().core().set_window(window_ms);
        self.as_mut().set_window_ms(kept);
        self.as_mut().set_strictness(q("custom"));
        self.as_mut().set_playing(false);
    }

    fn change_string_colors(mut self: Pin<&mut Self>, preset: &QString) {
        let preset = preset.to_string();
        if !["kante", "mono", "bright"].contains(&preset.as_str()) {
            return;
        }
        let value = q(&preset);
        // Display only: a running take goes on.
        self.as_mut()
            .core()
            .update_settings(|s| s.string_colors = preset);
        self.as_mut().set_string_colors(value);
    }

    fn change_left_handed(mut self: Pin<&mut Self>, on: bool) {
        self.as_mut().core().update_settings(|s| s.left_handed = on);
        self.as_mut().set_left_handed(on);
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

    fn import_files(mut self: Pin<&mut Self>, urls_json: &QString) {
        let urls: Vec<String> = serde_json::from_str(&urls_json.to_string()).unwrap_or_default();
        let files: Vec<PathBuf> = urls.iter().map(|u| local_path(u)).collect();
        let (added, failed) = self.as_mut().core().import_files(&files);
        let (text, recent) = {
            let this = self.as_ref();
            let core = &this.rust().core;
            let mut text = core.tr("library.imported", &[("n", added.to_string())]);
            for (name, reason) in &failed {
                text.push_str(&format!("\n{name}: {reason}"));
            }
            (text, core.recent_json())
        };
        self.as_mut().set_recent_json(q(&recent));
        self.as_mut().set_message(q(&text));
    }

    fn search_url(&self, site: &QString, query: &QString) -> QString {
        q(&self
            .rust()
            .core
            .search_url(&site.to_string(), &query.to_string()))
    }

    fn downloads_json(&self) -> QString {
        q(&self.rust().core.downloads_json())
    }

    fn song_search(mut self: Pin<&mut Self>, query: &QString) {
        let json = self.as_mut().core().song_search(&query.to_string());
        self.as_mut().set_song_search_json(q(&json));
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

    fn setlist_add(mut self: Pin<&mut Self>, name: &QString, path: &QString) {
        let path = PathBuf::from(path.to_string());
        self.as_mut().core().setlist_add(&name.to_string(), &path);
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

    fn licences_json(&self) -> QString {
        q(&self.rust().core.licences_json())
    }

    fn progress_json(&self) -> QString {
        q(&self.rust().core.progress_json())
    }

    fn home_json(&self) -> QString {
        q(&self.rust().core.home_json())
    }

    fn change_scoring(mut self: Pin<&mut Self>, strictness: &QString, pitch_cents: f64) {
        let strictness = strictness.to_string();
        if !["auto", "loose", "normal", "strict", "custom"].contains(&strictness.as_str()) {
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

    fn change_pocket(mut self: Pin<&mut Self>, pocket: &QString) {
        let pocket = pocket.to_string();
        if !["free", "onbeat", "laidback", "pushed"].contains(&pocket.as_str()) {
            return;
        }
        let value = q(&pocket);
        // Read when the run ends: a running take keeps going.
        self.as_mut().core().update_settings(|s| s.pocket = pocket);
        self.as_mut().set_pocket(value);
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
            "drums" => {
                self.as_mut().set_drums_volume(value);
                self.as_mut().core().remix(|s| s.drums_volume = value)
            }
            "bass" => {
                self.as_mut().set_bass_volume(value);
                self.as_mut().core().remix(|s| s.bass_volume = value)
            }
            "prompt" => {
                self.as_mut().set_prompt_volume(value);
                self.as_mut().core().remix(|s| s.prompt_volume = value)
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

    fn change_shaping(mut self: Pin<&mut Self>, band: &QString, value: f64) {
        let range = kontra_appcore::Core::shaping_range_db();
        let db = value.clamp(-range, range);
        let hint = match band.to_string().as_str() {
            "low" => {
                self.as_mut().set_monitor_low_db(db);
                self.as_mut().core().remix(|s| s.monitor_low_db = db)
            }
            "mid" => {
                self.as_mut().set_monitor_mid_db(db);
                self.as_mut().core().remix(|s| s.monitor_mid_db = db)
            }
            "high" => {
                self.as_mut().set_monitor_high_db(db);
                self.as_mut().core().remix(|s| s.monitor_high_db = db)
            }
            "compress" => {
                let amount = value.clamp(0.0, 1.0);
                self.as_mut().set_monitor_compress(amount);
                self.as_mut().core().remix(|s| s.monitor_compress = amount)
            }
            "warmth" => {
                let amount = value.clamp(0.0, 1.0);
                self.as_mut().set_monitor_warmth(amount);
                self.as_mut().core().remix(|s| s.monitor_warmth = amount)
            }
            _ => None,
        };
        if let Some(hint) = hint {
            self.as_mut().set_message(q(&hint));
        }
    }

    fn reset_shaping(mut self: Pin<&mut Self>) {
        self.as_mut().set_monitor_low_db(0.0);
        self.as_mut().set_monitor_mid_db(0.0);
        self.as_mut().set_monitor_high_db(0.0);
        self.as_mut().set_monitor_compress(0.0);
        self.as_mut().set_monitor_warmth(0.0);
        let hint = self.as_mut().core().remix(|s| {
            s.monitor_low_db = 0.0;
            s.monitor_mid_db = 0.0;
            s.monitor_high_db = 0.0;
            s.monitor_compress = 0.0;
            s.monitor_warmth = 0.0;
        });
        if let Some(hint) = hint {
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

    fn change_transpose(mut self: Pin<&mut Self>, semitones: i32) {
        if let Err(text) = self.as_mut().core().set_transpose(semitones) {
            self.as_mut().show_error(&text);
        }
        // The run goes on in the new key: what waits and where it is stay as they are.
        let kept = self.as_ref().rust().core.has_run();
        let holding = *self.as_ref().holding_index();
        self.as_mut().set_run_kept(kept);
        self.as_mut().refresh_arrangement();
        self.as_mut().set_run_kept(false);
        if kept {
            let position = self.as_ref().rust().core.position_s();
            self.as_mut().set_position(position);
            // The note waited for is the same, its name and fret are the new key's.
            let text = usize::try_from(holding)
                .ok()
                .and_then(|index| self.as_ref().rust().core.wait_text(index, None));
            if let Some(text) = text {
                self.as_mut().set_holding_index(holding);
                self.as_mut().set_holding_text(q(&text));
            }
        }
    }

    fn choose_backing(mut self: Pin<&mut Self>, url: &QString) {
        let url = url.to_string();
        let path = (!url.is_empty()).then(|| local_path(&url));
        self.as_mut().core().set_backing(path, 0.0);
        let name = self.as_ref().rust().core.backing_name();
        self.as_mut().set_backing_name(q(&name));
        self.as_mut().show_stems();
        self.as_mut().set_playing(false);
    }

    fn remove_bass(mut self: Pin<&mut Self>, on: bool) {
        self.as_mut().core().remove_bass(on);
        self.as_mut().show_stems();
        self.as_mut().set_playing(false);
    }

    fn cancel_stems(mut self: Pin<&mut Self>) {
        self.as_mut().core().cancel_stems();
        self.as_mut().show_stems();
    }

    fn forget_stems(mut self: Pin<&mut Self>) {
        self.as_mut().core().forget_stems();
        self.as_mut().show_stems();
        self.as_mut().set_playing(false);
    }

    fn refresh_stems(mut self: Pin<&mut Self>) {
        self.as_mut().core().recheck_stems();
        self.as_mut().show_stems();
    }

    fn show_stems(mut self: Pin<&mut Self>) {
        let state = self.as_mut().core().stems_json();
        self.as_mut().set_stems_json(q(&state));
    }

    fn finish_setup(mut self: Pin<&mut Self>) {
        self.as_mut().core().stop_tuner();
        self.as_mut()
            .core()
            .update_settings(|s| s.setup_done = true);
        self.as_mut().set_tuner_active(false);
        self.as_mut().set_setup_done(true);
    }

    fn learn_json(&self) -> QString {
        q(&self.rust().core.learn_json())
    }

    fn lesson_json(&self, id: &QString) -> QString {
        q(&self.rust().core.lesson_json(&id.to_string()))
    }

    fn lesson_seen(mut self: Pin<&mut Self>, id: &QString, page: i32) {
        let page = usize::try_from(page).unwrap_or(0);
        self.as_mut().core().lesson_seen(&id.to_string(), page);
    }

    fn lesson_ear(mut self: Pin<&mut Self>, id: &QString, page: i32, block: i32) -> bool {
        let (page, block) = (usize::try_from(page), usize::try_from(block));
        let (Ok(page), Ok(block)) = (page, block) else {
            return false;
        };
        self.as_mut().set_playing(false);
        let state = self
            .as_mut()
            .core()
            .lesson_ear(&id.to_string(), page, block);
        let started = state.is_ok();
        self.as_mut().show_ear(state);
        started
    }

    fn term_ear(mut self: Pin<&mut Self>, id: &QString, block: i32) {
        let Ok(block) = usize::try_from(block) else {
            return;
        };
        self.as_mut().set_playing(false);
        let state = self.as_mut().core().term_ear(&id.to_string(), block);
        self.as_mut().show_ear(state);
    }

    /// The ear state of a lesson or glossary block, or its error ("" for none).
    fn show_ear(mut self: Pin<&mut Self>, state: Result<String, String>) {
        match state {
            Ok(state) => self.as_mut().set_ear_state(q(&state)),
            Err(text) if text.is_empty() => {}
            Err(text) => self.as_mut().show_error(&text),
        }
    }

    fn lesson_answer(
        mut self: Pin<&mut Self>,
        id: &QString,
        page: i32,
        block: i32,
        option: i32,
    ) -> bool {
        let at = (
            usize::try_from(page),
            usize::try_from(block),
            usize::try_from(option),
        );
        let (Ok(page), Ok(block), Ok(option)) = at else {
            return false;
        };
        self.as_mut()
            .core()
            .lesson_answer(&id.to_string(), page, block, option)
    }

    fn review_json(&self) -> QString {
        q(&self.rust().core.review_json())
    }

    fn chapter_check_json(&self, chapter: &QString) -> QString {
        q(&self.rust().core.chapter_check_json(&chapter.to_string()))
    }

    fn chapter_pass(mut self: Pin<&mut Self>, chapter: &QString, since: f64) -> bool {
        // Seconds since 1970 from JSON; a negative or broken value asks for nothing earlier.
        let since = if since.is_finite() && since > 0.0 {
            since as u64
        } else {
            0
        };
        self.as_mut()
            .core()
            .chapter_pass(&chapter.to_string(), since)
    }

    fn course_preview(&self, url: &QString) -> QString {
        let path = local_path(&url.to_string());
        q(&self.rust().core.course_preview(&path))
    }

    fn course_add(mut self: Pin<&mut Self>, url: &QString) -> QString {
        let path = local_path(&url.to_string());
        q(&self.as_mut().core().course_add(&path))
    }

    fn course_fetch(mut self: Pin<&mut Self>, url: &QString) {
        let json = self.as_mut().core().course_fetch(&url.to_string());
        self.as_mut().set_course_fetch_json(q(&json));
    }

    fn lesson_video(&self, id: &QString, page: i32, block: i32) -> QString {
        let (Ok(page), Ok(block)) = (usize::try_from(page), usize::try_from(block)) else {
            return QString::default();
        };
        q(&self.rust().core.lesson_video(&id.to_string(), page, block))
    }

    fn course_remove(mut self: Pin<&mut Self>, id: &QString) -> QString {
        q(&self.as_mut().core().course_remove(&id.to_string()))
    }

    fn choose_course(mut self: Pin<&mut Self>, id: &QString) {
        self.as_mut().core().choose_course(&id.to_string());
    }

    fn lesson_exercise(mut self: Pin<&mut Self>, id: &QString, page: i32, block: i32) -> bool {
        let (page, block) = (usize::try_from(page), usize::try_from(block));
        let (Ok(page), Ok(block)) = (page, block) else {
            return false;
        };
        let path = self
            .as_mut()
            .core()
            .lesson_exercise(&id.to_string(), page, block);
        self.as_mut().open_exercise(path)
    }

    fn finale_exercise(mut self: Pin<&mut Self>, chapter: &QString) -> bool {
        let path = self.as_mut().core().finale_exercise(&chapter.to_string());
        self.as_mut().open_exercise(path)
    }

    /// Leaves ear training and opens a lesson's exercise like any file.
    fn open_exercise(mut self: Pin<&mut Self>, path: Result<std::path::PathBuf, String>) -> bool {
        match path {
            Ok(path) => {
                self.as_mut().ear_stop();
                let result = self.as_mut().core().open(&path);
                self.as_mut().after_open(result)
            }
            Err(text) => {
                if !text.is_empty() {
                    self.as_mut().show_error(&text);
                }
                false
            }
        }
    }

    fn glossary_json(&self, id: &QString) -> QString {
        q(&self.rust().core.glossary_json(&id.to_string()))
    }

    fn glossary_list_json(&self) -> QString {
        q(&self.rust().core.glossary_list_json())
    }

    fn term_json(&self, topic: &QString) -> QString {
        q(&self.rust().core.term_json(&topic.to_string()))
    }

    fn quiz_start(mut self: Pin<&mut Self>, kind: &QString) -> QString {
        q(&self.as_mut().core().quiz_start(&kind.to_string()))
    }

    fn quiz_next(mut self: Pin<&mut Self>) -> QString {
        q(&self.as_mut().core().quiz_next())
    }

    fn quiz_answer(mut self: Pin<&mut Self>, midi: f64) -> bool {
        self.as_mut().core().quiz_answer(midi)
    }

    fn ear_start(mut self: Pin<&mut Self>, kind: &QString) {
        self.as_mut().set_playing(false);
        match self.as_mut().core().ear_start(&kind.to_string()) {
            Ok(state) => self.as_mut().set_ear_state(q(&state)),
            Err(text) => self.as_mut().show_error(&text),
        }
    }

    fn ear_play(mut self: Pin<&mut Self>) {
        let state = self.as_mut().core().ear_play();
        self.as_mut().set_ear_state(q(&state));
    }

    fn ear_next(mut self: Pin<&mut Self>) {
        let state = self.as_mut().core().ear_next();
        self.as_mut().set_ear_state(q(&state));
    }

    fn ear_stop(mut self: Pin<&mut Self>) {
        self.as_mut().core().ear_stop();
        self.as_mut().set_ear_state(QString::default());
    }

    fn ear_answer(mut self: Pin<&mut Self>, midi: i32) {
        let state = self.as_mut().core().ear_answer(f64::from(midi));
        self.as_mut().set_ear_state(q(&state));
    }

    fn quiz_json(&self) -> QString {
        q(&self.rust().core.quiz_json())
    }

    fn backing_json(&self) -> QString {
        q(&self.rust().core.backing_json())
    }

    fn backing_analyse(mut self: Pin<&mut Self>) {
        let result = self.as_mut().core().backing_analyse();
        self.as_mut().after_retune(result);
    }

    fn backing_adjust(
        mut self: Pin<&mut Self>,
        downbeat: i32,
        shift_ms: f64,
        bar: i32,
        bar_ms: f64,
    ) {
        let bar = usize::try_from(bar).ok().map(|b| (b, bar_ms));
        let result = self.as_mut().core().backing_adjust(downbeat, shift_ms, bar);
        self.as_mut().after_retune(result);
    }

    fn backing_half_beat(mut self: Pin<&mut Self>) {
        let result = self.as_mut().core().backing_half_beat();
        self.as_mut().after_retune(result);
    }

    fn backing_clear(mut self: Pin<&mut Self>) {
        let result = self.as_mut().core().backing_clear();
        self.as_mut().after_retune(result);
    }

    fn exercises_json(&self) -> QString {
        q(&self.rust().core.bundled_json())
    }

    fn generate_exercise(
        mut self: Pin<&mut Self>,
        kind: &QString,
        variant: &QString,
        rhythm: &QString,
        key: i32,
        bpm: f64,
        judging: &QString,
    ) -> bool {
        let Some(spec) = Core::exercise_spec(
            &kind.to_string(),
            &variant.to_string(),
            &rhythm.to_string(),
            u8::try_from(key.rem_euclid(12)).unwrap_or(0),
            bpm,
            &judging.to_string(),
        ) else {
            return false;
        };
        match self.as_mut().core().generate_exercise(&spec) {
            Ok(path) => self.as_mut().open_file(&q(&path.to_string_lossy())),
            Err(e) => {
                self.as_mut().show_error(&e);
                false
            }
        }
    }

    fn open_demo(mut self: Pin<&mut Self>) -> bool {
        let path = Core::demo_path();
        self.as_mut().open_file(&q(&path.to_string_lossy()))
    }

    fn export_data(mut self: Pin<&mut Self>, url: &QString) {
        let path = local_path(&url.to_string());
        match self.as_ref().rust().core.export_data(&path) {
            Ok(text) => self.as_mut().set_message(q(&text)),
            Err(e) => self.as_mut().show_error(&e),
        }
    }

    fn import_data(mut self: Pin<&mut Self>, url: &QString, how: &QString) {
        let path = local_path(&url.to_string());
        let how = match how.to_string().as_str() {
            "replace" => DataImport::Replace,
            _ => DataImport::Merge,
        };
        match self.as_mut().core().import_data(&path, how) {
            Ok(text) => {
                self.as_mut().sync_settings();
                self.as_mut().set_message(q(&text));
            }
            Err(e) => self.as_mut().show_error(&e),
        }
    }

    /// Every settings property and list from the core, after the data changed underneath.
    fn sync_settings(mut self: Pin<&mut Self>) {
        let (s, texts, recent, tuning, keys, choice) = {
            let this = self.as_ref();
            let core = &this.rust().core;
            (
                core.settings().clone(),
                core.texts_json(),
                core.recent_json(),
                core.tuning_json(),
                core.key_range_json(),
                core.instrument_choice(),
            )
        };
        self.as_mut().set_texts(q(&texts));
        self.as_mut().set_recent_json(q(&recent));
        self.as_mut().set_tuning_json(q(&tuning));
        self.as_mut().set_key_range_json(q(&keys));
        self.as_mut().refresh_setlists();
        self.as_mut().set_mode(q(&s.mode));
        self.as_mut().set_view(q(&s.view));
        self.as_mut().set_style(q(&s.style));
        self.as_mut()
            .set_language(q(s.language.as_deref().unwrap_or("")));
        self.as_mut()
            .set_input_device(q(s.input_device.as_deref().unwrap_or("")));
        self.as_mut()
            .set_output_device(q(s.output_device.as_deref().unwrap_or("")));
        self.as_mut().set_strings(i32::from(s.strings));
        self.as_mut().set_fretless(s.instrument == Bass::Fretless);
        self.as_mut().set_instrument(q(instrument_key(choice)));
        self.as_mut()
            .set_laptop_jack(s.input_kind == kontra_appcore::InputKind::LaptopJack);
        self.as_mut()
            .set_connection(q(connection_key(s.input_kind)));
        self.as_mut().set_metronome(s.metronome);
        self.as_mut().set_count_in(s.count_in_bars > 0);
        self.as_mut().set_reference(s.reference_tones);
        self.as_mut().set_low_string_on_top(s.low_string_on_top);
        self.as_mut().set_play_marks(q(&s.play_marks));
        self.as_mut().set_motion(q(&s.motion));
        self.as_mut().set_strictness(q(&s.strictness));
        self.as_mut().set_pocket(q(&s.pocket));
        self.as_mut().set_pitch_cents(s.pitch_cents);
        self.as_mut().set_latency_ms(s.latency_ms);
        self.as_mut().set_backing_volume(s.backing_volume);
        self.as_mut().set_metronome_volume(s.metronome_volume);
        self.as_mut().set_tone_volume(s.tone_volume);
        self.as_mut().set_drums_volume(s.drums_volume);
        self.as_mut().set_bass_volume(s.bass_volume);
        self.as_mut().set_prompt_volume(s.prompt_volume);
        self.as_mut().set_adaptive(s.adaptive);
        self.as_mut().set_simple_view(s.simple_view);
        self.as_mut().set_tuning(q(&s.tuning));
        self.as_mut().set_a4_hz(s.a4_hz);
        self.as_mut().set_window_ms(s.window_ms);
        self.as_mut().set_string_colors(q(&s.string_colors));
        self.as_mut().set_left_handed(s.left_handed);
        self.as_mut().set_monitor(s.monitor);
        self.as_mut().set_monitor_volume(s.monitor_volume);
        self.as_mut().set_monitor_low_db(s.monitor_low_db);
        self.as_mut().set_monitor_mid_db(s.monitor_mid_db);
        self.as_mut().set_monitor_high_db(s.monitor_high_db);
        self.as_mut().set_monitor_compress(s.monitor_compress);
        self.as_mut().set_monitor_warmth(s.monitor_warmth);
        self.as_mut().set_setup_done(s.setup_done);
        self.as_mut().set_playing(false);
    }

    fn export_report(mut self: Pin<&mut Self>, song_id: &QString, url: &QString) {
        let path = local_path(&url.to_string());
        let result = self
            .as_ref()
            .rust()
            .core
            .export_report(&song_id.to_string(), &path);
        match result {
            Ok(text) => self.as_mut().set_message(q(&text)),
            Err(e) => self.as_mut().show_error(&e),
        }
    }

    fn export_package(mut self: Pin<&mut Self>, piece: &QString, url: &QString, audio: &QString) {
        let piece = local_path(&piece.to_string());
        let out = local_path(&url.to_string());
        let audio = match audio.to_string().as_str() {
            "embed" => PackageAudio::Embed,
            _ => PackageAudio::Link,
        };
        let result = self
            .as_ref()
            .rust()
            .core
            .export_package(&piece, &out, audio);
        match result {
            Ok(text) => self.as_mut().set_message(q(&text)),
            Err(e) => self.as_mut().show_error(&e),
        }
    }

    fn use_offered_input(mut self: Pin<&mut Self>) {
        let id = self.as_ref().offer_input().to_string();
        self.as_mut().core().use_offered_input(&id);
        self.as_mut().set_offer_input(QString::default());
        self.as_mut().set_offer_text(QString::default());
        self.as_mut().set_error_text(QString::default());
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
