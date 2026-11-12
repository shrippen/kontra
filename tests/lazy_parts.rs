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
/// declares a lazily made component.
fn is_lazy(lines: &[String], at: usize) -> bool {
    let mut level = indent(&lines[at]);
    let lazy = |line: &str| line.contains("sourceComponent:") || line.trim() == "Component {";
    if lazy(&lines[at]) {
        return true;
    }
    for line in lines[..at].iter().rev() {
        if line.trim().is_empty() || indent(line) >= level {
            continue;
        }
        if lazy(line) {
            return true;
        }
        level = indent(line);
    }
    false
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
