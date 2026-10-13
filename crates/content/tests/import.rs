//! End-to-end imports of generated fixtures in every supported format.

// Test helpers outside #[test] functions may panic too.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::Method;
use kontra_content::{import_bytes, import_file, FileKind, ImportError};
use kontra_model::{Arrangement, Note, Pitch, Technique};

/// (start, duration, pitch, string, fret, tied)
type Row = (u64, u64, u8, Option<u8>, Option<u8>, bool);

fn rows(a: &Arrangement) -> Vec<Row> {
    a.notes
        .iter()
        .map(|n| {
            (
                n.start_ticks,
                n.duration_ticks,
                n.pitch.0,
                n.string,
                n.fret,
                n.tied,
            )
        })
        .collect()
}

fn has(note: &Note, technique: Technique) -> bool {
    note.techniques.contains(&technique)
}

fn order(a: &Arrangement) -> Vec<u32> {
    a.bars.iter().map(|b| b.source_index).collect()
}

fn tempos(a: &Arrangement) -> Vec<(u64, f64)> {
    a.tempo.changes().iter().map(|c| (c.tick, c.bpm)).collect()
}

#[test]
fn gp5_bass_track() {
    let data = common::gp5(&common::bass_song());
    let imported = import_bytes(&data, FileKind::GuitarPro).unwrap();
    assert_eq!(imported.title(), "Fixture");
    assert_eq!(imported.artist(), "Kontra");
    let names: Vec<&str> = imported.tracks().iter().map(|t| t.name()).collect();
    assert_eq!(names, ["Guitar", "Electric Bass"]);
    assert_eq!(imported.suggested(), Some(1));
    assert!(!imported.tracks()[0].is_bass());
    assert_eq!(imported.tracks()[1].strings(), 4);

    let a = imported.arrangement(1).unwrap();
    assert_eq!(
        a.instrument.tuning.strings,
        [Pitch(28), Pitch(33), Pitch(38), Pitch(43)]
    );
    assert_eq!(a.instrument.tuning.name, "standard-4");
    assert_eq!(order(&a), [0, 1, 0, 1]);
    assert_eq!(tempos(&a), [(0, 120.0), (3840, 90.0), (11520, 90.0)]);
    let markers: Vec<(u64, &str)> = a
        .markers
        .iter()
        .map(|m| (m.tick, m.name.as_str()))
        .collect();
    assert_eq!(markers, [(3840, "Verse"), (11520, "Verse")]);

    let pass: Vec<Row> = vec![
        (0, 960, 28, Some(0), Some(0), false),
        (960, 960, 31, Some(0), Some(3), false),
        (1920, 1920, 35, Some(1), Some(2), false),
        (3840, 960, 35, Some(1), Some(2), true),
        (5760, 480, 38, Some(2), Some(0), false),
        (6240, 480, 40, Some(2), Some(2), false),
        (6720, 960, 28, Some(0), Some(0), false),
    ];
    let second: Vec<Row> = pass
        .iter()
        .map(|r| (r.0 + 7680, r.1, r.2, r.3, r.4, r.5))
        .collect();
    assert_eq!(rows(&a), [pass, second].concat());
    assert!(has(&a.notes[5], Technique::HammerOn));
    assert!(has(&a.notes[6], Technique::Muted));
    assert!(!a.notes[3].needs_attack());
}

#[test]
fn gp5_guitar_track_keeps_its_tuning() {
    let data = common::gp5(&common::bass_song());
    let a = import_bytes(&data, FileKind::GuitarPro)
        .unwrap()
        .arrangement(0)
        .unwrap();
    assert_eq!(a.instrument.tuning.string_count(), 6);
    // String 1 (high E) is index 5; string 2 (B) fret 1 is C4.
    assert_eq!(a.notes[0].pitch, Pitch(60));
    assert_eq!(a.notes[0].string, Some(4));
    assert_eq!(a.notes[1].string, Some(5));
}

fn check_gpif(a: &Arrangement) {
    assert_eq!(order(a), [0, 1, 0, 2]);
    // The tempo of bar 0 applies again on the second pass.
    assert_eq!(tempos(a), [(0, 100.0), (5760, 120.0), (7680, 100.0)]);
    let markers: Vec<(u64, &str)> = a
        .markers
        .iter()
        .map(|m| (m.tick, m.name.as_str()))
        .collect();
    assert_eq!(markers, [(0, "Intro"), (7680, "Intro")]);

    let first: Vec<Row> = vec![
        (0, 960, 33, Some(0), Some(5), false),
        (960, 1440, 35, Some(1), Some(2), false),
        (2400, 480, 36, Some(1), Some(3), false),
        (3840, 1920, 36, Some(1), Some(3), true),
        (5760, 640, 38, Some(1), Some(5), false),
        (6400, 640, 40, Some(1), Some(7), false),
        (7040, 640, 41, Some(2), Some(3), false),
    ];
    let second: Vec<Row> = first[..3]
        .iter()
        .map(|r| (r.0 + 7680, r.1, r.2, r.3, r.4, r.5))
        .collect();
    let last: Row = (11520, 3840, 28, Some(0), Some(0), false);
    assert_eq!(rows(a), [first, second, vec![last]].concat());
    assert!(has(&a.notes[1], Technique::Accent));
    assert!(has(&a.notes[5], Technique::HammerOn));
    assert!(has(&a.notes[6], Technique::Ghost));
    assert!(has(&a.notes[6], Technique::Slap));
    assert!(has(&a.notes[10], Technique::Muted));
}

