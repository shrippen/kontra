//! J-C4 Hören und nachspielen (P1 Lea, P3 Mira): Kontra plays a prompt, the bass answers. A note
//! struck while the prompt still sounds is no answer; the next one is judged.

use crate::common::{run_until, Profile, BPM, RATE};
use kontra_appcore::UiEvent;
use kontra_model::{Arrangement, Instrument, Note, Pitch, TempoMap, TICKS_PER_QUARTER};
use std::time::Duration;

const A1: u8 = 33;
const D2: u8 = 38;
/// The first note falls into the prompt (0.15–0.85 s), the second (2.3 s) after it.
const LEAD_IN_S: f64 = 0.3;

#[test]
fn notes_during_the_prompt_do_not_count() {
    let profile = Profile::new("c4");
    let take = profile.dir.join("gehoer.wav");
    // A1 during the prompt, D2 after it (beat 4 at 120 BPM). Only D2 may be judged.
    let mut a = Arrangement::new("Antwort", Instrument::default_bass());
    a.tempo = TempoMap::constant(BPM);
    for (beat, midi) in [(0, A1), (4, D2)] {
        a.notes.push(Note::new(
            beat * TICKS_PER_QUARTER,
            TICKS_PER_QUARTER,
            Pitch(midi),
        ));
    }
    a.normalize();
    let samples = kontra_session::render_performance(&a, 1.0, LEAD_IN_S, Default::default(), RATE);
    kontra_session::write_mono_wav(&take, &samples, RATE).unwrap();

    let mut core = profile.core(Some(&take));
    let first: serde_json::Value = serde_json::from_str(&core.ear_start("echo").unwrap()).unwrap();
    assert_eq!(first["total"], 1, "starts with one note");
    assert_eq!(first["prompting"], true);

    let events = run_until(&mut core, Duration::from_secs(4), |_| false);
    let states: Vec<serde_json::Value> = events
        .iter()
        .filter_map(|e| match e {
            UiEvent::Ear(json) => serde_json::from_str(json).ok(),
            _ => None,
        })
        .collect();
    assert!(
        states.iter().any(|s| s["prompting"] == false),
        "the UI learns when the prompt is over"
    );
    let last = states.last().unwrap();
    let played = last["played"].as_array().unwrap();
    assert_eq!(played.len(), 1, "{last}");
    assert_eq!(played[0]["note"], "D2");

    // Solved or not, the round goes on with a new prompt.
    let next: serde_json::Value = serde_json::from_str(&core.ear_next()).unwrap();
    assert_eq!(next["played"].as_array().unwrap().len(), 0);
    core.ear_stop();
}
