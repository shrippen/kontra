//! Physical design (Harmonicon W5): the layer rule of agent.md as a test, and a size budget for
//! every source file with a list of the files still to break up.
//!
//! ```text
//! app ─► appcore ─┐
//!                 ├─► session ─┬─► engine ─┬─► drivers
//! cli ────────────┘            │           ├─► dsp
//!                              └─► content └─► stretch
//! model: domain types, every crate may use it
//! ```

use std::path::{Path, PathBuf};

/// Each crate and the Kontra crates it may depend on (`[dependencies]`, `[build-dependencies]`);
/// tests (`[dev-dependencies]`) may reach further.
const ALLOWED: [(&str, &[&str]); 10] = [
    ("app", &["appcore"]),
    ("appcore", &["session", "model"]),
    ("cli", &["session", "model"]),
    ("session", &["engine", "content", "model"]),
    ("content", &["model"]),
    ("engine", &["drivers", "dsp", "stretch", "model"]),
    ("dsp", &["model"]),
    ("drivers", &[]),
    ("stretch", &[]),
    ("model", &[]),
];

/// Lines a Rust or QML file may have.
const BUDGET: usize = 1200;

/// Files over `BUDGET`, each with the most lines it may grow to until it is split (break-up
/// list, 29.10.2026). Lower a limit when a file shrinks; drop the entry once it fits.
const OVER_BUDGET: [(&str, usize); 6] = [
    ("crates/appcore/src/lib.rs", 8200),
    ("crates/app/src/bridge.rs", 2900),
    ("crates/content/src/exercise.rs", 2450),
    ("crates/app/qml/PracticePage.qml", 1400),
    ("crates/dsp/src/tracker.rs", 1450),
    ("crates/engine/src/input.rs", 1300),
];

/// Copied unchanged from Kante (`scripts/sync-kante.sh`), not ours to split.
const VENDORED: &str = "crates/app/qml/Kante";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The Kontra crates in the dependency sections of a `Cargo.toml` that count for the layers.
/// Example: `kontra-model.workspace = true` under `[dependencies]` → "model".
fn layered_deps(manifest: &str) -> Vec<String> {
    let mut deps = Vec::new();
    let mut counts = false;
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            counts = line.ends_with("dependencies]") && !line.contains("dev-dependencies");
            continue;
        }
        if !counts {
            continue;
        }
        let Some(name) = line.strip_prefix("kontra-") else {
            continue;
        };
        let end = name
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
            .unwrap_or(name.len());
        deps.push(name[..end].to_string());
    }
    deps
}

#[test]
fn reads_only_layered_sections() {
    let manifest = "[dependencies]\nkontra-model.workspace = true\nkontra-dsp = { path = \"x\" }\n\
                    [dev-dependencies]\nkontra-session.workspace = true\n\
                    [build-dependencies]\nkontra-drivers.workspace = true\n";
    assert_eq!(layered_deps(manifest), ["model", "dsp", "drivers"]);
}

#[test]
fn crates_depend_only_downwards() {
    let crates = std::fs::read_dir(root().join("crates")).expect("crates/");
    let mut seen = Vec::new();
    for entry in crates.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Ok(manifest) = std::fs::read_to_string(entry.path().join("Cargo.toml")) else {
            continue;
        };
        let Some((_, allowed)) = ALLOWED.iter().find(|(c, _)| *c == name) else {
            panic!("crate {name} has no place in the layers: add it to ALLOWED (and agent.md)");
        };
        for dep in layered_deps(&manifest) {
            assert!(
                allowed.contains(&dep.as_str()),
                "{name} depends on {dep}: only {allowed:?} lie directly below it"
            );
        }
        seen.push(name);
    }
    assert_eq!(seen.len(), ALLOWED.len(), "crates found: {seen:?}");
}

/// Every `.rs` and `.qml` file below `dir`, as paths relative to the repository root.
fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for path in entries.filter_map(Result::ok).map(|e| e.path()) {
        let relative = path.strip_prefix(root()).unwrap_or(&path).to_path_buf();
        if relative.starts_with(VENDORED) {
            continue;
        }
        if path.is_dir() {
            sources(&path, out);
            continue;
        }
        if path.extension().is_some_and(|x| x == "rs" || x == "qml") {
            out.push(relative);
        }
    }
}

#[test]
fn files_stay_in_budget() {
    let mut files = Vec::new();
    sources(&root().join("crates"), &mut files);
    sources(&root().join("tests"), &mut files);
    assert!(files.len() > 100, "found only {} files", files.len());

    for file in &files {
        let text = std::fs::read_to_string(root().join(file)).expect("readable");
        let lines = text.lines().count();
        let name = file.to_string_lossy().replace('\\', "/");
        match OVER_BUDGET.iter().find(|(f, _)| *f == name) {
            Some((_, limit)) => {
                assert!(
                    lines <= *limit,
                    "{name}: {lines} lines, its limit is {limit}: split it rather than raise it"
                );
                assert!(
                    lines > BUDGET,
                    "{name}: {lines} lines fit the budget now; drop it from OVER_BUDGET"
                );
            }
            None => assert!(
                lines <= BUDGET,
                "{name}: {lines} lines, over the budget of {BUDGET}: split it"
            ),
        }
    }

    for (name, _) in OVER_BUDGET {
        assert!(
            root().join(name).exists(),
            "{name} is gone: drop it from OVER_BUDGET"
        );
    }
}
