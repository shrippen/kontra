//! `kontra-cli plugins` against the test plugins of `crates/test-clap-gain` (built as examples by
//! `cargo test --workspace`): the scan lists the good files and marks the one that crashes and
//! the one that hangs as broken, without losing the others.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const CLI: &str = env!("CARGO_BIN_EXE_kontra-cli");
const GAIN_ID: &str = "de.arianw.kontra.test.gain";
const GAIN_MONO_ID: &str = "de.arianw.kontra.test.gain-mono";
const PLUGINS: [&str; 4] = [
    "kontra_clap_crash",
    "kontra_clap_gain",
    "kontra_clap_gain_mono",
    "kontra_clap_hang",
];
/// Short, so the hanging plugin does not slow the test down.
const SCAN_TIMEOUT_S: &str = "3";

/// A folder with every test plugin as `<name>.clap` (the host recognises plugin files by that
/// extension).
fn plugin_dir(tag: &str) -> PathBuf {
    let exe = std::env::current_exe().expect("test binary path");
    let profile = exe.parent().and_then(Path::parent).expect("target folder");
    let examples = profile.join("examples");
    // Inside the target folder (`cargo clean` removes it), not in the RAM-backed temp folder.
    let dir = profile
        .join("plugin-tests")
        .join(format!("cli-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp folder");

    for name in PLUGINS {
        let built = examples.join(format!(
            "{}{name}{}",
            std::env::consts::DLL_PREFIX,
            std::env::consts::DLL_SUFFIX
        ));
        assert!(
            built.exists(),
            "{} is missing: `cargo test --workspace` builds the test plugins",
            built.display()
        );
        std::fs::copy(&built, dir.join(format!("{name}.clap"))).expect("copy plugin");
    }
    dir
}

/// Runs the CLI with the plugin folder as the only search path.
fn cli(dir: &Path, args: &[&str]) -> Output {
    Command::new(CLI)
        .args(args)
        .env("CLAP_PATH", dir)
        .env("HOME", dir.join("no-home"))
        .output()
        .expect("start kontra-cli")
}

fn file_of(path: &Value) -> String {
    Path::new(path.as_str().expect("path"))
        .file_name()
        .expect("file name")
        .to_string_lossy()
        .into_owned()
}

#[test]
fn scan_marks_crashing_and_hanging_plugins_and_goes_on() {
    let dir = plugin_dir("scan");
    let out = cli(
        &dir,
        &["plugins", "scan", "--json", "--timeout-s", SCAN_TIMEOUT_S],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report: Value = serde_json::from_slice(&out.stdout).expect("JSON report");

    // Both good plugins are found, although the crashing file is scanned before them.
    let found: Vec<(&str, String)> = report["plugins"]
        .as_array()
        .expect("plugins")
        .iter()
        .filter(|p| {
            p["path"]
                .as_str()
                .is_some_and(|f| f.starts_with(dir.to_str().unwrap_or("")))
        })
        .map(|p| (p["id"].as_str().expect("id"), file_of(&p["path"])))
        .collect();
    assert!(found.contains(&(GAIN_ID, "kontra_clap_gain.clap".into())));
    assert!(found.contains(&(GAIN_MONO_ID, "kontra_clap_gain_mono.clap".into())));
    assert_eq!(found.len(), 2);

    // The two bad files are named with a reason.
    let broken = report["broken"].as_array().expect("broken");
    let reason = |file: &str| {
        broken
            .iter()
            .find(|b| file_of(&b["path"]) == file)
            .and_then(|b| b["reason"].as_str())
            .unwrap_or_else(|| panic!("{file} not marked broken: {broken:?}"))
            .to_string()
    };
    assert!(reason("kontra_clap_crash.clap").starts_with("crashed"));
    assert!(reason("kontra_clap_hang.clap").starts_with("no answer"));
    std::fs::remove_dir_all(&dir).expect("clean up");
}

#[test]
fn scan_prints_text_by_default() {
    let dir = plugin_dir("text");
    let out = cli(&dir, &["plugins", "scan", "--timeout-s", SCAN_TIMEOUT_S]);
    let text = String::from_utf8_lossy(&out.stdout);

    assert!(text.contains("CLAP  Kontra Test Gain — Kontra Tests  [de.arianw.kontra.test.gain]"));
    assert!(text.contains("BROKEN"));
    assert!(text.contains(" broken file(s); searched: "));
    std::fs::remove_dir_all(&dir).expect("clean up");
}

#[test]
fn params_and_run() {
    let dir = plugin_dir("run");
    let gain = dir.join("kontra_clap_gain.clap");
    let gain = gain.to_str().expect("path");

    let out = cli(&dir, &["plugins", "params", gain]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(text.contains("latency: 64 samples"));
    assert!(text.contains("7  gain  min 0  max 4  default 1  current 1"));

    let input = dir.join("in.wav");
    let output = dir.join("out.wav");
    let samples: Vec<f32> = (0..4800).map(|i| (i as f32 * 0.05).sin()).collect();
    kontra_session::write_mono_wav(&input, &samples, 48_000).expect("write input");
    let out = cli(
        &dir,
        &[
            "plugins",
            "run",
            gain,
            input.to_str().expect("path"),
            output.to_str().expect("path"),
            "--param",
            "7=0.5",
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("written:"));
    assert!(output.exists());

    // A bad parameter is an error, not a silent skip.
    let wrong = dir.join("wrong.wav");
    let out = cli(
        &dir,
        &[
            "plugins",
            "run",
            gain,
            input.to_str().expect("path"),
            wrong.to_str().expect("path"),
            "--param",
            "99=1",
        ],
    );
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("99"));
    assert!(!wrong.exists());
    std::fs::remove_dir_all(&dir).expect("clean up");
}

/// A timeout that is no length of time is a message, not a panic (review #248).
#[test]
fn scan_refuses_a_timeout_below_zero() {
    let dir = plugin_dir("timeout");
    for bad in ["--timeout-s=-1", "--timeout-s=inf", "--timeout-s=NaN"] {
        let out = cli(&dir, &["plugins", "scan", bad]);
        assert_eq!(out.status.code(), Some(1), "{bad}");
        assert!(String::from_utf8_lossy(&out.stderr).contains("--timeout-s"));
    }
    std::fs::remove_dir_all(&dir).expect("clean up");
}
