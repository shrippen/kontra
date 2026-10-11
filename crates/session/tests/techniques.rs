//! Technique scoring through a session (LH-BEW-06): apart from the hit rate.

use kontra_model::{
    Arrangement, DetectedNote, InputSource, Instrument, Note, Pitch, Technique, TempoMap,
    TICKS_PER_QUARTER,
};
use kontra_session::{
    Mode, ScoreSettings, Session, SessionEvent, TechniqueOutcome, TechniqueTally,
};

/// At 120 BPM, one quarter apart on the E string: E1, then A1 hammered on, then G1 slapped.
fn piece() -> Arrangement {
    let mut a = Arrangement::new("Techniken", Instrument::default_bass());
    a.tempo = TempoMap::constant(120.0);
    let notes: [(u8, Option<Technique>); 3] = [
        (28, None),
        (33, Some(Technique::HammerOn)),
        (31, Some(Technique::Slap)),
    ];
    for (i, (midi, technique)) in notes.iter().enumerate() {
        let mut n = Note::new(
            i as u64 * TICKS_PER_QUARTER,
            TICKS_PER_QUARTER,
            Pitch(*midi),
        );
        n.string = Some(0);
        n.techniques = technique.iter().copied().collect();
        a.notes.push(n);
    }
    a.normalize();
    a
}

fn played(onset_s: f64, midi: f64, technique: Option<Technique>) -> DetectedNote {
    DetectedNote {
        onset_s,
        midi,
        confidence: 0.9,
        level_db: -12.0,
        source: InputSource::Line,
        technique,
    }
}

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

#[test]
fn hammer_on_counts_apart_and_is_no_extra() {
    let mut s = Session::new(piece(), Mode::Song, ScoreSettings::default());
    s.play(0.0);
    let take = [
        played(0.0, 28.0, None),
        played(0.5, 33.0, Some(Technique::HammerOn)),
        played(1.0, 31.0, None), // plucked, not slapped
    ];
    run(&mut s, &take, 2.0);
    let summary = s.summary();
    assert_eq!(summary.extra, 0, "the hammer-on is not an extra note");
    assert_eq!(summary.hit_rate, 1.0, "a plucked slap still hits");
    assert_eq!(
        summary.techniques,
        vec![
            TechniqueTally {
                technique: Technique::HammerOn,
                played: 1,
                judged: 1
            },
            TechniqueTally {
                technique: Technique::Slap,
                played: 0,
                judged: 1
            },
        ]
    );
    let slap = s.technique_results().iter().find(|r| r.note == 2).unwrap();
    assert_eq!(slap.outcome, TechniqueOutcome::Other(None));
}

#[test]
fn unheard_techniques_count_as_not_played() {
    let mut s = Session::new(piece(), Mode::Song, ScoreSettings::default());
    s.play(0.0);
    let events = run(&mut s, &[played(0.0, 28.0, None)], 2.0);
    let told: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::Technique(t) => Some((t.note, t.outcome)),
            _ => None,
        })
        .collect();
    assert_eq!(
        told,
        vec![
            (1, TechniqueOutcome::NotHeard),
            (2, TechniqueOutcome::NotHeard)
        ]
    );
}

#[test]
fn microphone_leaves_slap_unjudged() {
    let settings =
        ScoreSettings::defaults(Instrument::default_bass().kind, InputSource::Microphone);
    let mut s = Session::new(piece(), Mode::Song, settings);
    s.play(0.0);
    run(&mut s, &[played(0.0, 28.0, None)], 2.0);
    let summary = s.summary();
    assert_eq!(summary.untold_techniques, vec![Technique::Slap]);
    assert!(summary
        .techniques
        .iter()
        .all(|t| t.technique != Technique::Slap));
}

