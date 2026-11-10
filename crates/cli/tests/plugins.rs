//! `kontra-cli plugins` against the test plugins of `crates/test-clap-gain` and
//! `crates/test-vst3-gain` (built as examples by `cargo test --workspace`): the scan lists the
//! good files and marks the ones that crash and the ones that hang as broken, without losing the
//! others.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const CLI: &str = env!("CARGO_BIN_EXE_kontra-cli");
const GAIN_ID: &str = "de.arianw.kontra.test.gain";
const GAIN_MONO_ID: &str = "de.arianw.kontra.test.gain-mono";
const VST3_PLUGINS: [&str; 4] = [
    "kontra_vst3_crash",
    "kontra_vst3_gain",
    "kontra_vst3_gain_mono",
    "kontra_vst3_hang",
];
const PLUGINS: [&str; 4] = [
    "kontra_clap_crash",
    "kontra_clap_gain",
    "kontra_clap_gain_mono",
    "kontra_clap_hang",
];
/// Short, so the hanging plugin does not slow the test down.
const SCAN_TIMEOUT_S: &str = "3";

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const ARCH_DIR: &str = "x86_64-linux";
#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
const ARCH_DIR: &str = "aarch64-linux";
#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
const ARCH_DIR: &str = "x86_64-win";
#[cfg(all(target_os = "windows", target_arch = "aarch64"))]
const ARCH_DIR: &str = "arm64-win";
#[cfg(windows)]
const BINARY_EXTENSION: &str = "vst3";
#[cfg(not(windows))]
const BINARY_EXTENSION: &str = "so";

/// Where the host looks for VST3 bundles when the home folder (Windows: the local app data
/// folder) is `dir/home`.
fn vst3_folder(dir: &Path) -> PathBuf {
    if cfg!(windows) {
        dir.join("home")
            .join("Programs")
            .join("Common")
            .join("VST3")
    } else {
        dir.join("home").join(".vst3")
    }
}

/// The folder of the CLAP files (`<name>.clap`) and a home folder with the VST3 bundles
/// (`<name>.vst3/Contents/<arch>/<name>.<so|vst3>`); the host recognises plugins by that layout.
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

    for name in VST3_PLUGINS {
        let built = examples.join(format!(
            "{}{name}{}",
            std::env::consts::DLL_PREFIX,
            std::env::consts::DLL_SUFFIX
        ));
        assert!(built.exists(), "{} is missing", built.display());
        let arch = vst3_folder(&dir)
            .join(format!("{name}.vst3"))
            .join("Contents")
            .join(ARCH_DIR);
        std::fs::create_dir_all(&arch).expect("bundle folder");
        std::fs::copy(&built, arch.join(format!("{name}.{BINARY_EXTENSION}"))).expect("copy");
    }
    dir
}

/// Runs the CLI with the plugin folder as the only search path.
fn cli(dir: &Path, args: &[&str]) -> Output {
    Command::new(CLI)
        .args(args)
        .env("CLAP_PATH", dir)
        .env("HOME", dir.join("home"))
        .env("LOCALAPPDATA", dir.join("home"))
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

    // The good plugins of both formats are found, although the crashing file is scanned first.
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
    let vst3: Vec<&str> = report["plugins"]
        .as_array()
        .expect("plugins")
        .iter()
        .filter(|p| {
            p["format"] == "vst3"
                && p["path"]
                    .as_str()
                    .is_some_and(|f| f.starts_with(dir.to_str().unwrap_or("")))
        })
        .map(|p| p["name"].as_str().expect("name"))
        .collect();
    assert!(found.contains(&(GAIN_ID, "kontra_clap_gain.clap".into())));
    assert!(found.contains(&(GAIN_MONO_ID, "kontra_clap_gain_mono.clap".into())));
    assert_eq!(found.len(), 4);
    assert!(vst3.contains(&"Kontra Test Gain"));
    assert!(vst3.contains(&"Kontra Test Gain Mono"));

    // The bad files are named with a reason.
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
    assert!(reason("kontra_vst3_crash.vst3").starts_with("crashed"));
    assert!(reason("kontra_vst3_hang.vst3").starts_with("no answer"));
    std::fs::remove_dir_all(&dir).expect("clean up");
}

#[test]
fn scan_prints_text_by_default() {
    let dir = plugin_dir("text");
    let out = cli(&dir, &["plugins", "scan", "--timeout-s", SCAN_TIMEOUT_S]);
    let text = String::from_utf8_lossy(&out.stdout);

    assert!(text.contains("CLAP  Kontra Test Gain — Kontra Tests  [de.arianw.kontra.test.gain]"));
    assert!(text.contains("VST3  Kontra Test Gain — Kontra Tests  ["));
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

    // The same for a VST3 bundle.
    let bundle = vst3_folder(&dir).join("kontra_vst3_gain.vst3");
    let out = cli(&dir, &["plugins", "params", bundle.to_str().expect("path")]);
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
    let out = cli(
        &dir,
        &["plugins", "run", gain, "x.wav", "y.wav", "--param", "99=1"],
    );
    assert!(!out.status.success());
    std::fs::remove_dir_all(&dir).expect("clean up");
}
