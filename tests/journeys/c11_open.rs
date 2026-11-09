//! J-C11 Eigene Linie (P1 Lea, P3 Mira; LH-BEW-13): an open exercise judges no written note;
//! each attack counts by how it fits the chord. Lea plays chord tones only: all fit, all are
//! chord tones, and the run is saved like any other.

use crate::common::{piece, render, run_until, write_marked, Profile};
use kontra_appcore::UiEvent;
use std::time::Duration;

/// Marker of an open exercise (`exercise::OPEN_TEXT` in kontra-content).
const OPEN: &str = "kontra:open";
/// Am7 over the bar: A, C, E, G are its tones.
const CHORD: &str = "chord Am7";
const EXAMPLE: [u8; 4] = [33, 33, 33, 33];
const CHORD_TONES: [u8; 4] = [33, 36, 40, 43];

#[test]
fn a_line_of_chord_tones_fits_and_is_saved() {
    let profile = Profile::new("c11");
    let score = profile.dir.join("frei.mid");
    write_marked(&score, "Frei", &EXAMPLE, &[OPEN, CHORD]);
    let take = profile.dir.join("lea.wav");
    render(&take, &piece("Frei", &CHORD_TONES), Default::default());

    let mut core = profile.core(Some(&take));
    core.open(&score).unwrap();
    core.set_mode("song");
    core.play();
    let events = run_until(&mut core, Duration::from_secs(10), |e| {
        matches!(e, UiEvent::Finished { .. })
    });
    let summary = events
        .iter()
        .find_map(|e| match e {
            UiEvent::Finished { summary_json } => Some(summary_json.clone()),
            _ => None,
        })
        .expect("finished");
    core.stop_live();
    let s: serde_json::Value = serde_json::from_str(&summary).unwrap();
    assert_eq!(s["hitRateText"], "100 % passend", "{s}");
    assert_eq!(
        s["countsText"],
        "4 Akkordtöne · 0 in der Tonart · 0 fremd · Akkordtöne 100 %"
    );

    // Saved: the progress page lists the piece.
    let progress: serde_json::Value = serde_json::from_str(&core.progress_json()).unwrap();
    assert_eq!(
        progress["pieces"].as_array().map(Vec::len),
        Some(1),
        "{progress}"
    );
}
