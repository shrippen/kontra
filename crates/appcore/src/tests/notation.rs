//! Notation: bars, staff, rests, hand positions, note names.

use super::*;
use crate::summary::hardest_notes;

#[test]
fn written_beats_round_to_sixteenths() {
    let q = kontra_model::TICKS_PER_QUARTER;
    assert_eq!(written_beats(q / 2 - 40), 0.5);
    assert_eq!(written_beats(q), 1.0);
    assert_eq!(written_beats(10), 0.25);
}

#[test]
fn hardest_notes_put_misses_before_timing() {
    let note = |severity: u8, offset: f64| json!({ "severity": severity, "offsetMs": offset });
    let notes = [
        note(0, 5.0),
        note(1, -40.0),
        note(3, 0.0),
        note(1, 60.0),
        note(2, 10.0),
    ];
    let hard = hardest_notes(&notes);
    let order: Vec<f64> = hard
        .iter()
        .map(|n| n["offsetMs"].as_f64().unwrap())
        .collect();
    assert_eq!(order, [0.0, 10.0, 60.0, -40.0]);
}

fn positions_of(frets: &[u8]) -> Vec<u8> {
    let notes: Vec<_> = frets
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let mut n = kontra_model::Note::new(i as u64 * Q, Q, kontra_model::Pitch(40));
            n.fret = Some(*f);
            n
        })
        .collect();
    hand_positions(&notes)
}

#[test]
fn hand_stays_while_notes_fit_and_shifts_ahead() {
    assert_eq!(positions_of(&[3, 5, 0, 6]), [3, 3, 3, 3]);
    // 7 does not fit 3–6; 6–9 holds it and the notes after it.
    assert_eq!(positions_of(&[3, 5, 7, 9, 7]), [3, 3, 6, 6, 6]);
    // Open strings first: the hand waits for the first fretted note.
    assert_eq!(positions_of(&[0, 0, 2, 4]), [2, 2, 2, 2]);
    assert_eq!(positions_of(&[0, 0]), [1, 1]);
}

#[test]
fn bars_carry_repeat_signs_and_endings() {
    let mut a = arrangement(&[(0, 4 * Q, 34), (4 * Q, 4 * Q, 34)]);
    a.bars[0].repeat_open = true;
    a.bars[1].repeat_close = true;
    a.bars[1].endings = 0b11;
    let dir = temp_dir("repeats");
    let mut core = Core::new(&dir, QaOptions::default());
    core.loaded = Some(Loaded {
        path: dir.join("x.mid"),
        full: a.clone(),
        arrangement: a,
        tracks: Vec::new(),
        track: 0,
    });
    let bars: serde_json::Value = serde_json::from_str(&core.bars_json()).unwrap();
    assert_eq!(bars[0]["repeatStart"], true);
    assert_eq!(bars[0]["repeatEnd"], false);
    assert_eq!(bars[1]["repeatEnd"], true);
    assert_eq!(bars[1]["endings"], json!([1, 2]));
}

/// QA 27.10. (M5): a chord symbol the file marks as one shows, also a plain triad ("C"),
/// which as a MIDI marker could be a rehearsal mark.
#[test]
fn bars_carry_chord_symbols() {
    let mut a = arrangement(&[(0, 4 * Q, 34), (4 * Q, 4 * Q, 34)]);
    a.markers.push(kontra_model::Marker {
        tick: 0,
        name: "C".into(),
        kind: kontra_model::MarkerKind::Chord,
    });
    a.markers.push(kontra_model::Marker {
        tick: 4 * Q,
        name: "A".into(),
        kind: kontra_model::MarkerKind::Section,
    });
    a.markers.push(kontra_model::Marker {
        tick: 2 * Q,
        name: "Bbm7".into(),
        kind: kontra_model::MarkerKind::Chord,
    });
    let dir = temp_dir("chords");
    let mut core = Core::new(&dir, QaOptions::default());
    core.loaded = Some(Loaded {
        path: dir.join("x.musicxml"),
        full: a.clone(),
        arrangement: a,
        tracks: Vec::new(),
        track: 0,
    });
    let bars: serde_json::Value = serde_json::from_str(&core.bars_json()).unwrap();
    assert_eq!(bars[0]["chord"], "C");
    assert_eq!(bars[1]["chord"], serde_json::Value::Null);
    // Within a bar: for the staff's own chord list.
    let staff = staff(core.arrangement().unwrap());
    assert_eq!(staff["chords"][0]["text"], "Bbm7");
    assert_eq!(staff["chords"].as_array().map(Vec::len), Some(1));
    let _ = std::fs::remove_dir_all(dir);
}

