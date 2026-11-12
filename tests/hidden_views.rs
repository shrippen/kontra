//! Hidden views stand still (benchmark 9.11.2026): the practice page keeps lane and tab and
//! makes notation, combined and keys only while chosen (#303); it shows one. A hidden view that
//! follows the position redraws on every tick: in the notation view the hidden tab lane alone
//! took 0.18 ms of each 16 ms tick.
//!
//! ```text
//! ctl.position ─► shown view   (moves)
//!              ─► hidden views (0, still)
//! ```
//!
//! The page needs the Rust bridge to load, so the test reads its source: each view's `position`
//! binding asks for visibility first.

// Test helpers: a missing file fails the test.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

const PAGE: &str = "crates/app/qml/PracticePage.qml";
/// The views of the practice page, by QML id, with the item whose visibility they follow
/// (empty: their own).
const VIEWS: [(&str, &str); 6] = [
    ("lane", ""),
    ("tabStaff", ""),
    ("bassStaff", ""),
    ("combinedBass", "combined."),
    ("combinedTab", "combined."),
    ("keyLane", "keysView."),
];

fn page() -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read_to_string(root.join(PAGE)).unwrap()
}

/// The `position:` binding of the item with `id`: the first one after its id.
fn position_of<'a>(text: &'a str, id: &str) -> &'a str {
    let start = text
        .find(&format!("id: {id}\n"))
        .unwrap_or_else(|| panic!("no item {id} in {PAGE}"));
    text[start..]
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("position:"))
        .unwrap_or_else(|| panic!("{id} has no position"))
}

#[test]
fn hidden_views_skip_position() {
    let text = page();
    let moving: Vec<String> = VIEWS
        .iter()
        .map(|(id, owner)| (id, owner, position_of(&text, id)))
        // Exactly this: a reversed condition would stop the shown view (review #228).
        .filter(|(_, owner, binding)| {
            *binding != format!("position: {owner}visible ? ctl.position : 0")
        })
        .map(|(id, _, binding)| format!("{id}: {binding}"))
        .collect();
    assert!(
        moving.is_empty(),
        "views that move while hidden: {moving:?}"
    );
}
