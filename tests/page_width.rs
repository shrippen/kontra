//! Every page but the practice page keeps the same reading width (owner, 12.11.2026): pages of
//! different width made the layout jump between Learn, Quiz and Ear.
//!
//! The pages need the Rust bridge to load, so the test reads their source.

// Test helpers: a missing file fails the test.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

const QML: &str = "crates/app/qml";
/// Full width: the notation view.
const FULL_WIDTH_PAGE: &str = "PracticePage.qml";
const LEFT: &str = "leftPadding: applicationWindow().readingPad(width)";
const RIGHT: &str = "rightPadding: leftPadding";

fn read(path: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read_to_string(root.join(path)).unwrap()
}

/// The page files listed in `pages` of Main.qml.
fn page_files() -> Vec<String> {
    let main = read(&format!("{QML}/Main.qml"));
    let start = main.find("readonly property var pages").unwrap();
    let block = &main[start..start + main[start..].find("})").unwrap()];
    block
        .split("Qt.resolvedUrl(\"")
        .skip(1)
        .map(|rest| rest[..rest.find('"').unwrap()].to_string())
        .collect()
}

#[test]
fn pages_share_reading_width() {
    let files = page_files();
    assert!(files.len() > 5, "pages not found in Main.qml: {files:?}");
    let missing: Vec<_> = files
        .iter()
        .filter(|file| *file != FULL_WIDTH_PAGE)
        .filter(|file| {
            let text = read(&format!("{QML}/{file}"));
            !(text.contains(LEFT) && text.contains(RIGHT))
        })
        .collect();
    assert!(
        missing.is_empty(),
        "pages without the reading width: {missing:?}"
    );
}
