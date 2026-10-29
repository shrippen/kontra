//! A pitch told after its note, once the tone settled (decision round 11): the note is judged
//! again, the display keeps its speed.

use kontra_model::{
    Arrangement, DetectedNote, InputSource, Instrument, Note, Pitch, PitchRevision, TempoMap,
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

fn played(onset_s: f64, midi: f64) -> DetectedNote {
    DetectedNote {
        onset_s,
        midi,
        confidence: 0.9,
        level_db: -12.0,
        source: InputSource::Line,
        technique: None,
        over_background: false,
    }
}

/// Feeds each note 80 ms after its onset, as the engine emits it.
fn run(session: &mut Session, notes: &[DetectedNote], until: f64) -> Vec<SessionEvent> {
    let mut events = Vec::new();
    let mut pending = notes.to_vec();
    let mut t = 0.0;
    while t <= until {
        pending.retain(|n| {
            if n.onset_s + 0.08 <= t {
                session.on_note(n);
                return false;
            }
            true
        });
        events.extend(session.update(t));
        t += 0.01;
    }
    events
}

fn settled(onset_s: f64, midi: f64) -> PitchRevision {
    PitchRevision { onset_s, midi }
}

#[test]
fn flat_start_becomes_a_hit() {
    // D2 emitted 70 ct flat (rounds to C#2), settled on D2: wrong note, then a hit.
    let mut s = Session::new(piece(), Mode::Song, ScoreSettings::default());
    s.play(0.0);
    let take = [played(0.0, 37.3), played(0.5, 40.0), played(1.0, 41.0)];
    run(&mut s, &take, 0.15);
    let first = s.results().iter().find(|r| r.note == Some(0)).copied();
    assert!(
        matches!(first.map(|r| r.outcome), Some(Outcome::WrongPitch { .. })),
        "{first:?}"
    );

    s.on_pitch_revision(&settled(0.0, 38.02));
    run(&mut s, &take[1..], 2.0);
    let of_first: Vec<_> = s.results().iter().filter(|r| r.note == Some(0)).collect();
    assert_eq!(of_first.len(), 1, "replaced, not followed: {of_first:?}");
    assert!(of_first[0].is_hit(), "{of_first:?}");
    assert_eq!(s.summary().hit_rate, 1.0);
}

#[test]
fn revision_is_told_as_a_result() {
    let mut s = Session::new(piece(), Mode::Song, ScoreSettings::default());
    s.play(0.0);
    run(&mut s, &[played(0.0, 37.3)], 0.15);
    s.on_pitch_revision(&settled(0.0, 38.02));
    let events = s.update(0.16);
    let hit = events.iter().any(
        |e| matches!(e, SessionEvent::Result(r) if r.note == Some(0) && r.outcome == Outcome::Hit),
    );
    assert!(hit, "{events:?}");
}

#[test]
fn unknown_onset_changes_nothing() {
    let mut s = Session::new(piece(), Mode::Song, ScoreSettings::default());
    s.play(0.0);
    run(&mut s, &[played(0.0, 37.3)], 0.15);
    let before = s.results().to_vec();
    s.on_pitch_revision(&settled(0.25, 38.0));
    assert_eq!(s.results(), &before[..]);
}

#[test]
fn wait_ends_when_the_flat_start_was_right() {
    // Wait mode holds on D2; the flat attempt does not release it, the revision does.
    let mut s = Session::new(piece(), Mode::Wait, ScoreSettings::default());
    s.play(0.0);
    run(&mut s, &[played(0.0, 37.3)], 0.15);
    assert_eq!(s.next_expected().map(|e| e.index), Some(0));

    s.on_pitch_revision(&settled(0.0, 38.02));
    assert_eq!(
        s.next_expected().map(|e| e.index),
        Some(1),
        "the wait is over"
    );
    let of_first: Vec<_> = s.results().iter().filter(|r| r.note == Some(0)).collect();
    assert!(of_first.iter().all(|r| r.is_hit()), "{of_first:?}");
}
