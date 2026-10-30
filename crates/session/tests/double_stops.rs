//! Double stops judged by both tones (R8, LH-BEW-11): the verdict on which tones sounded
//! judges each tone of the double stop again.

use kontra_model::{
    Arrangement, DetectedNote, DoubleStopHeard, DoubleStopVerdict, InputSource, Instrument, Note,
    NoteRetraction, Pitch, TempoMap, TICKS_PER_QUARTER,
};
use kontra_session::{Mode, NoteResult, Outcome, ScoreSettings, Session};

/// A1 + E2 (a fifth) on beat 1, A1 + A2 (an octave) on beat 2, at 120 BPM.
fn piece() -> Arrangement {
    let mut a = Arrangement::new("Quinten", Instrument::default_bass());
    a.tempo = TempoMap::constant(120.0);
    for (beat, midi) in [(0, 33u8), (0, 40), (1, 33), (1, 45)] {
        a.notes.push(Note::new(
            beat * TICKS_PER_QUARTER,
            TICKS_PER_QUARTER,
            Pitch(midi),
        ));
    }
    a.normalize();
    a
}

fn played(onset_s: f64, midi: f64, source: InputSource) -> DetectedNote {
    DetectedNote {
        onset_s,
        midi,
        confidence: 0.9,
        level_db: -12.0,
        source,
        technique: None,
        over_background: false,
    }
}

/// The session after the note at 0 s heard as `midi` and the verdict `heard` on it.
fn judged(midi: f64, heard: DoubleStopHeard, source: InputSource) -> Session {
    let settings = ScoreSettings {
        source,
        ..ScoreSettings::default()
    };
    let mut s = Session::new(piece(), Mode::Song, settings);
    s.play(0.0);
    s.update(0.08);
    s.on_note(&played(0.0, midi, source));
    s.update(0.16);
    s.on_double_stop(&DoubleStopVerdict {
        onset_s: 0.0,
        heard,
    });
    s
}

/// The last result of each tone of the fifth (note indices after `normalize`: lower first).
fn fifth(s: &Session) -> [Option<NoteResult>; 2] {
    let tone = |midi: u8| {
        let index = s
            .arrangement()
            .notes
            .iter()
            .position(|n| n.start_ticks == 0 && n.pitch.0 == midi)?;
        s.results()
            .iter()
            .rev()
            .find(|r| r.note == Some(index))
            .copied()
    };
    [tone(33), tone(40)]
}

fn outcome(r: Option<NoteResult>) -> Option<Outcome> {
    r.map(|r| r.outcome)
}

/// The mix heard as a wrong pitch (D#2, between the tones), both tones heard: two hits, not a
/// wrong note.
#[test]
fn both_heard_turns_a_mixed_pitch_into_hits() {
    let before = judged(39.0, DoubleStopHeard::Unknown, InputSource::Line);
    let [lower, upper] = fifth(&before);
    assert!(
        matches!(outcome(lower), Some(Outcome::WrongPitch { .. })),
        "{lower:?}"
    );
    assert!(
        matches!(outcome(upper), Some(Outcome::WrongPitch { .. })),
        "{upper:?}"
    );

    let s = judged(39.0, DoubleStopHeard::Both, InputSource::Line);
    let [lower, upper] = fifth(&s);
    assert_eq!(outcome(lower), Some(Outcome::Hit));
    assert_eq!(outcome(upper), Some(Outcome::Hit));
    assert_eq!(
        lower.and_then(|r| r.cents),
        None,
        "the mix has no cents of its own"
    );
    assert!(lower.and_then(|r| r.offset_ms).is_some());
}

/// The tracker heard the lower tone 10 ct sharp, both sounded: the lower keeps its cents.
#[test]
fn a_tone_the_tracker_heard_keeps_its_cents() {
    let s = judged(33.1, DoubleStopHeard::Both, InputSource::Line);
    let [lower, upper] = fifth(&s);
    assert_eq!(outcome(lower), Some(Outcome::Hit));
    assert!((lower.and_then(|r| r.cents).unwrap_or(0.0) - 10.0).abs() < 1e-6);
    assert_eq!(outcome(upper), Some(Outcome::Hit));
}