fn check_gpif_tracks(imported: &kontra_content::ImportResult) {
    assert_eq!(imported.title(), "Gpif Fixture");
    assert_eq!(imported.suggested(), Some(1));
    assert_eq!(imported.tracks()[1].name(), "Tieftöner");
    assert_eq!(imported.tracks()[1].strings(), 4);
    check_gpif(&imported.arrangement(1).unwrap());
}

#[test]
fn gp7_stored_and_deflated() {
    let score = common::gpif();
    for method in [Method::Stored, Method::Deflated] {
        let data = common::zip(
            &[
                ("VERSION", b"7.0".as_slice()),
                ("Content/score.gpif", score.as_bytes()),
            ],
            method,
        );
        check_gpif_tracks(&import_bytes(&data, FileKind::GuitarPro7).unwrap());
    }
}

#[test]
fn gpx_plain_and_compressed() {
    let score = common::gpif();
    let system = common::bcfs(&[
        ("misc.xml", b"<x/>".as_slice()),
        ("score.gpif", score.as_bytes()),
    ]);
    check_gpif_tracks(&import_bytes(&system, FileKind::GuitarPro6).unwrap());

    let compressed = common::bcfz(&system);
    assert!(compressed.len() < system.len() / 2, "back references used");
    check_gpif_tracks(&import_bytes(&compressed, FileKind::GuitarPro6).unwrap());
}

#[test]
fn gpx_without_score_is_an_error() {
    let system = common::bcfs(&[("other.xml", b"<x/>".as_slice())]);
    let result = import_bytes(&system, FileKind::GuitarPro6);
    assert!(matches!(result, Err(ImportError::Malformed(_))));
}

#[test]
fn midi_tracks_tempo_and_tab() {
    let imported = import_bytes(&common::midi(), FileKind::Midi).unwrap();
    let names: Vec<&str> = imported.tracks().iter().map(|t| t.name()).collect();
    assert_eq!(names, ["Piano", "Track 3"]);
    assert_eq!(imported.suggested(), Some(1));
    assert_eq!(imported.tracks()[1].strings(), 0);

    let a = imported.arrangement(1).unwrap();
    assert_eq!(a.bars.len(), 2);
    assert_eq!(a.bars[0].signature.numerator, 3);
    assert_eq!(a.bars[1].start_ticks, 2880);
    assert_eq!(tempos(&a), [(0, 100.0), (2880, 150.0)]);

    let pitches: Vec<(u64, u64, u8)> = a
        .notes
        .iter()
        .map(|n| (n.start_ticks, n.duration_ticks, n.pitch.0))
        .collect();
    assert_eq!(
        pitches,
        [
            (0, 960, 28),
            (960, 960, 33),
            (1920, 960, 35),
            (2880, 960, 40),
            (3840, 1920, 28),
            (3840, 1920, 35)
        ]
    );
    // Generated tab: every note placed, low notes near the nut.
    assert!(a
        .notes
        .iter()
        .all(|n| n.string.is_some() && n.fret.is_some()));
    assert_eq!((a.notes[0].string, a.notes[0].fret), (Some(0), Some(0)));
    assert_eq!((a.notes[1].string, a.notes[1].fret), (Some(1), Some(0)));
    // The closing chord E1+B1 takes two strings: open E and A string fret 2.
    assert_eq!((a.notes[4].string, a.notes[4].fret), (Some(0), Some(0)));
    assert_eq!((a.notes[5].string, a.notes[5].fret), (Some(1), Some(2)));

    // Piano chords keep all their notes.
    let piano = imported.arrangement(0).unwrap();
    assert_eq!(piano.notes.len(), 6);
    assert_eq!(piano.notes[0].pitch, Pitch(60));
    assert_eq!(piano.notes[2].pitch, Pitch(67));
}

