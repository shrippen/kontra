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
