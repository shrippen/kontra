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

    // Step 1: play fails; the state says so, once (no second bar), with the system's reason,
    // and stays.
    let start = core.play();
    assert!(
        !start.iter().any(|e| matches!(e, UiEvent::Error(_))),
        "{start:?}"
    );
    assert_eq!(core.input_state(), InputState::Failed);
    assert!(
        core.input_detail().contains("take.wav"),
        "{}",
        core.input_detail()
    );
    run_until(&mut core, Duration::from_millis(300), |_| false);
    assert_eq!(core.input_state(), InputState::Failed);

    // Step 2: plugged in, "Erneut versuchen": connected, named, waiting for play.
    render(&take, &piece("Riff", &RIFF), Default::default());
    let retry = core.retry_input();
    assert!(retry.is_empty(), "{retry:?}");
    assert_eq!(core.input_state(), InputState::Connected);
    assert_eq!(core.input_detail(), "take.wav");
    assert!(!core.is_playing());

    // Step 3 (`a_retry_keeps_the_run`). Step 4: the run closed, nothing to show.
    core.stop_live();
    assert_eq!(core.input_state(), InputState::Off);

    // Step 5: the tuner's input fails; leaving the tuner ends its state.
    std::fs::write(&take, b"no audio").unwrap();
    assert!(core.start_tuner().is_err());
    assert_eq!(core.input_state(), InputState::Failed);
    core.stop_tuner();
    assert_eq!(core.input_state(), InputState::Off);
}

/// Review #213: a backing track that does not decode is no input fault; it says so as before.
#[test]
fn a_broken_backing_is_no_input_fault() {
    let profile = Profile::new("f4-backing");
    let score = profile.dir.join("riff.mid");
    write_midi(&score, "Riff", &RIFF);
    let take = profile.dir.join("take.wav");
    render(&take, &piece("Riff", &RIFF), Default::default());
    let backing = profile.dir.join("backing.wav");
    std::fs::write(&backing, b"no audio").unwrap();
    let mut core = profile.core(Some(&take));
    core.open(&score).unwrap();
    core.set_backing(Some(backing), 0.0);
    run_until(&mut core, Duration::from_millis(500), |_| false);

    let start = core.play();
    assert!(
        start.iter().any(|e| matches!(e, UiEvent::Error(_))),
        "{start:?}"
    );
    assert_ne!(core.input_state(), InputState::Failed);
}

/// Step 3: a retry during a run keeps the run (review #213: it was stored twice).
#[test]
fn a_retry_keeps_the_run() {
    let profile = Profile::new("f4-run");
    let score = profile.dir.join("riff.mid");
    write_midi(&score, "Riff", &RIFF);
    let take = profile.dir.join("take.wav");
    render(&take, &piece("Riff", &RIFF), Default::default());
    let mut core = profile.core(Some(&take));
    core.open(&score).unwrap();

    // A retry during a run keeps the run: it goes on from its place and is stored once.
    core.play();
    run_until(&mut core, Duration::from_millis(3000), |_| false);
    let at_s = core.position_s();
    assert!(at_s > 0.5, "{at_s}");
    assert!(core.retry_input().is_empty());
    assert_eq!(core.input_state(), InputState::Connected);
    assert!(core.is_playing());
    assert!(core.position_s() >= at_s, "{} vs {at_s}", core.position_s());
    run_until(&mut core, Duration::from_secs(8), |e| {
        matches!(e, UiEvent::Finished { .. })
    });
    let progress: serde_json::Value = serde_json::from_str(&core.progress_json()).unwrap();
    assert_eq!(progress["pieces"][0]["runs"], 1, "{progress}");
}
