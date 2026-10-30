//! The note list after a run (`Core::note_details`): timing, wrong notes, chord tones.

use super::*;

/// QA 13.10.: a wrong note says which note was played ("C2 gespielt"), not "+100 Cent":
/// the semitone distance read like an intonation error.
#[test]
fn wrong_note_names_the_played_note() {
    let dir = temp_dir("wrong-note");
    let mut core = Core::new(&dir, QaOptions::default());
    core.update_settings(|s| s.language = Some("de".into()));
    core.open(&demo()).unwrap();
    let expected = core.arrangement().unwrap().notes[0].pitch;
    let played = kontra_model::Pitch(expected.0 + 1);
    let result = NoteResult {
        pass: 0,
        note: Some(0),
        bar: Some(0),
        outcome: Outcome::WrongPitch {
            played_midi: f64::from(played.0) + 0.1,
        },
        offset_ms: Some(0.0),
        cents: Some(110.0),
        position_s: 0.0,
    };
    let notes = core.note_details(&[result], &[], &[]);
    let text = notes[0]["text"].as_str().unwrap();
    let name = core.note_label(&played.to_string());
    assert!(
        text.contains(&core.tr("summary.note.played", &[("played", name)])),
        "{text}"
    );
    assert!(!text.contains("Cent"), "{text}");
    let _ = std::fs::remove_dir_all(dir);
}

/// QA 27.10. (P3 Mira, M6): "Alle Noten" gave no timing for a note within a millisecond
/// (a played take is that exact); LH-BEW-01 wants every note's timing in ms.
#[test]
fn every_note_tells_its_timing() {
    let dir = temp_dir("note-timing");
    let mut core = Core::new(&dir, QaOptions::default());
    core.update_settings(|s| s.language = Some("en".into()));
    core.open(&demo()).unwrap();
    let result = NoteResult {
        pass: 0,
        note: Some(0),
        bar: Some(0),
        outcome: Outcome::Hit,
        offset_ms: Some(0.4),
        cents: Some(3.0),
        position_s: 0.0,
    };
    // A missed note played 40 ms early, beside a strict window (M2): its own timing too.
    let missed = NoteResult {
        note: Some(1),
        outcome: Outcome::Missed,
        offset_ms: None,
        cents: None,
        ..result
    };
    let outside = OutsideNote {
        pass: 0,
        note: 1,
        offset_ms: -40.0,
    };
    let notes = core.note_details(&[result, missed], &[], &[outside]);
    let text = notes[0]["text"].as_str().unwrap();
    assert!(text.contains("0 ms"), "{text}");
    let text = notes[1]["text"].as_str().unwrap();
    assert!(text.contains("40 ms early"), "{text}");
    let _ = std::fs::remove_dir_all(dir);
}

/// R8 (review #205): a double stop with one tone heard says so at the missed tone ("nur
/// ein Ton"), a missed single note does not.
#[test]
fn one_tone_of_a_double_stop_says_so() {
    let a = arrangement(&[(0, Q, 33), (0, Q, 40), (Q, Q, 45)]);
    let dir = temp_dir("one-tone");
    let mut core = Core::new(&dir, QaOptions::default());
    core.update_settings(|s| s.language = Some("de".into()));
    core.loaded = Some(Loaded {
        path: dir.join("x.mid"),
        full: a.clone(),
        arrangement: a,
        tracks: Vec::new(),
        track: 0,
    });
    let result = |note: usize, outcome: Outcome| NoteResult {
        pass: 0,
        note: Some(note),
        bar: Some(0),
        outcome,
        offset_ms: None,
        cents: None,
        position_s: 0.0,
    };
    let results = [
        result(0, Outcome::Hit),
        result(1, Outcome::Missed),
        result(2, Outcome::Missed),
    ];
    let notes = core.note_details(&results, &[], &[]);
    let hint = core.tr("summary.note.one.tone", &[]);
    let text = |i: usize| notes[i]["text"].as_str().unwrap_or_default().to_string();
    assert!(text(1).contains(&hint), "{} / {hint}", text(1));
    assert!(!text(0).contains(&hint), "{}", text(0));
    assert!(!text(2).contains(&hint), "{}", text(2));
    let _ = std::fs::remove_dir_all(dir);
}

/// QA 27.10. (M4): in B♭ the note list said A♯1 where the notation writes B♭; names follow
/// the key of the note's bar, as in the notation.
#[test]
fn note_list_spells_in_the_key() {
    let mut a = arrangement(&[(0, 4 * Q, 34), (4 * Q, 4 * Q, 37)]);
    a.bars[0].key_fifths = -2;
    a.bars[1].key_fifths = 2;
    let dir = temp_dir("spelling");
    let mut core = Core::new(&dir, QaOptions::default());
    core.update_settings(|s| s.language = Some("en".into()));
    core.loaded = Some(Loaded {
        path: dir.join("x.mid"),
        full: a.clone(),
        arrangement: a,
        tracks: Vec::new(),
        track: 0,
    });
    let result = |note: usize, played: u8| NoteResult {
        pass: 0,
        note: Some(note),
        bar: Some(note),
        outcome: Outcome::WrongPitch {
            played_midi: f64::from(played),
        },
        offset_ms: Some(0.0),
        cents: None,
        position_s: 0.0,
    };
    let notes = core.note_details(&[result(0, 39), result(1, 39)], &[], &[]);
    let flat = notes[0]["text"].as_str().unwrap();
    assert!(flat.contains("B♭1") && flat.contains("E♭2"), "{flat}");
    let sharp = notes[1]["text"].as_str().unwrap();
    assert!(sharp.contains("C♯2") && sharp.contains("D♯2"), "{sharp}");
    assert!(core.wait_text(0, None).unwrap().contains("B♭1"));
    // The keyboard names the keys in the piece's key too.
    let keys: serde_json::Value = serde_json::from_str(&core.key_range_json()).unwrap();
    assert_eq!(
        (keys["names"][10].as_str(), keys["names"][1].as_str()),
        (Some("B♭"), Some("D♭"))
    );
    let _ = std::fs::remove_dir_all(dir);
}
