//! Coaching: summary texts, pulse, building blocks, probe, home offers.

use super::*;

/// LH-MOD-06: density per section, shown in the notes, suggested between runs.
#[test]
fn density_thins_notes_per_section_and_is_suggested() {
    let dir = temp_dir("density");
    let mut core = Core::new(&dir, QaOptions::default());
    core.open(&demo()).unwrap();
    let count = |core: &Core| {
        let notes: serde_json::Value = serde_json::from_str(&core.notes_json()).unwrap();
        notes.as_array().unwrap().len()
    };
    assert_eq!(count(&core), 64);

    // Eighth notes: on the beats half of them stay, for the whole piece.
    core.set_density(Density::Beats, DensityLock::Free);
    assert_eq!(count(&core), 32);

    // A section starts with the piece's density and can have its own.
    assert!(!core.set_loop(0, 1));
    core.set_density(Density::Sparse, DensityLock::Free);
    assert_eq!(count(&core), 16);
    assert!(core.clear_loop());
    assert_eq!(count(&core), 32);

    // Suggestions only in song and workshop mode, not when fixed or switched off.
    core.update_settings(|s| s.mode = "song".into());
    assert!(matches!(
        core.density_hint(0.95),
        Some(UiEvent::DensitySuggestion { level: 3, .. })
    ));
    assert!(core.density_hint(0.75).is_none());
    core.set_density(Density::Beats, DensityLock::Fixed);
    assert!(core.density_hint(0.95).is_none());
    core.set_density(Density::Beats, DensityLock::Free);
    core.update_settings(|s| s.adaptive = false);
    assert!(core.density_hint(0.95).is_none());
    // Not after a probe without help: it judges, it adapts nothing (QA 27.10., J8).
    core.update_settings(|s| s.adaptive = true);
    core.probe = Some(Probe { speed: 1.0 });
    assert!(core.density_hint(0.95).is_none());
    core.probe = None;

    // Kept after a restart.
    let mut again = Core::new(&dir, QaOptions::default());
    again.open(&demo()).unwrap();
    assert_eq!(count(&again), 32);
    let _ = std::fs::remove_dir_all(dir);
}

/// Start page: the last piece, the first bundled exercise that does not sit yet, and the
/// practice of the last seven days. Empty on a first start.
#[test]
fn home_summary() {
    let dir = temp_dir("home");
    let mut core = Core::new(&dir, QaOptions::default());
    let day = SECONDS_PER_DAY;
    let now = 100 * day;
    let home: serde_json::Value = serde_json::from_str(&core.home_json_at(now)).unwrap();
    assert!(home["last"].is_null());
    let first = home["next"]["path"].as_str().unwrap().to_string();
    assert_eq!(home["week"]["minutes"], 0.0);

    // The first exercise sits (above the workshop band): the next one is offered.
    core.open(Path::new(&first)).unwrap();
    let a = core.arrangement().unwrap().clone();
    let run = |unix, rate| Run {
        spread_ms: None,
        finished_unix: unix,
        practice_s: 120.0,
        speed: 1.0,
        hit_rate: rate,
        mode: "song".into(),
        section: None,
        hardest_bars: Vec::new(),
        hesitant_bars: Vec::new(),
    };
    // The first exercise is judged by time only: it sits with a small spread, too (I4).
    core.progress_store()
        .record(
            &a,
            Run {
                spread_ms: Some(12.0),
                ..run(now - 10 * day, 0.95)
            },
        )
        .unwrap();
    core.progress_store()
        .record(&a, run(now - day, 0.5))
        .unwrap();
    let home: serde_json::Value = serde_json::from_str(&core.home_json_at(now)).unwrap();
    assert_ne!(home["next"]["path"], first.as_str());
    assert_eq!(home["last"]["path"], first.as_str());
    assert_eq!(home["last"]["best"], 0.95);
    // Only the run of the last seven days counts for the week.
    assert_eq!(home["week"]["minutes"], 2.0);
    assert_eq!(home["week"]["runs"], 1);

    // The plan marks it as sitting and counts it in its level.
    let levels: serde_json::Value = serde_json::from_str(&core.bundled_json()).unwrap();
    let first_level = &levels[0];
    assert_eq!(first_level["mastered"], 1);
    assert_eq!(first_level["items"][0]["mastered"], true);
    assert_eq!(first_level["items"][1]["mastered"], false);
    let _ = std::fs::remove_dir_all(dir);
}

