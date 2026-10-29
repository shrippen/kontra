//! The string a note was heard on (R7): feedback in the summary, the score stays pitch-based.

use kontra_model::{
    Arrangement, DetectedNote, InputSource, Instrument, Note, Pitch, StringHeard, StringVerdict,
    Technique, TempoMap, Unclear, TICKS_PER_QUARTER,
};
use kontra_session::{Mode, ScoreSettings, Session, StringTally};

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
