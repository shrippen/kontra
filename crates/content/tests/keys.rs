//! Key signatures from every format, per bar and in concert pitch.

// Test helpers outside #[test] functions may panic too.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use kontra_content::{import_bytes, FileKind};
use kontra_model::Arrangement;
use midly::num::{u15, u28, u4, u7};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};

fn keys(a: &Arrangement) -> Vec<i8> {
    a.bars.iter().map(|b| b.key_fifths).collect()
}

/// One bass part; `attributes[i]` goes into measure `i`, which holds one whole E.
fn musicxml(attributes: &[&str], extra: &[&str]) -> String {
    let mut measures = String::new();
    for (i, (attributes, extra)) in attributes.iter().zip(extra).enumerate() {
        measures.push_str(&format!(
            r#"<measure number="{n}"><attributes>{attributes}</attributes>{extra}<note><pitch><step>E</step><octave>1</octave></pitch><duration>4</duration></note></measure>"#,
            n = i + 1,
        ));
    }
    format!(
        r#"<?xml version="1.0"?><score-partwise><part-list><score-part id="P1"><part-name>Bass</part-name></score-part></part-list><part id="P1">{measures}</part></score-partwise>"#
    )
}

const FIRST: &str =
    "<divisions>1</divisions><key><fifths>-2</fifths><mode>major</mode></key><time><beats>4</beats><beat-type>4</beat-type></time>";

#[test]
fn musicxml_key_per_bar_through_repeats() {
    // |: B♭ major | G major :| (no key) → played B♭ G B♭ G G.
    let text = musicxml(
        &[FIRST, "<key><fifths>1</fifths></key>", ""],
        &[
            r#"<barline location="left"><repeat direction="forward"/></barline>"#,
            r#"<barline location="right"><repeat direction="backward"/></barline>"#,
            "",
        ],
    );
    let a = import_bytes(text.as_bytes(), FileKind::MusicXml)
        .unwrap()
        .arrangement(0)
        .unwrap();
    assert_eq!(keys(&a), [-2, 1, -2, 1, 1]);
}

#[test]
fn musicxml_key_of_transposing_part_is_concert() {
    // B♭ instrument written in C major sounds in B♭ major; an octave shift keeps the key.
    let clarinet = "<divisions>1</divisions><key><fifths>0</fifths></key><transpose><diatonic>-1</diatonic><chromatic>-2</chromatic></transpose>";
    let text = musicxml(&[clarinet], &[""]);
    let a = import_bytes(text.as_bytes(), FileKind::MusicXml)
        .unwrap()
        .arrangement(0)
        .unwrap();
    assert_eq!(keys(&a), [-2]);

    let bass = "<divisions>1</divisions><key><fifths>-2</fifths></key><transpose><diatonic>0</diatonic><chromatic>0</chromatic><octave-change>-1</octave-change></transpose>";
    let text = musicxml(&[bass], &[""]);
    let a = import_bytes(text.as_bytes(), FileKind::MusicXml)
        .unwrap()
        .arrangement(0)
        .unwrap();
    assert_eq!(keys(&a), [-2]);
}

#[test]
fn musicxml_octave_clef_keeps_sounding_pitch() {
    // A bass clef with "8vb" only changes where notes are drawn; <pitch> stays sounding,
    // so E1 imports as E1 (MIDI 28), not an octave lower.
    let clef = "<divisions>1</divisions><clef><sign>F</sign><line>4</line><clef-octave-change>-1</clef-octave-change></clef>";
    let text = musicxml(&[clef], &[""]);
    let a = import_bytes(text.as_bytes(), FileKind::MusicXml)
        .unwrap()
        .arrangement(0)
        .unwrap();
    assert_eq!(a.notes[0].pitch.0, 28);
}

#[test]
fn gp5_song_key_and_bar_key() {
    let mut song = common::bass_song();
    song.key = -2;
    song.heads[1].key = Some(3);
    let data = common::gp5(&song);
    let imported = import_bytes(&data, FileKind::GuitarPro).unwrap();
    let a = imported.arrangement(1).unwrap();
    assert_eq!(keys(&a), [-2, 3, -2, 3]);
}

#[test]
fn gpif_key_per_master_bar() {
    let text = common::gpif_bars(&[
        "<Key><AccidentalCount>-3</AccidentalCount><Mode>Major</Mode></Key>",
        "",
        "<Key><AccidentalCount>2</AccidentalCount><Mode>Minor</Mode></Key>",
    ]);
    let data = common::zip(
        &[("Content/score.gpif", text.as_bytes())],
        common::Method::Stored,
    );
    let a = import_bytes(&data, FileKind::GuitarPro7)
        .unwrap()
        .arrangement(0)
        .unwrap();
    assert_eq!(keys(&a), [-3, -3, 2]);
}

fn event(delta: u32, kind: TrackEventKind<'static>) -> TrackEvent<'static> {
    TrackEvent {
        delta: u28::new(delta),
        kind,
    }
}

fn note(key: u8, on: bool) -> TrackEventKind<'static> {
    let vel = u7::new(if on { 100 } else { 0 });
    TrackEventKind::Midi {
        channel: u4::new(0),
        message: MidiMessage::NoteOn {
            key: u7::new(key),
            vel,
        },
    }
}

#[test]
fn midi_key_signature_events() {
    // 480 PPQ, 4/4: E♭ major from bar 1, A major from bar 3.
    let header = Header::new(Format::SingleTrack, Timing::Metrical(u15::new(480)));
    let mut smf = Smf::new(header);
    let key = |fifths, at| {
        event(
            at,
            TrackEventKind::Meta(MetaMessage::KeySignature(fifths, false)),
        )
    };
    let mut track = vec![key(-3, 0)];
    for bar in 0..3 {
        if bar == 2 {
            track.push(key(3, 0));
        }
        track.push(event(0, note(28, true)));
        track.push(event(1920, note(28, false)));
    }
    track.push(event(0, TrackEventKind::Meta(MetaMessage::EndOfTrack)));
    smf.tracks.push(track);
    let mut data = Vec::new();
    smf.write_std(&mut data).unwrap();

    let a = import_bytes(&data, FileKind::Midi)
        .unwrap()
        .arrangement(0)
        .unwrap();
    assert_eq!(keys(&a), [-3, -3, 3]);
}
