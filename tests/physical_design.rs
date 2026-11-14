//! Physical design (Harmonicon W5): the layer rule of agent.md as a test. File and function
//! sizes are checked by `tools/structure-check.py` (`structure.baseline`).
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
const ALLOWED: [(&str, &[&str]); 12] = [
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
    // Test plugins for the plugin host; they depend on no Kontra crate.
    ("test-clap-gain", &[]),
    ("test-vst3-gain", &[]),
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Where a line of a `Cargo.toml` stands, for the layers.
enum Section {
    /// Not a layered dependency section (`[package]`, `[dev-dependencies]`, …).
    Other,
    /// `[dependencies]`, `[build-dependencies]`: one dependency per line.
    List,
    /// `[dependencies.engine]`: one dependency, named by the table or its `package`.
    Table(String),
}

impl Section {
    fn of(header: &str) -> Section {
        if header.contains("dev-dependencies") {
            return Section::Other;
        }
        if header.ends_with("dependencies") {
            return Section::List;
        }
        match header.rsplit_once('.') {
            Some((section, key)) if section.ends_with("dependencies") => {
                Section::Table(key.to_string())
            }
            _ => Section::Other,
        }
    }
}

/// The Kontra crates in the dependency sections of a `Cargo.toml` that count for the layers.
/// Example: `kontra-model.workspace = true` under `[dependencies]` → "model"; so do
/// `[dependencies.kontra-model]` and `model = { package = "kontra-model", … }`.
fn layered_deps(manifest: &str) -> Vec<String> {
    let mut deps = Vec::new();
    let mut section = Section::Other;
    for line in manifest.lines() {
        // Comments off: `[dependencies] # …` is still the header.
        let line = line.split('#').next().unwrap_or_default().trim();
        if let Some(header) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            if let Section::Table(name) = &section {
                deps.extend(kontra_name(name));
            }
            section = Section::of(header);
            continue;
        }
        match &mut section {
            Section::Other => {}
            // A renamed dependency counts by its package, not its key.
            Section::List => deps.extend(kontra_name(package(line).unwrap_or(line))),
            Section::Table(name) => {
                if let Some(renamed) = package(line) {
                    *name = renamed.to_string();
                }
            }
        }
    }
    if let Section::Table(name) = &section {
        deps.extend(kontra_name(name));
    }
    deps
}

/// The value of a `package = "…"` key in `line` (either quote), not "package" in a path.
/// Every occurrence counts: `path = "../package/x", package = "kontra-…"` renames.
fn package(line: &str) -> Option<&str> {
    line.match_indices("package").find_map(|(at, _)| {
        let before = &line[..at];
        let key_starts = before.is_empty() || before.ends_with([' ', '{', ',']);
        let value = line[at + "package".len()..]
            .trim_start()
            .strip_prefix('=')?;
        key_starts.then(|| value.trim_start().trim_start_matches(['"', '\'']))
    })
}

/// "engine" from "kontra-engine…" (quoted too); nothing for other crates.
fn kontra_name(text: &str) -> Option<String> {
    let name = text
        .trim_start_matches(['"', '\''])
        .strip_prefix("kontra-")?;
    let end = name
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
        .unwrap_or(name.len());
    Some(name[..end].to_string())
}

#[test]
fn reads_only_layered_sections() {
    let manifest = "[dependencies]\nkontra-model.workspace = true\nkontra-dsp = { path = \"x\" }\n\
                    [dev-dependencies]\nkontra-session.workspace = true\n\
                    [build-dependencies]\nkontra-drivers.workspace = true\n";
    assert_eq!(layered_deps(manifest), ["model", "dsp", "drivers"]);
}

/// Other spellings Cargo takes: a table per dependency, a renamed package, a commented header.
#[test]
fn reads_every_spelling() {
    let manifest = "[dependencies.kontra-engine]\nworkspace = true\n\
                    [dependencies] # layered\nengine = { package = \"kontra-dsp\", path = \"x\" }\n\
                    [dev-dependencies.kontra-session]\nworkspace = true\n";
    assert_eq!(layered_deps(manifest), ["engine", "dsp"]);

    // "package" in a path is no rename; single quotes are TOML too; a table renames as well.
    let manifest = "[dependencies]\nkontra-dsp = { path = \"../package/dsp\" }\n\
                    model = { package = 'kontra-model' }\n\
                    [dependencies.store]\npackage = \"kontra-drivers\"\n\
                    [build-dependencies.kontra-engine]\nworkspace = true\n";
    assert_eq!(
        layered_deps(manifest),
        ["dsp", "model", "drivers", "engine"]
    );
}

/// Review #194: "package" in a path before the rename, quoted keys and tables.
#[test]
fn reads_quoted_and_late_renames() {
    let manifest =
        "[dependencies]\nstore = { path = \"../package/x\", package = \"kontra-drivers\" }\n\
                    \"kontra-dsp\" = { workspace = true }\n\
                    [dependencies.\"kontra-engine\"]\nworkspace = true\n";
    assert_eq!(layered_deps(manifest), ["drivers", "dsp", "engine"]);
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