/// I4: the threshold judges the spread as the summary shows it (whole ms): ±25.4 reads
/// "±25 ms" and sits.
#[test]
fn the_pulse_threshold_rounds_like_the_text() {
    assert!(Core::run_sits(Judging::TimeOnly, 0.95, Some(25.4)));
    assert!(!Core::run_sits(Judging::TimeOnly, 0.95, Some(25.5)));
}

/// I4: a groove judged by time only sits when the hits are there and the spread is small;
/// before that, the frets of the next exercise wait.
#[test]
fn the_pulse_sits_on_a_small_spread() {
    let dir = temp_dir("pulse");
    let mut core = Core::new(&dir, QaOptions::default());
    core.update_settings(|s| s.language = Some("de".into()));
    let levels: serde_json::Value = serde_json::from_str(&core.bundled_json()).unwrap();
    let first = levels[0]["items"][0]["path"].as_str().unwrap().to_string();
    core.open(Path::new(&first)).unwrap();
    assert_eq!(core.arrangement().unwrap().judging, Judging::TimeOnly);
    let a = core.arrangement().unwrap().clone();
    let run = |spread| Run {
        spread_ms: Some(spread),
        finished_unix: 1,
        practice_s: 60.0,
        speed: 1.0,
        hit_rate: 0.95,
        mode: "song".into(),
        section: None,
        hardest_bars: Vec::new(),
        hesitant_bars: Vec::new(),
    };
    let sits = |core: &Core| {
        let levels: serde_json::Value = serde_json::from_str(&core.bundled_json()).unwrap();
        levels[0]["items"][0]["mastered"] == true
    };

    // All hits, but ±31 ms: not yet.
    core.progress_store().record(&a, run(31.0)).unwrap();
    assert!(!sits(&core));
    let next: serde_json::Value = serde_json::from_str(&core.home_json()).unwrap();
    assert_eq!(
        next["next"]["path"],
        first.as_str(),
        "the pulse is still the next step"
    );

    // ±18 ms: the pulse sits.
    core.progress_store().record(&a, run(18.0)).unwrap();
    assert!(sits(&core));
    let _ = std::fs::remove_dir_all(dir);
}

/// I6: phrases cut where the line rests; a groove with a rest in every bar makes blocks of
/// two bars, a line without rests of four.
#[test]
fn chain_cuts_phrases_at_rests() {
    let dir = temp_dir("chainphrases");
    let mut core = Core::new(&dir, QaOptions::default());
    core.update_settings(|s| s.language = Some("de".into()));
    let open = |core: &mut Core, rhythm| {
        let spec = Core::exercise_spec("groove", "145", rhythm, 4, 80.0, "").unwrap();
        let path = core.generate_exercise(&spec).unwrap();
        core.open(&path).unwrap();
        core.set_mode("workshop");
        core.set_loop(0, 7);
    };

    open(&mut core, "rests");
    core.start_chain(Blocks::Phrases, ChainOrder::Forward);
    assert_eq!(core.looping, Some((0, 1)));
    let chain: serde_json::Value = serde_json::from_str(&core.chain_json()).unwrap();
    assert_eq!(chain["steps"], 7, "1 · 2 · 1–2 · 3 · 1–3 · 4 · 1–4");

    open(&mut core, "eighths");
    core.start_chain(Blocks::Phrases, ChainOrder::Forward);
    assert_eq!(core.looping, Some((0, 3)), "no rests: four bars");

    // Bars 1–4 without rests are one phrase: the blocks take the whole piece (review #115).
    core.stop_chain();
    core.set_loop(0, 3);
    core.start_chain(Blocks::Phrases, ChainOrder::Forward);
    let chain: serde_json::Value = serde_json::from_str(&core.chain_json()).unwrap();
    assert!(chain["steps"].as_u64().unwrap() > 1, "{chain}");
    let _ = std::fs::remove_dir_all(dir);
}

