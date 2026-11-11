//! J-B1 Lied durchspielen (P2 Jens): folder with a broken file, setlist, play through, mixer
//! and view while playing, next piece.

use crate::common::{judged, piece, render, run_until, write_midi, Profile};
use kontra_appcore::{Opened, UiEvent};
use std::time::Duration;

const RIFF: [u8; 8] = [28, 33, 38, 43, 28, 33, 38, 43];

#[test]
fn play_through_a_setlist() {
    let profile = Profile::new("b1");
    let folder = profile.dir.join("Freitag");
    std::fs::create_dir_all(&folder).unwrap();
    for (i, title) in ["Eins", "Zwei", "Drei"].iter().enumerate() {
        write_midi(&folder.join(format!("{}-{title}.mid", i + 1)), title, &RIFF);
    }
    std::fs::write(folder.join("4-kaputt.gp5"), b"FICHIER GUITAR PRO v5.00 cut").unwrap();
    let take = profile.dir.join("take.wav");
    render(&take, &piece("Eins", &RIFF), Default::default());
    let mut core = profile.core(Some(&take));

    // Step 1: one understandable message for the broken file, three pieces imported.
    let (added, failed) = core.import_folder(&folder, true);
    assert_eq!(added, 3);
    assert_eq!(failed.len(), 1, "{failed:?}");
    assert!(failed[0].1.starts_with("Die Datei"), "{failed:?}");

    // Step 2-3: the setlist "Freitag", first piece, play mode, through to the end.
    assert_eq!(core.open_from_setlist("Freitag", 0).unwrap(), Opened::Ready);
    core.set_mode("play");
    let start = core.play();
    assert!(
        !start.iter().any(|e| matches!(e, UiEvent::Error(_))),
        "{start:?}"
    );
    let mut events = run_until(&mut core, Duration::from_secs(3), |_| false);

    // Mixer and view while playing: no restart. Shaping the monitored bass leaves detection alone.
    let hint = core.remix(|s| {
        s.backing_volume = 0.3;
        s.metronome_volume = 1.0;
        s.monitor = true;
        s.monitor_low_db = 4.0;
        s.monitor_compress = 0.5;
        s.monitor_warmth = 0.5;
        s.monitor_limit = false;
    });
    assert!(hint.is_none(), "{hint:?}");
    // The mixer's effect list while playing (LH-EIN-19): add, reorder, switch, set, remove.
    for kind in ["overdrive", "delay", "widener"] {
        assert!(core.edit_effects("add", 0, kind, 0.0).is_none());
    }
    core.edit_effects("up", 1, "", 0.0);
    core.edit_effects("on", 1, "", 0.0);
    core.edit_effects("on", 1, "", 1.0);
    core.edit_effects("set", 0, "mix", 0.4);
    core.edit_effects("remove", 2, "", 0.0);
    let chain = core.settings().monitor_chain;
    assert_eq!(chain.slots().len(), 2);
    assert_eq!(chain.slots()[0].kind, kontra_model::EffectKind::Delay);
    assert!(chain.slots()[1].on);
    // Limiter back on, then "Neutral": effects off, the limiter on (LH-EIN-17, review #230).
    core.remix(|s| s.monitor_limit = true);
    assert!(core.settings().monitor_limit);
    core.remix(|s| s.monitor_limit = false);
    assert!(core.neutral_shaping().is_none());
    let s = core.settings();
    assert!(s.monitor_limit);
    assert_eq!(
        (s.monitor_low_db, s.monitor_compress, s.monitor_warmth),
        (0.0, 0.0, 0.0)
    );
    assert!(s.monitor_chain.is_empty());
    core.set_view("tab");
    assert!(core.is_playing());
    events.extend(run_until(&mut core, Duration::from_secs(8), |e| {
        matches!(e, UiEvent::Finished { .. })
    }));

    // No stop, every note judged on time.
    let states = judged(&events);
    assert_eq!(states.len(), RIFF.len(), "{states:?}");
    assert!(states.iter().all(|s| *s == "hit"), "{states:?}");
    assert!(!events.iter().any(|e| matches!(e, UiEvent::Holding { .. })));
    assert_eq!(core.view(), "tab");

    // Step 5: next piece in one call (one click in the UI).
    // Titles come from the file names (the MIDI track name names the part).
    assert_eq!(core.next_title().as_deref(), Some("2-Zwei"));
    assert_eq!(core.open_next().unwrap(), Opened::Ready);
    assert_eq!(core.arrangement().unwrap().title, "2-Zwei");
}
