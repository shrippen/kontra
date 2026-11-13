//! The prompt an LLM gets to write a piece plan (LH-BIB-16): fixed instructions, the
//! `PiecePlan` schema and one example, embedded so the model never has to guess the shape;
//! and reading its answer back out of whatever text came with it.

use kontra_content::exercise::{Level, PlanError};

/// The schema's file name in `kontra_content::json_schemas()` (`kontra-cli schemas`, W8).
const PIECE_PLAN_SCHEMA: &str = "piece-plan.schema.json";

/// A short, valid plan shown as the one example; kept in step with `piece_plan.rs` by a test.
const EXAMPLE_PLAN: &str = r#"{
  "title": "Beispielstück",
  "key": "A",
  "bpm": 100,
  "feel": "straight",
  "parts": [
    {
      "section": "verse",
      "bars": ["A", "A", "D", "E"],
      "times": 2,
      "groove": "eighths",
      "line": "roots",
      "fill": true,
      "drums": "full"
    }
  ]
}"#;

/// The fixed system prompt: the task, the schema, the example. Only the wish and the level
/// change between calls (`user_prompt`); nothing about the user goes out.
pub fn system_prompt() -> String {
    format!(
        "You write a bass practice piece for Kontra, a bass trainer for the electric bass \
         (range E1 to G3). From the player's wish, write one PiecePlan as JSON. Kontra renders \
         the bass line, drums and chord names from it; it does not run any code you write, only \
         this data.\n\
         \n\
         Answer with the JSON object only: no prose, no markdown code fence, nothing before the \
         opening brace or after the closing one.\n\
         \n\
         The JSON Schema it must follow:\n{}\n\
         \n\
         One example of a valid plan:\n{}\n",
        piece_plan_schema(),
        EXAMPLE_PLAN,
    )
}

/// The user's wish and the level they chose to play at.
pub fn user_prompt(wish: &str, level: Level) -> String {
    format!(
        "Wish: {}\nLevel to write the bass part at: {}.",
        wish.trim(),
        level_text(level),
    )
}

/// The retry turn after an invalid first answer: the problems found, by place in the plan.
pub fn retry_prompt(errors: &[PlanError]) -> String {
    let lines: Vec<String> = errors.iter().map(|e| format!("- {e}")).collect();
    format!(
        "That plan did not validate. Problems:\n{}\n\
         Answer again with one corrected, complete JSON object, the same rules as before.",
        lines.join("\n"),
    )
}

fn level_text(level: Level) -> &'static str {
    match level {
        Level::Easy => "easy",
        Level::Medium => "medium",
        Level::Hard => "hard",
    }
}

/// The piece plan schema as pretty JSON text; "{}" (an always-valid, if useless, schema) if it
/// could not be generated, so a prompt still goes out instead of failing to build one.
fn piece_plan_schema() -> String {
    kontra_content::json_schemas()
        .ok()
        .and_then(|schemas| {
            schemas
                .into_iter()
                .find(|(name, _)| *name == PIECE_PLAN_SCHEMA)
        })
        .map(|(_, text)| text)
        .unwrap_or_else(|| "{}".to_string())
}

/// The first JSON object in `text`: from its first `{` to its last `}`, so a code fence or a
/// sentence around the JSON (both common) is dropped without needing to recognise either.
pub fn extract_json(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    (end >= start).then(|| &text[start..=end])
}

#[cfg(test)]
mod tests {
    use super::*;
    use kontra_content::exercise::PiecePlan;

    #[test]
    fn example_plan_is_valid() {
        let plan = PiecePlan::from_json(EXAMPLE_PLAN).expect("json");
        plan.validate().expect("valid");
    }

    #[test]
    fn system_prompt_embeds_schema_and_example() {
        let prompt = system_prompt();
        assert!(prompt.contains("\"PiecePlan\""), "{prompt}");
        assert!(prompt.contains("Beispielstück"));
        assert!(prompt.contains("JSON object only"));
    }

    #[test]
    fn user_prompt_names_wish_and_level() {
        let prompt = user_prompt("Motown-Groove in A", Level::Medium);
        assert!(prompt.contains("Motown-Groove in A"));
        assert!(prompt.contains("medium"));
    }

    #[test]
    fn extract_json_drops_a_fence_and_prose() {
        let text = "Here you go:\n```json\n{\"title\":\"x\"}\n```\nEnjoy!";
        assert_eq!(extract_json(text), Some(r#"{"title":"x"}"#));
    }

    #[test]
    fn extract_json_none_without_braces() {
        assert_eq!(extract_json("sorry, I cannot do that"), None);
    }

    #[test]
    fn retry_prompt_lists_every_problem() {
        let plan =
            PiecePlan::from_json(r#"{"title":"","key":"Z","bpm":1,"feel":"straight","parts":[]}"#)
                .expect("json");
        let errors = plan.validate().expect_err("invalid");
        let prompt = retry_prompt(&errors);
        for e in &errors {
            assert!(prompt.contains(&e.to_string()), "{prompt}");
        }
    }
}