/// Rehearsal marks (LH-BIB-12, J10, Kante 1.50): the parts that begin in a bar, also
/// several, e.g. a MIDI file with "Chorus" and "B" in one bar; a chord name is none.
#[test]
fn bars_carry_sections() {
    let mut a = arrangement(&[(0, 4 * Q, 34), (4 * Q, 4 * Q, 34), (8 * Q, 4 * Q, 34)]);
    for (tick, name) in [
        (0, "Intro"),
        (4 * Q, "Chorus"),
        (5 * Q, "B"),
        (8 * Q, "Am7"),
    ] {
        a.markers.push(kontra_model::Marker {
            tick,
            name: name.into(),
            kind: kontra_model::MarkerKind::Section,
        });
    }
    let dir = temp_dir("sections");
    let mut core = Core::new(&dir, QaOptions::default());
    core.loaded = Some(Loaded {
        path: dir.join("x.mid"),
        full: a.clone(),
        arrangement: a,
        tracks: Vec::new(),
        track: 0,
    });
    let bars: serde_json::Value = serde_json::from_str(&core.bars_json()).unwrap();
    assert_eq!(bars[0]["section"], serde_json::json!(["Intro"]));
    assert_eq!(bars[1]["section"], serde_json::json!(["Chorus", "B"]));
    assert_eq!(bars[2]["section"], serde_json::Value::Null);

    // Review #174: a generated piece names its parts in the UI's language.
    let pop = Core::exercise_spec("song", "pop", "", 7, 90.0, "").unwrap();
    let piece = core.generate_exercise(&pop).unwrap();
    core.open(&piece).unwrap();
    let parts = |core: &Core| {
        let bars: Vec<serde_json::Value> = serde_json::from_str(&core.bars_json()).unwrap();
        bars.iter()
            .filter_map(|b| b["section"].as_array().cloned())
            .flatten()
            .filter_map(|n| n.as_str().map(str::to_string))
            .collect::<Vec<_>>()
    };
    core.update_settings(|s| s.language = Some("en".into()));
    assert_eq!(parts(&core)[..3], ["Intro", "Verse", "Chorus"]);
    core.update_settings(|s| s.language = Some("de".into()));
    assert_eq!(parts(&core)[..3], ["Intro", "Strophe", "Refrain"]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn staff_shows_the_key_of_the_first_bar() {
    let mut a = arrangement(&[(0, 4 * Q, 34), (4 * Q, 4 * Q, 34)]);
    a.bars[0].key_fifths = -2;
    a.bars[1].key_fifths = 1;
    assert_eq!(staff(&a)["keyFifths"], -2);
}

/// Rests as (start in quarters, length in quarters); 120 BPM: a quarter is 0.5 s.
fn rests_of(a: &Arrangement) -> Vec<(f64, f64)> {
    let value = staff(a);
    let rests = value["rests"].as_array().unwrap();
    rests
        .iter()
        .map(|r| {
            let time = r["time"].as_f64().unwrap();
            (time * 2.0, r["beats"].as_f64().unwrap())
        })
        .collect()
}

#[test]
fn staff_rests_fill_gaps_on_their_beat() {
    // | q q q - | q - - - | (empty) | q |
    let a = arrangement(&[
        (0, Q, 28),
        (Q, Q, 30),
        (2 * Q, Q, 31),
        (4 * Q, Q, 33),
        (12 * Q, Q, 35),
    ]);
    assert_eq!(
        rests_of(&a),
        [
            (3.0, 1.0),
            (5.0, 1.0),
            (6.0, 2.0),
            (8.0, 4.0),
            (13.0, 1.0),
            (14.0, 2.0)
        ]
    );
}

#[test]
fn staff_rests_ignore_short_played_notes_and_triplets() {
    // Staccato eighths (played 400 of 480 ticks), quarter triplets, a rest on beat 4.
    let t = Q * 2 / 3;
    let a = arrangement(&[
        (0, 400, 28),
        (Q / 2, 400, 30),
        (Q, t, 31),
        (Q + t, t, 33),
        (Q + 2 * t, t, 35),
    ]);
    assert_eq!(rests_of(&a), [(3.0, 1.0)]);
}

#[test]
fn staff_ties_start_at_the_note_before_the_continuation() {
    // G1 on beat 4 tied over the bar line; Kontra marks the second G1, Kante the first.
    let mut a = arrangement(&[(0, 3 * Q, 28), (3 * Q, Q, 31), (4 * Q, Q, 31)]);
    a.notes[2].tied = true;
    let value = staff(&a);
    let ties: Vec<bool> = value["notes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["tied"].as_bool().unwrap())
        .collect();
    assert_eq!(ties, [false, true, false]);
}

#[test]
fn staff_writes_bass_an_octave_up() {
    // B0 (MIDI 23) is written as B1 (35).
    let a = arrangement(&[(0, 4 * Q, 23)]);
    assert_eq!(staff(&a)["notes"][0]["midi"], 35);
}

#[test]
fn note_names_follow_the_language() {
    let dir = temp_dir("names");
    let mut core = Core::new(&dir, QaOptions::default());
    core.update_settings(|s| s.language = Some("de".into()));
    assert_eq!(core.note_label("B1"), "H1");
    assert_eq!(core.note_label("A#1"), "A♯1");
    assert_eq!(core.note_label("E2"), "E2");
    core.update_settings(|s| s.language = Some("en".into()));
    assert_eq!(core.note_label("B1"), "B1");
    let _ = std::fs::remove_dir_all(dir);
}

/// Wait mode (J-B2): the string by its name, not a number counted from the low string, and
/// after a wrong note what was played.
#[test]
fn wait_text_names_string_and_played_note() {
    let dir = temp_dir("wait");
    let mut core = Core::new(&dir, QaOptions::default());
    core.update_settings(|s| s.language = Some("de".into()));
    core.open(&demo()).unwrap();
    let a = core.arrangement().unwrap().clone();
    let (index, n) = a
        .notes
        .iter()
        .enumerate()
        .find(|(_, n)| n.string.is_some())
        .unwrap();
    let open = a.instrument.tuning.strings[usize::from(n.string.unwrap())]
        .name()
        .to_string();
    let text = core.wait_text(index, None).unwrap();
    assert!(text.contains(&format!("{open}-Saite")), "{text}");
    let played = f64::from(n.pitch.0) + 1.0;
    let text = core.wait_text(index, Some(played)).unwrap();
    let played_name = core.note_label(&kontra_model::Pitch(n.pitch.0 + 1).to_string());
    assert!(text.contains(&played_name), "{text}");
    let _ = std::fs::remove_dir_all(dir);
}

/// Kante 1.27: chord markers become chord symbols, section names do not.
#[test]
fn chord_names_are_told_from_sections() {
    for chord in [
        "Cm", "Am7", "F#m7b5", "Bbmaj7", "G7/B", "Dsus4", "Edim", "C(add9)", "E7",
    ] {
        assert!(is_chord_name(chord), "{chord}");
    }
    for section in ["Chorus", "Bridge", "A", "Bb", "Intro", "Verse 1", "Coda"] {
        assert!(!is_chord_name(section), "{section}");
    }
}

/// German writes B for B♭ and H for B: "Bb" must become "B", not "Hb".
#[test]
fn german_note_names() {
    let dir = temp_dir("names");
    let mut core = Core::new(&dir, QaOptions::default());
    core.update_settings(|s| s.language = Some("de".into()));
    let names: Vec<String> = ["B1", "Bb", "B#", "A#2", "Bbm"]
        .iter()
        .map(|n| core.note_label(n))
        .collect();
    assert_eq!(names, ["H1", "B", "H♯", "A♯2", "Bm"]);
    core.update_settings(|s| s.language = Some("en".into()));
    assert_eq!(core.note_label("Bb"), "B♭");
    assert_eq!(core.note_label("Ebm"), "E♭m");
    let _ = std::fs::remove_dir_all(dir);
}
