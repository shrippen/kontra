//! J-C6 Sitzt es ohne Noten? (P2 Jens, P1 Lea; DIDAKTIK I1): a bar practised in the workshop,
//! then played once without help; the summary sets both side by side and the probe is saved.

use crate::common::{piece, render, run_until, write_midi, Profile};
use kontra_appcore::UiEvent;
use std::time::Duration;

/// Two bars; the workshop and the probe play the first.
const LINE: [u8; 8] = [28, 31, 33, 35, 36, 35, 33, 31];

#[test]
fn probe_compares_with_the_workshop() {
    let profile = Profile::new("c6");
    let score = profile.dir.join("linie.mid");
    write_midi(&score, "Linie", &LINE);

    // Workshop: one clean pass of bar 1, then stopped (saved like the UI's stop).
    let clean = profile.dir.join("clean.wav");
    render(&clean, &piece("Linie", &LINE), Default::default());
    let mut core = profile.core(Some(&clean));
    core.open(&score).unwrap();
    core.set_mode("workshop");
    core.set_loop(0, 0);
    core.play();
    run_until(&mut core, Duration::from_secs(8), |e| {
        matches!(e, UiEvent::PassComplete { .. })
    });
    core.stop_live();

    // Without help: every second note missed.
    let shaky = profile.dir.join("shaky.wav");
    let mistakes = kontra_session::Mistakes {
        miss_every: 2,
        ..Default::default()
    };
    render(&shaky, &piece("Linie", &LINE), mistakes);
    let mut core = profile.core(Some(&shaky));
    core.open(&score).unwrap();
    core.set_mode("workshop");
    core.set_loop(0, 0);

    // Stopped after its first note: no probe, nothing saved apart from the workshop.
    core.start_probe();
    run_until(&mut core, Duration::from_secs(8), |e| {
        matches!(e, UiEvent::Judged { .. })
    });
    core.stop_live();
    let p: serde_json::Value = serde_json::from_str(&core.progress_json()).unwrap();
    assert_eq!(p["pieces"][0]["runs"], 1, "{p}");

    core.start_probe();
    assert!(core.is_probing());
    let events = run_until(&mut core, Duration::from_secs(8), |e| {
        matches!(e, UiEvent::Finished { .. })
    });
    let summary = events
        .iter()
        .find_map(|e| match e {
            UiEvent::Finished { summary_json } => Some(summary_json.clone()),
            _ => None,
        })
        .expect("the probe ends after one pass");
    // It only judges: no workshop pass and no density suggestion (QA 27.10., P2 Jens, J8).
    assert!(
        !events.iter().any(|e| matches!(
            e,
            UiEvent::PassComplete { .. } | UiEvent::DensitySuggestion { .. }
        )),
        "{events:?}"
    );
    let s: serde_json::Value = serde_json::from_str(&summary).unwrap();

    // The workshop run holds the start of its second pass too; the probe has one pass only.
    assert!(!s["probe"]["with"].as_str().unwrap().is_empty(), "{s}");
    assert!(
        s["probe"]["without"].as_str().unwrap().starts_with("50 %"),
        "{s}"
    );
    assert!(
        s["probe"]["text"]
            .as_str()
            .unwrap()
            .contains("Punkte weniger"),
        "{s}"
    );
    assert_eq!(s["countsText"], "2 von 4 · 0 falsch · 2 ausgelassen", "{s}");

    // Over: the next play is the workshop again; the probe is saved apart from it.
    assert!(!core.is_probing());
    let p: serde_json::Value = serde_json::from_str(&core.progress_json()).unwrap();
    assert_eq!(p["pieces"][0]["runs"], 2);
}