/// Wait mode (P1 Lea): a hammer-on played while Kontra waits for the next struck note is the
/// hammer-on, not a wrong note for the one it waits for.
#[test]
fn wait_mode_takes_a_hammer_on_as_such() {
    let mut s = Session::new(piece(), Mode::Wait, ScoreSettings::default());
    s.play(0.0);
    let take = [
        played(0.0, 28.0, None),
        played(0.5, 33.0, Some(Technique::HammerOn)),
        played(1.0, 31.0, Some(Technique::Slap)),
    ];
    run(&mut s, &take, 2.0);
    let wrong: Vec<_> = s
        .results()
        .iter()
        .filter(|r| matches!(r.outcome, kontra_session::Outcome::WrongPitch { .. }))
        .collect();
    assert!(wrong.is_empty(), "{wrong:?}");
    let told: Vec<_> = s
        .technique_results()
        .iter()
        .map(|t| (t.note, t.outcome))
        .collect();
    assert_eq!(
        told,
        vec![(1, TechniqueOutcome::Played), (2, TechniqueOutcome::Played)]
    );
}

/// A technique told after its note (the attack needed more audio): the note is judged again,
/// and the evaluation counts the later verdict.
#[test]
fn technique_told_later_judges_again() {
    let mut s = Session::new(piece(), Mode::Song, ScoreSettings::default());
    s.play(0.0);
    let take = [
        played(0.0, 28.0, None),
        played(0.5, 33.0, Some(Technique::HammerOn)),
        played(1.0, 31.0, None),
    ];
    run(&mut s, &take, 1.2);
    s.on_revision(&kontra_model::TechniqueRevision {
        onset_s: 1.0,
        technique: Technique::Slap,
    });
    run(&mut s, &[], 2.0);
    let slap = s
        .summary()
        .techniques
        .into_iter()
        .find(|t| t.technique == Technique::Slap);
    assert_eq!(slap.map(|t| (t.played, t.judged)), Some((1, 1)));
}

/// A harmonic sounds high and the tracker often hears it an octave or two lower (IDMT): its
/// pitch counts in any octave.
#[test]
fn harmonic_counts_in_any_octave() {
    let mut a = piece();
    a.notes[0].pitch = kontra_model::Pitch(52);
    a.notes[0].techniques = vec![Technique::Harmonic];
    let mut s = Session::new(a, Mode::Song, ScoreSettings::default());
    s.play(0.0);
    run(&mut s, &[played(0.0, 28.02, None)], 0.6);
    let first = s.results().iter().find(|r| r.note == Some(0)).unwrap();
    assert!(first.is_hit(), "{first:?}");
}

/// A dead note (x in the tab, `Muted`) has no pitch either: played as one, it hits.
#[test]
fn dead_note_played_muted_hits() {
    let mut a = piece();
    a.notes[0].techniques = vec![Technique::Muted];
    let mut s = Session::new(a, Mode::Song, ScoreSettings::default());
    s.play(0.0);
    run(&mut s, &[played(0.0, 0.0, Some(Technique::Ghost))], 0.6);
    let first = s.results().iter().find(|r| r.note == Some(0)).unwrap();
    assert!(first.is_hit(), "{first:?}");
}

/// A dead note heard with a pitch is told muted only later (the attack needs more audio): the
/// revision turns its wrong pitch into a hit, in song mode and while waiting.
#[test]
fn late_ghost_verdict_fixes_the_pitch() {
    let mut dead = piece();
    dead.notes[2].techniques = vec![Technique::Muted];
    for mode in [Mode::Song, Mode::Wait] {
        let mut s = Session::new(dead.clone(), mode, ScoreSettings::default());
        s.play(0.0);
        let take = [
            played(0.0, 28.0, None),
            played(0.5, 33.0, Some(Technique::HammerOn)),
            played(1.0, 36.0, None),
        ];
        run(&mut s, &take, 1.15);
        s.on_revision(&kontra_model::TechniqueRevision {
            onset_s: 1.0,
            technique: Technique::Ghost,
        });
        let last = s
            .results()
            .iter()
            .rev()
            .find(|r| r.note == Some(2))
            .unwrap();
        assert!(last.is_hit(), "{mode:?}: {last:?}");
        if mode == Mode::Wait {
            assert!(
                s.is_finished() || s.next_expected().is_none(),
                "the wait is over"
            );
        }
    }
}
