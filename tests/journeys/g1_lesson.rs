//! J-G1 Erste Lektion (P1 Lea): the course names its first lesson; "try it" hears the open E on
//! the bass and says what a wrong note was; reading every page finishes the lesson, and after a
//! restart the start page offers the next one.

use crate::common::{piece, run_until, Profile, RATE};
use kontra_appcore::UiEvent;
use serde_json::Value;
use std::time::Duration;

const FIRST: &str = "bass-teile";
const A1: u8 = 33;
const E1: u8 = 28;
/// The first note starts this long after the input opens.
const LEAD_IN_S: f64 = 0.3;

fn json(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

/// Page and block of the first "try it" in lesson `id`.
fn first_try(lesson: &Value) -> (usize, usize) {
    let pages = lesson["pages"].as_array().unwrap();
    pages
        .iter()
        .enumerate()
        .find_map(|(p, page)| {
            let blocks = page["blocks"].as_array()?;
            Some((p, blocks.iter().position(|b| b["kind"] == "try")?))
        })
        .unwrap()
}

#[test]
fn first_lesson_tries_reads_and_moves_on() {
    let profile = Profile::new("g1");
    // A wrong note (A1), then the open E the lesson asks for.
    let take = profile.dir.join("leersaite.wav");
    let samples = kontra_session::render_performance(
        &piece("Leersaite", &[A1, E1]),
        1.0,
        LEAD_IN_S,
        Default::default(),
        RATE,
    );
    kontra_session::write_mono_wav(&take, &samples, RATE).unwrap();

    let mut core = profile.core(Some(&take));
    let course = json(&core.learn_json());
    assert_eq!(course["next"]["id"], FIRST);

    let lesson = json(&core.lesson_json(FIRST));
    let (page, block) = first_try(&lesson);
    let first = json(&core.lesson_ear(FIRST, page, block).unwrap());
    assert_eq!(first["solved"], false);

    let events = run_until(
        &mut core,
        Duration::from_secs(3),
        |e| matches!(e, UiEvent::Ear(s) if s.contains("\"solved\":true")),
    );
    let last = events
        .iter()
        .rev()
        .find_map(|e| match e {
            UiEvent::Ear(s) => Some(json(s)),
            _ => None,
        })
        .unwrap();
    assert_eq!(last["solved"], true, "{last}");
    let played = last["played"].as_array().unwrap();
    assert_eq!(played[0]["right"], false, "the A first: {last}");
    assert_eq!(played[0]["note"], "A1", "a wrong note says what it was");
    core.ear_stop();

    let pages = lesson["pages"].as_array().unwrap().len();
    for page in 0..pages {
        core.lesson_seen(FIRST, page);
    }
    drop(core);

    // After a restart: the first lesson done, the start page offers the next one.
    let core = profile.core(None);
    let course = json(&core.learn_json());
    assert_eq!(course["chapters"][0]["units"][0]["done"], true);
    let home = json(&core.home_json());
    assert_eq!(home["lesson"], course["next"]);
    assert_ne!(home["lesson"]["id"], FIRST);
}
