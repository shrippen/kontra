//! J-C9 Bausteine (P1 Lea, P2 Jens; DIDAKTIK I6): the workshop cuts the chosen bars into blocks
//! and joins them step by step; each step that sits starts the next with a count-in.

use crate::common::{piece, render, run_until, write_midi, Profile, BPM};
use kontra_appcore::{ChainOrder, UiEvent};
use std::time::Duration;

/// Six equal bars: whichever bars loop, the take (played straight through) fits them.
const BAR: [u8; 4] = [28, 31, 33, 35];

#[test]
fn blocks_join_until_the_whole_part() {
    let profile = Profile::new("c9");
    let line: Vec<u8> = BAR.iter().copied().cycle().take(BAR.len() * 6).collect();
    let score = profile.dir.join("bausteine.mid");
    write_midi(&score, "Bausteine", &line);
    let take = profile.dir.join("take.wav");
    render(&take, &piece("Bausteine", &line), Default::default());

    let mut core = profile.core(Some(&take));
    core.open(&score).unwrap();
    core.set_mode("workshop");
    core.set_loop(0, 2);
    core.start_chain(1, ChainOrder::Forward);
    let chain = |core: &kontra_appcore::Core| -> serde_json::Value {
        serde_json::from_str(&core.chain_json()).unwrap()
    };
    assert_eq!(chain(&core)["text"], "Baustein: Takt 1 · Schritt 1 von 5");

    // 1 · 2 · 1–2 · 3 · 1–3, two clean passes each; one bar lasts 2 s at 120 BPM.
    let bar_s = 4.0 * 60.0 / BPM;
    core.play();
    let events = run_until(
        &mut core,
        Duration::from_secs(60),
        |e| matches!(e, UiEvent::ChainStep { chain_json } if chain_json.contains("\"last\":true")),
    );
    let steps = events
        .iter()
        .filter(|e| matches!(e, UiEvent::ChainStep { .. }))
        .count();
    assert_eq!(steps, 4, "{events:?}");
    assert_eq!(
        chain(&core)["text"],
        "Alles zusammen: Takte 1–3 · Schritt 5 von 5"
    );
    assert!(core.is_playing(), "the last step runs on");
    let (start, end) = core.loop_seconds();
    assert!((end - start - 3.0 * bar_s).abs() < 1e-6, "{start}–{end}");

    // Each step that sat is a run of its own; the last has only begun (nothing played yet).
    core.stop_live();
    let p: serde_json::Value = serde_json::from_str(&core.progress_json()).unwrap();
    assert_eq!(p["pieces"][0]["runs"], 4, "{p}");

    // Bars chosen by hand end the blocks.
    core.set_loop(1, 1);
    assert_eq!(core.chain_json(), "null");
}
