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
        technique: None,
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

/// Workshop over the whole riff at half speed: one pass is 8 s real, note `i` at `base + i`.
/// `hits[pass]` is how many of the eight notes are played right in that pass (the rest are missed).
fn workshop_run(hits: &[usize], until: f64) -> Vec<SessionEvent> {
    let workshop = Workshop {
        target_speed: 1.0,
        min_speed: 0.25,
        step: 0.05,
        band: (0.8, 0.9),
        passes_needed: 2,
    };
    let mut s = Session::new(riff(), Mode::Workshop(workshop), ScoreSettings::default());
    s.set_speed(0.0, 0.5);
    s.set_loop(
        0.0,
        Some(LoopRange {
            start_s: 0.0,
            end_s: 4.0,
        }),
    );
    s.play(0.0);
    let pitches = [28.0, 33.0, 38.0, 43.0, 28.0, 33.0, 38.0, 43.0];
    let mut notes = Vec::new();
    for (pass, count) in hits.iter().enumerate() {
        let base = pass as f64 * 8.0;
        for (i, midi) in pitches.iter().enumerate().take(*count) {
            notes.push(played(base + i as f64, *midi));
        }
    }
    run(&mut s, &notes, until)
}

/// Speed changes made by the workshop (the first event is the initial `set_speed`).
fn speed_changes(events: &[SessionEvent]) -> Vec<f64> {
    events
        .iter()
        .skip(1)
        .filter_map(|e| match e {
            SessionEvent::SpeedChanged(v) => Some(*v),
            _ => None,
        })
        .collect()
}

/// LH-MOD-04, target band (round 5): above 90 % hits in two passes in a row → one step faster.
#[test]
fn workshop_speeds_up_above_the_band() {
    let events = workshop_run(&[8, 8], 17.0);
    let rates: Vec<f64> = events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::PassComplete { hit_rate, .. } => Some(*hit_rate),
            _ => None,
        })
        .collect();
    assert_eq!(rates, vec![1.0, 1.0], "{events:?}");
    let changes = speed_changes(&events);
    assert_eq!(changes.len(), 1, "{events:?}");
    assert!((changes[0] - 0.55).abs() < 1e-9, "{changes:?}");
}

/// Below 80 % in two passes in a row → one step slower.
#[test]
fn workshop_slows_down_below_the_band() {
    let changes = speed_changes(&workshop_run(&[5, 5], 17.0));
    assert_eq!(changes.len(), 1, "{changes:?}");
    assert!((changes[0] - 0.45).abs() < 1e-9, "{changes:?}");
}

/// Inside the band (7 of 8 = 87.5 %) the tempo stays; one good pass alone does not count.
#[test]
fn workshop_holds_inside_the_band() {
    assert!(speed_changes(&workshop_run(&[7, 7, 7], 25.0)).is_empty());
    assert!(speed_changes(&workshop_run(&[8, 7, 8], 25.0)).is_empty());
}

/// QA P2 (F2): the first note of a loop pass played a little early falls before the wrap; it
/// belongs to the next pass's first note, not "extra" plus "missed".
#[test]
fn early_note_at_loop_start_counts_for_the_next_pass() {
    let mut s = Session::new(riff(), Mode::Song, ScoreSettings::default());
    s.set_loop(
        0.0,
        Some(LoopRange {
            start_s: 0.0,
            end_s: 2.0,
        }),
    );
    s.play(0.0);
    // Pass 0: notes at 0, 0.5, 1, 1.5; pass 1 starts at 2.0 – its first note 20 ms early.
    let notes = [
        played(0.0, 28.0),
        played(0.5, 33.0),
        played(1.0, 38.0),
        played(1.5, 43.0),
        played(1.98, 28.0),
        played(2.5, 33.0),
    ];
    run(&mut s, &notes, 3.0);
    let extras = s.results().iter().filter(|r| r.note.is_none()).count();
    assert_eq!(extras, 0, "{:?}", s.results());
    let first = s
        .results()
        .iter()
        .find(|r| r.pass == 1 && r.note == Some(0))
        .expect("pass 1 first note judged");
    assert_eq!(first.outcome, Outcome::Hit, "{:?}", s.results());
    assert!((first.offset_ms.unwrap() + 20.0).abs() < 1.0, "{first:?}");
}

