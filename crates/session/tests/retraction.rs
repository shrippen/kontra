//! Notes taken back (R1): the tracker tells a note early and may find it was no note of its own,
//! e.g. the fretting finger tapping the string before the pluck. The session judged it at once;
//! taken back, the judgement goes, and nothing counts twice.

use kontra_model::{
    Arrangement, DetectedNote, InputSource, Instrument, Note, NoteRetraction, Pitch, TempoMap,
    TICKS_PER_QUARTER,
};
use kontra_session::{Mode, Outcome, ScoreSettings, Session, SessionEvent};

/// At 120 BPM, one quarter apart: D2, E2, F2.
fn piece() -> Arrangement {
    let mut a = Arrangement::new("Walking", Instrument::default_bass());
    a.tempo = TempoMap::constant(120.0);
    for (i, midi) in [38u8, 40, 41].iter().enumerate() {
        a.notes.push(Note::new(
            i as u64 * TICKS_PER_QUARTER,
            TICKS_PER_QUARTER,
            Pitch(*midi),
        ));
    }
    a.normalize();
    a
}

fn played(onset_s: f64, midi: f64, level_db: f32) -> DetectedNote {
    DetectedNote {
        onset_s,
        midi,
        confidence: 0.9,
        level_db,
        source: InputSource::Line,
        technique: None,
        over_background: false,
    }
}

/// The clock from `from` to `until` in 10 ms steps.
fn run(session: &mut Session, from: f64, until: f64) -> Vec<SessionEvent> {
    let mut events = Vec::new();
    let mut t = from;
    while t <= until {
        events.extend(session.update(t));
        t += 0.01;
    }
    events
}

fn taken_back(onset_s: f64) -> NoteRetraction {
    NoteRetraction { onset_s }
}

fn of_note(s: &Session, note: usize) -> Vec<Outcome> {
    s.results()
        .iter()
        .filter(|r| r.note == Some(note))
        .map(|r| r.outcome)
        .collect()
}

#[test]
fn tap_taken_back_then_pluck_hits() {
    // The tap 60 ms early on D2 was judged a hit; taken back, the pluck takes the note: one hit
    // at the pluck's time, no extra note.
    let mut s = Session::new(piece(), Mode::Song, ScoreSettings::default());
    s.play(0.0);
    s.on_note(&played(-0.06, 38.0, -32.0));
    s.on_retraction(&taken_back(-0.06));
    s.on_note(&played(0.0, 38.0, -3.0));
    run(&mut s, 0.0, 0.3);
    assert_eq!(of_note(&s, 0), vec![Outcome::Hit], "{:?}", s.results());
    let first = s.results().iter().find(|r| r.note == Some(0)).copied();
    assert!(first
        .and_then(|r| r.offset_ms)
        .is_some_and(|ms| ms.abs() < 1.0));
    assert!(s.results().iter().all(|r| r.note.is_some()), "no extra");
}

#[test]
fn taken_back_alone_is_missed() {
    // Nothing follows the tap: the note is missed, as if the tap had never been told.
    let mut s = Session::new(piece(), Mode::Song, ScoreSettings::default());
    s.play(0.0);
    s.on_note(&played(0.0, 38.0, -32.0));
    s.on_retraction(&taken_back(0.0));
    run(&mut s, 0.0, 0.45);
    assert_eq!(of_note(&s, 0), vec![Outcome::Missed], "{:?}", s.results());
    assert_eq!(s.summary().hit_rate, 0.0);
}

#[test]
fn extra_taken_back_is_gone() {
    // A tap where no note is due was an extra note; taken back, it is gone.
    let mut s = Session::new(piece(), Mode::Song, ScoreSettings::default());
    s.play(0.0);
    s.on_note(&played(0.25, 30.0, -32.0));
    assert_eq!(s.results().len(), 1, "{:?}", s.results());
    s.on_retraction(&taken_back(0.25));
    assert!(s.results().is_empty(), "{:?}", s.results());
}

#[test]
fn later_revision_finds_its_note() {
    // An extra note taken back after another note was judged: that note's pitch revision still
    // finds its result (the results moved).
    let mut s = Session::new(piece(), Mode::Song, ScoreSettings::default());
    s.play(0.0);
    s.on_note(&played(0.25, 30.0, -32.0));
    s.on_note(&played(0.5, 39.3, -3.0));
    s.on_retraction(&taken_back(0.25));
    s.on_pitch_revision(&kontra_model::PitchRevision {
        onset_s: 0.5,
        midi: 40.02,
    });
    assert_eq!(of_note(&s, 1), vec![Outcome::Hit], "{:?}", s.results());
    assert_eq!(s.results().len(), 1, "{:?}", s.results());
}

#[test]
fn wait_mode_waits_again() {
    // Wait mode: the tap ended the wait on D2; taken back, D2 is waited for again and the pluck
    // ends it.
    let mut s = Session::new(piece(), Mode::Wait, ScoreSettings::default());
    s.play(0.0);
    run(&mut s, 0.0, 0.05);
    s.on_note(&played(0.04, 38.0, -32.0));
    assert_eq!(s.next_expected().map(|e| e.index), Some(1));
    s.on_retraction(&taken_back(0.04));
    assert_eq!(s.next_expected().map(|e| e.index), Some(0), "waits again");
    s.on_note(&played(0.1, 38.0, -3.0));
    assert_eq!(s.next_expected().map(|e| e.index), Some(1));
    assert_eq!(of_note(&s, 0), vec![Outcome::Hit], "{:?}", s.results());
}

#[test]
fn wait_mode_goes_back_to_the_reopened_note() {
    // The cursor moved on past the next note before the tap was taken back: Kontra waits for
    // the reopened D2 again, then goes on after the notes already played.
    let mut s = Session::new(piece(), Mode::Wait, ScoreSettings::default());
    s.play(0.0);
    run(&mut s, 0.0, 0.05);
    s.on_note(&played(0.04, 38.0, -32.0));
    s.on_note(&played(0.05, 40.0, -3.0));
    assert_eq!(s.next_expected().map(|e| e.index), Some(2));
    s.on_retraction(&taken_back(0.04));
    assert_eq!(s.next_expected().map(|e| e.index), Some(0), "waits again");
    s.on_note(&played(0.1, 38.0, -3.0));
    assert_eq!(s.next_expected().map(|e| e.index), Some(2), "E2 was played");
}
