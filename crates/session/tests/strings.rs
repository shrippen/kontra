//! The string a note was heard on (R7): feedback in the summary, the score stays pitch-based.

use kontra_model::{
    Arrangement, DetectedNote, InputSource, Instrument, Note, Pitch, StringHeard, StringVerdict,
    Technique, TempoMap, Unclear, TICKS_PER_QUARTER,
};
use kontra_session::{Mode, ScoreSettings, Session, SessionEvent, StringTally};

/// At 120 BPM, one quarter apart: G2, A2, B2 on the A string; the last palm-muted.
fn piece() -> Arrangement {
    let mut a = Arrangement::new("Lage", Instrument::default_bass());
    a.tempo = TempoMap::constant(120.0);
    for (i, midi) in [43u8, 45, 47].iter().enumerate() {
        let mut note = Note::new(
            i as u64 * TICKS_PER_QUARTER,
            TICKS_PER_QUARTER,
            Pitch(*midi),
        );
        note.string = Some(1);
        note.fret = Some(midi - 33);
        a.notes.push(note);
    }
    a.notes[2].techniques.push(Technique::PalmMute);
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

fn verdict(onset_s: f64, heard: StringHeard) -> StringVerdict {
    StringVerdict {
        onset_s,
        heard,
        log_b: Some(-3.9),
        partials: 20,
    }
}

#[test]
fn another_string_is_feedback_not_a_miss() {
    let mut s = Session::new(piece(), Mode::Song, ScoreSettings::default());
    s.play(0.0);
    let take = [played(0.0, 43.0), played(0.5, 45.0), played(1.0, 47.0)];
    let mut t = 0.0;
    let mut next = 0;
    while t <= 2.0 {
        if let Some(n) = take.get(next).filter(|n| n.onset_s + 0.08 <= t) {
            s.on_note(n);
            next += 1;
        }
        s.update(t);
        t += 0.01;
    }

    // G2 on the E string, A2 unclear (cut short), B2 palm-muted in the tab: not counted.
    s.on_string(&verdict(0.0, StringHeard::Other(0)));
    s.on_string(&verdict(0.5, StringHeard::Unclear(Unclear::ShortNote)));
    s.on_string(&verdict(1.0, StringHeard::Other(2)));
    let summary = s.summary();
    assert_eq!(summary.hit_rate, 1.0, "the score stays pitch-based");
    assert_eq!(
        summary.strings,
        StringTally {
            told: 1,
            other: 1,
            unsupported: 0,
        }
    );

    // A verdict for an onset no note has is dropped.
    s.on_string(&verdict(0.25, StringHeard::Expected));
    assert_eq!(
        s.summary().strings,
        StringTally {
            told: 1,
            other: 1,
            unsupported: 0,
        }
    );
}

#[test]
fn each_note_counts_once_by_its_last_verdict() {
    let heard = [
        (0, 0, StringHeard::Other(0)),
        (0, 0, StringHeard::Expected),
        (0, 1, StringHeard::Other(2)),
        (1, 0, StringHeard::Other(0)),
    ];
    assert_eq!(
        StringTally::of(&heard),
        StringTally {
            told: 3,
            other: 2,
            unsupported: 0,
        }
    );
}

#[test]
fn a_string_survives_the_rescoring_after_a_retraction() {
    // A tap 40 ms before the pluck of A2 is told first and takes the note; the pluck's string
    // comes, then the tap is taken back (R1) and the pluck scored again: its string stays.
    use kontra_model::NoteRetraction;
    let mut s = Session::new(piece(), Mode::Song, ScoreSettings::default());
    s.play(0.0);
    let mut t = 0.0;
    while t <= 0.45 {
        s.update(t);
        t += 0.01;
    }
    s.on_note(&played(0.0, 43.0));
    s.on_note(&played(0.46, 45.0));
    s.on_note(&played(0.5, 45.0));
    s.on_string(&verdict(0.5, StringHeard::Other(0)));
    s.on_retraction(&NoteRetraction { onset_s: 0.46 });
    while t <= 2.0 {
        s.update(t);
        t += 0.01;
    }
    assert_eq!(s.summary().strings.other, 1, "{:?}", s.strings_heard());
}

#[test]
fn the_run_waits_for_the_last_string() {
    // The analysis runs 0.8 s behind (a loaded machine): the last note is heard at 1.88 s, its
    // string told at 1.0 + 0.33 + 0.8 s (peak up to 30 ms late, 30 + 250 ms window, a block),
    // after the note's grace has run out. The run ends with it.
    const LAG_S: f64 = 0.8;
    const TOLD_AFTER_S: f64 = 0.33;
    let mut a = piece();
    a.notes[2].techniques.clear();
    let mut s = Session::new(a, Mode::Song, ScoreSettings::default());
    s.play(0.0);
    let take = [played(0.0, 43.0), played(0.5, 45.0), played(1.0, 47.0)];
    let mut next = 0;
    let mut told = false;
    let mut t = 0.0;
    while t <= 3.0 {
        s.on_heard(t - LAG_S);
        if let Some(n) = take.get(next).filter(|n| n.onset_s + 0.08 + LAG_S <= t) {
            s.on_note(n);
            next += 1;
        }
        if !told && 1.0 + TOLD_AFTER_S + LAG_S <= t {
            s.on_string(&verdict(1.0, StringHeard::Other(0)));
            told = true;
        }
        if s.update(t).contains(&SessionEvent::Finished) {
            break;
        }
        t += 0.01;
    }
    assert!(s.is_finished(), "finished by {t:.2} s");
    assert_eq!(s.summary().strings.other, 1, "finished at {t:.2} s");
}
