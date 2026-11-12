//! What the practice page does not show on opening is made later (owner 12.11.2026: a click on
//! "Practice" stopped the app for 460 ms in a debug build). The mixer card, the summary sheet, the
//! alignment sheet, the file dialog and the views that are not chosen sit in a `Loader`, so
//! they are not built with the page.
//!
//! The page needs the Rust bridge to load, so the test reads its source: each part is declared
//! below a `sourceComponent:` or a `Component {`.

// Test helpers: a missing file fails the test.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

const PAGE: &str = "crates/app/qml/PracticePage.qml";
const MIXER_CARD: &str = "crates/app/qml/MixerCard.qml";
const SUMMARY_LAYER: &str = "crates/app/qml/SummaryLayer.qml";
/// The parts, by file and what to look for: a type (`Mixer {`) or an id (`id: combined`).
const PARTS: [(&str, &str); 9] = [
    (MIXER_CARD, "Mixer {"),
    (SUMMARY_LAYER, "SummarySheet {"),
    (PAGE, "BackingAlign {"),
    (PAGE, "FileDialog {"),
    (PAGE, "NotationStaff {"),
    (PAGE, "id: bassStaff"),
    (PAGE, "id: combined"),
    (PAGE, "id: keysView"),
    (PAGE, "id: keyLane"),
];

fn read(path: &str) -> Vec<String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let text = std::fs::read_to_string(root.join(path)).unwrap();
    text.lines().map(str::to_string).collect()
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// Whether the line or one of its parents (the nearest line above with less indent, and so on)
/// declares a lazily made component: a `Component {`, or a `Loader`'s `sourceComponent:` whose
/// loader waits (`active:` or `asynchronous:`; a loader is active by default and builds at once).
fn is_lazy(lines: &[String], at: usize) -> bool {
    let mut level = indent(&lines[at]) + 1;
    for (index, line) in lines[..=at].iter().enumerate().rev() {
        if line.trim().is_empty() || indent(line) >= level {
            continue;
        }
        if line.trim() == "Component {" {
            return true;
        }
        if line.contains("sourceComponent:") {
            return loader_waits(lines, index);
        }
        level = indent(line);
    }
    false
}

/// Whether the loader around the `sourceComponent:` at `at` sets `active:` or `asynchronous:`.
fn loader_waits(lines: &[String], at: usize) -> bool {
    let own = indent(&lines[at]);
    let Some(start) = lines[..at]
        .iter()
        .rposition(|line| indent(line) < own && line.trim().ends_with('{'))
    else {
        return false;
    };
    lines[start + 1..]
        .iter()
        .take_while(|line| line.trim().is_empty() || indent(line) >= own)
        .filter(|line| indent(line) == own)
        .flat_map(|line| line.split(';'))
        .any(|part| {
            let part = part.trim();
            part.starts_with("active:") || part.starts_with("asynchronous:")
        })
}

#[test]
fn hidden_parts_load_later() {
    let eager: Vec<String> = PARTS
        .iter()
        .filter_map(|(file, part)| {
            let lines = read(file);
            let at = lines
                .iter()
                .position(|l| l.trim() == *part || l.trim().contains(&format!(": {part}")))
                .unwrap_or_else(|| panic!("no {part} in {file}"));
            (!is_lazy(&lines, at)).then(|| format!("{part} in {file}"))
        })
        .collect();
    assert!(eager.is_empty(), "built with the page: {eager:?}");
}
