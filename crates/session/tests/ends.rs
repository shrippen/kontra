//! Note ends judged (LH-BEW-10): a recording played through the input, the session judges
//! the notes the tab wants stopped by where they ended.

// Test set-up may fail loudly.
#![allow(clippy::unwrap_used)]

use kontra_engine::{Clock, Event, Input, InputKind, Pace};
use kontra_model::{
    Arrangement, DetectedNote, InputSource, Instrument, Note, NoteEnd, Pitch, Technique, TempoMap,
    TICKS_PER_QUARTER,
};
use kontra_session::{
    render_performance, write_mono_wav, EndHeard, Mode, MutingTally, ScoreSettings, Session,
};
use std::sync::mpsc::channel;

const RATE: u32 = 48_000;
const LEAD_IN_S: f64 = 0.5;
const EIGHTH: u64 = TICKS_PER_QUARTER / 2;
/// A1 D2 E2 A1 G1 A1, one per beat at 100 BPM.
const LINE: [u8; 6] = [33, 38, 40, 33, 31, 33];
/// Events reach the session this long after the audio they tell of (detection, the end's hold).
const TOLD_AFTER_S: f64 = 0.3;

/// Each note an eighth with an eighth rest after: every one but the last is to be stopped.
fn staccato() -> Arrangement {
    let mut a = Arrangement::new("Abdämpfen", Instrument::default_bass());
    a.tempo = TempoMap::constant(100.0);
    for (i, midi) in LINE.iter().enumerate() {
        a.notes.push(Note::new(
            i as u64 * TICKS_PER_QUARTER,
            EIGHTH,
            Pitch(*midi),
        ));
    }
    a.normalize();
    a
}

/// The line played with every note held through its rest (not muted).
fn ringing() -> Arrangement {
    let mut a = staccato();
    for n in &mut a.notes {
        n.duration_ticks = TICKS_PER_QUARTER;
    }
    a
}

