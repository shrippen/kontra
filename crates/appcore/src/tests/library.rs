//! Library: pieces, setlists, progress, data export, packages.

use super::*;

/// QA P2 (R1): a broken file gets a German reason in German, not "unknown file format".
#[test]
fn import_errors_are_translated() {
    let dir = temp_dir("broken");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("kaputt.gp5");
    std::fs::write(&file, b"FICHIER GUITAR PRO v5.00 but cut off").unwrap();
    let mut core = Core::new(&dir, QaOptions::default());
    core.update_settings(|s| s.language = Some("de".into()));
    let err = core.open(&file).unwrap_err();
    assert!(
        err.starts_with("Die Datei lässt sich nicht öffnen"),
        "{err}"
    );
    assert!(!err.contains("format") && !err.contains("file"), "{err}");
    let _ = std::fs::remove_dir_all(dir);
}

/// LH-ANZ-04, QA P2 (R7): each piece keeps its own view.
#[test]
fn view_is_remembered_per_piece() {
    let dir = temp_dir("view");
    let mut core = Core::new(&dir, QaOptions::default());
    core.open(&demo()).unwrap();
    core.set_view("tab");
    core.update_settings(|s| s.view = "lane".into());
    let mut again = Core::new(&dir, QaOptions::default());
    again.open(&demo()).unwrap();
    assert_eq!(again.view(), "tab");
    let _ = std::fs::remove_dir_all(dir);
}

/// QA 27.10. (P2 Jens, J9): a new piece opened at the tempo and loop of the one before.
#[test]
fn another_piece_starts_at_full_tempo() {
    let dir = temp_dir("fresh-piece");
    let mut core = Core::new(&dir, QaOptions::default());
    let blues = Core::exercise_spec("walking", "blues", "", 5, 110.0, "").unwrap();
    let first = core.generate_exercise(&blues).unwrap();
    core.open(&first).unwrap();
    core.set_speed(0.6);
    core.set_loop(1, 5);
    assert!(core.loop_seconds().0 > 0.0);

    let groove = Core::exercise_spec("song", "rock", "", 4, 100.0, "").unwrap();
    let second = core.generate_exercise(&groove).unwrap();
    core.open(&second).unwrap();
    assert_eq!(core.speed(), 1.0);
    assert_eq!(core.loop_seconds(), (-1.0, -1.0));

    // The same piece again keeps its tempo (a restart, the next round).
    core.set_speed(0.8);
    core.open(&second).unwrap();
    assert_eq!(core.speed(), 0.8);
    let _ = std::fs::remove_dir_all(dir);
}

// QA 27.10., J4: a Guitar Pro file titled "001" showed as "001"; a title without letters
// names nothing, the file name does.
#[test]
fn numeric_title_shows_the_file_name() {
    let dir = temp_dir("numeric-title");
    let file = dir.join("Riff in E.musicxml");
    let xml = r#"<?xml version="1.0"?>
<score-partwise version="3.1">
  <work><work-title>001</work-title></work>
  <part-list><score-part id="P1"><part-name>Bass</part-name></score-part></part-list>
  <part id="P1"><measure number="1">
    <attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time>
      <clef><sign>F</sign><line>4</line></clef></attributes>
    <note><pitch><step>E</step><octave>2</octave></pitch><duration>4</duration></note>
  </measure></part>
</score-partwise>"#;
    std::fs::write(&file, xml).unwrap();
    let mut core = Core::new(&dir, QaOptions::default());
    core.import_folder(&dir, false);
    assert!(core.recent_json().contains("\"title\":\"Riff in E\""));
    core.open(&file).unwrap();
    assert_eq!(core.loaded_title(), "Riff in E");
    let _ = std::fs::remove_dir_all(dir);
}

