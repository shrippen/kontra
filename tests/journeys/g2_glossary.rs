//! J-G2 Was ist …? (P1 Lea, P2 Jens): a term in a lesson links to the glossary; the entry is
//! short and leads to the lesson that teaches it, with the variant for the instrument.

use crate::common::Profile;
use serde_json::Value;

fn json(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

#[test]
fn a_lesson_term_opens_its_entry() {
    let profile = Profile::new("g2");
    let mut core = profile.core(None);
    let lesson = core.lesson_json("bass-teile");
    assert!(lesson.contains("href=\\\"term:bund\\\""), "{lesson}");

    let entry = json(&core.glossary_json("bund"));
    assert_eq!(entry["title"], "Bund");
    assert_eq!(entry["lesson"]["id"], "greifen");
    let index = json(&core.glossary_list_json());
    assert!(index.as_array().unwrap().iter().any(|t| t["id"] == "bund"));

    // A fretless bass has no frets: the entry says so, and lessons do not link it.
    core.update_settings(|s| s.fretless = true);
    let entry = core.glossary_json("bund");
    assert!(entry.contains("Markierungen"), "{entry}");
    assert!(!core.lesson_json("bass-teile").contains("term:bund"));
}

#[test]
fn a_question_mark_outside_the_lessons_opens_its_entry() {
    let profile = Profile::new("g2-topics");
    let core = profile.core(None);

    // Jens: the II-V-I of the walking exercise, then the seventh it leads to.
    let term = json(&core.term_json("progression.251"));
    assert_eq!(term["id"], "zwei-fuenf-eins");
    assert_ne!(term["text"], "");
    let entry = core.glossary_json(term["id"].as_str().unwrap());
    assert!(entry.contains("term:septakkord"), "{entry}");
    let chord = core.glossary_json("septakkord");
    assert!(chord.contains("term:septime"), "{chord}");

    // Lea: "Slide" in the evaluation, as the summary names the technique.
    let term = json(&core.term_json("technique.slide"));
    assert_eq!(term["title"], "Slide");

    // A choice without a term shows no "?".
    assert_eq!(core.term_json("rhythm.funk"), "");
}
