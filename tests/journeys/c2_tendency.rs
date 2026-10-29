//! J-C2 Timing-Tendenz (P3 Mira) and the summary values: a take 18 ms early with one wrong and
//! one missed note; the evaluation says so in words and numbers and the run is saved. Mira aimed
//! for laid-back (LH-BEW-08): the summary says she missed it, and by how much.

use crate::common::{piece, render, run_until, write_midi, Profile};
use kontra_appcore::UiEvent;
use std::time::Duration;

const LINE: [u8; 8] = [28, 31, 33, 35, 36, 35, 33, 31];

#[test]
fn summary_shows_tendency_and_counts() {
    let profile = Profile::new("c2");
    let score = profile.dir.join("linie.mid");
    write_midi(&score, "Linie", &LINE);
    let take = profile.dir.join("take.wav");
    let mistakes = kontra_session::Mistakes {
        offset_ms: -18.0,
        wrong_every: 4,
        miss_every: 7,
        ..Default::default()
    };
    render(&take, &piece("Linie", &LINE), mistakes);
    let mut core = profile.core(Some(&take));
    core.update_settings(|s| s.pocket = "laidback".into());
    core.open(&score).unwrap();
    core.set_mode("song");
    core.play();
    let events = run_until(&mut core, Duration::from_secs(12), |e| {
        matches!(e, UiEvent::Finished { .. })
    });
    let summary = events
        .iter()
        .find_map(|e| match e {
            UiEvent::Finished { summary_json } => Some(summary_json.clone()),
            _ => None,
        })
        .expect("finished");
    let s: serde_json::Value = serde_json::from_str(&summary).unwrap();

    // 8 notes: every 4th wrong (2), every 7th missed (1) → 5 hits, 62.5 % (rounded to even).
    assert_eq!(s["countsText"], "5 von 8 · 2 falsch · 1 ausgelassen", "{s}");
    assert_eq!(s["hitRateText"], "62 % getroffen", "{s}");
    let tendency = s["tendencyText"].as_str().unwrap();
    assert!(
        [
            "meist 17 ms zu früh",
            "meist 18 ms zu früh",
            "meist 19 ms zu früh"
        ]
        .contains(&tendency),
        "{tendency}"
    );
    // 18 ms early against 10 to 30 ms behind: 28 ms before the target range.
    let pocket = s["pocketText"].as_str().unwrap();
    assert!(
        [27, 28, 29]
            .map(|ms| format!(
                "Ziel laid-back (10 bis 30 ms dahinter): verfehlt, {ms} ms vor dem Zielbereich"
            ))
            .contains(&pocket.to_owned()),
        "{pocket}"
    );
    let counts: u64 = s["counts"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|c| c.as_u64())
        .sum();
    assert_eq!(counts, 5, "histogram holds the hits: {s}");
    assert!(!s["hardestBars"].as_array().unwrap().is_empty());

    // One observation (LH-BEW-09): 3 of 8 notes not sitting outweigh the timing.
    assert_eq!(
        s["observation"]["text"],
        "3 von 8 Noten saßen noch nicht. Langsamer üben bringt jetzt am meisten.",
        "{s}"
    );

    // Saved in the progress (LH-FOR-01).
    core.stop_live();
    let p: serde_json::Value = serde_json::from_str(&core.progress_json()).unwrap();
    assert_eq!(p["pieces"][0]["title"], "linie");
    assert_eq!(p["pieces"][0]["runs"], 1);
}
