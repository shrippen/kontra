//! J-B2 Warten-Modus (P1 Lea): three wrong notes, then the right one, then a long pause.

use crate::common::{judged, run_until, write_midi, Profile, BPM, COUNT_IN_S, RATE};
use kontra_appcore::UiEvent;
use kontra_model::{Arrangement, Instrument, Note, Pitch, TempoMap, TICKS_PER_QUARTER};
use std::time::Duration;

const RIFF: [u8; 4] = [28, 33, 38, 43];

/// What Lea plays (seconds after the count-in, MIDI): F1 three times, E1, A1, a pause, D2, G2.
const PLAYED: [(f64, u8); 7] = [
    (0.0, 29),
    (0.5, 29),
    (1.0, 29),
    (1.5, 28),
    (2.0, 33),
    (5.0, 38),
    (5.5, 43),
];

fn performance() -> Arrangement {
    let mut a = Arrangement::new("take", Instrument::default_bass());
    a.tempo = TempoMap::constant(BPM);
    let ticks_per_s = BPM / 60.0 * TICKS_PER_QUARTER as f64;
    for (at_s, midi) in PLAYED {
        let tick = (at_s * ticks_per_s) as u64;
        a.notes
            .push(Note::new(tick, TICKS_PER_QUARTER / 2, Pitch(midi)));
    }
    a.normalize();
    a
}

#[test]
fn waits_names_wrong_notes_and_survives_a_pause() {
    let profile = Profile::new("b2");
    let score = profile.dir.join("riff.mid");
    write_midi(&score, "Seven-Nation-Riff", &RIFF);
    let take = profile.dir.join("take.wav");
    let samples = kontra_session::render_performance(
        &performance(),
        1.0,
        COUNT_IN_S,
        Default::default(),
        RATE,
    );
    kontra_session::write_mono_wav(&take, &samples, RATE).unwrap();
    let mut core = profile.core(Some(&take));
    core.open(&score).unwrap();
    core.set_mode("wait");
    let start = core.play();
    assert!(
        !start.iter().any(|e| matches!(e, UiEvent::Error(_))),
        "{start:?}"
    );
    let events = run_until(&mut core, Duration::from_secs(12), |e| {
        matches!(e, UiEvent::Finished { .. })
    });

    // A wrong note is named in words, with the wanted note and string: "Gespielt: F1 …".
    let texts: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            UiEvent::Holding { index: 0, text } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        texts.iter().any(|t| t.contains("F1") && t.contains("E1")),
        "{texts:?}"
    );

    // Nothing is marked wrong or missed; all four notes count once played, also after the pause.
    let states = judged(&events);
    assert_eq!(states.len(), RIFF.len(), "{states:?}");
    assert!(
        states.iter().all(|s| ["hit", "early", "late"].contains(s)),
        "{states:?}"
    );
    assert!(events.iter().any(|e| matches!(e, UiEvent::Finished { .. })));
    assert!(!events
        .iter()
        .any(|e| matches!(e, UiEvent::Error(_) | UiEvent::DeviceLost(..))));
}
