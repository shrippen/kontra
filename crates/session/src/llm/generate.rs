//! Turns a player's wish into a `PiecePlan` (LH-BIB-16): ask the provider, validate the
//! answer, and on the first failure ask once more with the problems found. Synchronous over
//! `dyn Http`; the caller (appcore) runs it off a background thread.
//!
//! ```text
//! wish, level ─► prompt ─► Http::post_json ─► wire::parse_chat_answer ─► extract_json
//!                                                                            │
//!                PiecePlan ◄── validate ──────────────────────────── from_json
//!                   │ invalid (1st try)
//!                   └─► retry_prompt(errors) ─► ask again ─► validate ─► PiecePlan or Invalid
//! ```

use super::provider::Provider;
use super::wire::{self, PreparedRequest, Turn, WireError};
use super::{prompt, LLM_TIMEOUT};
use kontra_content::exercise::{Level, PiecePlan, PlanError};
use kontra_engine::{Http, HttpError};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GenError {
    #[error("no connection to the provider: {0}")]
    Http(HttpError),
    #[error("the provider declined to answer")]
    Declined,
    #[error("the answer was cut off before it finished")]
    Truncated,
    #[error("the answer had no JSON object in it")]
    Unparseable,
    #[error("the answer was not the shape expected: {0}")]
    Unreadable(String),
    #[error("the plan did not validate, twice:\n{0}")]
    Invalid(String),
}

impl From<HttpError> for GenError {
    fn from(e: HttpError) -> GenError {
        GenError::Http(e)
    }
}

impl From<WireError> for GenError {
    fn from(e: WireError) -> GenError {
        match e {
            WireError::Unreadable(m) => GenError::Unreadable(m),
            WireError::Declined => GenError::Declined,
            WireError::Truncated => GenError::Truncated,
        }
    }
}

/// Writes one piece plan from `wish` at `level`: one request, and on an invalid first answer
/// one retry with the problems found. `api_key` is `None` for a keyless provider (ollama,
/// lmstudio) or when none was given; session never reads or stores the key itself.
pub fn generate_plan(
    http: &dyn Http,
    provider: &Provider,
    base_url: &str,
    api_key: Option<&str>,
    model: &str,
    wish: &str,
    level: Level,
) -> Result<PiecePlan, GenError> {
    let system = prompt::system_prompt();
    let mut turns = vec![Turn::user(prompt::user_prompt(wish, level))];

    let first = ask(http, provider, base_url, api_key, model, &system, &turns)?;
    let retry_text = match parse_and_validate(&first) {
        Ok(plan) => return Ok(plan),
        Err(ParseOutcome::NoJson) => {
            "That answer had no JSON object in it. Answer again with the JSON object only."
                .to_string()
        }
        Err(ParseOutcome::Invalid(errors)) => prompt::retry_prompt(&errors),
    };
    turns.push(Turn::assistant(first));
    turns.push(Turn::user(retry_text));

    let second = ask(http, provider, base_url, api_key, model, &system, &turns)?;
    match parse_and_validate(&second) {
        Ok(plan) => Ok(plan),
        Err(ParseOutcome::NoJson) => Err(GenError::Unparseable),
        Err(ParseOutcome::Invalid(errors)) => Err(GenError::Invalid(
            errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n"),
        )),
    }
}

/// The model list of a provider; a "test connection" is this with no further use of the list.
pub fn list_models(
    http: &dyn Http,
    provider: &Provider,
    base_url: &str,
    api_key: Option<&str>,
) -> Result<Vec<String>, GenError> {
    let request = wire::build_models_request(provider, base_url, api_key);
    let body = http.get_with(&request.url, &wire::header_refs(&request.headers))?;
    Ok(wire::parse_model_list(&body)?)
}

enum ParseOutcome {
    NoJson,
    Invalid(Vec<PlanError>),
}

fn parse_and_validate(answer: &str) -> Result<PiecePlan, ParseOutcome> {
    let Some(json) = prompt::extract_json(answer) else {
        return Err(ParseOutcome::NoJson);
    };
    let plan = match PiecePlan::from_json(json) {
        Ok(plan) => plan,
        Err(e) => return Err(ParseOutcome::Invalid(vec![e])),
    };
    match plan.validate() {
        Ok(()) => Ok(plan),
        Err(errors) => Err(ParseOutcome::Invalid(errors)),
    }
}

