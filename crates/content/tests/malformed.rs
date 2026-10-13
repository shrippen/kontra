//! Truncated and corrupted input must give an error (or a valid result), never a panic.

// Test helpers outside #[test] functions may panic too.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{Method, Noise};
use kontra_content::{import_bytes, FileKind};

const ALL: [FileKind; 8] = [
    FileKind::GuitarPro,
    FileKind::GuitarPro6,
    FileKind::GuitarPro7,
    FileKind::Midi,
    FileKind::MusicXml,
    FileKind::MusicXmlZip,
    FileKind::MuseScore,
    FileKind::MuseScoreZip,
];

/// Imports and builds every track; only panics matter here.
fn exercise(data: &[u8], kind: FileKind) {
    let Ok(imported) = import_bytes(data, kind) else {
        return;
    };
    for track in 0..imported.tracks().len() {
        let _ = imported.arrangement(track);
    }
}

fn fixtures() -> Vec<(FileKind, Vec<u8>)> {
    let score = common::gpif();
    vec![
        (FileKind::GuitarPro, common::gp5(&common::bass_song())),
        (
            FileKind::GuitarPro6,
            common::bcfz(&common::bcfs(&[("score.gpif", score.as_bytes())])),
        ),
        (
            FileKind::GuitarPro7,
            common::zip(
                &[("Content/score.gpif", score.as_bytes())],
                Method::Deflated,
            ),
        ),
        (FileKind::Midi, common::midi()),
        (FileKind::MusicXml, common::musicxml().into_bytes()),
        (FileKind::MusicXmlZip, common::mxl()),
        (FileKind::MuseScore, common::mscx().into_bytes()),
        (FileKind::MuseScoreZip, common::mscz()),
    ]
}

#[test]
fn every_truncation_fails_cleanly() {
    for (kind, data) in fixtures() {
        // Every prefix of small files; a stride for the larger ones.
        let step = (data.len() / 2000).max(1);
        for len in (0..data.len()).step_by(step) {
            exercise(&data[..len], kind);
        }
    }
}

#[test]
fn truncated_gp5_reports_error() {
    let data = common::gp5(&common::bass_song());
    for len in [0, 10, 31, 200, data.len() / 2, data.len() - 20] {
        assert!(
            import_bytes(&data[..len], FileKind::GuitarPro).is_err(),
            "{len}"
        );
    }
}

#[test]
fn corrupted_bytes_never_panic() {
    let mut noise = Noise(0x9e37_79b9_7f4a_7c15);
    for (kind, data) in fixtures() {
        for round in 0..300 {
            let damaged = noise.flip(&data, 1 + round % 8);
            exercise(&damaged, kind);
        }
    }
}

#[test]
fn random_bytes_in_every_parser() {
    let mut noise = Noise(42);
    for kind in ALL {
        for len in [0usize, 1, 4, 31, 100, 4096] {
            let data: Vec<u8> = (0..len).map(|_| noise.next() as u8).collect();
            assert!(import_bytes(&data, kind).is_err());
        }
    }
}

#[test]
fn wrong_parser_for_content_fails() {
    for (kind, data) in fixtures() {
        for other in ALL {
            if other == kind {
                continue;
            }
            exercise(&data, other);
        }
    }
}

#[test]
fn magic_headers_with_garbage() {
    let mut noise = Noise(7);
    let heads: [&[u8]; 6] = [
        b"\x18FICHIER GUITAR PRO v5.10",
        b"BCFZ",
        b"BCFS",
        b"MThd\x00\x00\x00\x06\x00\x01\x00\x02\x01\xe0",
        b"PK\x03\x04",
        b"<score-partwise>",
    ];
    for head in heads {
        for len in [0usize, 8, 64, 1024] {
            let mut data = head.to_vec();
            data.extend((0..len).map(|_| noise.next() as u8));
            for kind in ALL {
                exercise(&data, kind);
            }
        }
    }
}

#[test]
fn huge_musescore_tuplet_fails_cleanly() {
    // A tuplet ratio near u64::MAX must not overflow the duration.
    let text = common::mscx().replace(
        "<normalNotes>2</normalNotes>",
        "<normalNotes>18446744073709551615</normalNotes>",
    );
    exercise(text.as_bytes(), FileKind::MuseScore);
}
