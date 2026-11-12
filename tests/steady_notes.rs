//! The notes stay put while playing (owner 12.11.2026): pressing play moved them, and so did
//! lines that came and went during a run. What shows above the notes on the practice page may
//! follow the player's choices (mode, view, mixer), not the run: no row of the page's column
//! shows by `ctl.playing`, a note being waited for or a run's end, and the low window's `lean`
//! does not follow the run either.
//! The footer under the notes has its own test (`tst_practicefooter.qml`).
//!
//! The page needs the Rust bridge to load, so the test reads its source.

// Test helpers: a missing file fails the test.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

const PAGE: &str = "crates/app/qml/PracticePage.qml";
/// What changes with a run, not by the player's choice.
const RUN_STATE: [&str; 4] = [
    "ctl.playing",
    "ctl.holdingIndex",
    "page.summaryReady",
    "page.lean",
];
/// A row of the page's column sets its visibility at this indent; deeper items sit in a row
/// whose height they do not change.
const ROW_VISIBLE: &str = "            visible:";

fn page() -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read_to_string(root.join(PAGE)).unwrap()
}

#[test]
fn lean_ignores_the_run() {
    let text = page();
    let lean = text
        .lines()
        .find(|line| line.contains("readonly property bool lean:"))
        .unwrap_or_else(|| panic!("no lean in {PAGE}"));
    assert!(
        !lean.contains("playing"),
        "lean follows the run: {}",
        lean.trim()
    );
}

#[test]
fn nothing_shows_by_the_run() {
    let text = page();
    let found: Vec<String> = text
        .lines()
        .enumerate()
        .filter(|(_, line)| line.starts_with(ROW_VISIBLE))
        .filter(|(_, line)| RUN_STATE.iter().any(|state| line.contains(state)))
        .filter(|(_, line)| !line.contains("page.lean") || !lean_is_steady(&text))
        .map(|(index, line)| format!("{}: {}", index + 1, line.trim()))
        .collect();
    assert!(found.is_empty(), "shown by the run:\n{}", found.join("\n"));
}

/// `lean` may hide lines when it does not follow the run itself.
fn lean_is_steady(text: &str) -> bool {
    text.lines()
        .find(|line| line.contains("readonly property bool lean:"))
        .is_some_and(|line| !line.contains("playing") && !line.contains("summaryReady"))
}
