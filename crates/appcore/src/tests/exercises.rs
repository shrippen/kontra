//! Courses, generated and bundled exercises, fretboard quiz.

use super::*;

/// J-G5: an added course appears beside the bundled one and becomes the one to learn;
/// the bundled one can be chosen again.
#[test]
fn an_added_course_is_learned_next() {
    use kontra_session::lesson::{new_course, pack, Author};
    use serde_json::{json, Value};
    let dir = temp_dir("course-add");
    let src = dir.join("src");
    let author = Author {
        provider: "Studio Weber".into(),
        license: "CC-BY-4.0".into(),
    };
    new_course(&src, "weber", &author).unwrap();
    let file = dir.join("weber.kontrakurs");
    std::fs::write(&file, pack(&src).unwrap()).unwrap();

    let mut core = Core::new(&dir.join("data"), QaOptions::default());
    let preview: Value = serde_json::from_str(&core.course_preview(&file)).unwrap();
    assert_eq!(preview["ok"], true);
    assert_eq!(preview["provider"], "Studio Weber");
    assert_eq!(preview["problems"], json!([]));
    assert_eq!(preview["replaces"], "none");

    assert_eq!(core.course_add(&file), "");
    let learn: Value = serde_json::from_str(&core.learn_json()).unwrap();
    assert_eq!(learn["course"], "weber.kurs");
    assert_eq!(learn["courses"].as_array().map(Vec::len), Some(2));
    assert_eq!(learn["next"]["id"], "weber.start");

    // The same package again, then a later revision: the preview says which (J-G5 step 4).
    let preview: Value = serde_json::from_str(&core.course_preview(&file)).unwrap();
    assert_eq!(preview["replaces"], "same");
    let manifest = src.join("manifest.json");
    let text = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(
        &manifest,
        text.replace("\"revision\": 1", "\"revision\": 2"),
    )
    .unwrap();
    std::fs::write(&file, pack(&src).unwrap()).unwrap();
    let preview: Value = serde_json::from_str(&core.course_preview(&file)).unwrap();
    assert_eq!(
        (
            &preview["replaces"],
            &preview["revision"],
            &preview["installed"]
        ),
        (&json!("newer"), &json!(2), &json!(1))
    );

    // Kept over a restart; the bundled course comes back by choice.
    let mut core = Core::new(&dir.join("data"), QaOptions::default());
    let learn: Value = serde_json::from_str(&core.learn_json()).unwrap();
    assert_eq!(learn["course"], "weber.kurs");
    assert_eq!(learn["courses"][1]["removable"], true);
    core.choose_course("bass");
    let learn: Value = serde_json::from_str(&core.learn_json()).unwrap();
    assert_eq!(learn["course"], "bass");
    assert_eq!(learn["courses"][0]["removable"], false);

    // Removed: the bundled course again; adding it back keeps what was learned.
    core.choose_course("weber.kurs");
    core.lesson_seen("weber.start", usize::MAX - 1);
    assert_eq!(core.course_remove("weber.kurs"), "");
    let learn: Value = serde_json::from_str(&core.learn_json()).unwrap();
    assert_eq!(learn["course"], "bass");
    assert_eq!(learn["courses"].as_array().map(Vec::len), Some(1));
    assert_eq!(core.course_add(&file), "");
    let learn: Value = serde_json::from_str(&core.learn_json()).unwrap();
    assert_ne!(learn["next"]["id"], "weber.start");
    std::fs::remove_dir_all(dir).unwrap();
}

