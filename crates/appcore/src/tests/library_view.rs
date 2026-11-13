//! The library page's data (LH-BIB-17): origins, linked folders, one JSON for the table.

use super::*;
use crate::folders::{walk_folder, Walked};
use crate::origin::{Origin, Places};
use serde_json::Value;
use std::time::{Duration, Instant};

fn touch(path: &Path) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, b"x").unwrap();
}

/// Waits for the walk in the background and the reading of files behind it; a hang fails the
/// test instead of the run.
fn wait_walked(core: &mut Core) {
    let until = Instant::now() + Duration::from_secs(20);
    let mut changed = false;
    loop {
        changed |= core.library_changed();
        if changed && !core.walk_busy() && !core.meta_busy() {
            return;
        }
        assert!(Instant::now() < until, "no walk ended");
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn page(core: &Core, source: &str, sort: &str, group: &str, query: &str) -> Value {
    serde_json::from_str(&core.library_json(source, sort, group, query)).unwrap()
}

/// The titles of the rows but for the bundled exercises, which every page has.
fn titles(page: &Value) -> Vec<String> {
    page["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["origin"] != "exercise")
        .map(|r| r["title"].as_str().unwrap().to_string())
        .collect()
}

/// Opens `name` in the data folder of a core: a copy of the demo piece, so it parses.
fn known_piece(core: &mut Core, path: &Path) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::copy(demo(), path).unwrap();
    core.open(path).unwrap();
}

/// A core whose library holds pieces with set values, saved and loaded again.
fn core_with(dir: &Path, entries: Vec<RecentEntry>) -> Core {
    let mut core = Core::new(dir, QaOptions::default());
    core.library.recent = entries;
    core
}

fn entry(title: &str, artist: &str) -> RecentEntry {
    RecentEntry {
        path: PathBuf::from(format!("/nowhere/{title}.mid")),
        title: title.into(),
        artist: artist.into(),
        ..Default::default()
    }
}

#[test]
fn origin_by_place() {
    let places = Places {
        bundled: vec!["/share/exercises".into(), "/share/demo/etude-1.mid".into()],
        exercises: "/data/exercises".into(),
        pieces: "/data/pieces".into(),
        downloads: Some("/home/me/Downloads".into()),
        folders: vec!["/home/me/tabs".into(), "/home/me/tabs/bass".into()],
    };
    let table = [
        (
            "/share/exercises/1 Start/01 Scale.mid",
            Origin::Exercise,
            None,
        ),
        ("/share/demo/etude-1.mid", Origin::Exercise, None),
        ("/data/exercises/Walking.mid", Origin::Exercise, None),
        ("/data/pieces/Pop in G.mid", Origin::Generated, None),
        ("/home/me/Downloads/riff.gp5", Origin::Download, None),
        (
            "/home/me/tabs/a/b/riff.gp5",
            Origin::Folder,
            Some("/home/me/tabs"),
        ),
        // The inner link names the folder.
        (
            "/home/me/tabs/bass/riff.gp5",
            Origin::Folder,
            Some("/home/me/tabs/bass"),
        ),
        // Siblings by name only are not inside.
        ("/home/me/tabs-old/riff.gp5", Origin::Imported, None),
        ("/home/me/Downloads2/riff.gp5", Origin::Imported, None),
        ("/media/stick/riff.gp5", Origin::Imported, None),
    ];
    for (path, origin, root) in table {
        let found = places.classify(Path::new(path));
        assert_eq!(found.origin, origin, "{path}");
        assert_eq!(found.root, root.map(Path::new), "{path}");
    }
    // A linked folder wins over the download folder when it is the same place.
    let both = Places {
        downloads: Some("/home/me/tabs".into()),
        ..places
    };
    assert_eq!(
        both.classify(Path::new("/home/me/tabs/x.mid")).origin,
        Origin::Folder
    );
}

#[test]
fn origin_keys_round_trip() {
    for origin in Origin::ALL {
        assert_eq!(Origin::from_key(origin.key()), Some(origin));
    }
    assert_eq!(Origin::from_key("nope"), None);
}

/// Exercises and pieces in a form go to different folders of the data folder, so the library
/// tells them apart by place.
#[test]
fn generated_exercises_and_pieces_have_their_origins() {
    let dir = temp_dir("origins");
    let mut core = Core::new(&dir, QaOptions::default());
    let scale = Core::exercise_spec("scale", "major", "", 0, 90.0, "", PieceVariety::default());
    let song = Core::exercise_spec("song", "pop", "", 7, 90.0, "", PieceVariety::default());
    let scale = core.generate_exercise(&scale.unwrap()).unwrap();
    let song = core.generate_exercise(&song.unwrap()).unwrap();
    core.open(&scale).unwrap();
    core.open(&song).unwrap();

    let places = core.places();
    assert_eq!(places.classify(&scale).origin, Origin::Exercise);
    assert_eq!(places.classify(&song).origin, Origin::Generated);
    let page = page(&core, "generated", "title", "none", "");
    assert_eq!(page["rows"].as_array().unwrap().len(), 1);
    assert_eq!(page["rows"][0]["path"], song.to_string_lossy().as_ref());
    let _ = std::fs::remove_dir_all(dir);
}

/// Moving a file to another place changes its origin; progress is keyed by the song id and
/// stays.
#[test]
fn a_moved_file_keeps_its_progress() {
    let dir = temp_dir("moved");
    let mut core = Core::new(&dir, QaOptions::default());
    core.set_download_dir(&dir.join("Downloads"));
    let before = dir.join("Downloads/etude.mid");
    known_piece(&mut core, &before);
    let a = core.arrangement().unwrap().clone();
    let run = kontra_session::Run {
        spread_ms: None,
        bars: Vec::new(),
        finished_unix: 5_000,
        practice_s: 60.0,
        speed: 1.0,
        hit_rate: 0.9,
        mode: "song".into(),
        section: None,
        hardest_bars: Vec::new(),
        hesitant_bars: Vec::new(),
    };
    core.progress_store().record(&a, run).unwrap();
    let first = page(&core, "download", "title", "none", "");
    assert_eq!(first["rows"][0]["bestPercent"], 90.0);

    let after = dir.join("tabs/etude.mid");
    std::fs::create_dir_all(after.parent().unwrap()).unwrap();
    std::fs::rename(&before, &after).unwrap();
    core.link_folder(&dir.join("tabs")).unwrap();
    core.open(&after).unwrap();
    let page = page(&core, "folder", "title", "none", "etude");
    let row = page["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["path"] == after.to_string_lossy().as_ref())
        .unwrap();
    assert_eq!(row["origin"], "folder");
    assert_eq!(row["bestPercent"], 90.0);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_linked_folder_is_found_in_its_subfolders() {
    let dir = temp_dir("walk");
    let tabs = dir.join("tabs");
    for file in [
        "a.gp5",
        "rock/b.MID",
        "rock/slap/c.mscz",
        "d.musicxml",
        // Not scores, hidden, or plain XML.
        "notes.txt",
        "cover.png",
        "data.xml",
        ".hidden/e.gp5",
        "rock/.draft.gp5",
    ] {
        touch(&tabs.join(file));
    }
    let walked = walk_folder(&tabs);
    let names: Vec<String> = walked
        .files
        .iter()
        .map(|f| {
            // "/" between the levels on every system (Windows writes "\\").
            let parts: Vec<String> = f
                .strip_prefix(&tabs)
                .unwrap()
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect();
            parts.join("/")
        })
        .collect();
    assert_eq!(
        names,
        ["a.gp5", "d.musicxml", "rock/b.MID", "rock/slap/c.mscz"]
    );
    assert!(!walked.missing && !walked.cut_off);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_walk_stops_at_its_limits() {
    let dir = temp_dir("limits");
    let deep = (0..=crate::folders::DEPTH_MAX + 1)
        .map(|i| format!("l{i}"))
        .collect::<Vec<_>>()
        .join("/");
    touch(&dir.join("top.mid"));
    touch(&dir.join(&deep).join("deep.mid"));
    let walked = walk_folder(&dir);
    let names: Vec<_> = walked.files.iter().filter_map(|f| f.file_name()).collect();
    assert!(names.contains(&std::ffi::OsStr::new("top.mid")));
    assert!(
        !names.contains(&std::ffi::OsStr::new("deep.mid")),
        "too deep"
    );

    let many = dir.join("many");
    std::fs::create_dir_all(&many).unwrap();
    for i in 0..crate::folders::FILES_MAX + 3 {
        std::fs::write(many.join(format!("{i:05}.mid")), b"").unwrap();
    }
    let walked = walk_folder(&many);
    assert_eq!(walked.files.len(), crate::folders::FILES_MAX);
    assert!(walked.cut_off);
    let _ = std::fs::remove_dir_all(dir);
}

#[cfg(unix)]
#[test]
fn a_link_to_a_folder_does_not_loop() {
    let dir = temp_dir("loop");
    touch(&dir.join("tabs/a.mid"));
    std::os::unix::fs::symlink(&dir, dir.join("tabs/up")).unwrap();
    let walked = walk_folder(&dir.join("tabs"));
    assert_eq!(walked.files.len(), 1);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_missing_folder_is_marked_not_failed() {
    let dir = temp_dir("missing");
    let tabs = dir.join("tabs");
    touch(&tabs.join("a.mid"));
    let mut core = Core::new(&dir, QaOptions::default());
    core.link_folder(&tabs).unwrap();
    wait_walked(&mut core);
    assert_eq!(titles(&page(&core, "all", "title", "none", "")).len(), 1);

    std::fs::remove_dir_all(&tabs).unwrap();
    core.library_rescan();
    wait_walked(&mut core);
    let page = page(&core, "all", "title", "none", "");
    assert_eq!(page["counts"]["folders"][0]["missing"], true);
    assert!(titles(&page).is_empty());
    // Linking what is no folder is refused with a reason.
    assert!(core.link_folder(&dir.join("nope")).is_err());
    assert_eq!(core.library.linked_folders, [tabs]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_rescan_sees_added_and_removed_files() {
    let dir = temp_dir("rescan");
    let tabs = dir.join("tabs");
    touch(&tabs.join("a.mid"));
    touch(&tabs.join("sub/b.mid"));
    let mut core = Core::new(&dir, QaOptions::default());
    core.link_folder(&tabs).unwrap();
    wait_walked(&mut core);
    assert_eq!(titles(&page(&core, "all", "title", "none", "")), ["a", "b"]);

    std::fs::remove_file(tabs.join("a.mid")).unwrap();
    touch(&tabs.join("sub/c.mid"));
    core.library_rescan();
    wait_walked(&mut core);
    assert_eq!(titles(&page(&core, "all", "title", "none", "")), ["b", "c"]);

    core.unlink_folder(&tabs);
    assert!(titles(&page(&core, "all", "title", "none", "")).is_empty());
    assert!(
        core.library_changed(),
        "the page is told an unlink changed it"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn links_are_kept_between_runs() {
    let dir = temp_dir("kept");
    let tabs = dir.join("tabs");
    touch(&tabs.join("a.mid"));
    let mut core = Core::new(&dir, QaOptions::default());
    core.link_folder(&tabs).unwrap();
    core.link_folder(&tabs).unwrap();
    drop(core);
    let core = Core::new(&dir, QaOptions::default());
    assert_eq!(core.library.linked_folders, [tabs]);
    let _ = std::fs::remove_dir_all(dir);
}

fn slow_walk(_: &Path) -> Walked {
    std::thread::sleep(Duration::from_millis(600));
    Walked::default()
}

/// The page opens while a slow share is read: asking for the walk returns at once.
#[test]
fn a_slow_walk_does_not_block_the_caller() {
    let dir = temp_dir("slow");
    let tabs = dir.join("tabs");
    std::fs::create_dir_all(&tabs).unwrap();
    let mut core = Core::new(&dir, QaOptions::default());
    core.walk_with(slow_walk);
    let started = Instant::now();
    core.link_folder(&tabs).unwrap();
    core.library_rescan();
    let _ = core.library_json("all", "title", "none", "");
    core.tick();
    assert!(started.elapsed() < Duration::from_millis(300));
    assert!(!core.library_changed(), "not done yet");
    wait_walked(&mut core);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn bundled_exercises_are_listed_unopened() {
    let dir = temp_dir("bundled");
    let core = Core::new(&dir, QaOptions::default());
    let page = page(&core, "exercise", "title", "none", "");
    assert!(page["rows"].as_array().unwrap().len() > 10);
    assert_eq!(
        page["counts"]["origins"]["exercise"],
        page["rows"].as_array().unwrap().len()
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn rows_carry_what_the_table_shows() {
    let dir = temp_dir("rows");
    let mut core = Core::new(&dir, QaOptions::default());
    core.open(&demo()).unwrap();
    let page = page(&core, "recent", "title", "none", "");
    let row = &page["rows"][0];
    assert_eq!(row["title"], "Etude 1");
    assert_eq!(row["format"], "MID");
    assert_eq!(row["origin"], "exercise");
    assert!(row["bpm"].as_f64().unwrap() > 0.0, "{row}");
    assert!(row["lastPlayed"].as_u64().unwrap() > 0);
    assert_eq!(row["bestPercent"], Value::Null);
    assert_eq!(row["missing"], false);
    assert!(page["groups"].as_array().unwrap().is_empty());
    let _ = std::fs::remove_dir_all(dir);
}

fn values() -> Vec<RecentEntry> {
    let mut a = entry("Alpha", "Zed");
    a.tempo_bpm = Some(90.0);
    a.last_unix = 300;
    let mut b = entry("beta", "Amy");
    b.tempo_bpm = Some(120.0);
    b.last_unix = 100;
    let c = entry("Gamma", "");
    let mut d = entry("Delta", "Amy");
    d.tempo_bpm = Some(60.0);
    d.last_unix = 200;
    vec![a, b, c, d]
}

/// Sorting by each column in both directions; a piece without a value is last in both.
#[test]
fn every_column_sorts_both_ways_with_gaps_last() {
    let dir = temp_dir("sort");
    let core = core_with(&dir, values());
    let order = |sort: &str| titles(&page(&core, "all", sort, "none", ""));
    assert_eq!(order("title"), ["Alpha", "beta", "Delta", "Gamma"]);
    assert_eq!(order("title:desc"), ["Gamma", "Delta", "beta", "Alpha"]);
    assert_eq!(order("tempo"), ["Delta", "Alpha", "beta", "Gamma"]);
    assert_eq!(order("tempo:desc"), ["beta", "Alpha", "Delta", "Gamma"]);
    assert_eq!(order("lastPlayed"), ["beta", "Delta", "Alpha", "Gamma"]);
    assert_eq!(
        order("lastPlayed:desc"),
        ["Alpha", "Delta", "beta", "Gamma"]
    );
    // Everything is imported: ties fall back to the title.
    assert_eq!(order("origin"), ["Alpha", "beta", "Delta", "Gamma"]);
    assert_eq!(order("origin:desc"), ["Alpha", "beta", "Delta", "Gamma"]);
    // Nobody played: the result column has no values; still stable by title.
    assert_eq!(order("result"), ["Alpha", "beta", "Delta", "Gamma"]);
    assert_eq!(order("nonsense"), order("title"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_result_column_sorts_by_the_best_run() {
    let dir = temp_dir("sort-result");
    let mut core = Core::new(&dir, QaOptions::default());
    let mut rows = Vec::new();
    for (title, rate) in [("Low", Some(0.5)), ("None", None), ("High", Some(0.95))] {
        let mut e = entry(title, "");
        e.song_id = format!(
            "{:016x}",
            title.len() as u64 * 7 + u64::from(title.as_bytes()[0])
        );
        if let Some(hit_rate) = rate {
            let progress = kontra_session::SongProgress {
                song_id: e.song_id.clone(),
                title: title.into(),
                artist: String::new(),
                runs: vec![kontra_session::Run {
                    spread_ms: None,
                    bars: Vec::new(),
                    finished_unix: 10,
                    practice_s: 1.0,
                    speed: 1.0,
                    hit_rate,
                    mode: "song".into(),
                    section: None,
                    hardest_bars: Vec::new(),
                    hesitant_bars: Vec::new(),
                }],
            };
            core.progress_store().save(&progress).unwrap();
        }
        rows.push(e);
    }
    core.library.recent = rows;
    let order = |sort: &str| titles(&page(&core, "all", sort, "none", ""));
    assert_eq!(order("result"), ["Low", "High", "None"]);
    assert_eq!(order("result:desc"), ["High", "Low", "None"]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn groups_by_artist_origin_and_folder() {
    let dir = temp_dir("group");
    let tabs = dir.join("tabs");
    touch(&tabs.join("rock/riff.mid"));
    touch(&tabs.join("top.mid"));
    let mut core = core_with(&dir, values());
    core.update_settings(|s| s.language = Some("en".into()));
    core.link_folder(&tabs).unwrap();
    wait_walked(&mut core);

    let by_artist = page(&core, "all", "title", "artist", "");
    let keys: Vec<&str> = by_artist["groups"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["key"].as_str().unwrap())
        .collect();
    // By name; the nameless group (no artist: the scanned files and "Gamma") last.
    assert_eq!(keys, ["amy", "zed", ""]);
    assert_eq!(by_artist["groups"][0]["count"], 2);
    let bundled = by_artist["counts"]["origins"]["exercise"].as_u64().unwrap();
    assert_eq!(
        by_artist["groups"][2]["count"].as_u64().unwrap(),
        3 + bundled
    );
    assert_eq!(by_artist["groups"][2]["title"], "No artist");
    // Rows come group by group, in the column's order inside.
    assert_eq!(titles(&by_artist)[..2], ["beta", "Delta"]);
    assert_eq!(by_artist["rows"][0]["group"], "amy");
    assert_eq!(by_artist["rows"][0]["title"], "beta");

    let by_origin = page(&core, "all", "title", "origin", "");
    let origins: Vec<&str> = by_origin["groups"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["key"].as_str().unwrap())
        .collect();
    assert!(origins.len() >= 3, "{origins:?}");
    let position = |k| origins.iter().position(|o| *o == k).unwrap();
    assert!(
        position("exercise") < position("imported") && position("imported") < position("folder")
    );

    let by_folder = page(&core, "all", "title", "folder", "");
    let folders: Vec<&str> = by_folder["groups"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["key"].as_str().unwrap())
        .collect();
    assert_eq!(folders, ["tabs", "tabs/rock", ""]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_query_looks_in_title_artist_tag_and_folder() {
    let dir = temp_dir("query");
    let tabs = dir.join("tabs");
    touch(&tabs.join("Funk/riff.mid"));
    let mut entries = values();
    entries[1].tags = vec!["Slap".into()];
    let mut core = core_with(&dir, entries);
    core.link_folder(&tabs).unwrap();
    wait_walked(&mut core);
    let find = |q: &str| titles(&page(&core, "all", "title", "none", q));
    assert_eq!(find("alp"), ["Alpha"]);
    assert_eq!(find("AMY"), ["beta", "Delta"]);
    assert_eq!(find("slap"), ["beta"]);
    assert_eq!(find("funk"), ["riff"]);
    // Every word must match, each somewhere.
    assert_eq!(find("amy slap"), ["beta"]);
    assert!(find("amy zzz").is_empty());
    assert_eq!(find("  ").len(), 5);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn sources_pick_their_pieces_and_counts_cover_all() {
    let dir = temp_dir("counts");
    let tabs = dir.join("tabs");
    touch(&tabs.join("a.mid"));
    touch(&tabs.join("rock/b.mid"));
    touch(&tabs.join("rock/slap/c.mid"));
    let mut entries = values();
    entries.push(RecentEntry {
        path: tabs.join("rock/b.mid"),
        title: "Bee".into(),
        last_unix: 50,
        ..Default::default()
    });
    let mut core = core_with(&dir, entries);
    core.library.setlists.push(Setlist {
        name: "Gig".into(),
        pieces: vec![PathBuf::from("/nowhere/Alpha.mid"), tabs.join("rock/b.mid")],
    });
    core.link_folder(&tabs).unwrap();
    wait_walked(&mut core);

    let all = page(&core, "all", "title", "none", "");
    let counts = &all["counts"];
    // Four known, Bee among the files of the folder (three), the bundled exercises.
    let bundled = counts["origins"]["exercise"].as_u64().unwrap();
    assert_eq!(counts["all"].as_u64().unwrap(), 4 + 3 + bundled);
    assert_eq!(counts["origins"]["imported"], 4);
    assert_eq!(counts["origins"]["folder"], 3);
    assert_eq!(counts["recent"], 4);
    assert_eq!(counts["setlists"][0]["count"], 2);
    let root = &counts["folders"][0];
    assert_eq!(root["name"], "tabs");
    assert_eq!(root["count"], 3);
    assert_eq!(root["children"][0]["name"], "rock");
    assert_eq!(root["children"][0]["count"], 2);
    assert_eq!(root["children"][0]["children"][0]["name"], "slap");
    assert_eq!(root["children"][0]["children"][0]["count"], 1);

    let source = |key: String| titles(&page(&core, &key, "title", "none", ""));
    assert_eq!(source("folder".into()), ["a", "Bee", "c"]);
    let rock = format!("folder:{}", tabs.join("rock").display());
    assert_eq!(source(rock), ["Bee", "c"]);
    assert_eq!(source("setlist:Gig".into()), ["Alpha", "Bee"]);
    assert_eq!(source("setlist:none".into()), Vec::<String>::new());
    assert_eq!(source("recent".into()), ["Alpha", "Bee", "beta", "Delta"]);
    assert_eq!(
        source("imported".into()),
        ["Alpha", "beta", "Delta", "Gamma"]
    );
    // The counts do not follow the source or the query.
    let narrow = page(&core, "imported", "title", "none", "alpha");
    assert_eq!(narrow["counts"], all["counts"]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_known_piece_gone_from_disk_is_marked() {
    let dir = temp_dir("gone");
    let mut core = Core::new(&dir, QaOptions::default());
    let file = dir.join("lost.mid");
    known_piece(&mut core, &file);
    assert_eq!(
        page(&core, "imported", "title", "none", "")["rows"][0]["missing"],
        false
    );
    std::fs::remove_file(&file).unwrap();
    assert_eq!(
        page(&core, "imported", "title", "none", "")["rows"][0]["missing"],
        true
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// `recent_json` stays as the current page reads it.
#[test]
fn recent_json_is_unchanged() {
    let dir = temp_dir("recent-json");
    let mut core = Core::new(&dir, QaOptions::default());
    core.open(&demo()).unwrap();
    let recent: Value = serde_json::from_str(&core.recent_json()).unwrap();
    for key in [
        "path",
        "title",
        "artist",
        "best",
        "practiceMin",
        "lastUnix",
        "lastRunUnix",
        "tags",
        "hasBacking",
    ] {
        assert!(recent[0].get(key).is_some(), "{key}");
    }
    let _ = std::fs::remove_dir_all(dir);
}

/// The backup carries the linked folders; one from before them still loads.
#[test]
fn a_backup_carries_linked_folders() {
    let dir = temp_dir("backup-folders");
    let tabs = dir.join("tabs");
    std::fs::create_dir_all(&tabs).unwrap();
    let mut core = Core::new(&dir.join("a"), QaOptions::default());
    core.link_folder(&tabs).unwrap();
    let file = dir.join("backup.json");
    core.export_data(&file).unwrap();

    let mut other = Core::new(&dir.join("b"), QaOptions::default());
    other.import_data(&file, DataImport::Merge).unwrap();
    assert_eq!(other.library.linked_folders, [tabs]);

    // A backup written before linked folders has no such field.
    let mut text: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    text["library"]
        .as_object_mut()
        .unwrap()
        .remove("linked_folders");
    std::fs::write(&file, text.to_string()).unwrap();
    let mut old = Core::new(&dir.join("c"), QaOptions::default());
    old.import_data(&file, DataImport::Replace).unwrap();
    assert!(old.library.linked_folders.is_empty());
    let _ = std::fs::remove_dir_all(dir);
}

/// Pieces in a form lay in `exercises/` before they got `pieces/`: a start moves them, keeps
/// the library entry and the progress attached, never overwrites, and does nothing twice.
#[test]
fn old_pieces_move_to_their_folder() {
    let dir = temp_dir("move-pieces");
    let mut core = Core::new(&dir, QaOptions::default());
    core.update_settings(|s| s.language = Some("de".into()));
    let song = Core::exercise_spec("song", "pop", "", 7, 90.0, "", PieceVariety::default());
    let scale = Core::exercise_spec("scale", "major", "", 0, 90.0, "", PieceVariety::default());
    let piece = core.generate_exercise(&song.unwrap()).unwrap();
    let scale = core.generate_exercise(&scale.unwrap()).unwrap();

    // As an older Kontra left it: the piece in `exercises/`, opened, with a run.
    let old = dir.join("exercises").join(piece.file_name().unwrap());
    std::fs::rename(&piece, &old).unwrap();
    core.open(&old).unwrap();
    core.open(&scale).unwrap();
    let a = core.arrangement().unwrap().clone();
    core.open(&old).unwrap();
    let a_piece = core.arrangement().unwrap().clone();
    assert_ne!(a.title, a_piece.title);
    let run = kontra_session::Run {
        spread_ms: None,
        bars: Vec::new(),
        finished_unix: 5_000,
        practice_s: 60.0,
        speed: 1.0,
        hit_rate: 0.8,
        mode: "song".into(),
        section: None,
        hardest_bars: Vec::new(),
        hesitant_bars: Vec::new(),
    };
    core.progress_store().record(&a_piece, run).unwrap();
    core.library.setlists.push(Setlist {
        name: "Gig".into(),
        pieces: vec![old.clone()],
    });
    core.save_library();
    drop(core);

    let check = |core: &Core| {
        assert!(piece.exists() && !old.exists());
        assert!(scale.exists(), "an exercise stays");
        let page = page(core, "generated", "title", "none", "");
        assert_eq!(page["rows"].as_array().unwrap().len(), 1);
        assert_eq!(page["rows"][0]["path"], piece.to_string_lossy().as_ref());
        assert_eq!(page["rows"][0]["bestPercent"], 80.0);
        assert_eq!(
            core.library.setlists[0].pieces,
            std::slice::from_ref(&piece)
        );
    };
    let core = Core::new(&dir, QaOptions::default());
    check(&core);
    let before = core.library.clone();
    drop(core);
    let core = Core::new(&dir, QaOptions::default());
    check(&core);
    assert_eq!(core.library, before, "a second start changes nothing");
    drop(core);

    // A file already at the target is not overwritten.
    std::fs::write(
        dir.join("exercises").join(piece.file_name().unwrap()),
        b"old",
    )
    .unwrap();
    let _ = Core::new(&dir, QaOptions::default());
    assert_ne!(std::fs::read(&piece).unwrap(), b"old");
    let _ = std::fs::remove_dir_all(dir);
}

/// English pieces are named "{style} piece": the catalog's text after `{style}` tells them.
#[test]
fn old_english_pieces_move_too() {
    let dir = temp_dir("move-pieces-en");
    let mut core = Core::new(&dir, QaOptions::default());
    core.update_settings(|s| s.language = Some("en".into()));
    let song = Core::exercise_spec("song", "pop", "", 7, 90.0, "", PieceVariety::default());
    let scale = Core::exercise_spec("scale", "major", "", 0, 90.0, "", PieceVariety::default());
    let piece = core.generate_exercise(&song.unwrap()).unwrap();
    let scale = core.generate_exercise(&scale.unwrap()).unwrap();
    let old = dir.join("exercises").join(piece.file_name().unwrap());
    std::fs::rename(&piece, &old).unwrap();
    drop(core);

    let _core = Core::new(&dir, QaOptions::default());
    assert!(piece.exists() && !old.exists(), "the piece moves");
    assert!(scale.exists(), "an exercise stays");
    let _ = std::fs::remove_dir_all(dir);
}

/// `~/tabs` and `~/tabs/bass`: the outer folder's count is what its source shows.
#[test]
fn nested_links_count_what_the_source_shows() {
    let dir = temp_dir("nested-count");
    let tabs = dir.join("tabs");
    let bass = tabs.join("bass");
    touch(&tabs.join("a.mid"));
    touch(&bass.join("b.mid"));
    touch(&bass.join("deep/c.mid"));
    let mut core = Core::new(&dir, QaOptions::default());
    core.link_folder(&tabs).unwrap();
    core.link_folder(&bass).unwrap();
    wait_walked(&mut core);

    let all = page(&core, "all", "title", "none", "");
    let outer = all["counts"]["folders"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["path"] == tabs.to_string_lossy().as_ref())
        .unwrap();
    let shown = page(
        &core,
        &format!("folder:{}", tabs.display()),
        "title",
        "none",
        "",
    );
    assert_eq!(shown["rows"].as_array().unwrap().len(), 3);
    assert_eq!(outer["count"], 3);
    assert_eq!(outer["children"][0]["name"], "bass");
    assert_eq!(outer["children"][0]["count"], 2);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn artists_differing_in_case_are_one_group() {
    let dir = temp_dir("group-case");
    let core = core_with(&dir, vec![entry("one", "Queen"), entry("two", "queen")]);
    let all = page(&core, "all", "title", "artist", "");
    let groups: Vec<&Value> = all["groups"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|g| g["key"] != "")
        .collect();
    assert_eq!(groups.len(), 1, "{groups:?}");
    assert_eq!(groups[0]["count"], 2);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn folders_differing_in_case_are_one_group() {
    let dir = temp_dir("group-folder-case");
    let (upper, lower) = (dir.join("a/Tabs"), dir.join("b/tabs"));
    touch(&upper.join("a.mid"));
    touch(&lower.join("b.mid"));
    touch(&upper.join("c.mid"));
    let mut core = core_with(&dir, Vec::new());
    core.link_folder(&upper).unwrap();
    core.link_folder(&lower).unwrap();
    // The second link came during the first walk: a second walk follows, and the wait covers it.
    wait_walked(&mut core);
    let all = page(&core, "all", "title", "folder", "");
    let groups: Vec<&Value> = all["groups"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|g| g["key"] != "")
        .collect();
    assert_eq!(groups.len(), 1, "{groups:?}");
    assert_eq!(groups[0]["count"], 3);
    let _ = std::fs::remove_dir_all(dir);
}

fn gone_walk(_: &Path) -> Walked {
    Walked {
        missing: true,
        ..Walked::default()
    }
}

/// The page asks the walk, never the disk: no `is_dir` on the UI thread.
#[test]
fn missing_comes_from_the_walk() {
    let dir = temp_dir("missing-walk");
    let tabs = dir.join("tabs");
    std::fs::create_dir_all(&tabs).unwrap();
    let mut core = Core::new(&dir, QaOptions::default());
    core.walk_with(slow_walk);
    core.link_folder(&tabs).unwrap();
    std::fs::remove_dir_all(&tabs).unwrap();
    let before = page(&core, "all", "title", "none", "");
    assert_eq!(
        before["counts"]["folders"][0]["missing"], false,
        "not walked yet"
    );
    wait_walked(&mut core);

    std::fs::create_dir_all(&tabs).unwrap();
    core.walk_with(gone_walk);
    core.library_rescan();
    wait_walked(&mut core);
    let after = page(&core, "all", "title", "none", "");
    assert_eq!(after["counts"]["folders"][0]["missing"], true);
    let _ = std::fs::remove_dir_all(dir);
}

/// A MusicXML score with a title, a composer and a tempo.
fn score(path: &Path, title: &str, artist: &str, bpm: u32) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let note = "<note><pitch><step>E</step><octave>1</octave></pitch><duration>1</duration>\
        <notations><technical><string>4</string><fret>0</fret></technical></notations></note>";
    let xml = format!(
        "<?xml version=\"1.0\"?><score-partwise><work><work-title>{title}</work-title></work>\
        <identification><creator type=\"composer\">{artist}</creator></identification>\
        <part-list><score-part id=\"P1\"><part-name>Bass</part-name></score-part></part-list>\
        <part id=\"P1\"><measure number=\"1\"><attributes><divisions>1</divisions><time>\
        <beats>4</beats><beat-type>4</beat-type></time></attributes><sound tempo=\"{bpm}\"/>\
        {note}{note}{note}{note}</measure></part></score-partwise>"
    );
    std::fs::write(path, xml).unwrap();
}

/// QA 13.11. (Q5): files of a linked folder showed their file name, no artist and no tempo, so
/// grouping by artist and sorting by tempo did nothing for them.
#[test]
fn folder_files_show_title_artist_and_tempo() {
    let dir = temp_dir("meta-folder");
    let mut core = Core::new(&dir, QaOptions::default());
    score(
        &dir.join("tabs/rock/song-1.musicxml"),
        "Night Drive",
        "Jaco Trio",
        95,
    );
    core.link_folder(&dir.join("tabs")).unwrap();
    wait_walked(&mut core);

    let page = page(&core, "folder", "title", "none", "");
    let row = &page["rows"][0];
    assert_eq!(row["title"], "Night Drive", "{row}");
    assert_eq!(row["artist"], "Jaco Trio", "{row}");
    assert_eq!(row["bpm"], 95.0, "{row}");
    let _ = std::fs::remove_dir_all(dir);
}

/// QA 13.11. (Q5): bundled exercises had no tempo, though each is written at a known one.
#[test]
fn bundled_exercises_have_their_tempo() {
    let dir = temp_dir("meta-bundled");
    let core = Core::new(&dir, QaOptions::default());
    let page = page(&core, "exercise", "title", "none", "");
    let rows = page["rows"].as_array().unwrap();
    let without: Vec<_> = rows.iter().filter(|r| r["bpm"].is_null()).collect();
    assert!(
        without.is_empty(),
        "{} of {} without tempo",
        without.len(),
        rows.len()
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// QA 13.11. (Q5): pieces in the library from before the tempo column had none.
#[test]
fn old_entries_get_their_tempo() {
    let dir = temp_dir("meta-old");
    let path = dir.join("Downloads/old.musicxml");
    score(&path, "Old One", "", 128);
    let mut core = core_with(
        &dir,
        vec![RecentEntry {
            path: path.clone(),
            title: "Old One".into(),
            last_unix: 1_000,
            ..Default::default()
        }],
    );
    core.library_rescan();
    wait_walked(&mut core);

    let page = page(&core, "all", "title", "none", "Old One");
    assert_eq!(page["rows"][0]["bpm"], 128.0, "{}", page["rows"][0]);
    assert_eq!(
        core.library.recent[0].tempo_bpm,
        Some(128.0),
        "kept in the library"
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// QA 13.11. (Q6): a file moved from Downloads into a linked folder stood twice, the old
/// entry with "file missing" and the new file without title or result.
#[test]
fn a_moved_piece_is_found_again() {
    let dir = temp_dir("meta-moved");
    let mut core = Core::new(&dir, QaOptions::default());
    core.set_download_dir(&dir.join("Downloads"));
    let before = dir.join("Downloads/etude.mid");
    known_piece(&mut core, &before);
    let a = core.arrangement().unwrap().clone();
    let run = kontra_session::Run {
        spread_ms: None,
        bars: Vec::new(),
        finished_unix: 5_000,
        practice_s: 60.0,
        speed: 1.0,
        hit_rate: 0.9,
        mode: "song".into(),
        section: None,
        hardest_bars: Vec::new(),
        hesitant_bars: Vec::new(),
    };
    core.progress_store().record(&a, run).unwrap();
    core.setlist_create("Friday");
    core.setlist_add("Friday", &before);

    let after = dir.join("tabs/rock/etude.mid");
    std::fs::create_dir_all(after.parent().unwrap()).unwrap();
    std::fs::rename(&before, &after).unwrap();
    core.link_folder(&dir.join("tabs")).unwrap();
    wait_walked(&mut core);

    let page = page(&core, "all", "title", "none", "etude");
    let rows = page["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0]["path"], after.to_string_lossy().as_ref());
    assert_eq!(rows[0]["origin"], "folder");
    assert_eq!(rows[0]["missing"], false);
    assert_eq!(rows[0]["bestPercent"], 90.0);
    assert!(rows[0]["lastPlayed"].as_u64().is_some());
    assert_eq!(
        core.library.setlists[0].pieces,
        vec![after.clone()],
        "the setlist follows"
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// Review #326: two entries of the same piece (downloaded twice) whose files are gone must not
/// both follow the one file in the folder; one follows, the other stays as missing.
#[test]
fn two_entries_never_follow_one_file() {
    let dir = temp_dir("meta-twice");
    let mut core = Core::new(&dir, QaOptions::default());
    core.set_download_dir(&dir.join("Downloads"));
    let (first, second) = (
        dir.join("Downloads/etude.mid"),
        dir.join("Downloads/etude (1).mid"),
    );
    known_piece(&mut core, &first);
    known_piece(&mut core, &second);

    let after = dir.join("tabs/etude.mid");
    std::fs::create_dir_all(after.parent().unwrap()).unwrap();
    std::fs::rename(&first, &after).unwrap();
    std::fs::remove_file(&second).unwrap();
    core.link_folder(&dir.join("tabs")).unwrap();
    wait_walked(&mut core);

    let paths: Vec<&PathBuf> = core.library.recent.iter().map(|e| &e.path).collect();
    assert_eq!(
        paths.iter().filter(|p| ***p == after).count(),
        1,
        "{paths:?}"
    );
    assert_eq!(
        core.library.recent.len(),
        2,
        "the other entry stays: {paths:?}"
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// Review #326: files of an unlinked folder leave the cache of what was read.
#[test]
fn the_cache_forgets_unlinked_files() {
    let dir = temp_dir("meta-cache");
    let mut core = Core::new(&dir, QaOptions::default());
    let tabs = dir.join("tabs");
    score(&tabs.join("song.musicxml"), "Cached", "", 90);
    core.link_folder(&tabs).unwrap();
    wait_walked(&mut core);
    let cache = crate::store::cache_dir(&dir).join("library-meta.json");
    assert!(std::fs::read_to_string(&cache)
        .unwrap()
        .contains("song.musicxml"));

    core.unlink_folder(&tabs);
    core.library_rescan();
    wait_walked(&mut core);
    assert!(!std::fs::read_to_string(&cache)
        .unwrap()
        .contains("song.musicxml"));
    let _ = std::fs::remove_dir_all(dir);
}
