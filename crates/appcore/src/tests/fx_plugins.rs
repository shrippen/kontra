//! Plugins in the effect chain (LH-EIN-20, LH-EIN-21): adding, parameters, what is saved, a
//! plugin that is gone, and the search. The test plugin is the one of `crates/test-clap-gain`
//! (built as an example by `cargo test --workspace`).

use super::*;
use kontra_session::plugins::{probe_file, PluginFormat, ProbeCommand, ScanReport};
use serde_json::Value;
use std::sync::atomic::{AtomicU32, Ordering};

const GAIN_PARAM: &str = "p7";
const GAIN_MAX: f64 = 4.0;
/// The test plugin's trigger and answer (`GAIN_TRIGGER`, `GAIN_REPORTED` in its crate).
const TRIGGER_GAIN: f64 = 3.5;
const REPORTED_GAIN: f64 = 0.5;

/// The built CLAP gain plugin as `<name>.clap` in the target folder (the host recognises plugin
/// files by that extension).
fn gain_file() -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let exe = std::env::current_exe().expect("test binary path");
    let profile = exe.parent().and_then(Path::parent).expect("target folder");
    let built = profile.join("examples").join(format!(
        "{}kontra_clap_gain{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    ));
    assert!(
        built.exists(),
        "{} is missing: `cargo test --workspace` builds the test plugins",
        built.display()
    );
    let dir = profile.join("plugin-tests").join(format!(
        "appcore-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("folder");
    let file = dir.join("kontra_clap_gain.clap");
    std::fs::copy(&built, &file).expect("copy plugin");
    file
}

/// A core whose last scan found the gain plugin.
fn core_with_plugin(name: &str) -> Core {
    let mut core = Core::new(&temp_dir(name), QaOptions::default());
    let plugins = probe_file(PluginFormat::Clap, &gain_file()).expect("probe");
    core.plugins.set_report(ScanReport {
        plugins,
        broken: Vec::new(),
    });
    core
}

fn effects(core: &Core) -> Value {
    serde_json::from_str(&core.effects_json()).unwrap()
}

#[test]
fn a_found_plugin_becomes_a_slot_with_sliders() {
    let mut core = core_with_plugin("fx-plugin-add");
    let json = effects(&core);
    assert_eq!(json["plugins"][0]["name"], "Kontra Test Gain");
    assert_eq!(json["plugins"][0]["format"], "CLAP");
    let key = json["plugins"][0]["key"].as_str().unwrap().to_string();

    assert!(core.edit_effects("add", 0, &key, 0.0).is_none());
    let slot = &effects(&core)["slots"][0];
    assert_eq!(slot["kind"], "plugin");
    assert_eq!(slot["name"], "Kontra Test Gain");
    assert_eq!(slot["problem"], "");
    let gain = &slot["values"][0];
    assert_eq!(gain["key"], GAIN_PARAM);
    // Default 1 on a range of 0 to 4: a quarter of the slider.
    assert_eq!(gain["value"], 0.25);

    // The middle of the slider is half of the range, in the plugin's own units.
    core.edit_effects("set", 0, GAIN_PARAM, 0.5);
    let param = &core.settings().plugins[0].params[0];
    assert_eq!(param.value, GAIN_MAX / 2.0);
    assert_eq!(effects(&core)["slots"][0]["values"][0]["value"], 0.5);
}

#[test]
fn the_plugin_and_its_values_survive_a_restart() {
    let mut core = core_with_plugin("fx-plugin-keep");
    core.edit_effects("add", 0, "plugin:0", 0.0);
    core.edit_effects("add", 0, "reverb", 0.0);
    core.edit_effects("set", 0, GAIN_PARAM, 0.5);
    let saved = core.settings().clone();
    let dir = core.data_dir.clone();
    drop(core);

    let again = Core::new(&dir, QaOptions::default());
    assert_eq!(again.settings().plugins, saved.plugins);
    assert_eq!(again.settings().monitor_chain, saved.monitor_chain);
    // The test plugin's state is its gain as four bytes; the settings hold it as text.
    assert_eq!(saved.plugins[0].state, 1.0f32.to_le_bytes());
    let text = std::fs::read_to_string(dir.join("settings.json")).unwrap();
    assert!(text.contains("\"AACAPw==\""), "{text}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn removing_the_slot_removes_the_plugin() {
    let mut core = core_with_plugin("fx-plugin-remove");
    core.edit_effects("add", 0, "plugin:0", 0.0);
    core.edit_effects("add", 0, "plugin:0", 0.0);
    let ids: Vec<u32> = core.settings().plugins.iter().map(|p| p.id).collect();
    assert_eq!(
        ids,
        [1, 2],
        "two slots, two instances with their own values"
    );

    core.edit_effects("remove", 0, "", 0.0);
    let ids: Vec<u32> = core.settings().plugins.iter().map(|p| p.id).collect();
    assert_eq!(ids, [2]);
    core.neutral_shaping();
    assert!(core.settings().plugins.is_empty());
}

#[test]
fn a_plugin_that_does_not_load_is_not_added() {
    let mut core = core_with_plugin("fx-plugin-broken");
    let mut report = core.plugins.report().clone();
    report.plugins[0].path = "/nonexistent/gone.clap".into();
    core.plugins.set_report(report);

    let hint = core.edit_effects("add", 0, "plugin:0", 0.0).unwrap();
    assert!(hint.contains("Kontra Test Gain"), "{hint}");
    assert!(core.settings().monitor_chain.is_empty());
    assert!(core.settings().plugins.is_empty());
}

#[test]
fn a_plugin_that_is_gone_says_so_and_the_chain_stays() {
    let mut core = core_with_plugin("fx-plugin-gone");
    core.edit_effects("add", 0, "plugin:0", 0.0);
    core.edit_effects("add", 0, "reverb", 0.0);
    core.reconfigure(|s| s.plugins[0].path = "/nonexistent/gone.clap".into());

    let json = effects(&core);
    assert!(!json["slots"][0]["problem"].as_str().unwrap().is_empty());
    assert_eq!(json["slots"][1]["problem"], "");
    assert_eq!(json["slots"].as_array().unwrap().len(), 2);
}

#[test]
fn a_reason_from_the_engine_reaches_the_slot() {
    let mut core = core_with_plugin("fx-plugin-reason");
    core.edit_effects("add", 0, "plugin:0", 0.0);
    let id = core.settings().plugins[0].id;
    // The engine notes it when it cannot load an instance for a chain.
    core.plugins.problems.note(id, "the plugin crashed".into());
    let problem = effects(&core)["slots"][0]["problem"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(problem.contains("the plugin crashed"), "{problem}");
}

/// Review #262: a removed plugin's reason stayed noted, and the next plugin, given the same id,
/// showed it.
#[test]
fn a_new_plugin_shows_no_old_reason() {
    let mut core = core_with_plugin("fx-plugin-old-reason");
    core.edit_effects("add", 0, "plugin:0", 0.0);
    let id = core.settings().plugins[0].id;
    core.plugins.problems.note(id, "the plugin crashed".into());
    core.edit_effects("remove", 0, "", 0.0);

    core.edit_effects("add", 0, "plugin:0", 0.0);
    assert_eq!(effects(&core)["slots"][0]["problem"], "");
}

/// Review #262: plugin values showed as "2.00" in German too.
#[test]
fn a_plugin_value_reads_in_the_language() {
    let mut core = core_with_plugin("fx-plugin-shown");
    core.update_settings(|s| s.language = Some("de".into()));
    core.edit_effects("add", 0, "plugin:0", 0.0);
    let shown = |core: &Core| effects(core)["slots"][0]["values"][0]["shown"].clone();
    assert_eq!(
        shown(&core),
        "1",
        "the default gain, without trailing zeros"
    );
    core.edit_effects("set", 0, GAIN_PARAM, 0.375);
    assert_eq!(shown(&core), "1,5");
    core.update_settings(|s| s.language = Some("en".into()));
    assert_eq!(shown(&core), "1.5");
}

#[test]
fn the_search_runs_in_the_background_and_is_kept() {
    let dir = temp_dir("fx-plugin-scan");
    let mut core = Core::new(&dir, QaOptions::default());
    // A probe that calls every file broken: nothing of the system is loaded.
    core.plugins.set_probe(ProbeCommand {
        program: "sh".into(),
        args: vec!["-c".into(), "echo '{\"Failed\":\"for the test\"}'".into()],
    });
    core.edit_effects("scan", 0, "", 0.0);
    assert!(effects(&core)["scan"]["running"].as_bool().unwrap());

    let start = std::time::Instant::now();
    while core.poll_effects().is_none() {
        assert!(start.elapsed().as_secs() < 30, "the search does not end");
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(!effects(&core)["scan"]["running"].as_bool().unwrap());
    assert!(dir.join("plugins.json").exists());

    // Found plugins and broken files come back after a restart, without a new search.
    let kept = core.plugins.report().clone();
    let again = Core::new(&dir, QaOptions::default());
    assert_eq!(again.plugins.report(), &kept);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_chain_latency_is_a_line_for_the_page() {
    let core = Core::new(&temp_dir("fx-latency"), QaOptions::default());
    assert_eq!(effects(&core)["latencyText"], "", "nothing runs: no line");
    assert_eq!(core.latency_text(Some(0.0)), "", "no delay: no line");
    assert_eq!(core.latency_text(None), "");

    let line = core.latency_text(Some(1.33));
    assert!(line.contains("1.3") || line.contains("1,3"), "{line}");
}

/// FX5: a value the plugin changed by itself (the engine reports it as plugin id, parameter id,
/// value) lands in the settings, the slider follows, and it is there after a restart.
#[test]
fn a_change_made_by_the_plugin_is_saved() {
    let mut core = core_with_plugin("fx-plugin-own-change");
    core.edit_effects("add", 0, "plugin:0", 0.0);
    let id = core.settings().plugins[0].id;
    let param = 7;

    assert!(core.take_plugin_changes(&[(id, param, 0.5)]));
    assert_eq!(core.settings().plugins[0].params[0].value, 0.5);
    // 0.5 of a range of 0 to 4.
    assert_eq!(effects(&core)["slots"][0]["values"][0]["value"], 0.125);

    // The same value again is no news; a plugin or parameter that is not there is ignored.
    assert!(!core.take_plugin_changes(&[(id, param, 0.5)]));
    assert!(!core.take_plugin_changes(&[(id + 1, param, 1.0), (id, param + 1, 1.0)]));
    assert_eq!(core.settings().plugins[0].params[0].value, 0.5);

    let dir = core.data_dir.clone();
    drop(core);
    let again = Core::new(&dir, QaOptions::default());
    assert_eq!(again.settings().plugins[0].params[0].value, 0.5);
    let _ = std::fs::remove_dir_all(dir);
}

/// Review #268: the tuner plays the chain too ("Bass hören" on a cable), but its plugins'
/// own changes never reached the settings.
#[test]
fn a_change_in_the_tuner_is_saved() {
    let mut core = core_with_plugin("fx-plugin-tuner-change");
    let take = core.data_dir.join("take.wav");
    kontra_session::write_mono_wav(&take, &vec![0.0; 48_000], 48_000).unwrap();
    core.qa.input_file = Some(take);
    core.update_settings(|s| s.monitor = true);
    core.edit_effects("add", 0, "plugin:0", 0.0);
    core.start_tuner().unwrap();

    // The test plugin answers this gain with its own change to GAIN_REPORTED (0.5).
    let trigger = (TRIGGER_GAIN / GAIN_MAX) as f32;
    core.edit_effects("set", 0, GAIN_PARAM, f64::from(trigger));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while core.settings().plugins[0].params[0].value != REPORTED_GAIN {
        assert!(
            std::time::Instant::now() < deadline,
            "the tuner's change never arrived"
        );
        let _ = core.poll_effects();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    core.stop_tuner();
    let _ = std::fs::remove_dir_all(core.data_dir.clone());
}
