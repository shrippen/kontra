//! Ear training lets the bass through like the tuner (LH-EIN-18, owner round 23).

use crate::{Core, InputKind, QaOptions};

/// A core on a silent recording with the null output: no device opens.
fn core(name: &str) -> (Core, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("kontra-ear-bass-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);
    let silent = serde_json::json!({ "output_device": "kontra:null" });
    let _ = std::fs::write(dir.join("settings.json"), silent.to_string());
    let take = dir.join("take.wav");
    kontra_session::write_mono_wav(&take, &vec![0.0; 48_000], 48_000).unwrap();
    let qa = QaOptions {
        input_file: Some(take),
        laptop_jack: false,
    };
    (Core::new(&dir, qa), dir)
}

#[test]
fn ear_training_plays_the_bass_through() {
    let (mut core, dir) = core("on");
    core.update_settings(|s| {
        s.monitor = true;
        s.monitor_volume = 0.6;
    });
    core.ear_start("echo").unwrap();
    assert_eq!(core.ear_mix().monitor, 0.6);
    core.ear_stop();
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn ear_training_is_silent_when_monitoring_is_off() {
    let (mut core, dir) = core("off");
    core.update_settings(|s| s.monitor = false);
    core.ear_start("echo").unwrap();
    assert_eq!(core.ear_mix().monitor, 0.0);
    core.remix(|s| s.monitor = true);
    assert!(core.ear_mix().monitor > 0.0, "the mixer carries the switch");
    core.remix(|s| s.monitor = false);
    assert_eq!(core.ear_mix().monitor, 0.0);
    core.ear_stop();
    let _ = std::fs::remove_dir_all(dir);
}

/// A microphone hears the speakers: no feedback loop.
#[test]
fn ear_training_does_not_play_a_microphone() {
    let (mut core, dir) = core("mic");
    core.update_settings(|s| {
        s.monitor = true;
        s.input_kind = InputKind::Microphone;
    });
    core.ear_start("echo").unwrap();
    assert_eq!(core.ear_mix().monitor, 0.0);
    core.ear_stop();
    let _ = std::fs::remove_dir_all(dir);
}
