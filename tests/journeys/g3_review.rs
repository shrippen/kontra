//! J-G3 Wiederholen (P1 Lea): the questions of chapter 1, one answered wrong; days later the start
//! page names them as due, the wrong one first.

use crate::common::Profile;
use serde_json::Value;

const DAY_S: u64 = 86_400;

fn json(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

/// Every question of chapter 1: (lesson, page, block, right option).
fn chapter_questions(core: &kontra_appcore::Core) -> Vec<(String, usize, usize, usize)> {
    let course = json(&core.learn_json());
    let mut found = Vec::new();
    for unit in course["chapters"][0]["units"].as_array().unwrap() {
        let id = unit["id"].as_str().unwrap();
        let lesson = json(&core.lesson_json(id));
        for (p, page) in lesson["pages"].as_array().unwrap().iter().enumerate() {
            for (b, block) in page["blocks"].as_array().unwrap().iter().enumerate() {
                if block["kind"] == "question" {
                    let answer = block["answer"].as_u64().unwrap() as usize;
                    found.push((id.to_string(), p, b, answer));
                }
            }
        }
    }
    found
}

/// Days pass: every stored due date moves `days` closer.
fn days_pass(profile: &Profile, days: u64) {
    let path = profile.dir.join("learning.json");
    let mut state = json(&std::fs::read_to_string(&path).unwrap());
    for recall in state["questions"].as_object_mut().unwrap().values_mut() {
        let due = recall["due_unix"].as_u64().unwrap();
        recall["due_unix"] = (due - days * DAY_S).into();
    }
    std::fs::write(&path, state.to_string()).unwrap();
}

#[test]
fn questions_come_back_the_wrong_one_first() {
    let profile = Profile::new("g3");
    let mut core = profile.core(None);
    let questions = chapter_questions(&core);
    assert!(questions.len() > 2, "chapter 1 has questions");

    let (wrong, rest) = questions.split_first().unwrap();
    assert!(!core.lesson_answer(&wrong.0, wrong.1, wrong.2, wrong.3 + 1));
    for (id, page, block, answer) in rest {
        assert!(core.lesson_answer(id, *page, *block, *answer));
    }
    assert_eq!(
        json(&core.home_json())["review"],
        0,
        "nothing due the same day"
    );

    days_pass(&profile, 2);
    let review = json(&core.review_json());
    assert_eq!(review["questions"].as_array().unwrap().len(), 1, "{review}");
    assert_eq!(review["questions"][0]["lesson"]["id"], wrong.0.as_str());

    days_pass(&profile, 1);
    assert_eq!(json(&core.home_json())["review"], questions.len());
}