/// Only the lower tone sounded: it hits, the upper is missed ("nur ein Ton"); today any tone
/// counted for both.
#[test]
fn one_tone_heard_misses_the_other() {
    let before = judged(33.0, DoubleStopHeard::Unknown, InputSource::Line);
    let [lower, upper] = fifth(&before);
    assert_eq!(outcome(lower), Some(Outcome::Hit));
    assert_eq!(outcome(upper), Some(Outcome::Hit));

    let s = judged(33.0, DoubleStopHeard::Lower, InputSource::Line);
    let [lower, upper] = fifth(&s);
    assert_eq!(outcome(lower), Some(Outcome::Hit));
    assert_eq!(outcome(upper), Some(Outcome::Missed));
    assert_eq!(upper.and_then(|r| r.offset_ms), None);

    let s = judged(40.0, DoubleStopHeard::Upper, InputSource::Line);
    let [lower, upper] = fifth(&s);
    assert_eq!(outcome(lower), Some(Outcome::Missed));
    assert_eq!(outcome(upper), Some(Outcome::Hit));
}

/// A verdict naming the tone the tracker did not hear contradicts it: nothing changes.
#[test]
fn a_contradicting_verdict_changes_nothing() {
    let s = judged(40.0, DoubleStopHeard::Lower, InputSource::Line);
    let [lower, upper] = fifth(&s);
    assert_eq!(outcome(lower), Some(Outcome::Hit));
    assert_eq!(outcome(upper), Some(Outcome::Hit));
}

/// The verdict is final: a pitch told later does not judge the tones again.
#[test]
fn verdict_is_final() {
    let mut s = judged(39.0, DoubleStopHeard::Both, InputSource::Line);
    s.on_pitch_revision(&kontra_model::PitchRevision {
        onset_s: 0.0,
        midi: 38.0,
    });
    let [lower, upper] = fifth(&s);
    assert_eq!(outcome(lower), Some(Outcome::Hit));
    assert_eq!(outcome(upper), Some(Outcome::Hit));
}

/// MIDI hears every tone itself (`ChordHearing::Every`): no verdict judges anything there.
#[test]
fn midi_needs_no_verdict() {
    let s = judged(39.0, DoubleStopHeard::Both, InputSource::Midi);
    let [lower, _] = fifth(&s);
    assert!(
        matches!(outcome(lower), Some(Outcome::WrongPitch { .. })),
        "{lower:?}"
    );
}

/// Each tone's result counts once in the summary: two hits, nothing extra.
#[test]
fn summary_counts_both_tones() {
    let s = judged(39.0, DoubleStopHeard::Both, InputSource::Line);
    let summary = s.summary();
    let judged_fifth = s
        .results()
        .iter()
        .filter(|r| r.position_s == 0.0)
        .collect::<Vec<_>>();
    assert_eq!(judged_fifth.len(), 2, "{judged_fifth:?}");
    assert_eq!(summary.hits, 2);
}

/// A tap on the lower string before the double stop is taken back (R1), told before or after
/// the pluck: the verdict on the pluck judges both tones, nothing counts twice.
#[test]
fn verdict_after_a_tap_taken_back() {
    for pluck_first in [false, true] {
        let mut s = Session::new(piece(), Mode::Song, ScoreSettings::default());
        s.play(0.0);
        s.update(0.08);
        let tap = NoteRetraction { onset_s: -0.06 };
        s.on_note(&played(-0.06, 33.0, InputSource::Line));
        if pluck_first {
            s.on_note(&played(0.0, 39.0, InputSource::Line));
            s.on_retraction(&tap);
        } else {
            s.on_retraction(&tap);
            s.on_note(&played(0.0, 39.0, InputSource::Line));
        }
        s.update(0.16);
        s.on_double_stop(&DoubleStopVerdict {
            onset_s: 0.0,
            heard: DoubleStopHeard::Both,
        });
        let [lower, upper] = fifth(&s);
        let at = format!("pluck first: {pluck_first}, {:?}", s.results());
        assert_eq!(outcome(lower), Some(Outcome::Hit), "{at}");
        assert_eq!(outcome(upper), Some(Outcome::Hit), "{at}");
        assert_eq!(s.results().len(), 2, "{at}");
    }
}
