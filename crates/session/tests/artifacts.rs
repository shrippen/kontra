//! False alarms of a real bass (hardware check 13.10., H5): a faint detection just before the
//! attack (the fretting finger leaving the open string, a string change) must not take the next
//! note when the attack with the right pitch follows inside its window.

use kontra_model::{
    Arrangement, DetectedNote, InputSource, Instrument, Note, Pitch, PitchRevision, Technique,
    TempoMap, TICKS_PER_QUARTER,
};
use kontra_session::{Mode, Outcome, ScoreSettings, Session, Strictness};

const EIGHTH: u64 = TICKS_PER_QUARTER / 2;
/// Eighths at 120 BPM, A1 C2 D2 E2: 250 ms apart.
const LINE: [u8; 4] = [33, 36, 38, 40];
const EIGHTH_S: f64 = 0.25;
/// Open E string, sounding when the finger leaves the A1 (5th fret).
const OPEN_E: f64 = 28.0;

fn piece() -> Arrangement {
    let mut a = Arrangement::new("Achtel", Instrument::default_bass());
    a.tempo = TempoMap::constant(120.0);
    for (i, midi) in LINE.iter().enumerate() {
        a.notes
            .push(Note::new(i as u64 * EIGHTH, EIGHTH, Pitch(*midi)));
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
        technique: (midi == 0.0).then_some(Technique::Muted),
    }
}

/// The line played on time, with `artifact` in between.
fn take(artifact: DetectedNote) -> Vec<DetectedNote> {
    let mut notes: Vec<DetectedNote> = LINE
        .iter()
        .enumerate()
        .map(|(i, m)| played(i as f64 * EIGHTH_S, f64::from(*m), -10.0))
        .collect();
    notes.push(artifact);
    notes.sort_by(|a, b| a.onset_s.total_cmp(&b.onset_s));
    notes
}

/// Feeds each note 80 ms after its onset, as the engine emits it.
fn run(session: &mut Session, notes: &[DetectedNote], until: f64) {
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
        session.update(t);
        t += 0.01;
    }
}

fn session(strictness: Strictness) -> Session {
    let settings = ScoreSettings {
        strictness,
        ..ScoreSettings::default()
    };
    let mut s = Session::new(piece(), Mode::Song, settings);
    s.play(0.0);
    s
}

fn outcome_of(s: &Session, note: usize) -> Vec<Outcome> {
    s.results()
        .iter()
        .filter(|r| r.note == Some(note))
        .map(|r| r.outcome)
        .collect()
}

#[test]
fn open_string_before_the_attack_takes_no_note() {
    // The open E 50 ms before the C2, 26 dB below it.
    let mut s = session(Strictness::Normal);
    run(&mut s, &take(played(EIGHTH_S - 0.05, OPEN_E, -36.0)), 2.0);

    assert_eq!(outcome_of(&s, 1), vec![Outcome::Hit]);
    let summary = s.summary();
    assert_eq!(summary.hit_rate, 1.0);
    assert_eq!(summary.extra, 1);
}

#[test]
fn hit_keeps_the_attack_timing() {
    let mut s = session(Strictness::Normal);
    run(&mut s, &take(played(EIGHTH_S - 0.05, OPEN_E, -36.0)), 2.0);

    let hit = s.results().iter().find(|r| r.note == Some(1)).copied();
    let offset = hit.and_then(|r| r.offset_ms).unwrap_or(f64::NAN);
    assert!(offset.abs() < 1.0, "{hit:?}");
}

#[test]
fn pitchless_noise_before_the_attack_takes_no_note() {
    // Loose window (90 ms): a muted thump 80 ms before the D2.
    let mut s = session(Strictness::Loose);
    run(
        &mut s,
        &take(played(2.0 * EIGHTH_S - 0.08, 0.0, -38.0)),
        2.0,
    );

    assert_eq!(outcome_of(&s, 2), vec![Outcome::Hit]);
    assert_eq!(s.summary().extra, 1);
}

#[test]
fn wrong_note_without_a_better_one_stays_wrong() {
    // A real wrong note: nothing better follows, the note stays wrong.
    let mut s = session(Strictness::Normal);
    let mut notes = take(played(9.0, 40.0, -10.0));
    notes.retain(|n| n.midi != 36.0);
    notes.push(played(EIGHTH_S, 35.0, -10.0));
    notes.sort_by(|a, b| a.onset_s.total_cmp(&b.onset_s));
    run(&mut s, &notes, 2.0);

    assert!(matches!(
        outcome_of(&s, 1)[..],
        [Outcome::WrongPitch { .. }]
    ));
}

#[test]
fn late_pitch_of_the_artifact_leaves_the_hit() {
    // The artifact's pitch told later must not judge the note the attack took over.
    let mut s = session(Strictness::Normal);
    let artifact = played(EIGHTH_S - 0.05, OPEN_E, -36.0);
    run(&mut s, &take(artifact), 0.5);
    s.on_pitch_revision(&PitchRevision {
        onset_s: artifact.onset_s,
        midi: 28.0,
    });
    run(&mut s, &[], 2.0);

    assert_eq!(outcome_of(&s, 1), vec![Outcome::Hit]);
}

#[test]
fn note_settled_right_is_not_taken_over() {
    // A flat start revised to the right pitch is the note: a later attack is an extra one.
    let mut s = session(Strictness::Normal);
    let flat = played(EIGHTH_S - 0.03, 35.3, -12.0);
    let notes: Vec<DetectedNote> = take(flat)
        .into_iter()
        .filter(|n| n.onset_s < EIGHTH_S - 0.01 || n.onset_s > EIGHTH_S + 0.01)
        .collect();
    run(&mut s, &notes, 0.5);
    s.on_pitch_revision(&PitchRevision {
        onset_s: flat.onset_s,
        midi: 36.0,
    });
    run(&mut s, &[played(EIGHTH_S + 0.02, 36.0, -10.0)], 2.0);

    assert_eq!(outcome_of(&s, 1), vec![Outcome::Hit]);
    let hit = s.results().iter().find(|r| r.note == Some(1)).copied();
    let offset = hit.and_then(|r| r.offset_ms).unwrap_or(f64::NAN);
    assert!((offset + 30.0).abs() < 1.0, "{hit:?}");
}
