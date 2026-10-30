//! J-F4 Eingang gestört (P2 Jens, P1 Lea, P7 Sam): the input does not open; its state says so
//! and stays. "Erneut versuchen" opens it once it works again, a run keeps its place.

use crate::common::{piece, render, run_until, write_midi, Profile};
use kontra_appcore::{InputState, UiEvent};
use std::time::Duration;

const RIFF: [u8; 8] = [28, 33, 38, 43, 28, 33, 38, 43];

#[test]
fn a_failed_input_can_be_retried() {
    let profile = Profile::new("f4");
    let score = profile.dir.join("riff.mid");
    write_midi(&score, "Riff", &RIFF);
    // The cable is out: the "input" delivers no audio.
    let take = profile.dir.join("take.wav");
    std::fs::write(&take, b"no audio").unwrap();
    let mut core = profile.core(Some(&take));
    core.open(&score).unwrap();

    // Step 1: play fails; the state says so and stays.
    let start = core.play();
    assert!(
        start.iter().any(|e| matches!(e, UiEvent::Error(_))),
        "{start:?}"
    );
    assert_eq!(core.input_state(), InputState::Failed);
    run_until(&mut core, Duration::from_millis(300), |_| false);
    assert_eq!(core.input_state(), InputState::Failed);

    // Step 2: plugged in, "Erneut versuchen": connected, named, waiting for play.
    render(&take, &piece("Riff", &RIFF), Default::default());
    let retry = core.retry_input();
    assert!(retry.is_empty(), "{retry:?}");
    assert_eq!(core.input_state(), InputState::Connected);
    assert_eq!(core.input_name(), "take.wav");
    assert!(!core.is_playing());

    // Step 3: a retry during a run keeps its place, paused.
    core.play();
    run_until(&mut core, Duration::from_millis(2500), |_| false);
    let at_s = core.position_s();
    assert!(at_s > 0.5, "{at_s}");
    assert!(core.retry_input().is_empty());
    assert_eq!(core.input_state(), InputState::Connected);
    // The jump takes effect after the engine's start lead.
    run_until(&mut core, Duration::from_millis(300), |_| false);
    assert!(
        (core.position_s() - at_s).abs() < 0.2,
        "{} vs {at_s}",
        core.position_s()
    );
    assert!(!core.is_playing());

    // Step 4: the run closed, nothing to show.
    core.stop_live();
    assert_eq!(core.input_state(), InputState::Off);

    // Step 5: the tuner's input fails; leaving the tuner ends its state.
    std::fs::write(&take, b"no audio").unwrap();
    assert!(core.start_tuner().is_err());
    assert_eq!(core.input_state(), InputState::Failed);
    core.stop_tuner();
    assert_eq!(core.input_state(), InputState::Off);
}
