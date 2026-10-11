//! Natural harmonics sound at a partial of the string (LH-BIB-01): a harmonic at fret 5 of the
//! E string is E3, not the fretted A1.

// Test helpers outside #[test] functions may panic too.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use kontra_content::{import_bytes, FileKind};
use kontra_model::{Pitch, Technique};

#[test]
fn gpif_natural_harmonic_sounds_at_its_partial() {
    let fretted = r#"<Note id="0"><Properties><Property name="String"><String>0</String></Property><Property name="Fret"><Fret>5</Fret></Property></Properties></Note>"#;
    let harmonic = r#"<Note id="0"><Properties><Property name="String"><String>0</String></Property><Property name="Fret"><Fret>5</Fret></Property><Property name="HarmonicType"><HType>Natural</HType></Property><Property name="HarmonicFret"><HFret>5</HFret></Property></Properties></Note>"#;
    let score = common::gpif();
    assert!(score.contains(fretted));
    let score = score.replace(fretted, harmonic);
    let file = common::zip(
        &[("Content/score.gpif", score.as_bytes())],
        common::Method::Stored,
    );
    let a = import_bytes(&file, FileKind::GuitarPro7)
        .unwrap()
        .arrangement(1)
        .unwrap();
    assert_eq!(
        a.notes[0].pitch,
        Pitch(52),
        "E3, two octaves over the E string"
    );
    assert_eq!(a.notes[0].fret, Some(5), "the tab keeps the touched fret");
    assert!(a.notes[0].techniques.contains(&Technique::Harmonic));
}
