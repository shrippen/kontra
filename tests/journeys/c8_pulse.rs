//! J-C8 Erst der Puls (P1 Lea, P5 Sam; DIDAKTIK I4): an open-string groove judges only the
//! attack. The take plays the A string where the score asks for E, on time: every note hits.
//! The same take against a normal score is wrong throughout; only the groove's summary names
//! the spread the frets wait for. Step 4: the generator writes a
//! groove judged by time only next to the judged one, not over it.

use crate::common::{judged, piece, render, run_until, write_marked, write_midi, Profile};
use kontra_appcore::{Core, UiEvent};
use kontra_model::Judging;
use std::time::Duration;

/// Marker of an exercise judged by time only (`exercise::TIMING_TEXT` in kontra-content).
const TIMING: &str = "kontra:timing";
const OPEN_E: [u8; 4] = [28; 4];
const OPEN_A: [u8; 4] = [33; 4];

#[test]
fn open_string_counts_the_attack() {
    let profile = Profile::new("c8");
    let take = profile.dir.join("a.wav");
    render(&take, &piece("Puls", &OPEN_A), Default::default());

    let timing = profile.dir.join("puls.mid");
    write_marked(&timing, "Puls", &OPEN_E, &[TIMING]);
    let (judged, spread) = play(&profile, &timing, &take);
    assert_eq!(judged, ["hit"; 4]);
    // The summary names the spread the frets wait for (I4).
    assert!(spread.contains("±25 ms"), "{spread}");

    let normal = profile.dir.join("normal.mid");
    write_midi(&normal, "Puls", &OPEN_E);
    let (judged, spread) = play(&profile, &normal, &take);
    assert!(!judged.contains(&"hit"));
    assert!(!spread.contains("±25 ms"), "{spread}");
}

#[test]
fn generator_keeps_both_judgings() {
    let profile = Profile::new("c8-generator");
    let mut core = profile.core(None);
    let spec = |judging| Core::exercise_spec("groove", "145", "eighths", 4, 80.0, judging).unwrap();

    let timing = core.generate_exercise(&spec("time")).unwrap();
    let normal = core.generate_exercise(&spec("pitch")).unwrap();
    assert_ne!(
        timing, normal,
        "the second file must not overwrite the first"
    );
    assert!(timing.exists() && normal.exists());

    // Each opens with its own judging (and so its own title and progress).
    core.open(&timing).unwrap();
    assert_eq!(core.arrangement().unwrap().judging, Judging::TimeOnly);
    core.open(&normal).unwrap();
    assert_eq!(core.arrangement().unwrap().judging, Judging::PitchAndTime);
}

/// Plays `score` once against `take`; returns the judged states and the summary's spread text.
fn play(
    profile: &Profile,
    score: &std::path::Path,
    take: &std::path::Path,
) -> (Vec<&'static str>, String) {
    let mut core = profile.core(Some(take));
    core.open(score).unwrap();
    core.set_mode("play");
    core.play();
    let events = run_until(&mut core, Duration::from_secs(8), |e| {
        matches!(e, UiEvent::Finished { .. })
    });
    core.stop_live();
    let spread = events
        .iter()
        .find_map(|e| match e {
            UiEvent::Finished { summary_json } => {
                let v: serde_json::Value = serde_json::from_str(summary_json).ok()?;
                Some(v["spreadText"].as_str()?.to_string())
            }
            _ => None,
        })
        .unwrap_or_default();
    (judged(&events), spread)
}
