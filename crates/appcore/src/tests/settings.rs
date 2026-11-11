//! Settings: instrument, tuning, mixer, scoring, devices.

use super::*;
use kontra_session::{Connection, DeviceInfo, Limit, Shaping};

#[test]
fn midi_choice_by_name_falls_back_to_the_first() {
    let port = |id: &str, name: &str| kontra_session::MidiPort {
        id: id.into(),
        name: name.into(),
    };
    let ports = [port("20:0", "Keystation 49"), port("24:0", "Launchkey")];
    assert_eq!(pick_midi(&ports, None), (Some(0), MidiChoice::Found));
    assert_eq!(
        pick_midi(&ports, Some("Launchkey")),
        (Some(1), MidiChoice::Found)
    );
    assert_eq!(
        pick_midi(&ports, Some("24:0")),
        (Some(1), MidiChoice::Found),
        "an id stored before"
    );
    assert_eq!(
        pick_midi(&ports, Some("MPK mini")),
        (Some(0), MidiChoice::Missing)
    );
    assert_eq!(
        pick_midi(&[], Some("MPK mini")),
        (None, MidiChoice::Missing)
    );
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

/// J-E1 (P5): the keyboard view spans what the bass plays, E1 (28) to G4 (67) on a
/// four-string with 24 frets; a five-string reaches down to B0 (23).
#[test]
fn key_range_follows_the_instrument() {
    let dir = temp_dir("keys");
    let mut core = Core::new(&dir, QaOptions::default());
    let range: serde_json::Value = serde_json::from_str(&core.key_range_json()).unwrap();
    assert_eq!(
        (range["low"].as_u64(), range["high"].as_u64()),
        (Some(28), Some(67))
    );
    core.reconfigure(|s| s.strings = 5);
    let range: serde_json::Value = serde_json::from_str(&core.key_range_json()).unwrap();
    assert_eq!(range["low"].as_u64(), Some(23));
    assert_eq!(range["names"].as_array().map(Vec::len), Some(12));
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
    let off = gains(&settings, &Default::default());
    assert_eq!((off.backing, off.click, off.tone), (0.25, 1.0, 0.0));
    assert_eq!(off.monitor, 0.0);
    settings.monitor = true;
    assert_eq!(gains(&settings, &Default::default()).monitor, 0.5);

    // Unshaped, but the limiter on by default (LH-EIN-17).
    assert_eq!(
        off.shaping,
        Shaping {
            limit: Limit::On,
            ..Shaping::FLAT
        }
    );
    settings.monitor_low_db = 3.0;
    settings.monitor_compress = 0.5;
    settings.monitor_warmth = 0.25;
    settings.set_monitor_drive(0.75);
    settings.set_monitor_width(1.0);
    settings.monitor_limit = false;
    let shaped = gains(&settings, &Default::default()).shaping;
    assert_eq!(
        (shaped.low_db, shaped.compress, shaped.warmth),
        (3.0, 0.5, 0.25)
    );
    assert_eq!(shaped.chain, settings.monitor_chain);
    assert_eq!(shaped.chain.slots().len(), 2);
    assert_eq!(shaped.limit, Limit::Off);
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
    let auto = core.score_settings(InstrumentKind::Fretless);
    assert_eq!(auto.pitch_cents, 30.0);
    assert_eq!(auto.strictness.window_ms(), 60.0);
    core.reconfigure(|s| {
        s.strictness = "strict".into();
        s.pitch_cents = 20.0;
    });
    let chosen = core.score_settings(InstrumentKind::Fretless);
    assert_eq!(chosen.strictness.window_ms(), 25.0);
    assert_eq!(chosen.pitch_cents, 20.0);
    let _ = std::fs::remove_dir_all(dir);
}

/// LH-EIN-09 (QA 27.10. S11): the instrument is chosen on its own and stored apart from the
/// input; "bass over MIDI" is the bass profile with the MIDI input.
#[test]
fn instrument_is_chosen_apart_from_input() {
    let dir = temp_dir("instrument");
    let mut core = Core::new(&dir, QaOptions::default());
    assert_eq!(core.instrument_choice(), InstrumentChoice::EBass);

    // Double bass: no frets, microphone suggested, at most five strings, scored as double.
    core.reconfigure(|s| s.strings = 6);
    core.choose_instrument(InstrumentChoice::Double).unwrap();
    assert_eq!(core.settings().instrument, Bass::Double);
    assert_eq!(core.settings().input_kind, InputKind::Microphone);
    assert_eq!(core.settings().strings, 5);
    assert_eq!(core.instrument_choice(), InstrumentChoice::Double);
    assert_eq!(
        instrument_from(core.settings()).kind,
        InstrumentKind::Double
    );

    // A pickup instead of the microphone stays when the double bass is chosen again.
    core.reconfigure(|s| s.input_kind = InputKind::Line);
    core.choose_instrument(InstrumentChoice::Double).unwrap();
    assert_eq!(core.settings().input_kind, InputKind::Line);

    // Bass over MIDI: the MIDI input, the bass profile kept underneath.
    core.choose_instrument(InstrumentChoice::Midi).unwrap();
    assert_eq!(core.settings().input_kind, InputKind::Midi);
    assert_eq!(core.settings().instrument, Bass::Double);
    assert_eq!(core.instrument_choice(), InstrumentChoice::Midi);
    assert_eq!(instrument_from(core.settings()).kind, InstrumentKind::Keys);

    // Back to the e-bass: frets, and a cable instead of MIDI.
    core.choose_instrument(InstrumentChoice::EBass).unwrap();
    assert_eq!(core.settings().instrument, Bass::Fretted);
    assert_eq!(core.settings().input_kind, InputKind::Line);
    assert_eq!(core.instrument_choice(), InstrumentChoice::EBass);

    // An e-bass keeps being fretless when chosen again.
    core.reconfigure(|s| s.instrument = Bass::Fretless);
    core.choose_instrument(InstrumentChoice::EBass).unwrap();
    assert_eq!(core.settings().instrument, Bass::Fretless);
    let _ = std::fs::remove_dir_all(dir);
}

/// A piece opened for a double bass is scored as one (intonation, double-bass lessons).
#[test]
fn piece_takes_the_double_bass() {
    let dir = temp_dir("instrument-piece");
    let mut core = Core::new(&dir, QaOptions::default());
    core.choose_instrument(InstrumentChoice::Double).unwrap();
    core.reconfigure(|s| s.input_kind = InputKind::Line);
    core.open(&demo()).unwrap();
    let kind = core.arrangement().unwrap().instrument.kind;
    assert_eq!(kind, InstrumentKind::Double);
    assert_eq!(core.score_settings(kind).pitch_cents, 30.0);
    let _ = std::fs::remove_dir_all(dir);
}

/// A double bass at the microphone leaves it for a cable when the e-bass is chosen.
#[test]
fn ebass_after_double_bass_takes_the_cable() {
    let dir = temp_dir("instrument-mic");
    let mut core = Core::new(&dir, QaOptions::default());
    core.choose_instrument(InstrumentChoice::Double).unwrap();
    core.choose_instrument(InstrumentChoice::EBass).unwrap();
    assert_eq!(core.settings().input_kind, InputKind::Line);
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

fn open_strings(core: &Core) -> Vec<String> {
    let strings: Vec<serde_json::Value> = serde_json::from_str(&core.tuning_json()).unwrap();
    strings
        .iter()
        .map(|s| s["full"].as_str().unwrap().to_string())
        .collect()
}

/// LH-TUN-02: the presets for four and five strings, and a custom tuning that only fits its
/// string count.
#[test]
fn tunings_follow_the_settings() {
    let names = |strings: u8, tuning: &str| {
        let settings = Settings {
            strings,
            tuning: tuning.into(),
            custom_tuning: vec!["C1".into(), "G1".into(), "C2".into(), "F2".into()],
            ..Settings::default()
        };
        names_of(&tuning_from(&settings)).join(" ")
    };
    assert_eq!(names(4, "standard"), "E1 A1 D2 G2");
    assert_eq!(names(4, "drop"), "D1 A1 D2 G2");
    assert_eq!(names(5, "drop"), "A0 E1 A1 D2 G2");
    assert_eq!(names(4, "half-down"), "D#1 G#1 C#2 F#2");
    assert_eq!(names(4, "whole-down"), "D1 G1 C2 F2");
    assert_eq!(names(4, "custom"), "C1 G1 C2 F2");
    assert_eq!(names(5, "custom"), "B0 E1 A1 D2 G2");
}

/// LH-TUN-02 acceptance: a custom tuning is used by the tuner and for the tab of a MIDI file
/// (no tuning of its own), and survives a restart.
#[test]
fn custom_tuning_reaches_tuner_and_tab() {
    let dir = temp_dir("custom-tuning");
    let mut core = Core::new(&dir, QaOptions::default());
    core.open(&demo()).unwrap();
    core.set_tuning("drop").unwrap();
    assert_eq!(open_strings(&core), ["D1", "A1", "D2", "G2"]);
    core.set_custom_string(3, 41).unwrap(); // G2 → F2
    assert_eq!(core.settings().tuning, "custom");
    assert_eq!(open_strings(&core), ["D1", "A1", "D2", "F2"]);

    // Every note sits on a string and fret that sound its pitch in the new tuning.
    let a = core.arrangement().unwrap();
    assert_eq!(a.instrument.tuning.strings[3].0, 41);
    for n in &a.notes {
        let (string, fret) = (n.string.unwrap(), n.fret.unwrap());
        assert_eq!(a.instrument.pitch_at(string, fret), Some(n.pitch));
    }

    let again = Core::new(&dir, QaOptions::default());
    assert_eq!(open_strings(&again), ["D1", "A1", "D2", "F2"]);
    let _ = std::fs::remove_dir_all(dir);
}

/// LH-TUN-04: the reference pitch reaches the instrument the tuner and scoring listen with.
#[test]
fn reference_pitch_is_used() {
    let dir = temp_dir("reference");
    let mut core = Core::new(&dir, QaOptions::default());
    assert_eq!(core.instrument().a4_hz, 440.0);
    assert_eq!(core.set_reference(442.0).unwrap(), 442.0);
    assert_eq!(core.instrument().a4_hz, 442.0);
    core.open(&demo()).unwrap();
    assert_eq!(core.arrangement().unwrap().instrument.a4_hz, 442.0);
    assert_eq!(core.set_reference(470.0).unwrap(), 450.0);
    assert_eq!(core.arrangement().unwrap().instrument.a4_hz, 450.0);
    let _ = std::fs::remove_dir_all(dir);
}

/// LH-BEW-02: a free window in ms, kept within 20–150 ms.
#[test]
fn free_timing_window() {
    let dir = temp_dir("window");
    let mut core = Core::new(&dir, QaOptions::default());
    assert_eq!(core.set_window(35.0), 35.0);
    assert_eq!(core.settings().strictness, "custom");
    let settings = core.score_settings(InstrumentKind::Fretted);
    assert_eq!(settings.strictness.window_ms(), 35.0);
    assert_eq!(core.set_window(5.0), 20.0);
    let _ = std::fs::remove_dir_all(dir);
}

/// An audio input as the system lists it.
fn device(id: &str, connection: Connection, is_default: bool) -> DeviceInfo {
    DeviceInfo {
        id: id.into(),
        name: id.into(),
        direction: Direction::Input,
        is_default,
        connection,
        sample_rate: 48_000,
        channels: 2,
        past_pipewire: false,
    }
}

/// LH-EIN-02: Scarlett Solo and the internal card → the interface, default or not.
#[test]
fn interface_before_internal_card() {
    let internal = device("hda", Connection::BuiltIn, true);
    let scarlett = device("scarlett", Connection::Usb, false);
    let list = [internal.clone(), scarlett.clone()];
    assert_eq!(
        preferred_input(&list, InputKind::Line).map(|d| d.id.as_str()),
        Some("scarlett")
    );

    let usb_default = device("usb-default", Connection::Usb, true);
    let list = [scarlett, internal.clone(), usb_default];
    assert_eq!(
        preferred_input(&list, InputKind::Line).map(|d| d.id.as_str()),
        Some("usb-default")
    );

    let list = [internal, device("pipewire", Connection::Other, false)];
    assert_eq!(preferred_input(&list, InputKind::Line), None);
}

/// QA 27.10. (B8, P1 Lea): with "Klinke direkt in den Laptop" chosen, the input still said
/// "Automatisch: Scarlett Solo" and Kontra listened to the interface. The laptop's own card is
/// the automatic input then, the system's default first.
#[test]
fn laptop_jack_takes_the_built_in_card() {
    let list = [
        device("scarlett", Connection::Usb, true),
        device("hda", Connection::BuiltIn, false),
        device("hdmi", Connection::Hdmi, false),
    ];
    let preferred = |kind| preferred_input(&list, kind).map(|d| d.id.as_str());
    assert_eq!(preferred(InputKind::LaptopJack), Some("hda"));
    assert_eq!(preferred(InputKind::Line), Some("scarlett"));
    // A USB microphone for the double bass stays the first choice (LH-EIN-11).
    assert_eq!(preferred(InputKind::Microphone), Some("scarlett"));
    assert_eq!(
        chosen_input(None, &list, InputKind::LaptopJack).as_deref(),
        Some("hda")
    );
    // Chosen by the player: kept, whatever the connection.
    assert_eq!(
        chosen_input(Some("scarlett"), &list, InputKind::LaptopJack).as_deref(),
        Some("scarlett")
    );
    // No built-in card (a desktop with an interface only): the system decides.
    assert_eq!(preferred_input(&list[..1], InputKind::LaptopJack), None);
}

/// Review #231: a USB webcam (input only) is no interface; "Automatic" skips it
/// for the cable and takes it only for "Microphone" (LH-EIN-02, LH-EIN-11).
#[test]
fn a_usb_microphone_is_no_interface() {
    let list = [
        device("webcam", Connection::UsbMicrophone, true),
        device("hda", Connection::BuiltIn, false),
    ];
    assert_eq!(preferred_input(&list, InputKind::Line), None);
    assert_eq!(preferred_input(&list, InputKind::Midi), None);
    assert_eq!(
        preferred_input(&list, InputKind::Microphone).map(|d| d.id.as_str()),
        Some("webcam")
    );

    let list = [
        device("webcam", Connection::UsbMicrophone, true),
        device("scarlett", Connection::Usb, false),
    ];
    assert_eq!(
        preferred_input(&list, InputKind::Line).map(|d| d.id.as_str()),
        Some("scarlett")
    );
}

/// Review #73: a saved device that is not listed (an ALSA id under PipeWire, an unplugged
/// interface) means "Automatic": the interface first, not the system default.
#[test]
fn unlisted_choice_is_automatic() {
    let list = [
        device("hda", Connection::BuiltIn, true),
        device("scarlett", Connection::Usb, false),
    ];
    let input = |saved: Option<&str>| chosen_input(saved, &list, InputKind::Line);
    assert_eq!(
        input(Some("alsa:hw:CARD=2,DEV=0")).as_deref(),
        Some("scarlett")
    );
    assert_eq!(input(Some("hda")).as_deref(), Some("hda"));
    assert_eq!(input(None).as_deref(), Some("scarlett"));

    assert_eq!(
        chosen_output(Some("alsa:front:CARD=USB,DEV=0"), &list),
        None
    );
    assert_eq!(chosen_output(Some("hda"), &list).as_deref(), Some("hda"));
    // The engine's timer output is never listed and stays (tests must stay silent).
    assert_eq!(
        chosen_output(Some(kontra_session::NULL_DEVICE), &[]).as_deref(),
        Some(kontra_session::NULL_DEVICE)
    );
}

/// Round 8: ear training prompts sound at their own volume, not the reference tone's.
#[test]
fn ear_prompts_have_their_own_volume() {
    let settings = Settings {
        tone_volume: 0.2,
        prompt_volume: 0.9,
        ..Settings::default()
    };
    assert_eq!(ear_gains(&settings, &Default::default()).tone, 0.9);
    assert_eq!(gains(&settings, &Default::default()).tone, 0.2);
}

/// LH-NF-11: the licence list names Kontra (MIT) and only permissive crate licences.
#[test]
fn licence_list_is_complete_and_permissive() {
    let core = Core::new(&temp_dir("licences"), QaOptions::default());
    let list: serde_json::Value = serde_json::from_str(&core.licences_json()).unwrap();
    let list = list.as_array().unwrap();
    assert_eq!(list[0]["name"], "Kontra");
    assert_eq!(list[0]["license"], "MIT");
    assert!(list.iter().any(|c| c["name"] == "cpal"));
    let permissive = [
        "MIT",
        "Apache-2.0",
        "BSD",
        "ISC",
        "Zlib",
        "Unlicense",
        "0BSD",
        "Unicode",
    ];
    for c in &list[3..] {
        let licence = c["license"].as_str().unwrap();
        assert!(permissive.iter().any(|p| licence.contains(p)), "{c}");
        assert!(!licence.contains("GPL"), "{c}");
    }
}

/// Audit 10.11.: "Band reagiert" reached a running run only after it ended; settings act while
/// Kontra runs.
#[test]
fn reacting_band_reaches_the_running_run() {
    let dir = temp_dir("band-live");
    let take = dir.join("take.wav");
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
    core.set_reacting_band(true);
    let band = core.live.as_ref().map(|l| l.practice().options().band);
    assert_eq!(band, Some(kontra_session::Band::Reacting));
    core.stop_live();
    let _ = std::fs::remove_dir_all(dir);
}

/// LH-EIN-19: the mixer's chain list: add, move, switch, set and remove through the core, the
/// list the page shows follows, the settings are saved, and a full chain says so.
#[test]
fn the_chain_is_edited_through_the_core() {
    let dir = temp_dir("chain-edit");
    let mut core = Core::new(&dir, QaOptions::default());
    let list =
        |core: &Core| -> serde_json::Value { serde_json::from_str(&core.effects_json()).unwrap() };
    let names = |core: &Core| -> Vec<String> {
        list(core)["slots"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["kind"].as_str().unwrap().to_string())
            .collect()
    };
    assert_eq!(list(&core)["empty"], true);

    for kind in ["delay", "overdrive", "reverb"] {
        assert!(core.edit_effects("add", 0, kind, 0.0).is_none());
    }
    assert_eq!(names(&core), ["delay", "overdrive", "reverb"]);

    // Order: the last one up, the first one down; the ends stay put.
    core.edit_effects("up", 2, "", 0.0);
    core.edit_effects("down", 0, "", 0.0);
    assert_eq!(names(&core), ["reverb", "delay", "overdrive"]);
    core.edit_effects("up", 0, "", 0.0);
    core.edit_effects("down", 2, "", 0.0);
    assert_eq!(names(&core), ["reverb", "delay", "overdrive"]);
    let json = list(&core);
    assert_eq!(json["slots"][0]["first"], true);
    assert_eq!(json["slots"][2]["last"], true);

    // Values: mix only where there is a dry part; clamped; unknown keys change nothing.
    core.edit_effects("set", 1, "b", 0.8);
    core.edit_effects("set", 1, "mix", 2.0);
    core.edit_effects("set", 2, "mix", 0.1);
    core.edit_effects("set", 2, "b", 0.9);
    core.edit_effects("on", 0, "", 0.0);
    let slots = core.settings().monitor_chain.slots().to_vec();
    assert_eq!((slots[1].b, slots[1].mix), (0.8, 1.0));
    assert_eq!((slots[2].b, slots[2].mix), (0.0, 1.0));
    assert!(!slots[0].on);
    let delay = &list(&core)["slots"][1];
    assert_eq!(delay["values"].as_array().unwrap().len(), 3);
    assert_eq!(
        list(&core)["slots"][2]["values"].as_array().unwrap().len(),
        1
    );

    // It is saved.
    let again = Core::new(&dir, QaOptions::default());
    assert_eq!(
        again.settings().monitor_chain,
        core.settings().monitor_chain
    );

    // Remove, then fill up: the ninth effect is refused with a hint.
    core.edit_effects("remove", 0, "", 0.0);
    assert_eq!(names(&core), ["delay", "overdrive"]);
    while !list(&core)["full"].as_bool().unwrap() {
        assert!(core.edit_effects("add", 0, "chorus", 0.0).is_none());
    }
    assert_eq!(names(&core).len(), kontra_model::CHAIN_SLOTS);
    let hint = core.edit_effects("add", 0, "fuzz", 0.0);
    assert!(hint.is_some_and(|h| h.contains('8')));
    assert_eq!(names(&core).len(), kontra_model::CHAIN_SLOTS);

    // "Neutral" empties the list.
    core.neutral_shaping();
    assert_eq!(list(&core)["empty"], true);
}

/// LH-EIN-19 (Runde 19): named chains through the core: save, list, load, delete, kept on disk.
#[test]
fn chains_are_saved_by_name() {
    let dir = temp_dir("chain-presets");
    let mut core = Core::new(&dir, QaOptions::default());
    let presets = |core: &Core| -> serde_json::Value {
        serde_json::from_str::<serde_json::Value>(&core.effects_json()).unwrap()["presets"].clone()
    };
    core.edit_effects("add", 0, "delay", 0.0);
    core.edit_effects("add", 0, "reverb", 0.0);
    let saved = core.settings().monitor_chain;

    assert!(core.edit_effects("save", 0, "Ambient", 0.0).is_none());
    core.neutral_shaping();
    assert!(core.edit_effects("save", 0, "Clean", 0.0).is_none());
    assert_eq!(presets(&core), serde_json::json!(["Ambient", "Clean"]));

    core.edit_effects("load", 0, "", 0.0);
    assert_eq!(core.settings().monitor_chain, saved);
    let again = Core::new(&dir, QaOptions::default());
    assert_eq!(again.settings().chain_presets.len(), 2);
    assert_eq!(again.settings().monitor_chain, saved);

    core.edit_effects("delete", 1, "", 0.0);
    assert_eq!(presets(&core), serde_json::json!(["Ambient"]));
    assert_eq!(
        core.settings().monitor_chain,
        saved,
        "deleting keeps the chain"
    );
}
