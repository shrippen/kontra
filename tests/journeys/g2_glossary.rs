//! J-G2 Was ist …? (P1 Lea, P2 Jens): a term in a lesson links to the glossary; the entry is
//! short and leads to the lesson that teaches it. Terms for other instruments stay out.

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

    // A fretless bass has no frets: the term is not in its glossary.
    core.update_settings(|s| s.fretless = true);
    let index = json(&core.glossary_list_json());
    assert!(!index.as_array().unwrap().iter().any(|t| t["id"] == "bund"));
    assert!(!core.lesson_json("bass-teile").contains("term:bund"));
}