/// J-G5: a course video plays from the package only when asked, its subtitles come with the
/// lesson.
#[test]
fn a_course_video_is_unpacked_on_request() {
    use kontra_session::lesson::{new_course, pack, Author};
    use serde_json::Value;
    let dir = temp_dir("course-video");
    let src = dir.join("src");
    let author = Author {
        provider: "Studio Weber".into(),
        license: "CC-BY-4.0".into(),
    };
    new_course(&src, "weber", &author).unwrap();
    std::fs::create_dir_all(src.join("media")).unwrap();
    std::fs::write(src.join("media/a.webm"), b"video").unwrap();
    for lang in ["de", "en"] {
        let vtt = format!("WEBVTT\n\n00:01.000 --> 00:02.000\n{lang}\n");
        std::fs::write(src.join(format!("media/a.{lang}.vtt")), vtt).unwrap();
        let unit = src.join(format!("units/weber.start/{lang}.md"));
        let mut text = std::fs::read_to_string(&unit).unwrap();
        text += &format!("::: video src=media/a.webm subtitles=media/a.{lang}.vtt\n");
        std::fs::write(&unit, text).unwrap();
    }
    let file = dir.join("weber.kontrakurs");
    std::fs::write(&file, pack(&src).unwrap()).unwrap();
    let data = dir.join("data");
    let mut core = Core::new(&data, QaOptions::default());
    core.update_settings(|s| s.language = Some("de".into()));
    assert_eq!(core.course_add(&file), "");

    let lesson: Value = serde_json::from_str(&core.lesson_json("weber.start")).unwrap();
    let video = &lesson["pages"][0]["blocks"][1];
    assert_eq!(video["kind"], "video");
    assert_eq!(video["cues"][0]["text"], "de");
    assert_eq!(video["cues"][0]["startMs"], 1000);

    let cache = store::cache_dir(&data);
    assert!(
        !cache.join("courses/weber/media/a.webm").exists(),
        "no video unpacked before a click"
    );
    let url = core.lesson_video("weber.start", 0, 1);
    // A Windows drive follows the URL's own slash: file:///C:/…
    let scheme = if cfg!(windows) { "file:///" } else { "file://" };
    let path = url.strip_prefix(scheme).unwrap();
    assert_eq!(std::fs::read(path).unwrap(), b"video");
    assert_eq!(
        core.lesson_video("weber.start", 0, 0),
        "",
        "a text block plays nothing"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

/// J-G5: a course from an address is fetched, previewed and added like a file; only https.
#[test]
fn a_course_from_an_address_is_added() {
    use kontra_session::lesson::{new_course, pack, Author};
    use serde_json::Value;
    let dir = temp_dir("course-fetch");
    let src = dir.join("src");
    let author = Author {
        provider: "Studio Weber".into(),
        license: "CC-BY-4.0".into(),
    };
    new_course(&src, "weber", &author).unwrap();
    let file = dir.join("weber.kontrakurs");
    std::fs::write(&file, pack(&src).unwrap()).unwrap();
    let mut core = Core::new(&dir.join("data"), QaOptions::default());
    core.fake_course_web(&file);

    let refused: Value = serde_json::from_str(&core.course_fetch("http://x.example")).unwrap();
    assert_eq!(refused["state"], "error");

    let started: Value = serde_json::from_str(&core.course_fetch("https://x.example")).unwrap();
    assert_eq!(started["state"], "running");
    let done: Value = loop {
        if let Some(json) = core.course_fetch_poll() {
            break serde_json::from_str(&json).unwrap();
        }
        std::thread::yield_now();
    };
    assert_eq!(done["state"], "done");
    let path = PathBuf::from(done["path"].as_str().unwrap());
    let preview: Value = serde_json::from_str(&core.course_preview(&path)).unwrap();
    assert_eq!(preview["provider"], "Studio Weber");
    assert_eq!(core.course_add(&path), "");
    let learn: Value = serde_json::from_str(&core.learn_json()).unwrap();
    assert_eq!(learn["course"], "weber.kurs");
    // Kept to look for an update there.
    assert_eq!(learn["courses"][1]["address"], "https://x.example");
    std::fs::remove_dir_all(dir).unwrap();
}

/// LH-LRN-14: a course explaining a bundled term adds its text under its name; the index
/// lists the term once.
#[test]
fn a_term_from_two_courses_shows_both_texts() {
    use kontra_session::lesson::{new_course, pack, Author};
    use serde_json::Value;
    let dir = temp_dir("course-term");
    let src = dir.join("src");
    let author = Author {
        provider: "Studio Weber".into(),
        license: "CC-BY-4.0".into(),
    };
    new_course(&src, "weber", &author).unwrap();
    std::fs::create_dir_all(src.join("glossary")).unwrap();
    for (lang, name, text) in [
        ("de", "Oktave", "Bei Weber."),
        ("en", "Octave", "At Weber."),
    ] {
        let entry = format!("## {name}\nid: oktave\n\n{text}\n");
        std::fs::write(src.join(format!("glossary/{lang}.md")), entry).unwrap();
    }
    let file = dir.join("weber.kontrakurs");
    std::fs::write(&file, pack(&src).unwrap()).unwrap();
    let mut core = Core::new(&dir.join("data"), QaOptions::default());
    core.update_settings(|s| s.language = Some("de".into()));
    assert_eq!(core.course_add(&file), "");

    let entry: Value = serde_json::from_str(&core.glossary_json("oktave")).unwrap();
    let froms: Vec<&str> = entry["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|b| b["from"].as_str())
        .collect();
    assert_eq!(froms, ["Aus: Bass-Kurs", "Aus: Mein Kurs"]);
    assert!(entry.to_string().contains("Bei Weber."));

    let list: Value = serde_json::from_str(&core.glossary_list_json()).unwrap();
    let octaves = list
        .as_array()
        .unwrap()
        .iter()
        .filter(|t| t["id"] == "oktave");
    assert_eq!(octaves.count(), 1);
    std::fs::remove_dir_all(dir).unwrap();
}

/// Two courses explain the same new term, one for any instrument, one for keys: on bass only
/// the first shows, alone and without "From:". Terms for `any` stay on every instrument.
#[test]
fn a_course_term_for_another_instrument_is_left_out() {
    use kontra_session::lesson::{new_course, pack, Author};
    use serde_json::Value;
    let dir = temp_dir("course-term-scope");
    let author = Author {
        provider: "Studio Weber".into(),
        license: "CC-BY-4.0".into(),
    };
    let mut core = Core::new(&dir.join("data"), QaOptions::default());
    core.update_settings(|s| s.language = Some("de".into()));
    for (id, scope, text) in [
        ("weber", "any", "Für alle."),
        ("tasten", "keys", "Auf Tasten."),
    ] {
        let src = dir.join(id);
        new_course(&src, id, &author).unwrap();
        std::fs::create_dir_all(src.join("glossary")).unwrap();
        for (lang, name) in [("de", "Groove"), ("en", "Groove")] {
            let entry = format!("## {name}\nid: groove-kniff\nfor: {scope}\n\n{text}\n");
            std::fs::write(src.join(format!("glossary/{lang}.md")), entry).unwrap();
        }
        let file = dir.join(format!("{id}.kontrakurs"));
        std::fs::write(&file, pack(&src).unwrap()).unwrap();
        assert_eq!(core.course_add(&file), "", "{}", core.course_preview(&file));
    }

    let entry = core.glossary_json("groove-kniff");
    assert!(entry.contains("Für alle."), "{entry}");
    assert!(!entry.contains("Auf Tasten."), "{entry}");
    let entry: Value = serde_json::from_str(&entry).unwrap();
    let froms = entry["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|b| b["from"].is_string());
    assert_eq!(froms.count(), 0);
    std::fs::remove_dir_all(dir).unwrap();
}

/// LH-LRN-04: a generated exercise is written to the data folder and opens like a piece.
#[test]
fn generated_exercise_opens() {
    let dir = temp_dir("exercise");
    let mut core = Core::new(&dir, QaOptions::default());
    assert!(Core::exercise_spec(
        "groove",
        "251",
        "nope",
        0,
        90.0,
        "",
        PieceVariety::default()
    )
    .is_none());
    let spec = Core::exercise_spec(
        "walking",
        "blues",
        "",
        5,
        110.0,
        "",
        PieceVariety::default(),
    )
    .unwrap();
    let path = core.generate_exercise(&spec).unwrap();
    assert!(path.starts_with(dir.join("exercises")));
    assert!(matches!(core.open(&path), Ok(Opened::Ready)));
    let a = core.arrangement().unwrap();
    // Twelve bars and a closing bar on the tonic.
    assert_eq!(a.notes.len(), 49);
    assert_eq!(a.markers[0].name, "F7");
    let _ = std::fs::remove_dir_all(dir);
}

/// LH-LRN-04: the generator also writes scale patterns, triads, finger exercises and odd
/// meters, named in the UI's language.
#[test]
fn generator_offers_patterns_fingers_and_meters() {
    use exercise::{ExerciseKind as K, Fingers, Progression, Rhythm, ScaleKind, Sequence};
    let dir = temp_dir("generator");
    let mut core = Core::new(&dir, QaOptions::default());
    let spec = |kind, variant, rhythm, key| {
        Core::exercise_spec(
            kind,
            variant,
            rhythm,
            key,
            80.0,
            "",
            PieceVariety::default(),
        )
        .unwrap()
    };
    let thirds = spec("scale", "dorian", "thirds", 2);
    assert_eq!(
        thirds.kind,
        K::Sequence(ScaleKind::Dorian, Sequence::Thirds)
    );
    assert_eq!(
        spec("scale", "major", "", 0).kind,
        K::Scale(ScaleKind::Major)
    );
    assert_eq!(
        spec("triads", "pop", "", 7).kind,
        K::Triads(Progression::Pop)
    );
    let fingers = spec("chromatic", "1324", "", 5);
    assert_eq!(fingers.kind, K::Chromatic(Fingers::Crossed));
    let meter = spec("groove", "minor", "78", 9);
    assert_eq!(
        meter.kind,
        K::Groove(Progression::Minor, Rhythm::SevenEight)
    );
    assert!(Core::exercise_spec(
        "chromatic",
        "1123",
        "",
        5,
        80.0,
        "",
        PieceVariety::default()
    )
    .is_none());

    core.update_settings(|s| s.language = Some("en".into()));
    let title = |core: &Core, s| core.exercise_title(&exercise::describe(s));
    assert_eq!(
        title(&core, &fingers),
        "Finger exercise 1-3-2-4 from fret 5 (80 BPM)"
    );
    assert_eq!(title(&core, &thirds), "Dorian in thirds in D (80 BPM)");
    assert_eq!(
        title(&core, &meter),
        "7/8 groove, minor cadence in Am (80 BPM)"
    );
    core.update_settings(|s| s.language = Some("de".into()));
    assert_eq!(
        title(&core, &fingers),
        "Fingerübung 1-3-2-4 ab Bund 5 (80 BPM)"
    );

    let path = core.generate_exercise(&meter).unwrap();
    assert!(matches!(core.open(&path), Ok(Opened::Ready)));
    let a = core.arrangement().unwrap();
    assert_eq!(a.bars[0].signature.numerator, 7);

    // A piece (LH-BIB-12) the way the generator page asks for it: played once, titled in
    // the UI's language, with drums.
    let piece = spec("song", "funk", "", 9);
    assert_eq!(piece.repeats, 1);
    assert_eq!(title(&core, &piece), "Stück Funk in Am (80 BPM)");
    core.update_settings(|s| s.language = Some("en".into()));
    assert_eq!(title(&core, &piece), "Funk piece in Am (80 BPM)");
    let path = core.generate_exercise(&piece).unwrap();
    assert!(matches!(core.open(&path), Ok(Opened::Ready)));
    assert!(!core.arrangement().unwrap().drums.is_empty());

    // Its level: medium is the plain piece, the others say so in the title.
    assert_eq!(spec("song", "funk", "medium", 9).kind, piece.kind);
    let easy = spec("song", "funk", "easy", 9);
    assert_eq!(title(&core, &easy), "Funk piece (easy) in Am (80 BPM)");
    core.update_settings(|s| s.language = Some("de".into()));
    assert_eq!(title(&core, &easy), "Stück Funk (leicht) in Am (80 BPM)");
    assert!(Core::exercise_spec(
        "song",
        "funk",
        "brutal",
        9,
        80.0,
        "",
        PieceVariety::default()
    )
    .is_none());

    // A groove judged by time only (DIDAKTIK I4): its own title, the file carries it.
    let timing = Core::exercise_spec(
        "groove",
        "tonic",
        "eighths",
        4,
        80.0,
        "time",
        PieceVariety::default(),
    )
    .unwrap();
    assert_eq!(timing.judging, Judging::TimeOnly);
    assert_eq!(
        title(&core, &timing),
        "Groove Achtel auf einem Akkord in E (80 BPM), nur Timing"
    );
    let path = core.generate_exercise(&timing).unwrap();
    assert!(matches!(core.open(&path), Ok(Opened::Ready)));
    assert_eq!(core.arrangement().unwrap().judging, Judging::TimeOnly);
    let normal = Core::exercise_spec(
        "groove",
        "tonic",
        "eighths",
        4,
        80.0,
        "pitch",
        PieceVariety::default(),
    )
    .unwrap();
    assert_ne!(
        core.generate_exercise(&normal).unwrap(),
        path,
        "no overwrite"
    );
    assert!(Core::exercise_spec(
        "groove",
        "tonic",
        "eighths",
        4,
        80.0,
        "loud",
        PieceVariety::default()
    )
    .is_none());

    // Open (LH-BEW-13): the line is made up, the file says so.
    let open = Core::exercise_spec(
        "groove",
        "145",
        "rests",
        9,
        80.0,
        "open",
        PieceVariety::default(),
    )
    .unwrap();
    assert_eq!(open.judging, Judging::Open);
    assert!(
        title(&core, &open).ends_with(", frei"),
        "{}",
        title(&core, &open)
    );
    let path = core.generate_exercise(&open).unwrap();
    assert!(matches!(core.open(&path), Ok(Opened::Ready)));
    assert_eq!(core.arrangement().unwrap().judging, Judging::Open);
    let _ = std::fs::remove_dir_all(dir);
}

/// LH-BEW-13: an open run's summary names the fit, not hits of the example line.
#[test]
fn open_summary_counts_by_fit() {
    let dir = temp_dir("open-summary");
    let mut core = Core::new(&dir, QaOptions::default());
    core.update_settings(|s| s.language = Some("de".into()));
    core.open(&Core::demo_path()).unwrap();
    let a = core.arrangement().unwrap().clone();
    let mut s = kontra_session::Summary::from_results(&[], &a);
    s.hit_rate = 0.9;
    s.improvisation = Some(kontra_session::Improvisation {
        chord_tones: 6,
        scale_tones: 3,
        foreign: 1,
        rests: 4,
        rests_kept: 3,
    });
    let v: serde_json::Value = serde_json::from_str(&core.summary_json(&s, &[], &[])).unwrap();
    assert_eq!(v["hitRateText"], "90 % passend");
    assert_eq!(
        v["countsText"],
        "6 Akkordtöne · 3 in der Tonart · 1 fremd · Akkordtöne 60 % · Pausen: 3 von 4 frei gelassen"
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// LH-LRN-02: the quiz asks in words, counts a right answer once and then shows the places.
#[test]
fn quiz_asks_and_shows_the_answer() {
    let dir = temp_dir("quiz");
    let mut core = Core::new(&dir, QaOptions::default());
    let q: serde_json::Value = serde_json::from_str(&core.quiz_start("notes", 0, 4)).unwrap();
    assert!(
        q["prompt"].as_str().unwrap().contains("-Saite")
            || q["prompt"].as_str().unwrap().contains("string")
    );
    assert_eq!(q["positions"].as_array().unwrap().len(), 0);
    // Play every pitch of the neck until one answers.
    let solved = (28..=55).any(|m| core.quiz_answer(f64::from(m)));
    assert!(solved);
    let q: serde_json::Value = serde_json::from_str(&core.quiz_json()).unwrap();
    assert_eq!(q["right"], 1);
    assert!(!q["positions"].as_array().unwrap().is_empty());
    let q: serde_json::Value = serde_json::from_str(&core.quiz_next()).unwrap();
    assert_eq!(
        (q["asked"].as_u64(), q["solved"].as_bool()),
        (Some(2), Some(false))
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// QA 13.10.: the open question is not counted. Before any answer there is no score (it read
/// "0 of 1 found"); a solved question counts, a skipped one never, one left after a wrong
/// note once the next is asked.
#[test]
fn quiz_score_counts_finished_questions() {
    let dir = temp_dir("quiz-score");
    let mut core = Core::new(&dir, QaOptions::default());
    core.update_settings(|s| s.language = Some("en".into()));
    let q: serde_json::Value = serde_json::from_str(&core.quiz_start("notes", 0, 4)).unwrap();
    assert_eq!(q["score"], "");
    assert!((28..=55).any(|m| core.quiz_answer(f64::from(m))));
    let q: serde_json::Value = serde_json::from_str(&core.quiz_json()).unwrap();
    assert_eq!(q["score"], "1 of 1 found");
    // Skipped is no miss (LH-LRN-18); left after a wrong note is one.
    core.quiz_next();
    let q: serde_json::Value = serde_json::from_str(&core.quiz_next()).unwrap();
    assert_eq!(q["score"], "1 of 1 found", "one skipped");
    assert!(!core.quiz_answer(0.0));
    let q: serde_json::Value = serde_json::from_str(&core.quiz_next()).unwrap();
    assert_eq!(q["score"], "1 of 2 found");
    let _ = std::fs::remove_dir_all(dir);
}

/// LH-LRN-18: the overview has a row per string and a column per fret asked; a place shows its
/// answers once it was asked, the others stay empty. 0–12 leaves out fret 12 (the open note).
#[test]
fn quiz_heat_counts_per_place() {
    let dir = temp_dir("quiz-heat");
    let mut core = Core::new(&dir, QaOptions::default());
    core.quiz_start("notes", 0, 4);
    assert!(!core.quiz_answer(0.0));
    let q: serde_json::Value = serde_json::from_str(&core.quiz_json()).unwrap();
    assert_eq!(q["heat"]["frets"], serde_json::json!([0, 1, 2, 3, 4]));
    let rows = q["heat"]["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 4);
    let asked: Vec<&serde_json::Value> = rows
        .iter()
        .flat_map(|r| r.as_array().unwrap())
        .filter(|c| !c.is_null())
        .collect();
    assert_eq!(asked, [&serde_json::json!({ "right": 0, "wrong": 1 })]);

    let q: serde_json::Value = serde_json::from_str(&core.quiz_start("notes", 0, 12)).unwrap();
    assert_eq!(q["heat"]["frets"].as_array().unwrap().len(), 12);
    let _ = std::fs::remove_dir_all(dir);
}

/// LH-BIB-08: the bundled set lists eight levels with at least 30 exercises.
#[test]
fn bundled_exercises_by_level() {
    let dir = temp_dir("bundled");
    let mut core = Core::new(&dir, QaOptions::default());
    // Level names follow the UI language; the test must not depend on the machine's.
    core.update_settings(|s| s.language = Some("de".into()));
    let levels: serde_json::Value = serde_json::from_str(&core.bundled_json()).unwrap();
    let levels = levels.as_array().unwrap();
    assert_eq!(levels.len(), 9);
    assert_eq!(levels[0]["level"], "Einstieg");
    // Each item says what it trains.
    let goal = levels[0]["items"][0]["goal"].as_str().unwrap();
    assert!(goal.starts_with("Ziel:"), "{goal}");
    // And names the topic of its "?" (LH-LRN-09).
    let topic = levels[0]["items"][0]["goalTopic"].as_str().unwrap();
    assert!(topic.starts_with("goal."), "{topic}");
    let total: usize = levels
        .iter()
        .map(|l| l["items"].as_array().unwrap().len())
        .sum();
    assert!(total >= 30);
    assert_eq!(levels[0]["items"][0]["best"], -1.0);
    let _ = std::fs::remove_dir_all(dir);
}

/// Bundled exercises and their levels are named in the UI's language; German matches the
/// titles in the files, so nothing changes for German players but the spelling of B♭.
#[test]
fn exercise_titles_follow_the_language() {
    let dir = temp_dir("titles");
    let mut core = Core::new(&dir, QaOptions::default());
    let titles = |core: &Core| -> (Vec<String>, Vec<String>) {
        let list: serde_json::Value = serde_json::from_str(&core.bundled_json()).unwrap();
        let levels = list.as_array().unwrap();
        let names = levels
            .iter()
            .map(|l| l["level"].as_str().unwrap().to_string())
            .collect();
        let items = levels
            .iter()
            .flat_map(|l| l["items"].as_array().unwrap().iter())
            .map(|i| i["title"].as_str().unwrap().to_string())
            .collect();
        (names, items)
    };
    core.update_settings(|s| s.language = Some("en".into()));
    let (levels, items) = titles(&core);
    assert_eq!(levels[0], "Getting started");
    assert!(
        items.contains(&"Major scale in G (60 BPM)".to_string()),
        "{items:?}"
    );
    assert!(items.iter().all(|t| !t.contains("exercise.")), "{items:?}");

    core.update_settings(|s| s.language = Some("de".into()));
    let (levels, items) = titles(&core);
    assert_eq!(
        levels.len(),
        exercise::bundled_specs()
            .iter()
            .map(|(l, _)| l)
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    );
    assert!(
        levels.iter().all(|l| !l.starts_with("exercise.")),
        "{levels:?}"
    );
    // The list goes level by level; appended exercises sit in earlier levels.
    // Inside a level in plan place (level 1 in learning order).
    let mut bundled: Vec<_> = exercise::bundled().into_iter().enumerate().collect();
    bundled.sort_by_key(|(i, (level, ..))| (*level, exercise::plan_place(i + 1)));
    for ((_, (_, file, ex)), title) in bundled.iter().zip(&items) {
        // German: B is H, B♭ is B.
        let german = ex.title.replace("in B ", "in H ").replace("in Bb", "in B");
        let german = ["Eb", "Ab", "Db", "Gb"]
            .iter()
            .fold(german, |t, k| {
                t.replace(&format!("in {k}"), &format!("in {}♭", &k[..1]))
            })
            .replace('#', "♯");
        assert_eq!(title, &german, "{file}");
    }
    let _ = std::fs::remove_dir_all(dir);
}

/// LH-LRN-19: call and response on a groove with chords; the tab shows after two failed
/// tries, not after one or a good one.
#[test]
fn call_and_response_shows_the_tab_after_two_failed_tries() {
    let dir = temp_dir("call");
    let mut core = Core::new(&dir, QaOptions::default());
    let spec = Core::exercise_spec(
        "groove",
        "145",
        "quarters",
        9,
        80.0,
        "",
        PieceVariety::default(),
    )
    .unwrap();
    let path = core.generate_exercise(&spec).unwrap();
    core.open(&path).unwrap();
    assert!(!core.call_hidden());
    assert!(core.set_call(true));
    assert!(core.call_on());
    assert!(!core.arrangement().unwrap().calls.is_empty());
    assert!(core.call_hidden());

    core.call_try(0.9);
    core.call_try(0.5);
    assert!(core.call_hidden(), "one failed try");
    core.call_try(0.6);
    assert!(!core.call_hidden(), "two failed tries");

    // Off: the piece as written again.
    assert!(core.set_call(false));
    assert!(!core.call_on());
    assert!(core.arrangement().unwrap().calls.is_empty());
    let _ = std::fs::remove_dir_all(dir);
}

/// Start page and library: an old run still counts as "played before" (not a first start), and a
/// piece shows when it was last played, not when it was last opened (PR #299).
#[test]
fn old_run_counts_and_recent_names_last_run() {
    let dir = temp_dir("lastrun");
    let mut core = Core::new(&dir, QaOptions::default());
    let day = SECONDS_PER_DAY;
    let now = 100 * day;
    let home: serde_json::Value = serde_json::from_str(&core.home_json_at(now)).unwrap();
    assert_eq!(home["totalRuns"], 0);

    let first = home["next"]["path"].as_str().unwrap().to_string();
    core.open(Path::new(&first)).unwrap();
    let recent: serde_json::Value = serde_json::from_str(&core.recent_json()).unwrap();
    assert!(recent[0]["lastRunUnix"].is_null());

    let a = core.arrangement().unwrap().clone();
    let run_at = now - 20 * day;
    core.progress_store()
        .record(
            &a,
            Run {
                spread_ms: None,
                finished_unix: run_at,
                practice_s: 60.0,
                speed: 1.0,
                hit_rate: 0.5,
                mode: "song".into(),
                section: None,
                hardest_bars: Vec::new(),
                hesitant_bars: Vec::new(),
                bars: Vec::new(),
            },
        )
        .unwrap();
    let home: serde_json::Value = serde_json::from_str(&core.home_json_at(now)).unwrap();
    assert_eq!(home["week"]["runs"], 0);
    assert_eq!(home["totalRuns"], 1);
    let recent: serde_json::Value = serde_json::from_str(&core.recent_json()).unwrap();
    assert_eq!(recent[0]["lastRunUnix"], run_at);
    let _ = std::fs::remove_dir_all(dir);
}

/// Bug (review of PR #316): a piece's `generated_path` ignored `seed` and `length`, so a
/// variant or a shorter/longer form of the same style and key overwrote the plain piece's
/// file and its progress.
#[test]
fn a_piece_s_path_and_title_tell_its_seed_and_length_apart() {
    let dir = temp_dir("piece-paths");
    let core = Core::new(&dir, QaOptions::default());
    let spec = |seed, length| exercise::ExerciseSpec {
        seed,
        length,
        ..exercise::ExerciseSpec {
            kind: exercise::ExerciseKind::Song(exercise::Style::Pop, exercise::Level::Medium),
            key: 7,
            bpm: 90.0,
            slow: false,
            repeats: 1,
            ramp_bpm: 0.0,
            judging: Judging::PitchAndTime,
            seed: 0,
            length: exercise::Length::Normal,
        }
    };
    let path = |seed, length| core.generated_path(&spec(seed, length));
    let plain = path(0, exercise::Length::Normal);
    let variant = path(7, exercise::Length::Normal);
    let short = path(0, exercise::Length::Short);
    let long = path(0, exercise::Length::Long);
    assert_ne!(plain, variant, "a variant overwrote the plain piece");
    assert_ne!(plain, short, "a short piece overwrote the plain piece");
    assert_ne!(plain, long, "a long piece overwrote the plain piece");
    assert_ne!(short, long);
    assert_eq!(plain, path(0, exercise::Length::Normal), "same seed and length, same path");

    let title = core.exercise_title(&exercise::describe(&spec(7, exercise::Length::Short)));
    assert!(title.contains("7"), "{title}: no sign of the seed");
    let _ = std::fs::remove_dir_all(dir);
}

/// Bug (review of PR #316): `piece_path` only replaced `/ : # ♯ ♭`. A plan's title comes from
/// an LLM; `? * " < > | \` and control characters make an invalid path on Windows (or a
/// subdirectory, for `/`), and `generate_piece` would fail with a filesystem error there.
#[test]
fn piece_path_sanitizes_every_character_invalid_in_a_file_name() {
    let dir = temp_dir("piece-sanitize");
    let core = Core::new(&dir, QaOptions::default());
    let title = "A\\B/C:D*E?F\"G<H>I|J\n\tK ♯L ♭M #N";
    let path = core.piece_path(title);
    let name = path.file_stem().unwrap().to_str().unwrap().to_string();
    for bad in ['\\', '/', ':', '*', '?', '"', '<', '>', '|', '\n', '\t', '#', '♯', '♭'] {
        assert!(!name.contains(bad), "{name:?} still has {bad:?}");
    }
    assert!(name.starts_with("A-B-C-D-E-F-G-H-I-J"), "{name}");
    let _ = std::fs::remove_dir_all(dir);
}
