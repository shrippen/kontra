//! Chord symbols (`<harmony>`) in MusicXML (QA 27.10., M5; LH-BIB-03).

// Test helpers outside #[test] functions may panic too.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use kontra_content::{import_bytes, FileKind};
use kontra_model::{MarkerKind, TICKS_PER_QUARTER};

const C_MAJOR: &str = "<harmony><root><root-step>C</root-step></root><kind>major</kind></harmony>";
/// B♭m7 on beat 3 (divisions 1: two quarters in).
const B_FLAT_MINOR_7: &str = "<forward><duration>2</duration></forward><harmony><root><root-step>B</root-step><root-alter>-1</root-alter></root><kind text=\"-7\">minor-seventh</kind></harmony><backup><duration>2</duration></backup>";
const F_SHARP_OVER_A_SHARP: &str = "<harmony><root><root-step>F</root-step><root-alter>1</root-alter></root><kind>major</kind><bass><bass-step>A</bass-step><bass-alter>1</bass-alter></bass></harmony>";
const NO_CHORD: &str = "<harmony><root><root-step>C</root-step></root><kind>none</kind></harmony>";

#[test]
fn chord_symbols_become_chord_markers() {
    let first = format!("{C_MAJOR}{B_FLAT_MINOR_7}");
    let text = common::musicxml_bars(&[&first, F_SHARP_OVER_A_SHARP, NO_CHORD]);
    let a = import_bytes(text.as_bytes(), FileKind::MusicXml)
        .unwrap()
        .arrangement(0)
        .unwrap();
    let chords: Vec<(u64, &str)> = a
        .markers
        .iter()
        .filter(|m| m.kind == MarkerKind::Chord)
        .map(|m| (m.tick, m.name.as_str()))
        .collect();
    let bar = 4 * TICKS_PER_QUARTER;
    assert_eq!(
        chords,
        [(0, "C"), (2 * TICKS_PER_QUARTER, "Bbm7"), (bar, "F#/A#")]
    );
}