/// I6: a phrase starts at a bar the player hesitated at in two of the last runs; once is
/// a slip.
#[test]
fn chain_cuts_phrases_where_the_player_hesitates() {
    let dir = temp_dir("chainhesitant");
    let mut core = Core::new(&dir, QaOptions::default());
    let spec = Core::exercise_spec("groove", "145", "eighths", 4, 80.0, "").unwrap();
    let path = core.generate_exercise(&spec).unwrap();
    core.open(&path).unwrap();
    core.set_mode("workshop");
    core.set_loop(0, 7);
    let full = core.loaded.as_ref().unwrap().full.clone();
    let run = |hesitant: Vec<u32>| Run {
        finished_unix: 1,
        practice_s: 60.0,
        speed: 1.0,
        hit_rate: 0.8,
        mode: "workshop".into(),
        section: None,
        hardest_bars: Vec::new(),
        hesitant_bars: hesitant,
        spread_ms: None,
    };

    // Printed bar 3 is index 2.
    core.progress_store().record(&full, run(vec![3])).unwrap();
    core.start_chain(Blocks::Phrases, ChainOrder::Forward);
    assert_eq!(core.looping, Some((0, 3)), "once: four bars as before");

    core.stop_chain();
    core.progress_store()
        .record(&full, run(vec![3, 6]))
        .unwrap();
    core.start_chain(Blocks::Phrases, ChainOrder::Forward);
    assert_eq!(
        core.looping,
        Some((0, 1)),
        "twice: a phrase starts at bar 3"
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// I6: switching order or size while the blocks run cuts the bars chosen at the start, not
/// the current step (review #108).
#[test]
fn chain_restarts_over_the_chosen_bars() {
    let dir = temp_dir("chainrestart");
    let mut core = Core::new(&dir, QaOptions::default());
    core.update_settings(|s| s.language = Some("de".into()));
    core.open(&Core::demo_path()).unwrap();
    let bars = core.arrangement().unwrap().bars.len();
    assert!(bars > 6, "{bars}");
    core.set_mode("workshop");
    core.set_loop(0, 5);

    core.start_chain(Blocks::Bars(2), ChainOrder::Forward);
    assert_eq!(core.looping, Some((0, 1)));
    core.start_chain(Blocks::Bars(2), ChainOrder::Backward);
    assert_eq!(core.looping, Some((4, 5)), "the last block of bars 1–6");
    core.start_chain(Blocks::Bars(4), ChainOrder::Forward);
    assert_eq!(core.looping, Some((0, 3)));
    core.start_chain(Blocks::Bars(4), ChainOrder::Backward);
    assert_eq!(core.looping, Some((4, 5)));

    // Off: back to the chosen bars, so blocks can start over them again.
    core.stop_chain();
    assert_eq!(core.looping, Some((0, 5)));
    assert_eq!(core.chain_json(), "null");

    // Bars 1–4 make one block of 4: the whole piece, but the choice stays (review #108).
    core.set_loop(0, 3);
    core.start_chain(Blocks::Bars(4), ChainOrder::Forward);
    assert_eq!(core.looping, Some((0, 3)), "first block of the whole piece");
    core.start_chain(Blocks::Bars(2), ChainOrder::Backward);
    assert_eq!(core.looping, Some((2, 3)), "the last block of bars 1–4");
    core.start_chain(Blocks::Bars(4), ChainOrder::Forward);
    core.stop_chain();
    assert_eq!(core.looping, Some((0, 3)));

    // Leaving the workshop ends the blocks the same way.
    core.start_chain(Blocks::Bars(2), ChainOrder::Forward);
    assert_eq!(core.looping, Some((0, 1)));
    core.start_chain(Blocks::Bars(3), ChainOrder::Backward);
    assert_eq!(
        core.looping,
        Some((3, 3)),
        "bars 1–4 by 3 end in a block of one bar"
    );
    assert!(
        core.chain_json().contains("Baustein: Takt 4 ·"),
        "{}",
        core.chain_json()
    );
    core.set_mode("song");
    assert_eq!(core.looping, Some((0, 3)));
    assert_eq!(core.chain_json(), "null");
    let _ = std::fs::remove_dir_all(dir);
}

/// I5: with an aim against the beat the summary says whether the mean held it.
#[test]
fn summary_judges_the_pocket() {
    let dir = temp_dir("pocket");
    let mut core = Core::new(&dir, QaOptions::default());
    core.update_settings(|s| s.language = Some("de".into()));
    core.open(&Core::demo_path()).unwrap();
    let a = core.arrangement().unwrap().clone();
    let mut s = kontra_session::Summary::from_results(&[], &a);
    s.mean_offset_ms = Some(4.0);
    let text = |core: &Core, s: &kontra_session::Summary| {
        let v: serde_json::Value = serde_json::from_str(&core.summary_json(s, &[], &[])).unwrap();
        v["pocketText"].as_str().unwrap().to_owned()
    };
    assert_eq!(text(&core, &s), "", "no aim");

    core.update_settings(|s| s.pocket = "laidback".into());
    assert_eq!(
        text(&core, &s),
        "Ziel laid-back (10 bis 30 ms dahinter): verfehlt, 6 ms vor dem Zielbereich"
    );
    core.update_settings(|s| s.pocket = "onbeat".into());
    assert_eq!(text(&core, &s), "Ziel auf den Punkt (±5 ms): gehalten");

    // Just outside by less than half a ms: inside, as the text rounds.
    s.mean_offset_ms = Some(5.4);
    assert_eq!(text(&core, &s), "Ziel auf den Punkt (±5 ms): gehalten");
    s.mean_offset_ms = Some(5.6);
    assert_eq!(
        text(&core, &s),
        "Ziel auf den Punkt (±5 ms): verfehlt, 1 ms hinter dem Zielbereich"
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// LH-BEW-09: the summary tells one observation in words, none for a short run.
#[test]
fn summary_tells_one_observation() {
    let dir = temp_dir("observation");
    let mut core = Core::new(&dir, QaOptions::default());
    core.update_settings(|s| s.language = Some("de".into()));
    core.open(&Core::demo_path()).unwrap();
    let a = core.arrangement().unwrap().clone();
    let mut s = kontra_session::Summary::from_results(&[], &a);
    let observation = |core: &Core, s: &kontra_session::Summary| {
        let v: serde_json::Value = serde_json::from_str(&core.summary_json(s, &[], &[])).unwrap();
        v["observation"].clone()
    };
    assert!(observation(&core, &s).is_null(), "nothing played");

    s.judged = 10;
    s.hits = 5;
    s.missed = 5;
    s.hit_rate = 0.5;
    let v = observation(&core, &s);
    assert_eq!(
        v["text"],
        "5 von 10 Noten saßen noch nicht. Langsamer üben bringt jetzt am meisten."
    );
    assert_eq!(v["solid"], false);

    s.hits = 10;
    s.missed = 0;
    s.hit_rate = 1.0;
    s.late_hits = 8;
    let v = observation(&core, &s);
    assert_eq!(
        v["text"],
        "80 % der Treffer kamen nach dem Schlag. Spiel mit dem Klick, nicht hinterher."
    );

    s.late_hits = 0;
    assert_eq!(observation(&core, &s)["solid"], true);
    let _ = std::fs::remove_dir_all(dir);
}

/// LH-EIN-14: hits steadily behind the beat bring the hint, with the mean; none otherwise.
#[test]
fn summary_asks_to_recalibrate() {
    let dir = temp_dir("recalibrate");
    let mut core = Core::new(&dir, QaOptions::default());
    core.update_settings(|s| s.language = Some("de".into()));
    core.open(&Core::demo_path()).unwrap();
    let a = core.arrangement().unwrap().clone();
    let mut s = kontra_session::Summary::from_results(&[], &a);
    let hint = |core: &Core, s: &kontra_session::Summary| {
        let v: serde_json::Value = serde_json::from_str(&core.summary_json(s, &[], &[])).unwrap();
        v["recalibrateText"].clone()
    };
    assert_eq!(hint(&core, &s), "");

    s.judged = 10;
    s.hits = 10;
    s.mean_offset_ms = Some(25.0);
    s.after_beat_hits = 10;
    s.late_hits = 8;
    assert_eq!(
        hint(&core, &s),
        "Liegen die Treffer so beständig hinter dem Schlag (im Mittel 25 ms), kann auch die \
         gemessene Latenz nicht mehr stimmen."
    );

    // Review #195: 7 ms late is no lean yet ("solid"), but asks to calibrate. Not "solid"
    // next to it, and the hint stands on its own.
    s.mean_offset_ms = Some(7.0);
    s.late_hits = 0;
    let v: serde_json::Value = serde_json::from_str(&core.summary_json(&s, &[], &[])).unwrap();
    assert!(v["observation"].is_null(), "{v}");
    assert_eq!(
        v["recalibrateText"],
        "Die Treffer liegen beständig hinter dem Schlag (im Mittel 7 ms). Spielst du nicht \
         absichtlich so, stimmt vielleicht die gemessene Latenz nicht mehr."
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// R7: notes on another string than the tab's get a line beside the score; none without.
#[test]
fn summary_tells_other_strings() {
    let dir = temp_dir("strings");
    let mut core = Core::new(&dir, QaOptions::default());
    core.update_settings(|s| s.language = Some("de".into()));
    core.open(&Core::demo_path()).unwrap();
    let a = core.arrangement().unwrap().clone();
    let mut s = kontra_session::Summary::from_results(&[], &a);
    let text = |s: &kontra_session::Summary| {
        let v: serde_json::Value = serde_json::from_str(&core.summary_json(s, &[], &[])).unwrap();
        v["stringsText"].as_str().unwrap_or_default().to_string()
    };
    assert_eq!(text(&s), "");
    s.strings = kontra_session::StringTally {
        told: 12,
        other: 3,
        unsupported: 0,
    };
    assert_eq!(
        text(&s),
        "3 von 12 Noten auf einer anderen Saite als in der Tab"
    );
    s.strings = kontra_session::StringTally {
        told: 0,
        other: 0,
        unsupported: 5,
    };
    assert_eq!(
        text(&s),
        "Saiten erkennt Kontra mit diesem Instrument noch nicht (nur bundierte Bässe in E-A-D-G)"
    );

    // Review #207: some notes told, others not (a five-string's low B): both said.
    s.strings = kontra_session::StringTally {
        told: 12,
        other: 3,
        unsupported: 5,
    };
    assert_eq!(
        text(&s),
        "3 von 12 Noten auf einer anderen Saite als in der Tab\n\
         Bei 5 Noten erkennt Kontra die Saite mit diesem Instrument noch nicht (nur bundierte Bässe in E-A-D-G)"
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// Runde 17: a microphone does not judge double stops by both tones yet; the summary says so.
#[test]
fn summary_tells_untold_double_stops() {
    let dir = temp_dir("untold-double-stops");
    let mut core = Core::new(&dir, QaOptions::default());
    core.update_settings(|s| s.language = Some("de".into()));
    core.open(&Core::demo_path()).unwrap();
    let a = core.arrangement().unwrap().clone();
    let mut s = kontra_session::Summary::from_results(&[], &a);
    s.untold_double_stops = 2;
    let v: serde_json::Value = serde_json::from_str(&core.summary_json(&s, &[], &[])).unwrap();
    assert_eq!(
        v["untoldText"],
        "Doppelgriffe über Mikrofon bewertet Kontra noch nicht nach beiden Tönen: Ein Ton zählt \
         für beide."
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// Untold techniques and double stops together: two sentences, not one run-on line.
#[test]
fn untold_techniques_and_double_stops() {
    let dir = temp_dir("untold-both");
    let mut core = Core::new(&dir, QaOptions::default());
    core.update_settings(|s| s.language = Some("de".into()));
    core.open(&Core::demo_path()).unwrap();
    let a = core.arrangement().unwrap().clone();
    let mut s = kontra_session::Summary::from_results(&[], &a);
    s.untold_techniques = vec![kontra_model::Technique::Slap];
    s.untold_double_stops = 1;
    let v: serde_json::Value = serde_json::from_str(&core.summary_json(&s, &[], &[])).unwrap();
    let text = v["untoldText"].as_str().unwrap();
    assert!(text.contains("Slap\nDoppelgriffe"), "{text}");
    let _ = std::fs::remove_dir_all(dir);
}

/// Runde 17: slap and pop over a cable are hints; the summary says so.
#[test]
fn summary_tells_hint_techniques() {
    let dir = temp_dir("hint-techniques");
    let mut core = Core::new(&dir, QaOptions::default());
    core.update_settings(|s| s.language = Some("de".into()));
    core.open(&Core::demo_path()).unwrap();
    let a = core.arrangement().unwrap().clone();
    let mut s = kontra_session::Summary::from_results(&[], &a);
    s.hint_techniques = vec![Technique::Slap, Technique::Pop];
    let v: serde_json::Value = serde_json::from_str(&core.summary_json(&s, &[], &[])).unwrap();
    assert_eq!(
        v["untoldText"],
        "Nur als Hinweis gezeigt, nicht gewertet: Slap, Pop. Die Erkennung ist noch nicht bei \
         jedem Bass sicher."
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// LH-BEW-12: intonation in words as direction and spread, per string and position from two
/// parts on; none where intonation is not scored (fretted bass, no off-pitch note).
#[test]
fn summary_tells_intonation() {
    use kontra_session::{Intonation, IntonationPart};

    let dir = temp_dir("intonation");
    let mut core = Core::new(&dir, QaOptions::default());
    core.update_settings(|s| s.language = Some("de".into()));
    core.open(&Core::demo_path()).unwrap();
    let a = core.arrangement().unwrap().clone();
    let mut s = kontra_session::Summary::from_results(&[], &a);
    let flat = Intonation {
        mean_cent: -9.0,
        spread_cent: 6.0,
    };
    s.intonation = Some(flat);
    let summary = |core: &Core, s: &kontra_session::Summary| -> serde_json::Value {
        serde_json::from_str(&core.summary_json(s, &[], &[])).unwrap()
    };
    assert_eq!(summary(&core, &s)["intonationText"], "", "not scored");

    s.off_pitch = 2;
    let part = |string: u8, mean_cent: f64| IntonationPart {
        string,
        first_fret: 5,
        last_fret: 9,
        intonation: Intonation {
            mean_cent,
            spread_cent: 10.0,
        },
    };
    s.intonation_parts = vec![part(0, -12.0), part(1, 2.0)];
    let v = summary(&core, &s);
    assert_eq!(
        v["intonationText"],
        "2 unsauber · im Schnitt 9 Cent zu tief, Streuung ±6"
    );
    assert_eq!(
        v["intonationParts"],
        serde_json::json!([
            "E-Saite, Bund 5 bis 9: im Schnitt 12 Cent zu tief, Streuung ±10",
            "A-Saite, Bund 5 bis 9: im Schnitt richtig, Streuung ±10 Cent"
        ])
    );

    // One part repeats the line: none.
    s.intonation_parts.truncate(1);
    assert_eq!(summary(&core, &s)["intonationParts"], serde_json::json!([]));
    let _ = std::fs::remove_dir_all(dir);
}

/// I1: the summary of a blind probe sets it next to the last workshop run of the same bars.
#[test]
fn summary_compares_probe_with_workshop() {
    let dir = temp_dir("probesummary");
    let mut core = Core::new(&dir, QaOptions::default());
    core.update_settings(|s| s.language = Some("de".into()));
    core.open(&Core::demo_path()).unwrap();
    core.set_loop(1, 2);
    let a = core.arrangement().unwrap().clone();
    let (start_s, end_s) = core.loop_seconds();
    let mut s = kontra_session::Summary::from_results(&[], &a);
    s.hit_rate = 0.71;
    s.spread_ms = Some(19.0);
    let probe = |core: &Core, s: &kontra_session::Summary| {
        let v: serde_json::Value = serde_json::from_str(&core.summary_json(s, &[], &[])).unwrap();
        v["probe"].clone()
    };
    assert!(probe(&core, &s).is_null(), "not probing");

    core.probe = Some(Probe { speed: 1.0 });
    assert_eq!(probe(&core, &s)["with"], "", "nothing to compare yet");
    let run = Run {
        finished_unix: 1,
        practice_s: 60.0,
        speed: 0.8,
        hit_rate: 0.88,
        mode: WORKSHOP_MODE.into(),
        section: Some((start_s, end_s)),
        hardest_bars: Vec::new(),
        hesitant_bars: Vec::new(),
        spread_ms: Some(11.0),
    };
    core.progress_store().record(&a, run).unwrap();
    let found = probe(&core, &s);
    assert_eq!(found["with"], "88 % · ±11 ms");
    assert_eq!(found["without"], "71 % · ±19 ms");
    assert!(
        found["text"].as_str().unwrap().starts_with("17 "),
        "{found}"
    );
    s.hit_rate = 0.86;
    assert_eq!(probe(&core, &s)["text"], core.tr("probe.holds", &[]));
    core.probe = None;
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn probe_needs_a_section() {
    let dir = temp_dir("probeloop");
    let mut core = Core::new(&dir, QaOptions::default());
    core.open(&Core::demo_path()).unwrap();
    let events = core.start_probe();
    assert!(
        matches!(events.as_slice(), [UiEvent::Error(_)]),
        "{events:?}"
    );
    assert!(!core.is_probing());
    let _ = std::fs::remove_dir_all(dir);
}

/// PR #99 review: a probe plays no reference tones, even if the player turned them on.
#[test]
fn probe_has_no_reference_tones() {
    let dir = temp_dir("probetones");
    let mut core = Core::new(&dir, QaOptions::default());
    core.update_settings(|s| s.reference_tones = true);
    assert!(core.practice_options().reference_tones);
    core.probe = Some(Probe { speed: 1.0 });
    assert!(!core.practice_options().reference_tones);
    let _ = std::fs::remove_dir_all(dir);
}

/// PR #99 review: a probe whose input does not start must not stay set (notes hidden, the
/// next run saved as a probe).
#[test]
fn failed_start_ends_the_probe() {
    let dir = temp_dir("probefail");
    let mut core = Core::new(
        &dir,
        QaOptions {
            input_file: Some(dir.join("missing.wav")),
            laptop_jack: false,
        },
    );
    core.open(&Core::demo_path()).unwrap();
    core.set_loop(1, 2);
    // The failed input shows as its state, not as a message (LH-EIN-15).
    core.start_probe();
    assert_eq!(core.input_state(), InputState::Failed);
    assert!(!core.is_probing());
    let _ = std::fs::remove_dir_all(dir);
}

/// I1 on the start page: a section practised in the workshop before last night, not probed
/// since, comes back as "check without help"; today's practice waits, a probe ends it.
#[test]
fn home_offers_a_probe() {
    let dir = temp_dir("probehome");
    let mut core = Core::new(&dir, QaOptions::default());
    let day = SECONDS_PER_DAY;
    let now = 100 * day;
    core.open(&Core::demo_path()).unwrap();
    core.set_loop(1, 2);
    let a = core.arrangement().unwrap().clone();
    let (start_s, end_s) = core.loop_seconds();
    core.remember_section(start_s, end_s, 1, 2);
    let run = |unix, mode: &str| Run {
        finished_unix: unix,
        practice_s: 60.0,
        speed: 1.0,
        hit_rate: 0.9,
        mode: mode.into(),
        section: Some((start_s, end_s)),
        hardest_bars: Vec::new(),
        hesitant_bars: Vec::new(),
        spread_ms: None,
    };
    let offer = |core: &Core| {
        let home: serde_json::Value = serde_json::from_str(&core.home_json_at(now)).unwrap();
        home["probe"].clone()
    };
    let store = core.progress_store();
    store.record(&a, run(now - 3600, WORKSHOP_MODE)).unwrap();
    assert!(offer(&core).is_null(), "practised today");

    store.record(&a, run(now - day, WORKSHOP_MODE)).unwrap();
    let found = offer(&core);
    assert_eq!(
        (found["from"].as_u64(), found["to"].as_u64()),
        (Some(2), Some(3))
    );
    assert_eq!(found["startS"].as_f64(), Some(start_s));

    store.record(&a, run(now - day + 60, PROBE_MODE)).unwrap();
    assert!(offer(&core).is_null(), "probed since");
    let _ = std::fs::remove_dir_all(dir);
}

/// PR #99 review: a newer section the library cannot name (or whose file is gone) does not
/// hide an older one it can offer.
#[test]
fn probe_offer_skips_what_it_cannot_open() {
    let dir = temp_dir("probeskip");
    let mut core = Core::new(&dir, QaOptions::default());
    let day = SECONDS_PER_DAY;
    let now = 100 * day;
    core.open(&Core::demo_path()).unwrap();
    core.set_loop(1, 2);
    let a = core.arrangement().unwrap().clone();
    let (start_s, end_s) = core.loop_seconds();
    core.remember_section(start_s, end_s, 1, 2);
    let run = |unix, section| Run {
        finished_unix: unix,
        practice_s: 60.0,
        speed: 1.0,
        hit_rate: 0.9,
        mode: WORKSHOP_MODE.into(),
        section: Some(section),
        hardest_bars: Vec::new(),
        hesitant_bars: Vec::new(),
        spread_ms: None,
    };
    let store = core.progress_store();
    store
        .record(&a, run(now - 2 * day, (start_s, end_s)))
        .unwrap();
    // Newer, but bars the library never named.
    store
        .record(&a, run(now - day, (end_s, end_s + 2.0)))
        .unwrap();

    let home: serde_json::Value = serde_json::from_str(&core.home_json_at(now)).unwrap();
    assert_eq!(home["probe"]["startS"].as_f64(), Some(start_s), "{home}");
    let _ = std::fs::remove_dir_all(dir);
}

/// The start page's probe opens the piece looping the same bars, found by their times.
#[test]
fn loop_follows_section_times() {
    let dir = temp_dir("loopat");
    let mut core = Core::new(&dir, QaOptions::default());
    core.open(&Core::demo_path()).unwrap();
    core.set_loop(2, 3);
    let (start_s, end_s) = core.loop_seconds();
    assert_eq!(core.bars_at(start_s, end_s), Some((2, 3)));
    assert_eq!(core.bars_at(start_s + 0.3, end_s), None);
    let _ = std::fs::remove_dir_all(dir);
}

/// "Spots from yesterday" (round 11): the two bars hardest most often in the last days, from
/// the piece practised last; today's runs wait for a night, older ones are gone.
#[test]
fn home_offers_yesterdays_spots() {
    let dir = temp_dir("spots");
    let mut core = Core::new(&dir, QaOptions::default());
    let day = SECONDS_PER_DAY;
    let now = 100 * day;
    let levels: serde_json::Value = serde_json::from_str(&core.bundled_json()).unwrap();
    let path = levels[0]["items"][0]["path"].as_str().unwrap().to_string();
    core.open(Path::new(&path)).unwrap();
    let a = core.arrangement().unwrap().clone();
    let run = |unix, bars: &[u32]| Run {
        spread_ms: None,
        finished_unix: unix,
        practice_s: 60.0,
        speed: 1.0,
        hit_rate: 0.6,
        mode: "song".into(),
        section: None,
        hardest_bars: bars.to_vec(),
        hesitant_bars: Vec::new(),
    };
    let spots = |core: &Core| {
        let home: serde_json::Value = serde_json::from_str(&core.home_json_at(now)).unwrap();
        home["spots"].clone()
    };
    assert!(spots(&core).is_null(), "no runs");

    let store = core.progress_store();
    store.record(&a, run(now - 2 * day, &[3, 5])).unwrap();
    store.record(&a, run(now - day, &[5, 2])).unwrap();
    store.record(&a, run(now - 10 * day, &[7, 7])).unwrap();
    store.record(&a, run(now - 3600, &[9, 9])).unwrap();
    let found = spots(&core);
    assert_eq!(found["path"], path.as_str());
    assert_eq!(
        found["bars"],
        serde_json::json!([5, 2]),
        "most often, then latest"
    );
    assert!(!found["title"].as_str().unwrap().is_empty());
    let _ = std::fs::remove_dir_all(dir);
}

/// A bundled exercise that sits after the run offers the next one in the summary; a run below
/// the band or a piece of one's own does not (P1: one step on, QA 13.10.).
#[test]
fn summary_offers_the_next_exercise() {
    let dir = temp_dir("summarynext");
    let mut core = Core::new(&dir, QaOptions::default());
    let levels: serde_json::Value = serde_json::from_str(&core.bundled_json()).unwrap();
    let first = levels[0]["items"][0]["path"].as_str().unwrap().to_string();
    let second = levels[0]["items"][1]["path"].as_str().unwrap().to_string();
    core.open(Path::new(&first)).unwrap();
    let a = core.arrangement().unwrap().clone();
    let mut s = kontra_session::Summary::from_results(&[], &a);
    let next = |core: &Core, s: &kontra_session::Summary| {
        let v: serde_json::Value = serde_json::from_str(&core.summary_json(s, &[], &[])).unwrap();
        v["next"].clone()
    };

    // The first exercise is judged by time only (I4): the hits and a small spread.
    s.hit_rate = 0.95;
    s.spread_ms = Some(30.0);
    assert!(next(&core, &s).is_null(), "the spread is not small yet");
    s.spread_ms = Some(15.0);
    assert_eq!(next(&core, &s)["path"], second.as_str());
    s.hit_rate = 0.5;
    assert!(next(&core, &s).is_null());

    core.open(&Core::demo_path()).unwrap();
    s.hit_rate = 0.95;
    assert!(next(&core, &s).is_null());
    let _ = std::fs::remove_dir_all(dir);
}
