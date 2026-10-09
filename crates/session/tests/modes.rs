//! End-to-end checks of the four modes with simulated detections.

use kontra_model::{
    Arrangement, DetectedNote, InputSource, Instrument, Note, Pitch, TempoMap, TICKS_PER_QUARTER,
};
use kontra_session::{LoopRange, Mode, Outcome, ScoreSettings, Session, SessionEvent, Workshop};

/// Eight quarter notes at 120 BPM: E1 A1 D2 G2 E1 A1 D2 G2 (0.5 s apart, 2 bars).
fn riff() -> Arrangement {
    let mut a = Arrangement::new("Riff", Instrument::default_bass());
    a.tempo = TempoMap::constant(120.0);
    for (i, midi) in [28u8, 33, 38, 43, 28, 33, 38, 43].iter().enumerate() {
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
    }
}

/// Runs the clock in 10 ms steps up to `until`, feeding detections when their report time
/// (onset + 80 ms) is reached.
fn run(session: &mut Session, notes: &[DetectedNote], until: f64) -> Vec<SessionEvent> {
    let mut events = Vec::new();
    let mut pending: Vec<DetectedNote> = notes.to_vec();
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
fn song_mode_scores_hits_wrong_and_missed() {
    let mut s = Session::new(riff(), Mode::Song, ScoreSettings::default());
    s.play(0.0);
    let notes = [
        played(0.012, 28.05), // hit, 12 ms late
        played(0.49, 33.0),   // hit, 10 ms early
        played(1.0, 40.0),    // wrong pitch
        // 1.5 missed
        played(2.0, 28.0),
        played(2.5, 33.0),
        played(3.03, 38.0),
        played(3.5, 43.0),
        played(3.8, 30.0), // extra
    ];
    let events = run(&mut s, &notes, 5.0);
    let summary = s.summary();
    assert_eq!(summary.judged, 8);
    assert_eq!(summary.hits, 6);
    assert_eq!(summary.wrong, 1);
    assert_eq!(summary.missed, 1);
    assert_eq!(summary.extra, 1);
    assert!((summary.mean_offset_ms.unwrap() - (12.0 - 10.0 + 30.0) / 6.0).abs() < 0.01);
    assert!(events.contains(&SessionEvent::Finished));
    assert_eq!(summary.hardest_bars, vec![0]);
}

#[test]
fn latency_is_subtracted() {
    let settings = ScoreSettings {
        latency_ms: 40.0,
        ..ScoreSettings::default()
    };
    let mut s = Session::new(riff(), Mode::Song, settings);
    s.play(0.0);
    run(&mut s, &[played(0.04, 28.0)], 0.3);
    let hit = s.results().iter().find(|r| r.note == Some(0)).unwrap();
    assert!(hit.offset_ms.unwrap().abs() < 1e-6);
}

#[test]
fn wait_mode_holds_until_right_note() {
    let mut s = Session::new(riff(), Mode::Wait, ScoreSettings::default());
    s.play(0.0);
    // Nothing played for 2 s: the clock holds at the first note.
    let events = run(&mut s, &[], 2.0);
    assert!(events.contains(&SessionEvent::Holding { note: 0 }));
    assert_eq!(s.transport().position_at(2.0).0, 0.0);
    // Wrong, then right.
    s.on_note(&played(2.1, 30.0));
    s.on_note(&played(2.3, 28.0));
    let events = s.update(2.4);
    assert!(events.contains(&SessionEvent::Released));
    assert!(s
        .results()
        .iter()
        .any(|r| r.note == Some(0) && matches!(r.outcome, Outcome::WrongPitch { .. })));
    // Holds again at the second note (position 0.5).
    s.update(3.5);
    assert!((s.transport().position_at(3.5).0 - 0.5).abs() < 1e-9);
}

#[test]
fn workshop_speeds_up_after_clean_passes() {
    let workshop = Workshop {
        target_speed: 0.7,
        step: 0.1,
        clean_needed: 2,
    };
    let mut s = Session::new(riff(), Mode::Workshop(workshop), ScoreSettings::default());
    s.set_speed(0.0, 0.5);
    s.set_loop(
        0.0,
        Some(LoopRange {
            start_s: 0.0,
            end_s: 1.0,
        }),
    );
    s.play(0.0);
    // At 0.5 speed the loop (two notes at 0.0 and 0.5) lasts 2 s real: notes at real 0 and 1.
    let mut notes = Vec::new();
    for pass in 0..6 {
        let base = pass as f64 * 2.0;
        notes.push(played(base, 28.0));
        notes.push(played(base + 1.0, 33.0));
    }
    // Only the first passes are exact; after speeding up the real times no longer fit,
    // but the speed change itself is what we check.
    let events = run(&mut s, &notes, 4.6);
    let clean: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, SessionEvent::PassComplete { clean: true, .. }))
        .collect();
    assert_eq!(clean.len(), 2, "{events:?}");
    assert!(events
        .iter()
        .any(|e| matches!(e, SessionEvent::SpeedChanged(v) if (v - 0.6).abs() < 1e-9)));
}
