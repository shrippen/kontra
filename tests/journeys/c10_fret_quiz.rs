//! J-C10 Griffbrett gezielt (P1 Lea, P2 Jens): the quiz asks the chosen frets, keeps the range
//! and its answers over a restart, and asks a note Lea keeps missing measurably more often.
//! Skipping (time up) is no mistake; a wrong note is one.

use crate::common::Profile;
use kontra_appcore::quiz_stats::FILE;
use serde_json::{json, Value};

/// Open strings of the standard bass (E1, A1, D2, G2).
const OPEN: [u8; 4] = [28, 33, 38, 43];
const A1: u8 = 33;
/// B on the A string: fret 2, "H" in German.
const WEAK_FRET: u8 = 2;
const DRAWS: usize = 2000;
/// Below every string: never the asked note.
const WRONG_MIDI: f64 = 0.0;

fn json_of(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

fn misses(profile: &Profile) -> u64 {
    let text = std::fs::read_to_string(profile.dir.join(FILE)).unwrap();
    json_of(&text)["places"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["wrong"].as_u64().unwrap())
        .sum()
}

#[test]
fn missed_note_comes_more_often() {
    let profile = Profile::new("c10");

    // 1. The range: 0–4 at first, an own one is kept over a restart.
    {
        let mut core = profile.core(None);
        assert_eq!(
            json_of(&core.quiz_json())["range"],
            json!({"low": 0, "high": 4})
        );
        let q = json_of(&core.quiz_start("notes", 5, 9));
        assert_eq!(q["range"], json!({"low": 5, "high": 9}));
    }
    let core = profile.core(None);
    assert_eq!(
        json_of(&core.quiz_json())["range"],
        json!({"low": 5, "high": 9})
    );
    drop(core);

    // 2. Earlier days: every place of frets 0–4 found five times, B on the A string missed five
    //    times. Kontra asks it far more often than 1 in 20.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let places: Vec<Value> = OPEN
        .iter()
        .flat_map(|&open| (0..=4u8).map(move |fret| (open, fret)))
        .map(|(open, fret)| {
            let weak = open == A1 && fret == WEAK_FRET;
            let (right, wrong) = if weak { (0, 5) } else { (5, 0) };
            json!({"open": open, "fret": fret, "right": right, "wrong": wrong, "last_s": now})
        })
        .collect();
    let stats = json!({"range": {"low": 0, "high": 4}, "places": places});
    std::fs::write(profile.dir.join(FILE), stats.to_string()).unwrap();

    let mut core = profile.core(None);
    let mut weak = 0;
    let mut q = json_of(&core.quiz_start("notes", 0, 4));
    for _ in 0..DRAWS {
        if q["prompt"].as_str().unwrap().contains("H auf der A-Saite") {
            weak += 1;
        }
        q = json_of(&core.quiz_next());
    }
    assert!(
        weak > 2 * DRAWS / 20,
        "B on the A string asked {weak} of {DRAWS}"
    );

    // 3. Skipping records nothing; a wrong note counts once.
    let before = misses(&profile);
    core.quiz_next();
    assert_eq!(misses(&profile), before, "skipped");
    assert!(!core.quiz_answer(WRONG_MIDI));
    assert!(!core.quiz_answer(WRONG_MIDI));
    assert_eq!(misses(&profile), before + 1, "a wrong note, once");
}