/// QA P2 (F3): after a stall (stretching a backing track blocked the thread) notes arrive before
/// the clock was advanced past the loop end; they belong to the next pass, not "extra".
#[test]
fn notes_after_a_stall_land_in_the_right_pass() {
    let mut s = Session::new(riff(), Mode::Song, ScoreSettings::default());
    s.set_loop(
        0.0,
        Some(LoopRange {
            start_s: 0.0,
            end_s: 2.0,
        }),
    );
    s.play(0.0);
    // Nothing advanced the clock; three passes worth of notes arrive at once.
    for pass in 0..3 {
        let base = pass as f64 * 2.0;
        for (i, midi) in [28.0, 33.0, 38.0, 43.0].iter().enumerate() {
            s.on_note(&played(base + i as f64 * 0.5, *midi));
        }
    }
    s.update(6.6);
    let extras = s.results().iter().filter(|r| r.note.is_none()).count();
    let hits = s.results().iter().filter(|r| r.is_hit()).count();
    assert_eq!(extras, 0, "{:?}", s.results());
    assert_eq!(hits, 12, "{:?}", s.results());
}

/// Chords: any tone counts for the whole chord (the detector hears one note), and every chord
/// tone gets the result.
#[test]
fn chord_counts_once_for_all_tones() {
    let mut a = Arrangement::new("Chords", Instrument::default_bass());
    a.tempo = TempoMap::constant(120.0);
    for (beat, midi) in [(0u64, 28u8), (0, 35), (1, 33), (2, 38), (2, 45)] {
        a.notes.push(Note::new(
            beat * TICKS_PER_QUARTER,
            TICKS_PER_QUARTER,
            Pitch(midi),
        ));
    }
    a.normalize();
    let mut s = Session::new(a, Mode::Song, ScoreSettings::default());
    s.play(0.0);
    // Upper tone of the first chord, then the single note; the second chord is not played.
    run(&mut s, &[played(0.01, 35.0), played(0.5, 33.0)], 3.0);
    let summary = s.summary();
    assert_eq!(summary.hits, 3, "{:?}", s.results());
    assert_eq!(summary.missed, 2);
    assert_eq!(summary.extra, 0);
    let first_chord: Vec<_> = s
        .results()
        .iter()
        .filter(|r| r.position_s < 0.1)
        .map(|r| (r.note, r.outcome))
        .collect();
    assert_eq!(first_chord.len(), 2);
    assert!(first_chord.iter().all(|(_, o)| *o == Outcome::Hit));
}

/// Round 6: the right note 40 cents off on a fretless bass is "off pitch", not wrong; in Wait mode
/// it still releases the clock, and the summary counts it apart with its cents.
#[test]
fn off_pitch_is_its_own_outcome() {
    let fretless = ScoreSettings {
        pitch_cents: 30.0,
        ..ScoreSettings::default()
    };
    let mut s = Session::new(riff(), Mode::Wait, fretless);
    s.play(0.0);
    run(&mut s, &[], 1.0);
    s.on_note(&played(1.1, 28.4));
    let events = s.update(1.2);
    assert!(events.contains(&SessionEvent::Released));
    let r = s.results().iter().find(|r| r.note == Some(0)).unwrap();
    assert_eq!(r.outcome, Outcome::OffPitch);
    let summary = s.summary();
    assert_eq!((summary.off_pitch, summary.wrong, summary.hits), (1, 0, 0));
    assert!((summary.mean_abs_cents.unwrap() - 40.0).abs() < 1e-6);
}
