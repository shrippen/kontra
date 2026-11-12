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
/// A density suggestion also comes after each pass of a loop, while playing on.
const RUN_STATE: [&str; 5] = [
    "ctl.playing",
    "ctl.holdingIndex",
    "ctl.densitySuggestion",
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

/// The mixer is a layer over the notes (owner 12.11.): it slid in as a row of the page's column
/// and shrank the notes on every frame, a relayout each (FPS dropped). It sits in the notes
/// area's overlay now, whose room it does not change.
#[test]
fn mixer_lies_over_the_notes() {
    let text = page();
    let overlay = text
        .find("overlay: MixerCard {")
        .unwrap_or_else(|| panic!("the mixer is no overlay of the notes in {PAGE}"));
    assert_eq!(
        text.matches("MixerCard {").count(),
        1,
        "a second mixer at {overlay}"
    );
}

/// F5 changes the room, the spacing (presentation scale) and the window size within a moment;
/// marked as a resize, the staffs lay out once when it rests instead of up to three times.
#[test]
fn presentation_settles_like_a_resize() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let main = std::fs::read_to_string(root.join("crates/app/qml/Main.qml")).unwrap();
    let start = main
        .find("onPresentingChanged: {")
        .unwrap_or_else(|| panic!("no onPresentingChanged in Main.qml"));
    let body = &main[start..start + main[start..].find("\n    }").unwrap_or(0)];
    assert!(
        body.contains("noteResize()"),
        "the presentation does not count as a resize"
    );
}

/// The presentation hides the mixer button; a mixer open before F5 lay over the top string with
/// no button to close it (review #306). It closes with the presentation.
#[test]
fn mixer_leaves_the_presentation() {
    let text = page();
    let start = text.find("overlay: MixerCard {").unwrap_or(0);
    let open = text[start..]
        .lines()
        .find(|line| line.trim_start().starts_with("open:"))
        .unwrap_or_else(|| panic!("the mixer has no open binding in {PAGE}"));
    assert!(
        open.contains("!page.presenting"),
        "open in the presentation: {}",
        open.trim()
    );

    // "X" would flip the hidden mixer, which then shows (or stays shut) after Esc.
    let key = text
        .lines()
        .find(|line| line.contains("sequence: \"X\""))
        .unwrap_or_else(|| panic!("no X shortcut in {PAGE}"));
    assert!(
        key.contains("!page.presenting"),
        "X toggles the mixer in the presentation: {}",
        key.trim()
    );
}