fn check_musicxml(imported: &kontra_content::ImportResult) {
    assert_eq!(imported.title(), "Walking & Talking");
    assert_eq!(imported.artist(), "Kontra");
    assert_eq!(imported.suggested(), Some(1));
    assert_eq!(imported.tracks()[1].strings(), 4);

    let a = imported.arrangement(1).unwrap();
    assert_eq!(order(&a), [0, 1, 0, 2]);
    assert_eq!(tempos(&a), [(0, 90.0), (7680, 90.0)]);
    let pass: Vec<(u64, u64, u8, bool)> = vec![
        (0, 960, 28, false),
        (960, 960, 33, false),
        (960, 960, 40, false),
        (1920, 1920, 31, false),
        (3840, 1920, 31, true),
        (6720, 960, 36, false),
    ];
    let mut expected = pass.clone();
    expected.extend(pass.iter().take(4).map(|r| (r.0 + 7680, r.1, r.2, r.3)));
    expected.push((11520, 3840, 28, false));
    let found: Vec<(u64, u64, u8, bool)> = a
        .notes
        .iter()
        .map(|n| (n.start_ticks, n.duration_ticks, n.pitch.0, n.tied))
        .collect();
    assert_eq!(found, expected);

    // Tab from the file where given, generated elsewhere; the chord's E2 avoids the
    // A string the file gave to A1.
    assert_eq!((a.notes[1].string, a.notes[1].fret), (Some(1), Some(0)));
    assert_eq!((a.notes[2].string, a.notes[2].fret), (Some(2), Some(2)));
    assert_eq!((a.notes[3].string, a.notes[3].fret), (Some(0), Some(3)));
    assert!(a.notes.iter().all(|n| n.fret.is_some()));
    assert!(has(&a.notes[3], Technique::Staccato));
}

#[test]
fn musicxml_plain_and_compressed() {
    let text = common::musicxml();
    check_musicxml(&import_bytes(text.as_bytes(), FileKind::MusicXml).unwrap());
    check_musicxml(&import_bytes(&common::mxl(), FileKind::MusicXmlZip).unwrap());
}

#[test]
fn musicxml_timewise_is_unsupported() {
    let text = br#"<?xml version="1.0"?><score-timewise><measure number="1"/></score-timewise>"#;
    let result = import_bytes(text, FileKind::MusicXml);
    assert!(matches!(result, Err(ImportError::Unsupported(_))));
}

#[test]
fn import_file_detects_format_by_content() {
    let dir = std::env::temp_dir().join(format!("kontra-content-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let score = common::gpif();
    let files: Vec<(&str, Vec<u8>)> = vec![
        // Wrong extensions on purpose: content wins.
        ("song.bin", common::gp5(&common::bass_song())),
        (
            "song.gp",
            common::bcfz(&common::bcfs(&[("score.gpif", score.as_bytes())])),
        ),
        (
            "song.mxl",
            common::zip(
                &[("Content/score.gpif", score.as_bytes())],
                Method::Deflated,
            ),
        ),
        ("song.dat", common::midi()),
        ("song.txt", common::musicxml().into_bytes()),
        ("song.zip", common::mxl()),
    ];
    for (name, data) in &files {
        let path = dir.join(name);
        std::fs::write(&path, data).unwrap();
        let imported = import_file(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(imported.suggested().is_some(), "{name}");
    }

    let unknown = dir.join("noise.xyz");
    std::fs::write(&unknown, b"nothing to see").unwrap();
    assert!(matches!(
        import_file(&unknown),
        Err(ImportError::UnknownFormat)
    ));
    assert!(matches!(
        import_file(&dir.join("missing.gp5")),
        Err(ImportError::Io(_))
    ));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn missing_track_is_an_error() {
    let data = common::gp5(&common::bass_song());
    let imported = import_bytes(&data, FileKind::GuitarPro).unwrap();
    assert!(matches!(
        imported.arrangement(7),
        Err(ImportError::NoTrack(7))
    ));
}

#[test]
fn extensions() {
    assert_eq!(FileKind::from_extension("GP5"), Some(FileKind::GuitarPro));
    assert_eq!(FileKind::from_extension("gpx"), Some(FileKind::GuitarPro6));
    assert_eq!(FileKind::from_extension("mxl"), Some(FileKind::MusicXmlZip));
    assert_eq!(FileKind::from_extension("mp3"), None);
}

/// A palm-muted note keeps its pitch: it is not a dead note (x), which has none.
#[test]
fn palm_mute_is_no_dead_note() {
    let score = common::gpif().replace(r#"name="Muted""#, r#"name="PalmMuted""#);
    let data = common::zip(
        &[
            ("VERSION", b"7.0".as_slice()),
            ("Content/score.gpif", score.as_bytes()),
        ],
        Method::Stored,
    );
    let a = import_bytes(&data, FileKind::GuitarPro7)
        .unwrap()
        .arrangement(1)
        .unwrap();
    assert!(!has(&a.notes[10], Technique::Muted));
}
