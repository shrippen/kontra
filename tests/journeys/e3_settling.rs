//! J-E3 Ton schwingt ein (P6 Karl, P3 Mira): a walking line whose notes start up to a semitone
//! flat and rise to pitch within about 120 ms, as on the FiloBass double bass. The marks come as
//! fast as for any note; a note judged wrong at its flat start becomes a hit once the tone has
//! settled (decision round 11). Step 2 (R1, LH-BEW-15): a finger tapping the string before the
//! pluck is told early and taken back; the mark ends on the pluck's verdict.

use crate::common::{last_judged, run_until, write_midi, Profile, COUNT_IN_S, RATE};
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

/// QA 27.10. (P3 Mira): with the strict window or fretless the flat starts stayed
/// wrong (0 %); the settling must not depend on them.
#[test]
fn flat_starts_turn_into_hits_when_strict() {
    settles_into_hits("e3-strict", |s| s.strictness = "strict".into());
}

#[test]
fn flat_starts_turn_into_hits_when_fretless() {
    settles_into_hits("e3-fretless", |s| {
        s.instrument = kontra_appcore::Bass::Fretless;
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

/// The tap before each pluck: this long before it, this much quieter (−30 dB, hardware check
/// 13.10.).
const TAP_S: f64 = 0.06;
const TAP_LEVEL: f64 = 0.03;

/// Each plucked note sounds this long: the tap before the next falls into silence and is told
/// (from F#2 up in the same block as the pluck after it).
const PLUCK_S: f64 = 0.3;

/// How the pluck after each tap is played.
#[derive(Clone, Copy)]
enum Pluck {
    InTune,
    /// A semitone sharp: the tap was on the right pitch, the pluck is not.
    Sharp,
}

/// The line, each note after a faint tap on its own pitch.
fn tapped_line(pluck: Pluck) -> Vec<f32> {
    let rate = f64::from(RATE);
    let total = COUNT_IN_S + QUARTER_S * (LINE.len() as f64 + 1.0);
    let mut out = vec![0.0f32; (total * rate) as usize];
    for (i, midi) in LINE.iter().enumerate() {
        let hz = Pitch(*midi).hz(A4_HZ);
        let sharp = match pluck {
            Pluck::InTune => 1.0,
            Pluck::Sharp => 2f64.powf(1.0 / 12.0),
        };
        let pluck_s = COUNT_IN_S + i as f64 * QUARTER_S;
        for (start_s, length_s, level, hz) in [
            (pluck_s - TAP_S, TAP_S, TAP_LEVEL, hz),
            (pluck_s, PLUCK_S, 1.0, hz * sharp),
        ] {
            let from = (start_s * rate) as usize;
            let len = (length_s * rate) as usize;
            for (k, slot) in out[from..from + len].iter_mut().enumerate() {
                let t = k as f64 / rate;
                let phase = std::f64::consts::TAU * hz * t;
                let tone: f64 = (1..=6).map(|h| (phase * h as f64).sin() / h as f64).sum();
                let fade = ((length_s - t) / 0.005).min(1.0);
                let envelope = (t / 0.002).min(1.0) * (-t / 0.8).exp() * fade;
                *slot = (0.4 * level * envelope * tone) as f32;
            }
        }
    }
    out
}

#[test]
fn tap_before_the_pluck_is_taken_back() {
    taps_taken_back("e3-tap", "song");
}

#[test]
fn tap_before_the_pluck_is_taken_back_waiting() {
    taps_taken_back("e3-tap-wait", "wait");
}

fn taps_taken_back(name: &str, mode: &str) {
    let profile = Profile::new(name);
    let score = profile.dir.join("Walking.mid");
    write_midi(&score, "Walking", &LINE);
    let take = profile.dir.join("tippen.wav");
    kontra_session::write_mono_wav(&take, &tapped_line(Pluck::InTune), RATE).unwrap();
    let mut core = profile.core(Some(&take));
    core.open(&score).unwrap();
    core.set_mode(mode);
    core.play();
    let events = run_until(&mut core, Duration::from_secs(8), |e| {
        matches!(e, UiEvent::Finished { .. })
    });

    // Every note ends a hit on the pluck's time, no tap left as an extra note.
    let last = last_judged(&events);
    assert_eq!(last.len(), LINE.len(), "{last:?}");
    for (index, (state, offset_ms, _)) in &last {
        assert_eq!(*state, "hit", "note {index}: {last:?}");
        assert!(offset_ms.abs() < 15.0, "note {index}: {last:?}");
    }
    assert!(
        !events.iter().any(|e| matches!(e, UiEvent::Extra)),
        "{last:?}"
    );
}

/// Wait mode, the tap on D2 ends the wait, the pluck after it is sharp: the tap is taken back
/// and Kontra waits for D2 again. Its mark goes with the tap's verdict (LH-BEW-15), no "hit"
/// stays on the note waited for.
#[test]
fn tap_taken_back_clears_its_mark_waiting() {
    let profile = Profile::new("e3-tap-sharp");
    let score = profile.dir.join("Walking.mid");
    write_midi(&score, "Walking", &LINE);
    let take = profile.dir.join("tippen.wav");
    kontra_session::write_mono_wav(&take, &tapped_line(Pluck::Sharp), RATE).unwrap();
    let mut core = profile.core(Some(&take));
    core.open(&score).unwrap();
    core.set_mode("wait");
    core.play();
    let events = run_until(&mut core, Duration::from_secs(4), |_| false);

    let tapped = events
        .iter()
        .any(|e| matches!(e, UiEvent::Judged { index: 0, .. }));
    assert!(tapped, "the tap was judged first: {events:?}");
    let last = last_judged(&events);
    assert_eq!(last.get(&0).map(|j| j.0), Some("pending"), "{last:?}");

    // Waiting again after the tap, Kontra names the sharp pluck (J-E3 step 3, review #210).
    let reopened = events
        .iter()
        .position(|e| {
            matches!(
                e,
                UiEvent::Judged {
                    index: 0,
                    state: "pending",
                    ..
                }
            )
        })
        .unwrap_or(events.len());
    let named = core.wait_text(0, Some(f64::from(LINE[0] + 1))).unwrap();
    let names = |e: &UiEvent| matches!(e, UiEvent::Holding { index: 0, text } if *text == named);
    assert!(events[reopened..].iter().any(names), "{named}: {events:?}");
}