/// Notes and ends the input tells for `played`, rendered and read from a file.
fn heard(played: &Arrangement, name: &str) -> (Vec<DetectedNote>, Vec<NoteEnd>) {
    let samples = render_performance(played, 1.0, LEAD_IN_S, Default::default(), RATE);
    let dir = std::env::temp_dir().join(format!("kontra-ends-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let take = dir.join("take.wav");
    write_mono_wav(&take, &samples, RATE).unwrap();
    let kind = InputKind::File {
        path: take,
        pace: Pace::Fast,
        laptop_jack: false,
    };
    let (tx, rx) = channel();
    let input = Input::start(
        &kind,
        &played.instrument,
        InputSource::Line,
        &Clock::new(),
        tx,
        None,
    )
    .unwrap();
    let (mut notes, mut ends) = (Vec::new(), Vec::new());
    for event in rx.iter() {
        match event {
            Event::Note(n) => notes.push(n),
            Event::End(e) => ends.push(e),
            Event::Ended => break,
            _ => {}
        }
    }
    drop(input);
    let _ = std::fs::remove_dir_all(&dir);
    (notes, ends)
}

/// Runs `staccato()` over what the input heard of `played` at `source`.
fn judge(played: &Arrangement, source: InputSource, name: &str) -> Session {
    judge_score(staccato(), played, source, name)
}

/// Runs `score` over what the input heard of `played` at `source`.
fn judge_score(
    score: Arrangement,
    played: &Arrangement,
    source: InputSource,
    name: &str,
) -> Session {
    let (notes, ends) = heard(played, name);
    feed(score, &notes, &ends, source)
}

/// Runs `score` over `notes` and `ends`, each told `TOLD_AFTER_S` after it happened.
fn feed(
    score: Arrangement,
    notes: &[DetectedNote],
    ends: &[NoteEnd],
    source: InputSource,
) -> Session {
    let settings = ScoreSettings {
        source,
        ..ScoreSettings::default()
    };
    let mut session = Session::new(score, Mode::Song, settings);
    session.play(LEAD_IN_S);
    let (mut next_note, mut next_end) = (0, 0);
    for step in 0..=800 {
        let t = f64::from(step) * 0.01;
        while notes
            .get(next_note)
            .is_some_and(|n| n.onset_s + TOLD_AFTER_S <= t)
        {
            session.on_note(&notes[next_note]);
            next_note += 1;
        }
        while ends
            .get(next_end)
            .is_some_and(|e| e.end_s + TOLD_AFTER_S <= t)
        {
            session.on_end(&ends[next_end]);
            next_end += 1;
        }
        session.update(t);
    }
    session
}

#[test]
fn muted_notes_are_judged_in_time() {
    let session = judge(&staccato(), InputSource::Line, "muted");
    let summary = session.summary();
    assert_eq!(summary.hits as usize, LINE.len(), "{summary:?}");
    assert_eq!(
        summary.muting,
        MutingTally {
            judged: 5,
            in_time: 5,
            ..MutingTally::default()
        },
        "{:?}",
        session.ends_heard()
    );
}

#[test]
fn ringing_notes_are_judged_rang_on() {
    let session = judge(&ringing(), InputSource::Line, "ringing");
    let summary = session.summary();
    assert_eq!(summary.hits as usize, LINE.len(), "the notes stay hits");
    assert_eq!(summary.muting.rang_on, 5, "{:?}", session.ends_heard());
    let marks = &summary.ends;
    assert!(
        marks.iter().all(|m| m.heard == EndHeard::RangOn),
        "{marks:?}"
    );
}

#[test]
fn a_microphone_says_it_cannot_tell() {
    let session = judge(&ringing(), InputSource::Microphone, "mic");
    let muting = session.summary().muting;
    assert_eq!(
        muting,
        MutingTally {
            untold: 5,
            ..MutingTally::default()
        }
    );
}

/// Review #227: the last note, written staccato and left ringing, counts as rang on. No next
/// note ends it and its decay takes seconds, so before it was missing from the tally.
#[test]
fn a_ringing_last_note_counts() {
    let mut score = staccato();
    if let Some(last) = score.notes.last_mut() {
        last.techniques.push(Technique::Staccato);
    }
    // Played: every note held, the last one for two bars (the recording does not stop it).
    let mut played = ringing();
    if let Some(last) = played.notes.last_mut() {
        last.duration_ticks = 8 * TICKS_PER_QUARTER;
    }
    let session = judge_score(score, &played, InputSource::Line, "last");
    let summary = session.summary();
    assert_eq!(summary.muting.judged, 6, "{:?}", session.ends_heard());
    assert_eq!(summary.muting.rang_on, 6, "{:?}", session.ends_heard());
}

/// Review #227: a run scored from notes alone (`score_notes`, no ends given) judges no ends; it
/// counted every note to stop as rang on.
#[test]
fn notes_without_ends_judge_no_ends() {
    let score = staccato();
    let detected: Vec<DetectedNote> = score
        .notes
        .iter()
        .map(|n| DetectedNote {
            onset_s: score.tempo.seconds_at(n.start_ticks),
            midi: f64::from(n.pitch.0),
            confidence: 0.9,
            level_db: -12.0,
            source: InputSource::Line,
            technique: None,
            over_background: false,
        })
        .collect();
    let settings = ScoreSettings {
        source: InputSource::Line,
        ..ScoreSettings::default()
    };
    let scored = kontra_session::score_notes(&score, &detected, Mode::Song, settings, 1.0, 0.0);
    assert_eq!(scored.summary.hits as usize, LINE.len());
    assert_eq!(scored.summary.muting, MutingTally::default());
}

/// Review #227: a cable run with one note, written staccato and stopped while it still rings,
/// counts it as rang on. The input told no end, so the run counted as one without ends.
#[test]
fn a_lone_ringing_note_counts() {
    let mut score = staccato();
    score.notes.truncate(1);
    score.notes[0].techniques.push(Technique::Staccato);
    let mut played = score.clone();
    played.notes[0].duration_ticks = 8 * TICKS_PER_QUARTER;
    // Stopped while it rings: the end the recording's close tells never comes.
    let (notes, _) = heard(&played, "lone");
    let session = feed(score, &notes, &[], InputSource::Line);
    let summary = session.summary();
    assert_eq!(summary.muting.judged, 1, "{:?}", session.ends_heard());
    assert_eq!(summary.muting.rang_on, 1, "{:?}", session.ends_heard());
}