/// LH-FOR-01 / J-C1: recorded runs show up per piece and with their time for the day view.
#[test]
fn progress_lists_pieces_and_runs() {
    let dir = temp_dir("progress");
    let mut core = Core::new(&dir, QaOptions::default());
    core.open(&demo()).unwrap();
    let a = core.arrangement().unwrap().clone();
    for (unix, rate) in [(1_000, 0.7), (90_000, 0.92)] {
        let run = kontra_session::Run {
            spread_ms: None,
            finished_unix: unix,
            practice_s: 600.0,
            speed: 0.8,
            hit_rate: rate,
            mode: "song".into(),
            section: None,
            hardest_bars: Vec::new(),
            hesitant_bars: Vec::new(),
        };
        core.progress_store().record(&a, run).unwrap();
    }
    let p: serde_json::Value = serde_json::from_str(&core.progress_json()).unwrap();
    assert_eq!(p["pieces"][0]["title"], "Etude 1");
    assert_eq!(p["pieces"][0]["best"], 0.92);
    assert_eq!(p["pieces"][0]["practiceMin"], 20.0);
    assert_eq!(p["runs"].as_array().unwrap().len(), 2);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn tags_are_cleaned_listed_and_kept() {
    let dir = temp_dir("tags");
    let mut core = Core::new(&dir, QaOptions::default());
    core.open(&demo()).unwrap();
    let tags = [
        " Band ".to_string(),
        "slap".into(),
        "band".into(),
        String::new(),
    ];
    core.set_tags(&demo(), &tags);
    let recent: serde_json::Value = serde_json::from_str(&core.recent_json()).unwrap();
    assert_eq!(recent[0]["tags"], json!(["Band", "slap"]));
    assert_eq!(core.tags_json(), r#"["Band","slap"]"#);

    // Opening again keeps them.
    core.open(&demo()).unwrap();
    let again = Core::new(&dir, QaOptions::default());
    assert!(again.recent_json().contains("\"slap\""));
    let _ = std::fs::remove_dir_all(dir);
}

/// A bundled exercise in a setlist reads as in the plan: its title in the UI's language, not
/// "09 Groove Achtel …" from the file name (QA 13.10., P2).
#[test]
fn setlist_names_exercises_like_the_plan() {
    let dir = temp_dir("setlisttitle");
    let mut core = Core::new(&dir, QaOptions::default());
    core.update_settings(|s| s.language = Some("en".into()));
    let levels: serde_json::Value = serde_json::from_str(&core.bundled_json()).unwrap();
    let item = &levels[0]["items"][0];
    core.setlist_create("Probe");
    core.library
        .setlist_mut("Probe")
        .unwrap()
        .pieces
        .push(PathBuf::from(item["path"].as_str().unwrap()));
    let lists: serde_json::Value = serde_json::from_str(&core.setlists_json()).unwrap();
    assert_eq!(lists[0]["pieces"][0]["title"], item["title"]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn setlist_takes_library_pieces_unopened() {
    let dir = temp_dir("setlistadd");
    let mut core = Core::new(&dir, QaOptions::default());
    let levels: serde_json::Value = serde_json::from_str(&core.bundled_json()).unwrap();
    let path = PathBuf::from(levels[0]["items"][0]["path"].as_str().unwrap());
    core.open(&path).unwrap();
    core.setlist_create("Probe");
    core.setlist_add("Probe", Path::new("/nicht/in/der/bibliothek.gp5"));
    core.setlist_add("Probe", &path);
    let lists: serde_json::Value = serde_json::from_str(&core.setlists_json()).unwrap();
    assert_eq!(
        lists[0]["pieces"].as_array().unwrap().len(),
        1,
        "only library pieces"
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// Round 12: plan exercises open in a run-through (with the evaluation and "next exercise");
/// own pieces get back the mode last chosen for them.
#[test]
fn plan_exercises_open_in_a_run_through() {
    let dir = temp_dir("planmode");
    let mut core = Core::new(&dir, QaOptions::default());
    let levels: serde_json::Value = serde_json::from_str(&core.bundled_json()).unwrap();
    let exercise = PathBuf::from(levels[0]["items"][0]["path"].as_str().unwrap());
    core.open(&Core::demo_path()).unwrap();
    core.set_mode("workshop");
    core.open(&exercise).unwrap();
    assert_eq!(core.settings().mode, "song");
    core.set_mode("wait");
    core.open(&Core::demo_path()).unwrap();
    assert_eq!(
        core.settings().mode,
        "workshop",
        "the own piece's mode comes back"
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// Progress names a bundled exercise in the UI's language: its stored title is the German
/// progress key (QA 13.10.: German titles on the English progress page).
#[test]
fn progress_titles_follow_the_language() {
    let dir = temp_dir("progresstitle");
    let mut core = Core::new(&dir, QaOptions::default());
    core.update_settings(|s| s.language = Some("en".into()));
    let levels: serde_json::Value = serde_json::from_str(&core.bundled_json()).unwrap();
    let item = &levels[0]["items"][0];
    core.open(Path::new(item["path"].as_str().unwrap()))
        .unwrap();
    let a = core.arrangement().unwrap().clone();
    let run = Run {
        spread_ms: None,
        finished_unix: 1,
        practice_s: 60.0,
        speed: 1.0,
        hit_rate: 0.5,
        mode: "song".into(),
        section: None,
        hardest_bars: Vec::new(),
        hesitant_bars: Vec::new(),
    };
    core.progress_store().record(&a, run).unwrap();
    let progress: serde_json::Value = serde_json::from_str(&core.progress_json()).unwrap();
    assert_eq!(progress["pieces"][0]["title"], item["title"]);
    let _ = std::fs::remove_dir_all(dir);
}

/// A regenerated exercise keeps its progress: the file at the same path has new notes, so a
/// new song id; the runs of the old id move over (review #62: 19 files got a closing bar).
#[test]
fn regenerated_exercise_keeps_its_progress() {
    let dir = temp_dir("regenerated");
    let mut core = Core::new(&dir, QaOptions::default());
    let levels: serde_json::Value = serde_json::from_str(&core.bundled_json()).unwrap();
    let path = PathBuf::from(levels[0]["items"][0]["path"].as_str().unwrap());
    core.open(&path).unwrap();

    // The version before: one note less, played well, and the library knows its id.
    let mut before = core.arrangement().unwrap().clone();
    before.notes.pop();
    let old_id = ProgressStore::song_id(&before);
    let run = Run {
        spread_ms: Some(15.0),
        finished_unix: 1,
        practice_s: 60.0,
        speed: 1.0,
        hit_rate: 0.95,
        mode: "song".into(),
        section: None,
        hardest_bars: Vec::new(),
        hesitant_bars: Vec::new(),
    };
    core.progress_store().record(&before, run).unwrap();
    core.library.entry_mut(&path).unwrap().song_id = old_id.clone();
    let mastered = |core: &Core| {
        let levels: serde_json::Value = serde_json::from_str(&core.bundled_json()).unwrap();
        levels[0]["items"][0]["mastered"] == true
    };
    assert!(mastered(&core));

    // Opening the new file must not lose it.
    core.open(&path).unwrap();
    assert!(mastered(&core), "progress lost after regeneration");
    let home: serde_json::Value = serde_json::from_str(&core.home_json()).unwrap();
    assert_ne!(home["next"]["path"], path.to_string_lossy().as_ref());
    assert!(
        core.progress_store().load_id(&old_id).is_none(),
        "old id moved"
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// Runde 12: Groove (now level 4) comes before Walking Bass (now 5). Pieces played under
/// the old folders keep their progress and their place in setlists.
#[test]
fn swapped_levels_keep_progress_and_setlists() {
    let dir = temp_dir("swapped");
    let mut core = Core::new(&dir, QaOptions::default());
    core.update_settings(|s| s.language = Some("de".into()));
    let levels: serde_json::Value = serde_json::from_str(&core.bundled_json()).unwrap();
    assert_eq!(levels[3]["level"], "Groove");
    assert_eq!(levels[4]["level"], "Walking Bass");
    let groove = PathBuf::from(levels[3]["items"][0]["path"].as_str().unwrap());
    let walking = PathBuf::from(levels[4]["items"][0]["path"].as_str().unwrap());

    // Played before the swap: both under their old folders, one in a setlist.
    let mut old_paths = Vec::new();
    for (path, old_level) in [(&groove, "5 Groove"), (&walking, "4 Walking Bass")] {
        core.open(path).unwrap();
        let a = core.arrangement().unwrap().clone();
        let run = Run {
            spread_ms: None,
            finished_unix: 1,
            practice_s: 60.0,
            speed: 1.0,
            hit_rate: 0.95,
            mode: "song".into(),
            section: None,
            hardest_bars: Vec::new(),
            hesitant_bars: Vec::new(),
        };
        core.progress_store().record(&a, run).unwrap();
        let old = Core::exercises_dir()
            .join(old_level)
            .join(path.file_name().unwrap());
        core.library.entry_mut(path).unwrap().path = old.clone();
        old_paths.push(old);
    }
    core.library.setlists.push(Setlist {
        name: "Probe".into(),
        pieces: old_paths.clone(),
    });
    store::save(&dir.join("library.json"), &core.library).unwrap();

    let core = Core::new(&dir, QaOptions::default());
    let levels: serde_json::Value = serde_json::from_str(&core.bundled_json()).unwrap();
    assert_eq!(levels[3]["items"][0]["mastered"], true);
    assert_eq!(levels[4]["items"][0]["mastered"], true);
    assert_eq!(
        core.library.setlists[0].pieces,
        [groove.clone(), walking.clone()]
    );
    assert!(core.library.entry(&groove).is_some());
    let home: serde_json::Value = serde_json::from_str(&core.home_json()).unwrap();
    assert_ne!(home["next"]["path"], groove.to_string_lossy().as_ref());
    let _ = std::fs::remove_dir_all(dir);
}

/// Runde 12 renamed levels 4 and 5 but left the exercises in place: walking lines sat in
/// "4 Groove". Played from there, they keep their entry where they are now.
#[test]
fn exercises_played_in_the_wrong_level_are_found() {
    let dir = temp_dir("wronglevel");
    let mut core = Core::new(&dir, QaOptions::default());
    let levels: serde_json::Value = serde_json::from_str(&core.bundled_json()).unwrap();
    let walking = PathBuf::from(levels[4]["items"][0]["path"].as_str().unwrap());
    core.open(&walking).unwrap();
    let before = Core::exercises_dir()
        .join("4 Groove")
        .join(walking.file_name().unwrap());
    core.library.entry_mut(&walking).unwrap().path = before.clone();
    core.library.setlists.push(Setlist {
        name: "Probe".into(),
        pieces: vec![before],
    });
    store::save(&dir.join("library.json"), &core.library).unwrap();

    let core = Core::new(&dir, QaOptions::default());
    assert!(core.library.entry(&walking).is_some());
    assert_eq!(core.library.setlists[0].pieces, [walking]);
    let _ = std::fs::remove_dir_all(dir);
}

/// Played under its old folder before Runde 12 (moved to "5 Walking Bass", missing then)
/// and again from "4 Groove" since: both entries name one file now; the newer stays.
#[test]
fn two_entries_of_one_moved_exercise_become_one() {
    let dir = temp_dir("twoentries");
    let mut core = Core::new(&dir, QaOptions::default());
    let levels: serde_json::Value = serde_json::from_str(&core.bundled_json()).unwrap();
    let walking = PathBuf::from(levels[4]["items"][0]["path"].as_str().unwrap());
    core.open(&walking).unwrap();
    let mut older = core.library.entry(&walking).unwrap().clone();
    older.last_unix = 1;
    let newer = core.library.entry_mut(&walking).unwrap();
    newer.path = Core::exercises_dir()
        .join("4 Groove")
        .join(walking.file_name().unwrap());
    newer.last_unix = 2;
    core.library.recent.push(older);
    store::save(&dir.join("library.json"), &core.library).unwrap();

    let core = Core::new(&dir, QaOptions::default());
    let entries: Vec<_> = core
        .library
        .recent
        .iter()
        .filter(|e| e.path == walking)
        .collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].last_unix, 2);
    let _ = std::fs::remove_dir_all(dir);
}

/// LH-ANZ-11 (round 15): the practice page names the exercise's aim under its title; a
/// simple view is off until the setup or the settings switch it on.
#[test]
fn loaded_goal_names_the_aim() {
    let dir = temp_dir("loadedgoal");
    let mut core = Core::new(&dir, QaOptions::default());
    assert!(!core.settings().simple_view);
    core.update_settings(|s| s.language = Some("de".into()));
    let levels: serde_json::Value = serde_json::from_str(&core.bundled_json()).unwrap();
    let item = &levels[0]["items"][0];
    core.open(Path::new(item["path"].as_str().unwrap()))
        .unwrap();
    assert!(!item["goal"].as_str().unwrap().is_empty());
    assert_eq!(core.loaded_goal(), item["goal"].as_str().unwrap());

    core.open(&Core::demo_path()).unwrap();
    assert_eq!(core.loaded_goal(), "");
    let _ = std::fs::remove_dir_all(dir);
}

/// The practice page's heading names a bundled exercise in the UI's language too.
#[test]
fn loaded_title_follows_the_language() {
    let dir = temp_dir("loadedtitle");
    let mut core = Core::new(&dir, QaOptions::default());
    core.update_settings(|s| s.language = Some("en".into()));
    let levels: serde_json::Value = serde_json::from_str(&core.bundled_json()).unwrap();
    let item = &levels[0]["items"][0];
    core.open(Path::new(item["path"].as_str().unwrap()))
        .unwrap();
    assert_eq!(core.loaded_title(), item["title"].as_str().unwrap());
    core.open(&Core::demo_path()).unwrap();
    assert_eq!(core.loaded_title(), core.arrangement().unwrap().title);
    let _ = std::fs::remove_dir_all(dir);
}

/// The track choice offers what a bass can play: an exercise's drum track is not one
/// (QA 13.10.: "Bass" and the drums in the choice).
#[test]
fn track_choice_leaves_out_drums() {
    let dir = temp_dir("drumtrack");
    let mut core = Core::new(&dir, QaOptions::default());
    let levels: serde_json::Value = serde_json::from_str(&core.bundled_json()).unwrap();
    core.open(Path::new(levels[0]["items"][0]["path"].as_str().unwrap()))
        .unwrap();
    let tracks: serde_json::Value = serde_json::from_str(&core.tracks_json()).unwrap();
    assert_eq!(tracks["names"].as_array().unwrap().len(), 1, "{tracks}");
    assert_eq!(tracks["current"], 0);
    let _ = std::fs::remove_dir_all(dir);
}

/// A file without a clear bass track asks once; the choice is kept for the next opening
/// (QA 13.10.: "continue" on the start page did nothing, the library asked every time).
#[test]
fn chosen_track_is_kept() {
    let dir = temp_dir("track");
    std::fs::create_dir_all(&dir).unwrap();
    // One track, a melody around C5: nothing a bass would play, so no suggestion.
    let mut track = Vec::new();
    for pitch in [72u8, 74, 76, 77] {
        track.extend_from_slice(&[0x00, 0x90, pitch, 0x60, 0x60, 0x80, pitch, 0x00]);
    }
    track.extend_from_slice(&[0x00, 0xFF, 0x2F, 0x00]);
    let mut midi = b"MThd\0\0\0\x06\0\0\0\x01\0\x60MTrk".to_vec();
    midi.extend_from_slice(&(track.len() as u32).to_be_bytes());
    midi.extend_from_slice(&track);
    let file = dir.join("melodie.mid");
    std::fs::write(&file, midi).unwrap();

    let mut core = Core::new(&dir, QaOptions::default());
    assert!(matches!(core.open(&file), Ok(Opened::ChooseTrack(_))));
    core.choose_track(0).unwrap();

    let mut again = Core::new(&dir, QaOptions::default());
    assert!(matches!(again.open(&file), Ok(Opened::Ready)));
    let _ = std::fs::remove_dir_all(dir);
}

/// The demo the assistant opens has its plain path too (same file, same progress).
#[test]
fn demo_path_is_plain() {
    let demo = Core::demo_path();
    assert!(
        !demo
            .components()
            .any(|c| c == std::path::Component::ParentDir),
        "{demo:?}"
    );
}

/// An exercise opened by its plain path (file manager, library) counts in the plan too: the
/// plan's paths had "../.." in them, its progress showed "new" (QA 13.10.).
#[test]
fn exercise_progress_by_plain_path() {
    let dir = temp_dir("plainpath");
    let mut core = Core::new(&dir, QaOptions::default());
    let levels: serde_json::Value = serde_json::from_str(&core.bundled_json()).unwrap();
    let first = levels[0]["items"][0]["path"].as_str().unwrap().to_string();
    let plain = std::fs::canonicalize(&first).unwrap();
    core.open(&plain).unwrap();
    let a = core.arrangement().unwrap().clone();
    let run = Run {
        spread_ms: None,
        finished_unix: 1,
        practice_s: 60.0,
        speed: 1.0,
        hit_rate: 0.95,
        mode: "song".into(),
        section: None,
        hardest_bars: Vec::new(),
        hesitant_bars: Vec::new(),
    };
    core.progress_store().record(&a, run).unwrap();
    let levels: serde_json::Value = serde_json::from_str(&core.bundled_json()).unwrap();
    assert_eq!(levels[0]["items"][0]["best"], 0.95);
    let _ = std::fs::remove_dir_all(dir);
}

/// LH-FOR-01/-02: per section the best run (rate, tempo, date) and the workshop tempo of the
/// last 30 days; sections are named by their bars.
#[test]
fn progress_per_section_with_history() {
    let dir = temp_dir("sections");
    let mut core = Core::new(&dir, QaOptions::default());
    core.open(&demo()).unwrap();
    core.set_loop(4, 7);
    let (start, end) = core.loop_seconds();
    core.remember_section(start, end, 4, 7);
    let a = core.arrangement().unwrap().clone();
    let day = SECONDS_PER_DAY;
    let now = 100 * day;
    let runs = [
        (now - 40 * day, 0.95, 0.5, Some((start, end))),
        (now - 2 * day, 0.8, 0.6, Some((start, end))),
        (now - day, 0.85, 0.7, Some((start, end))),
        (now - day, 0.9, 1.0, None),
    ];
    for (unix, rate, speed, section) in runs {
        let run = Run {
            spread_ms: None,
            finished_unix: unix,
            practice_s: 60.0,
            speed,
            hit_rate: rate,
            mode: if section.is_some() {
                "workshop"
            } else {
                "song"
            }
            .into(),
            section,
            hardest_bars: Vec::new(),
            hesitant_bars: Vec::new(),
        };
        core.progress_store().record(&a, run).unwrap();
    }
    let p: serde_json::Value = serde_json::from_str(&core.progress_json_at(now)).unwrap();
    let sections = p["pieces"][0]["sections"].as_array().unwrap();
    assert_eq!(sections.len(), 2);
    assert_eq!(sections[0]["label"], core.tr("progress.section.whole", &[]));
    let bars = core.tr(
        "progress.section.bars",
        &[("from", "5".into()), ("to", "8".into())],
    );
    assert_eq!(sections[1]["label"], bars);
    assert_eq!(sections[1]["best"], 0.95);
    assert_eq!(sections[1]["unix"], now - 40 * day);
    let speeds: Vec<f64> = sections[1]["history"]
        .as_array()
        .unwrap()
        .iter()
        .map(|h| h["speed"].as_f64().unwrap())
        .collect();
    assert_eq!(speeds, [0.6, 0.7]);
    let _ = std::fs::remove_dir_all(dir);
}

/// LH-BIB-04 / J-B1: a folder becomes library pieces and a setlist; a broken file is named
/// with a translated reason; the setlist plays in order.
#[test]
fn folder_import_makes_a_setlist() {
    let dir = temp_dir("folder");
    let songs = dir.join("Freitag");
    std::fs::create_dir_all(&songs).unwrap();
    std::fs::copy(demo(), songs.join("1-etude.mid")).unwrap();
    std::fs::copy(
        demo().with_file_name("clicks.mid"),
        songs.join("2-clicks.mid"),
    )
    .unwrap();
    std::fs::write(songs.join("3-kaputt.gp5"), b"FICHIER GUITAR PRO v5.00 cut").unwrap();
    std::fs::write(songs.join("notes.txt"), b"not a score").unwrap();
    let mut core = Core::new(&dir, QaOptions::default());
    core.update_settings(|s| s.language = Some("de".into()));
    let (added, failed) = core.import_folder(&songs, true);
    assert_eq!(added, 2);
    assert_eq!(failed.len(), 1, "{failed:?}");
    assert_eq!(failed[0].0, "3-kaputt.gp5");
    assert!(failed[0].1.starts_with("Die Datei"), "{failed:?}");
    let lists: serde_json::Value = serde_json::from_str(&core.setlists_json()).unwrap();
    assert_eq!(lists[0]["name"], "Freitag");
    assert_eq!(lists[0]["pieces"].as_array().unwrap().len(), 2);

    assert_eq!(core.open_from_setlist("Freitag", 0).unwrap(), Opened::Ready);
    assert_eq!(
        core.next_title().as_deref(),
        Some(lists[0]["pieces"][1]["title"].as_str().unwrap())
    );
    core.open_next().unwrap();
    assert_eq!(core.next_title(), None);

    core.setlist_move("Freitag", 1, -1);
    let lists: serde_json::Value = serde_json::from_str(&core.setlists_json()).unwrap();
    assert!(lists[0]["pieces"][0]["path"]
        .as_str()
        .unwrap()
        .ends_with("2-clicks.mid"));
    let again = Core::new(&dir, QaOptions::default());
    assert!(again.setlists_json().contains("Freitag"));
    let _ = std::fs::remove_dir_all(dir);
}

/// LH-FOR-03: an export loaded on a second profile gives the same state (replace), or adds
/// to an existing one without doubling runs (merge); a foreign file changes nothing.
#[test]
fn data_export_and_import() {
    let dir = temp_dir("data");
    let mut first = Core::new(&dir.join("a"), QaOptions::default());
    first.update_settings(|s| s.language = Some("de".into()));
    first.reconfigure(|s| s.strings = 5);
    first.open(&demo()).unwrap();
    first.setlist_create("Freitag");
    first.setlist_add_loaded("Freitag");
    let a = first.arrangement().unwrap().clone();
    let run = kontra_session::Run {
        spread_ms: None,
        finished_unix: 7,
        practice_s: 300.0,
        speed: 0.8,
        hit_rate: 0.75,
        mode: "song".into(),
        section: None,
        hardest_bars: Vec::new(),
        hesitant_bars: Vec::new(),
    };
    first.progress_store().record(&a, run).unwrap();
    let file = dir.join("sicherung.json");
    let text = first.export_data(&file).unwrap();
    assert!(
        text.contains("1 Durchläufe") || text.contains("1 Durchl"),
        "{text}"
    );

    let mut second = Core::new(&dir.join("b"), QaOptions::default());
    let report = second.import_data(&file, DataImport::Replace).unwrap();
    assert!(report.contains("sicherung.json"), "{report}");
    assert_eq!(second.settings(), first.settings());
    assert_eq!(second.setlists_json(), first.setlists_json());
    assert_eq!(second.recent_json(), first.recent_json());
    assert_eq!(second.progress_json(), first.progress_json());

    // Merging the same file again adds nothing and keeps the own settings.
    second.update_settings(|s| s.strings = 4);
    let report = second.import_data(&file, DataImport::Merge).unwrap();
    assert!(
        report.contains("0 neue Stücke, 0 neue Setlisten, 0 neue Durchläufe"),
        "{report}"
    );
    assert_eq!(second.settings().strings, 4);
    assert_eq!(second.progress_json(), first.progress_json());

    std::fs::write(&file, r#"{"format": "something else"}"#).unwrap();
    assert!(second.import_data(&file, DataImport::Replace).is_err());
    assert_eq!(second.progress_json(), first.progress_json());
    let _ = std::fs::remove_dir_all(dir);
}

/// LH-FOR-04: the report is one HTML file with Kante's styles inside, the piece's figures,
/// a chart and every run with its hardest bars.
#[test]
fn progress_report_is_self_contained() {
    let dir = temp_dir("report");
    let mut core = Core::new(&dir, QaOptions::default());
    core.update_settings(|s| s.language = Some("de".into()));
    core.open(&demo()).unwrap();
    let a = core.arrangement().unwrap().clone();
    for (unix, rate) in [(1_791_504_000, 0.7), (1_791_590_400, 0.95)] {
        let run = kontra_session::Run {
            spread_ms: None,
            finished_unix: unix,
            practice_s: 600.0,
            speed: 0.8,
            hit_rate: rate,
            mode: "workshop".into(),
            section: None,
            hardest_bars: vec![3, 7],
            hesitant_bars: Vec::new(),
        };
        core.progress_store().record(&a, run).unwrap();
    }
    let p: serde_json::Value = serde_json::from_str(&core.progress_json()).unwrap();
    let id = p["pieces"][0]["songId"].as_str().unwrap().to_string();
    let file = dir.join("bericht.html");
    let text = core.export_report(&id, &file).unwrap();
    assert!(text.contains("bericht.html"), "{text}");
    let html = std::fs::read_to_string(&file).unwrap();
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("Übungsbericht: Etude 1"));
    assert!(html.contains("<svg class=\"chart\""));
    assert!(html.contains("2026-10-10") && html.contains("Werkstatt") && html.contains("3, 7"));
    assert!(html.contains("<b>20</b>"), "20 minutes");
    assert!(
        !html.contains("<link") && !html.contains("<script"),
        "no outside files"
    );
    assert!(core.export_report("0000000000000000", &file).is_err());
    let _ = std::fs::remove_dir_all(dir);
}

/// LH-BIB-07 / J-D2 (Runde 15): a piece shared as `.kontra` opens on a second profile with
/// the same notes, track, loops, target speed and aligned backing (only if embedded, copied
/// into the data folder); the teacher's progress stays home. Opening it twice adds nothing.
#[test]
fn package_moves_a_piece_to_another_profile() {
    let dir = temp_dir("package");
    std::fs::create_dir_all(&dir).unwrap();
    let backing = dir.join("song.wav");
    kontra_session::write_mono_wav(&backing, &vec![0.0; 4_800], 48_000).unwrap();
    let mut teacher = Core::new(&dir.join("thomas"), QaOptions::default());
    teacher.open(&demo()).unwrap();
    teacher.set_backing(Some(backing.clone()), 0.5);
    // An alignment as the beat sheet makes it: beats every half second from 0.25 s.
    let beats = (0..200).map(|i| 0.25 + f64::from(i) * 0.5).collect();
    teacher.library.beat_maps.insert(
        teacher.loaded_path().unwrap().to_path_buf(),
        store::BeatMap {
            beats,
            bpm: 120.0,
            shift_ms: 15.0,
            ..store::BeatMap::default()
        },
    );
    teacher.apply_beat_map().unwrap();
    teacher.set_loop(0, 1);
    teacher.save_loop();
    teacher.set_loop(4, 7);
    teacher.save_loop();
    teacher.save_loop();
    teacher.set_target_speed(0.8);
    let a = teacher.arrangement().unwrap().clone();
    let run = kontra_session::Run {
        spread_ms: None,
        finished_unix: 9,
        practice_s: 120.0,
        speed: 0.7,
        hit_rate: 0.85,
        mode: "workshop".into(),
        section: None,
        hardest_bars: vec![2],
        hesitant_bars: Vec::new(),
    };
    teacher.progress_store().record(&a, run).unwrap();
    assert_eq!(
        teacher.saved_loops(),
        vec![(0, 1), (4, 7)],
        "no double loop"
    );
    let linked = dir.join("etude-link.kontra");
    let embedded = dir.join("etude.kontra");
    teacher
        .export_package(&demo(), &linked, PackageAudio::Link)
        .unwrap();
    teacher
        .export_package(&demo(), &embedded, PackageAudio::Embed)
        .unwrap();
    assert!(
        std::fs::metadata(&embedded).unwrap().len() > std::fs::metadata(&linked).unwrap().len()
    );
    // The student has no access to the teacher's file: only the copy in the package plays.
    std::fs::remove_file(&backing).unwrap();

    let student_dir = dir.join("schueler");
    let mut student = Core::new(&student_dir, QaOptions::default());
    assert_eq!(student.open(&embedded).unwrap(), Opened::Ready);
    assert_eq!(student.notes_json(), teacher.notes_json());
    assert_eq!(student.bars_json(), teacher.bars_json(), "aligned bars");
    assert_eq!(student.backing_json(), teacher.backing_json());
    assert_eq!(student.backing_name(), "song.wav");
    let (audio, offset_s) = student.backing().unwrap();
    assert!(
        audio.starts_with(&student_dir) && audio.exists(),
        "{audio:?}"
    );
    assert_eq!(offset_s, teacher.backing().unwrap().1);
    assert_eq!(student.saved_loops(), vec![(0, 1), (4, 7)]);
    assert_eq!(student.target_speed(), 0.8);
    assert!(student.take_notice().is_some());
    let p: serde_json::Value = serde_json::from_str(&student.progress_json()).unwrap();
    assert_eq!(
        p["pieces"].as_array().unwrap().len(),
        0,
        "no teacher progress"
    );

    student.open(&embedded).unwrap();
    assert_eq!(
        student.saved_loops(),
        vec![(0, 1), (4, 7)],
        "no doubled loops"
    );

    std::fs::write(&linked, b"PK\x03\x04 broken").unwrap();
    assert!(student.open(&linked).is_err());
    let _ = std::fs::remove_dir_all(dir);
}

/// Review #164: a package without a backing track keeps its loops and target speed after a
/// restart; opening it again keeps the student's own target speed when it brings none.
#[test]
fn package_without_backing_is_kept() {
    let dir = temp_dir("package-plain");
    let mut teacher = Core::new(&dir.join("thomas"), QaOptions::default());
    teacher.open(&demo()).unwrap();
    teacher.set_loop(2, 3);
    teacher.save_loop();
    teacher.set_target_speed(0.8);
    let package = dir.join("plain.kontra");
    teacher
        .export_package(&demo(), &package, PackageAudio::Link)
        .unwrap();

    let student_dir = dir.join("schueler");
    let mut student = Core::new(&student_dir, QaOptions::default());
    student.open(&package).unwrap();
    assert!(student.backing().is_none());
    let score = student.loaded_path().unwrap().to_path_buf();
    drop(student);

    let mut again = Core::new(&student_dir, QaOptions::default());
    again.open(&score).unwrap();
    assert_eq!(again.saved_loops(), vec![(2, 3)]);
    assert_eq!(again.target_speed(), 0.8);

    // A package without a target speed leaves the student's own.
    let mut other = Core::new(&dir.join("andere"), QaOptions::default());
    other.open(&demo()).unwrap();
    // The same package again, now without a target speed.
    std::fs::create_dir_all(dir.join("neu")).unwrap();
    let bare = dir.join("neu").join("plain.kontra");
    other
        .export_package(&demo(), &bare, PackageAudio::Link)
        .unwrap();
    again.open(&score).unwrap();
    again.set_target_speed(0.7);
    let own_score = again.loaded_path().unwrap().to_path_buf();
    again.open(&bare).unwrap();
    assert_eq!(again.loaded_path().unwrap(), own_score.as_path());
    assert_eq!(again.target_speed(), 0.7);
    let _ = std::fs::remove_dir_all(dir);
}

/// Runde 15: the workshop climbs to the piece's target speed; a loop can be dropped again.
#[test]
fn target_speed_and_loops_per_piece() {
    let dir = temp_dir("target");
    let mut core = Core::new(&dir, QaOptions::default());
    core.open(&demo()).unwrap();
    assert_eq!(core.target_speed(), 1.0);
    core.set_target_speed(0.72);
    assert_eq!(core.target_speed(), 0.7, "steps of 5 %");
    core.update_settings(|s| s.mode = "workshop".into());
    let Mode::Workshop(w) = core.mode() else {
        panic!("workshop");
    };
    assert_eq!(w.target_speed, 0.7);
    core.set_loop(2, 3);
    core.save_loop();
    core.set_loop(0, 0);
    core.save_loop();
    core.remove_loop(0);
    assert_eq!(core.saved_loops(), vec![(0, 0)]);
    let mut again = Core::new(&dir, QaOptions::default());
    again.open(&demo()).unwrap();
    assert_eq!(again.target_speed(), 0.7);
    assert_eq!(again.saved_loops(), vec![(0, 0)]);
    let _ = std::fs::remove_dir_all(dir);
}

/// QA P2 (R9): another track of the file can be chosen after Kontra's own choice.
#[test]
fn switches_to_another_track() {
    let dir = temp_dir("tracks");
    let mut core = Core::new(&dir, QaOptions::default());
    core.open(&demo()).unwrap();
    let t: serde_json::Value = serde_json::from_str(&core.tracks_json()).unwrap();
    assert!(!t["names"].as_array().unwrap().is_empty());
    assert!(core.switch_track(0).is_ok());
    assert!(core.switch_track(99).is_err());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn open_lists_notes_and_remembers_recent() {
    let dir = temp_dir("open");
    let mut core = Core::new(&dir, QaOptions::default());
    assert_eq!(core.open(&demo()).unwrap(), Opened::Ready);
    let notes: serde_json::Value = serde_json::from_str(&core.notes_json()).unwrap();
    assert_eq!(notes.as_array().unwrap().len(), 64);
    let bars: serde_json::Value = serde_json::from_str(&core.bars_json()).unwrap();
    assert_eq!(bars[1]["number"], 2);
    let again = Core::new(&dir, QaOptions::default());
    assert!(again.recent_json().contains("Etude 1"));
    std::fs::remove_dir_all(dir).unwrap();
}
