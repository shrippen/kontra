//! Harmonics sound at a partial of the string (LH-BIB-01): a natural harmonic at fret 5 of the
//! E string is E3, not the fretted A1; an artificial one sounds over the fretted note.

// Test helpers outside #[test] functions may panic too.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use kontra_content::{import_bytes, FileKind};
use kontra_model::{Arrangement, Pitch, Technique};

/// General MIDI "fingered bass": the fixture's bass track.
const BASS_PROGRAM: i32 = 33;

// Guitar Pro 5 harmonic kinds.
const NATURAL: u8 = 1;
const ARTIFICIAL: u8 = 2;
const TAPPED: u8 = 3;
const PINCH: u8 = 4;

/// Pitch classes and octaves of a GP5 artificial harmonic.
const CLASS_A: u8 = 9;
const CLASS_E: u8 = 4;
const CLASS_B: u8 = 11;
const FLAT: u8 = 0xff;
const LOCO: u8 = 0;
const OTTAVA: u8 = 1;
/// 8vb: GP5's octaves 3 and 4 point down.
const OTTAVA_BASSA: u8 = 3;

/// Fret 5 of the E string.
const A1: Pitch = Pitch(33);
const A2: Pitch = Pitch(45);
const E3: Pitch = Pitch(52);

/// The bass track of the GP5 fixture with its first note at `fret` of the E string, played as
/// the harmonic stored in `bytes`.
fn gp5_harmonic(fret: i8, bytes: &'static [u8]) -> Arrangement {
    let mut song = common::bass_song();
    let bass = song
        .tracks
        .iter()
        .position(|t| t.program == BASS_PROGRAM)
        .unwrap();
    song.tracks[bass].bars[0][0].notes[0] = common::GpNote {
        harmonic: bytes,
        ..common::note(4, fret)
    };
    import_bytes(&common::gp5(&song), FileKind::GuitarPro)
        .unwrap()
        .arrangement(bass)
        .unwrap()
}

/// The GPIF fixture with its first note (fret 5 of the E string) given these extra properties.
fn gpif_harmonic(properties: &str) -> Arrangement {
    let fretted = r#"<Property name="Fret"><Fret>5</Fret></Property></Properties></Note>"#;
    let score = common::gpif();
    assert!(score.contains(fretted));
    let harmonic = fretted.replace("</Properties>", &format!("{properties}</Properties>"));
    let score = score.replacen(fretted, &harmonic, 1);
    let file = common::zip(
        &[("Content/score.gpif", score.as_bytes())],
        common::Method::Stored,
    );
    import_bytes(&file, FileKind::GuitarPro7)
        .unwrap()
        .arrangement(1)
        .unwrap()
}

/// One bass part (E A D G, written an octave up) with `notes` in its only measure.
fn musicxml(notes: &str) -> Arrangement {
    let text = format!(
        r#"<?xml version="1.0"?><score-partwise><part-list><score-part id="P1"><part-name>Bass</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time><staff-details><staff-lines>4</staff-lines><staff-tuning line="1"><tuning-step>E</tuning-step><tuning-octave>1</tuning-octave></staff-tuning><staff-tuning line="2"><tuning-step>A</tuning-step><tuning-octave>1</tuning-octave></staff-tuning><staff-tuning line="3"><tuning-step>D</tuning-step><tuning-octave>2</tuning-octave></staff-tuning><staff-tuning line="4"><tuning-step>G</tuning-step><tuning-octave>2</tuning-octave></staff-tuning></staff-details><transpose><diatonic>0</diatonic><chromatic>0</chromatic><octave-change>-1</octave-change></transpose></attributes>{notes}</measure></part></score-partwise>"#
    );
    import_bytes(text.as_bytes(), FileKind::MusicXml)
        .unwrap()
        .arrangement(0)
        .unwrap()
}

/// A whole note at a written pitch with optional notehead and technical content.
fn xml_note(chord: bool, step: &str, octave: u8, head: &str, technical: &str) -> String {
    let chord = if chord { "<chord/>" } else { "" };
    format!(
        r#"<note>{chord}<pitch><step>{step}</step><octave>{octave}</octave></pitch><duration>4</duration>{head}<notations><technical>{technical}</technical></notations></note>"#
    )
}

/// The only expected note: its sounding pitch and tab position.
fn single(a: &Arrangement) -> (Pitch, Option<u8>, Option<u8>) {
    assert_eq!(a.notes.len(), 1, "one note to play: {:?}", a.notes);
    let note = &a.notes[0];
    assert!(note.techniques.contains(&Technique::Harmonic));
    (note.pitch, note.string, note.fret)
}

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

#[test]
fn gp5_natural_harmonic_sounds_at_its_partial() {
    let mut song = common::bass_song();
    let bass = song
        .tracks
        .iter()
        .position(|t| t.program == BASS_PROGRAM)
        .unwrap();
    song.tracks[bass].bars[0][0].notes[0] = common::GpNote {
        harmonic: &[NATURAL],
        ..common::note(4, 5)
    };
    let a = import_bytes(&common::gp5(&song), FileKind::GuitarPro)
        .unwrap()
        .arrangement(bass)
        .unwrap();
    assert_eq!(
        a.notes[0].pitch,
        Pitch(52),
        "E3, two octaves over the E string"
    );
    assert_eq!(a.notes[0].fret, Some(5));
    assert!(a.notes[0].techniques.contains(&Technique::Harmonic));
}

