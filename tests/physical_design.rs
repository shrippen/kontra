//! Physical design (Harmonicon W5): the layer rule of ARCHITECTURE.md as a test. File and function
//! sizes are checked by `tools/structure-check.py` (`structure.baseline`).
//!
//! ```text
//! app ─► appcore ─┐
//!                 ├─► session ─┬─► engine ─┬─► drivers
//! cli ────────────┘            │           ├─► dsp
//!                              └─► content └─► stretch
//! model: domain types, every crate but app, stretch and the test plugins may use it
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
    ("drivers", &["model"]),
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
            panic!(
                "crate {name} has no place in the layers: add it to ALLOWED (and ARCHITECTURE.md)"
            );
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

/// Driver names session still passes on to appcore and cli. An entry goes when session stops
/// passing the name on.
const DRIVER_NAMES_PASSED_ON: [&str; 0] = [];

/// Engine names (drivers' included) session still passes on to the CLI: the detection corpus
/// tools (`service.rs`). Their types hold plain data and private counting methods of the
/// engine's corpus code, so they neither moved to `model` nor got a twin in session yet.
/// An entry goes when session stops passing the name on.
const ENGINE_NAMES_PASSED_ON: [&str; 29] = [
    "Annotation",
    "ChordTally",
    "DoubleStopTally",
    "EndTally",
    "Evaluation",
    "Expecting",
    "KorpusItem",
    "KorpusSet",
    "LOG_B_PER_FRET",
    "Matching",
    "Mistakes",
    "Pick",
    "Playing",
    "StringModel",
    "announce_second_tone",
    "double_stop_mixtures",
    "evaluate",
    "evaluate_double_stops",
    "evaluate_ends",
    "evaluate_expecting",
    "evaluate_laptop_jack",
    "evaluate_with",
    "generate",
    "generate_double_stops",
    "generate_midi_chords",
    "generate_staccato",
    "render_performance",
    "same_pitch",
    "score_chords",
];
/// The names a `pub use <prefix>…;` statement brings in, by their first path segment after the
/// prefix, aliases off. Example: `pub use kontra_engine::{a::B, C as D};` → "a", "C".
fn reexported(source: &str, prefix: &str) -> Vec<String> {
    let mut names = Vec::new();
    for statement in source.split(';') {
        let Some((_, rest)) = statement.split_once(&format!("pub use {prefix}::")) else {
            continue;
        };
        let rest = rest.trim().trim_start_matches('{').trim_end_matches('}');
        for item in rest.split(',') {
            let item = item.split(" as ").next().unwrap_or_default().trim();
            let first = item.split("::").next().unwrap_or_default().trim();
            if !first.is_empty() {
                names.push(first.to_string());
            }
        }
    }
    names
}

/// The `(name, alias)` pairs of `use <prefix>::name as alias;` and `use <prefix>::{name as
/// alias, …};` statements; `use <prefix> as alias;` gives ("", alias).
/// Example: `use kontra_engine::plugins as host;` → ("plugins", "host").
fn aliases(source: &str, prefix: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    for statement in source.split(';') {
        let statement = statement.trim();
        let Some(at) = statement.rfind("use ") else {
            continue;
        };
        let rest = statement[at + "use ".len()..].trim();
        if let Some(alias) = rest
            .strip_prefix(prefix)
            .and_then(|r| r.strip_prefix(" as "))
        {
            pairs.push((String::new(), alias.trim().to_string()));
            continue;
        }
        let Some(rest) = rest.strip_prefix(&format!("{prefix}::")) else {
            continue;
        };
        let rest = rest.trim().trim_start_matches('{').trim_end_matches('}');
        for item in rest.split(',') {
            let Some((name, alias)) = item.split_once(" as ") else {
                continue;
            };
            pairs.push((name.trim().to_string(), alias.trim().to_string()));
        }
    }
    pairs
}

/// Every `.rs` file under `dir`.
fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return files;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            files.extend(rust_files(&path));
        } else if path.extension().is_some_and(|e| e == "rs") {
            files.push(path);
        }
    }
    files
}

#[test]
fn reads_reexports() {
    let source = "pub use kontra_engine::plugins;\n\
                  pub use kontra_engine::{A, b::C, D as E};\n\
                  pub use other::F;\n";
    assert_eq!(
        reexported(source, "kontra_engine"),
        ["plugins", "A", "b", "D"]
    );
    let aliased = "use kontra_engine::plugins as host;\npub use host::{A, B};\n";
    assert_eq!(
        aliases(aliased, "kontra_engine"),
        [("plugins".to_string(), "host".to_string())]
    );
    let braced = "use kontra_engine::{A, plugins as host, B as C};\nuse kontra_engine as e;\n";
    assert_eq!(
        aliases(braced, "kontra_engine"),
        [
            ("plugins".to_string(), "host".to_string()),
            ("B".to_string(), "C".to_string()),
            (String::new(), "e".to_string()),
        ]
    );
}

/// The layer rule holds in the code, not only in `Cargo.toml`: session passes no driver name on
/// that engine passes on from `kontra_drivers`, and no engine name beyond the corpus tools of
/// `ENGINE_NAMES_PASSED_ON`; it wraps them in functions and types of its own.
#[test]
fn session_hides_the_drivers() {
    let engine = std::fs::read_to_string(root().join("crates/engine/src/lib.rs")).expect("engine");
    let driver_names = reexported(&engine, "kontra_drivers");
    assert!(
        driver_names.len() > 10,
        "engine's driver names: {driver_names:?}"
    );

    let mut engine_passed = Vec::new();
    for file in rust_files(&root().join("crates/session/src")) {
        let source = std::fs::read_to_string(&file).expect("session source");
        engine_passed.extend(reexported(&source, "kontra_engine"));
        // A module under another name (`use kontra_engine::plugins as host;`) passes on what
        // `pub use host::…` names, as the module itself. The crate under another name
        // (`use kontra_engine as e;`) passes on what `pub use e::…` names.
        for (name, alias) in aliases(&source, "kontra_engine") {
            if name.is_empty() {
                engine_passed.extend(reexported(&source, &alias));
                continue;
            }
            if !reexported(&source, &alias).is_empty() {
                engine_passed.push(name);
            }
        }
    }
    let passed_on: Vec<String> = engine_passed
        .iter()
        .filter(|n| driver_names.contains(n))
        .cloned()
        .collect();

    for name in &engine_passed {
        assert!(
            ENGINE_NAMES_PASSED_ON.contains(&name.as_str()),
            "session passes the engine name {name} on: wrap it in a function or type of its own"
        );
    }
    for name in ENGINE_NAMES_PASSED_ON {
        assert!(
            engine_passed.iter().any(|n| n == name),
            "session no longer passes {name} on: remove it from ENGINE_NAMES_PASSED_ON"
        );
    }

    for name in &passed_on {
        assert!(
            DRIVER_NAMES_PASSED_ON.contains(&name.as_str()),
            "session passes the driver name {name} on: wrap it in a function of its own"
        );
    }
    for name in DRIVER_NAMES_PASSED_ON {
        assert!(
            passed_on.iter().any(|n| n == name),
            "session no longer passes {name} on: remove it from DRIVER_NAMES_PASSED_ON"
        );
    }
}
