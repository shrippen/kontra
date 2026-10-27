//! J-E3 Ton schwingt ein (P6 Karl, P3 Mira): a walking line whose notes start up to a semitone
//! flat and rise to pitch within about 120 ms, as on the FiloBass double bass. The marks come as
//! fast as for any note; a note judged wrong at its flat start becomes a hit once the tone has
//! settled (decision round 11).

use crate::common::{run_until, write_midi, Profile, COUNT_IN_S, RATE};
use kontra_appcore::UiEvent;
use kontra_model::{Pitch, A4_HZ};
use std::time::Duration;

/// D2 E2 F2 G2, quarters at 120 BPM.
const LINE: [u8; 4] = [38, 40, 41, 43];
const QUARTER_S: f64 = 0.5;
/// How flat each note starts, and how long it takes to rise.
const FLAT_CENTS: f64 = -140.0;
const RISE_S: f64 = 0.12;
/// The player stops each note before the next, without a click.
const FADE_S: f64 = 0.05;

/// The line after the count-in, each note rising from `FLAT_CENTS` to its pitch.
fn rising_line() -> Vec<f32> {
    let rate = f64::from(RATE);
    let total = COUNT_IN_S + QUARTER_S * (LINE.len() as f64 + 1.0);
    let mut out = vec![0.0f32; (total * rate) as usize];
    for (i, midi) in LINE.iter().enumerate() {
        let hz = Pitch(*midi).hz(A4_HZ);
        let from = ((COUNT_IN_S + i as f64 * QUARTER_S) * rate) as usize;
        let mut phase = 0.0f64;
        for (k, slot) in out[from..from + (QUARTER_S * 0.95 * rate) as usize]
            .iter_mut()
            .enumerate()
        {
            let t = k as f64 / rate;
            let cents = FLAT_CENTS * (1.0 - t / RISE_S).max(0.0);
            phase += std::f64::consts::TAU * hz * 2f64.powf(cents / 1200.0) / rate;
            let tone: f64 = (1..=6).map(|h| (phase * h as f64).sin() / h as f64).sum();
            let fade = ((QUARTER_S * 0.95 - t) / FADE_S).min(1.0);
            let envelope = (t / 0.002).min(1.0) * (-t / 0.8).exp() * fade;
            *slot = (0.4 * envelope * tone) as f32;
        }
    }
    out
}

#[test]
fn flat_starts_turn_into_hits() {
    settles_into_hits("e3", |_| {});
}

/// QA 27.10. (P3 Mira): with the strict window, five strings or fretless the flat starts stayed
/// wrong (0 %); the settling must not depend on them.
#[test]
fn flat_starts_turn_into_hits_when_strict() {
    settles_into_hits("e3-strict", |s| s.strictness = "strict".into());
}

#[test]
fn flat_starts_turn_into_hits_on_five_fretless_strings() {
    settles_into_hits("e3-fretless", |s| {
        s.fretless = true;
    });
}

fn settles_into_hits(name: &str, choose: impl FnOnce(&mut kontra_appcore::Settings)) {
    let profile = Profile::new(name);
    let score = profile.dir.join("Walking.mid");
    write_midi(&score, "Walking", &LINE);
    let take = profile.dir.join("einschwingen.wav");
    kontra_session::write_mono_wav(&take, &rising_line(), RATE).unwrap();

    let mut core = profile.core(Some(&take));
    core.update_settings(choose);
    core.open(&score).unwrap();
    core.set_mode("song");
    core.play();
    let events = run_until(&mut core, Duration::from_secs(8), |e| {
        matches!(e, UiEvent::Finished { .. })
    });
    let marks: Vec<(usize, &str)> = events
        .iter()
        .filter_map(|e| match e {
            UiEvent::Judged { index, state, .. } => Some((*index, *state)),
            _ => None,
        })
        .collect();

    // Shown at once as heard, then corrected: each note's last mark is a hit.
    assert!(
        marks.iter().any(|(_, s)| *s == "wrong"),
        "the flat start is shown as heard: {marks:?}"
    );
    for index in 0..LINE.len() {
        let last = marks.iter().rev().find(|(i, _)| *i == index);
        assert_eq!(last.map(|m| m.1), Some("hit"), "note {index}: {marks:?}");
    }
    assert!(
        !events.iter().any(|e| matches!(e, UiEvent::Extra)),
        "{marks:?}"
    );
}