#[test]
fn gp5_artificial_harmonic_sounds_over_the_fretted_note() {
    // A1 fretted, A touched an octave up (AH12).
    let a = gp5_harmonic(5, &[ARTIFICIAL, CLASS_A, 0, OTTAVA]);
    assert_eq!((a.notes[0].pitch, a.notes[0].fret), (A2, Some(5)));

    // E over A1: a fifth above the octave (AH7), E3, not the E2 a fifth above A1.
    let a = gp5_harmonic(5, &[ARTIFICIAL, CLASS_E, 0, OTTAVA]);
    assert_eq!(a.notes[0].pitch, E3);

    // B♭1 at fret 6, B flat an octave up.
    let a = gp5_harmonic(6, &[ARTIFICIAL, CLASS_B, FLAT, OTTAVA]);
    assert_eq!(a.notes[0].pitch, Pitch(46));
    assert!(a.notes[0].techniques.contains(&Technique::Harmonic));
}

#[test]
fn gp5_harmonic_below_keeps_the_fretted_pitch() {
    // A over A1 marked 8vb: no harmonic sounds below the string, so it stays A1 (review 13.10.:
    // read as three octaves up).
    let a = gp5_harmonic(5, &[ARTIFICIAL, CLASS_A, 0, OTTAVA_BASSA]);
    assert_eq!(a.notes[0].pitch, A1);
}

#[test]
fn gp5_harmonic_without_partial_keeps_the_fretted_pitch() {
    // C over A1 in the same octave is no partial: unsure, so the fretted pitch stays.
    let a = gp5_harmonic(5, &[ARTIFICIAL, 0, 0, LOCO]);
    assert_eq!(a.notes[0].pitch, A1);

    // A pinch harmonic sounds where the pick touches: not stored.
    let a = gp5_harmonic(5, &[PINCH]);
    assert_eq!(a.notes[0].pitch, A1);
    assert!(a.notes[0].techniques.contains(&Technique::Harmonic));
}

#[test]
fn gp5_tapped_harmonic_sounds_over_the_fretted_note() {
    // Tapped 12 frets above A1.
    let a = gp5_harmonic(5, &[TAPPED, 12]);
    assert_eq!((a.notes[0].pitch, a.notes[0].fret), (A2, Some(5)));
}

#[test]
fn gpif_artificial_and_tapped_harmonics() {
    let artificial = gpif_harmonic(
        r#"<Property name="HarmonicType"><HType>Artificial</HType></Property><Property name="HarmonicFret"><HFret>12</HFret></Property>"#,
    );
    assert_eq!(single_first(&artificial), (A2, Some(5)));

    let tapped = gpif_harmonic(
        r#"<Property name="HarmonicType"><HType>Tap</HType></Property><Property name="HarmonicFret"><HFret>7</HFret></Property>"#,
    );
    assert_eq!(single_first(&tapped), (E3, Some(5)));

    let pinch = gpif_harmonic(
        r#"<Property name="HarmonicType"><HType>Pinch</HType></Property><Property name="HarmonicFret"><HFret>12</HFret></Property>"#,
    );
    assert_eq!(single_first(&pinch), (A1, Some(5)), "pinch: not stored");
}

/// Sounding pitch and fret of the first note, which must be a harmonic.
fn single_first(a: &Arrangement) -> (Pitch, Option<u8>) {
    assert!(a.notes[0].techniques.contains(&Technique::Harmonic));
    (a.notes[0].pitch, a.notes[0].fret)
}

#[test]
fn musicxml_natural_harmonic_from_the_tab() {
    // Written as the touched A2 (A1 sounding) with a diamond head, tab fret 5 of the E string.
    let note = xml_note(
        false,
        "A",
        2,
        "<notehead>diamond</notehead>",
        "<harmonic><natural/><touching-pitch/></harmonic><string>4</string><fret>5</fret>",
    );
    assert_eq!(single(&musicxml(&note)), (E3, Some(0), Some(5)));

    // Same without <harmonic>: the diamond head alone marks it (some exporters).
    let note = xml_note(
        false,
        "A",
        2,
        r#"<notehead filled="no">diamond</notehead>"#,
        "<string>4</string><fret>5</fret>",
    );
    assert_eq!(single(&musicxml(&note)), (E3, Some(0), Some(5)));
}

#[test]
fn musicxml_sounding_pitch_wins() {
    // Natural harmonic written at its sounding pitch E3 (written E4).
    let note = xml_note(
        false,
        "E",
        4,
        "",
        "<harmonic><natural/><sounding-pitch/></harmonic>",
    );
    assert_eq!(single(&musicxml(&note)).0, E3);
}

#[test]
fn musicxml_artificial_harmonic_as_chord() {
    // Spec layout: base A1 (tab fret 5), touching D2 (diamond), hidden sounding A3.
    let base = xml_note(
        false,
        "A",
        2,
        "",
        "<harmonic><artificial/><base-pitch/></harmonic><string>4</string><fret>5</fret>",
    );
    let touching = xml_note(
        true,
        "D",
        3,
        r#"<notehead filled="no">diamond</notehead>"#,
        "<harmonic><artificial/><touching-pitch/></harmonic>",
    );
    let sounding = xml_note(
        true,
        "A",
        4,
        "",
        "<harmonic><artificial/><sounding-pitch/></harmonic>",
    )
    .replace("<note>", r#"<note print-object="no">"#);
    let all = format!("{base}{touching}{sounding}");
    assert_eq!(single(&musicxml(&all)), (Pitch(57), Some(0), Some(5)));

    // Without the sounding note: base and touching (a fourth up → two octaves).
    let pair = format!("{base}{touching}");
    assert_eq!(single(&musicxml(&pair)), (Pitch(57), Some(0), Some(5)));
}