fn ask(
    http: &dyn Http,
    provider: &Provider,
    base_url: &str,
    api_key: Option<&str>,
    model: &str,
    system: &str,
    turns: &[Turn],
) -> Result<String, GenError> {
    let PreparedRequest { url, headers, body } =
        wire::build_chat_request(provider, base_url, api_key, model, system, turns);
    let answer = http.post_json(&url, &wire::header_refs(&headers), &body, LLM_TIMEOUT)?;
    Ok(wire::parse_chat_answer(provider.wire, &answer)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::provider::{provider, CUSTOM};
    use std::sync::Mutex;

    /// A canned answer per call, in order; records every request it was asked.
    #[derive(Debug, Default)]
    struct FakeHttp {
        answers: Mutex<Vec<Result<Vec<u8>, HttpError>>>,
        requests: Mutex<Vec<(String, Vec<u8>)>>,
    }

    impl FakeHttp {
        fn new(answers: Vec<&str>) -> FakeHttp {
            FakeHttp {
                answers: Mutex::new(
                    answers
                        .into_iter()
                        .rev()
                        .map(|a| Ok(a.as_bytes().to_vec()))
                        .collect(),
                ),
                requests: Mutex::new(Vec::new()),
            }
        }
    }

    impl Http for FakeHttp {
        fn get(&self, _url: &str) -> Result<Vec<u8>, HttpError> {
            Err(HttpError::NotSupported)
        }

        fn get_with(&self, url: &str, _headers: &[(&str, &str)]) -> Result<Vec<u8>, HttpError> {
            self.requests
                .lock()
                .unwrap()
                .push((url.to_string(), Vec::new()));
            self.answers
                .lock()
                .unwrap()
                .pop()
                .unwrap_or(Err(HttpError::NotSupported))
        }

        fn post_json(
            &self,
            url: &str,
            _headers: &[(&str, &str)],
            body: &[u8],
            _timeout: std::time::Duration,
        ) -> Result<Vec<u8>, HttpError> {
            self.requests
                .lock()
                .unwrap()
                .push((url.to_string(), body.to_vec()));
            self.answers
                .lock()
                .unwrap()
                .pop()
                .unwrap_or(Err(HttpError::NotSupported))
        }
    }

    fn chat_json(text: &str) -> String {
        serde_json::json!({"choices": [{"message": {"content": text}}]}).to_string()
    }

    const VALID_PLAN: &str = r#"{"title":"T","key":"A","bpm":100,"feel":"straight","parts":[
        {"section":"verse","bars":["A"],"times":1,"groove":"quarters","line":"roots","fill":false,"drums":"full"}
    ]}"#;

    #[test]
    fn a_valid_first_answer_needs_no_retry() {
        let http = FakeHttp::new(vec![&chat_json(VALID_PLAN)]);
        let plan = generate_plan(
            &http,
            provider("openai").unwrap(),
            provider("openai").unwrap().base_url,
            Some("sk-x"),
            "gpt-5",
            "a groove in A",
            Level::Medium,
        )
        .unwrap();
        assert_eq!(plan.title, "T");
        assert_eq!(http.requests.lock().unwrap().len(), 1);
    }

    #[test]
    fn an_invalid_first_answer_is_retried_with_its_errors() {
        let bad = r#"{"title":"","key":"Z","bpm":1,"feel":"straight","parts":[]}"#;
        let http = FakeHttp::new(vec![&chat_json(bad), &chat_json(VALID_PLAN)]);
        let plan = generate_plan(
            &http,
            provider("openai").unwrap(),
            provider("openai").unwrap().base_url,
            Some("sk-x"),
            "gpt-5",
            "a groove in A",
            Level::Medium,
        )
        .unwrap();
        assert_eq!(plan.title, "T");
        let requests = http.requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        // The retry's body carries the first answer and the errors found in it.
        let second: serde_json::Value = serde_json::from_slice(&requests[1].1).unwrap();
        let text = second["messages"].to_string();
        assert!(text.contains("key:"), "{text}");
    }

    #[test]
    fn a_second_invalid_answer_reports_its_problems() {
        let bad = r#"{"title":"","key":"Z","bpm":1,"feel":"straight","parts":[]}"#;
        let http = FakeHttp::new(vec![&chat_json(bad), &chat_json(bad)]);
        let err = generate_plan(
            &http,
            provider("openai").unwrap(),
            provider("openai").unwrap().base_url,
            Some("sk-x"),
            "gpt-5",
            "a groove in A",
            Level::Medium,
        )
        .unwrap_err();
        assert!(
            matches!(err, GenError::Invalid(ref s) if s.contains("key:")),
            "{err}"
        );
    }

    #[test]
    fn a_fenced_answer_is_still_read() {
        let fenced = format!("Sure!\n```json\n{VALID_PLAN}\n```\n");
        let http = FakeHttp::new(vec![&chat_json(&fenced)]);
        let plan = generate_plan(
            &http,
            provider("openai").unwrap(),
            provider("openai").unwrap().base_url,
            Some("sk-x"),
            "gpt-5",
            "a groove in A",
            Level::Medium,
        )
        .unwrap();
        assert_eq!(plan.title, "T");
    }

    #[test]
    fn no_json_at_all_is_retried_then_reported() {
        let http = FakeHttp::new(vec![
            &chat_json("sorry, I cannot help with that"),
            &chat_json("still no"),
        ]);
        let err = generate_plan(
            &http,
            provider("openai").unwrap(),
            provider("openai").unwrap().base_url,
            Some("sk-x"),
            "gpt-5",
            "a groove in A",
            Level::Medium,
        )
        .unwrap_err();
        assert_eq!(err, GenError::Unparseable);
        assert_eq!(http.requests.lock().unwrap().len(), 2);
    }

    #[test]
    fn anthropic_refusal_stops_without_a_retry() {
        let body = serde_json::json!({"content": [], "stop_reason": "refusal"}).to_string();
        let http = FakeHttp::new(vec![&body]);
        let err = generate_plan(
            &http,
            provider("anthropic").unwrap(),
            provider("anthropic").unwrap().base_url,
            Some("sk-ant"),
            "claude-opus-5-5",
            "a groove in A",
            Level::Medium,
        )
        .unwrap_err();
        assert_eq!(err, GenError::Declined);
        assert_eq!(http.requests.lock().unwrap().len(), 1);
    }

    #[test]
    fn a_keyless_provider_sends_no_auth_header() {
        let http = FakeHttp::new(vec![&chat_json(VALID_PLAN)]);
        generate_plan(
            &http,
            provider("ollama").unwrap(),
            provider("ollama").unwrap().base_url,
            None,
            "llama",
            "a groove in A",
            Level::Medium,
        )
        .unwrap();
    }

    #[test]
    fn custom_provider_uses_its_own_base_url() {
        let http = FakeHttp::new(vec![&chat_json(VALID_PLAN)]);
        generate_plan(
            &http,
            provider(CUSTOM).unwrap(),
            "http://localhost:8080/v1",
            None,
            "local-model",
            "a groove in A",
            Level::Medium,
        )
        .unwrap();
        let requests = http.requests.lock().unwrap();
        assert_eq!(requests[0].0, "http://localhost:8080/v1/chat/completions");
    }

    #[test]
    fn list_models_sorts_and_records_the_request() {
        let http = FakeHttp::new(vec![r#"{"data":[{"id":"z"},{"id":"a"}]}"#]);
        let models = list_models(
            &http,
            provider("openai").unwrap(),
            provider("openai").unwrap().base_url,
            Some("sk-x"),
        )
        .unwrap();
        assert_eq!(models, vec!["a", "z"]);
        assert_eq!(
            http.requests.lock().unwrap()[0].0,
            "https://api.openai.com/v1/models"
        );
    }

    #[test]
    fn an_http_error_is_never_a_silent_retry() {
        let http = FakeHttp::new(vec![]);
        let err = generate_plan(
            &http,
            provider("openai").unwrap(),
            provider("openai").unwrap().base_url,
            Some("sk-x"),
            "gpt-5",
            "a groove in A",
            Level::Medium,
        )
        .unwrap_err();
        assert_eq!(err, GenError::Http(HttpError::NotSupported));
        assert_eq!(http.requests.lock().unwrap().len(), 1);
    }
}
