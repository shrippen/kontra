//! J-B4 Bundlos und tief (P3 Mira): a five-string fretless, strict timing, a walking line down to
//! B0 played with deliberate intonation errors. Each note's cents must match the annotation
//! within 3 ct; beyond the fretless limit (30 ct) a note is "offpitch", not wrong. Reading the
//! notation in the bass clef is checked by eye (screenshot).

use crate::common::{run_until, write_midi, Profile, COUNT_IN_S, RATE};
use kontra_appcore::UiEvent;
use std::time::Duration;

/// B0 C1 D1 E1 on the B and E strings, then up and back down.
const LINE: [u8; 8] = [23, 24, 26, 28, 31, 33, 28, 23];
/// Intonation per note, ±10 to ±40 cents.
const CENTS: [f64; 8] = [12.0, -18.0, 25.0, -10.0, 38.0, -35.0, 15.0, -22.0];
const FRETLESS_LIMIT_CENTS: f64 = 30.0;
const TOLERANCE_CENTS: f64 = 3.0;

#[test]
fn low_fretless_line_reads_its_cents() {
    let profile = Profile::new("b4");
    let score = profile.dir.join("Walking.mid");
    write_midi(&score, "Walking", &LINE);
    let take = profile.dir.join("bundlos.wav");

    let mut core = profile.core(None);
    core.reconfigure(|s| {
        s.strings = 5;
        s.fretless = true;
        s.strictness = "strict".into();
    });
    core.open(&score).unwrap();
    let a = core.arrangement().unwrap().clone();
    assert_eq!(
        a.instrument.tuning.strings.len(),
        5,
        "the five-string profile"
    );
    assert!(
        a.notes.iter().all(|n| n.fret.is_some()),
        "B0 lies on the B string"
    );
    let mistakes = kontra_session::Mistakes {
        detune_cycle_cents: &CENTS,
        ..Default::default()
    };
    let samples = kontra_session::render_performance(&a, 1.0, COUNT_IN_S, mistakes, RATE);
    kontra_session::write_mono_wav(&take, &samples, RATE).unwrap();

    let mut core = profile.core(Some(&take));
    core.open(&score).unwrap();
    core.set_mode("song");
    core.play();
    let events = run_until(&mut core, Duration::from_secs(9), |e| {
        matches!(e, UiEvent::Finished { .. })
    });
    let mut judged: Vec<(usize, &str, f64)> = events
        .iter()
        .filter_map(|e| match e {
            UiEvent::Judged {
                index,
                state,
                cents,
                ..
            } => Some((*index, *state, *cents)),
            _ => None,
        })
        .collect();
    judged.sort_by_key(|(index, _, _)| *index);
    assert_eq!(
        judged.len(),
        LINE.len(),
        "every note heard, B0 to D1 too: {judged:?}"
    );
    for ((index, state, cents), wanted) in judged.iter().zip(CENTS) {
        assert!(
            (cents - wanted).abs() <= TOLERANCE_CENTS,
            "note {index}: {cents} ct, annotated {wanted}"
        );
        let expected = if wanted.abs() > FRETLESS_LIMIT_CENTS {
            "offpitch"
        } else {
            "hit"
        };
        assert_eq!(*state, expected, "note {index} at {wanted} ct");
    }
}
