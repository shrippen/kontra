//! Muting exercises (LH-BEW-09, LH-BEW-10): what the summary tells and when they sit.

use super::*;

/// Runde 22 (`mute`): in an exercise whose goal is muting, a run sits only when three quarters
/// of the judged ends were stopped in time; ends not judged do not count.
#[test]
fn a_muting_exercise_sits_on_muted_ends() {
    let dir = temp_dir("mutesits");
    let mut core = Core::new(&dir, QaOptions::default());
    core.update_settings(|s| s.language = Some("de".into()));
    let levels: serde_json::Value = serde_json::from_str(&core.bundled_json()).unwrap();
    let path_of = |title: &str| {
        let mut items = levels
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|l| l["items"].as_array().unwrap());
        let item = items.find(|i| i["title"].as_str().unwrap().contains(title));
        item.unwrap()["path"].as_str().unwrap().to_string()
    };
    let summary = |core: &Core, muting: kontra_session::MutingTally| {
        let a = core.arrangement().unwrap().clone();
        let mut s = kontra_session::Summary::from_results(&[], &a);
        s.judged = 31;
        s.hits = 31;
        s.hit_rate = 1.0;
        s.mean_offset_ms = Some(0.0);
        s.spread_ms = Some(10.0);
        s.muting = muting;
        serde_json::from_str::<serde_json::Value>(&core.summary_json(&s, &[], &[])).unwrap()
    };
    let tally = |in_time, rang_on| kontra_session::MutingTally {
        judged: in_time + rang_on,
        in_time,
        rang_on,
        ..Default::default()
    };

    core.open(Path::new(&path_of("Staccato"))).unwrap();

    // 0 of 31 muted: not solid, no next step.
    let v = summary(&core, tally(0, 31));
    assert!(v["next"].is_null(), "{v}");
    assert_eq!(v["observation"]["solid"], false, "{v}");
    assert!(
        v["observation"]["text"]
            .as_str()
            .unwrap()
            .contains("31 von 31"),
        "{v}"
    );

    // Three quarters in time: sits.
    assert!(summary(&core, tally(24, 7))["next"].is_object());
    assert!(summary(&core, tally(23, 8))["next"].is_null());

    // Ends not judged (microphone): unchanged, sits.
    assert!(summary(
        &core,
        kontra_session::MutingTally {
            untold: 31,
            ..Default::default()
        }
    )["next"]
        .is_object());

    // Another goal is unaffected by the ends.
    let first = levels[0]["items"][0]["path"].as_str().unwrap().to_string();
    core.open(Path::new(&first)).unwrap();
    assert!(summary(&core, tally(0, 31))["next"].is_object());
    let _ = std::fs::remove_dir_all(dir);
}

/// Runde 22 (`mute`): the exercise list follows the same rule as the summary: a muting
/// exercise is mastered only from three quarters of the ends stopped in time.
#[test]
fn the_list_masters_a_muting_exercise_on_muted_ends() {
    let dir = temp_dir("mutelist");
    let mut core = Core::new(&dir, QaOptions::default());
    core.update_settings(|s| s.language = Some("de".into()));
    let levels: serde_json::Value = serde_json::from_str(&core.bundled_json()).unwrap();
    let path = levels
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|l| l["items"].as_array().unwrap())
        .find(|i| i["title"].as_str().unwrap().contains("Staccato"))
        .map(|i| i["path"].as_str().unwrap().to_string())
        .unwrap();
    core.open(Path::new(&path)).unwrap();
    let a = core.arrangement().unwrap().clone();
    let run = |muted_in_time| kontra_session::Run {
        finished_unix: 1,
        practice_s: 60.0,
        speed: 1.0,
        hit_rate: 1.0,
        mode: "song".into(),
        section: None,
        hardest_bars: Vec::new(),
        hesitant_bars: Vec::new(),
        spread_ms: Some(10.0),
        muted_in_time,
        bars: Vec::new(),
    };
    let mastered = |core: &Core| {
        let levels: serde_json::Value = serde_json::from_str(&core.bundled_json()).unwrap();
        let mut items = levels
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|l| l["items"].as_array().unwrap());
        let item = items.find(|i| i["path"] == path.as_str()).unwrap();
        item["mastered"] == true
    };

    core.progress_store().record(&a, run(Some(0.0))).unwrap();
    assert!(!mastered(&core), "no end stopped: not mastered");

    // Ends not judged (microphone, old runs): not blocked.
    core.progress_store().record(&a, run(None)).unwrap();
    assert!(mastered(&core));
    let _ = std::fs::remove_dir_all(dir);
}
