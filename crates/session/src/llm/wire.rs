//! Request bodies and answer parsing of the two wire formats (`provider.rs`): OpenAI's
//! chat/completions shape (most providers) and Anthropic's own messages shape.

use super::provider::{Provider, WireFormat};
use kontra_engine::HttpError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Anthropic sends a longer answer back at `max_tokens`; a piece plan runs a few thousand.
const ANTHROPIC_MAX_TOKENS: u32 = 16_000;
const ANTHROPIC_VERSION: &str = "2026-10-21";
const ANTHROPIC_BETA: &str = "server-side-fallback-2026-07-01";

/// A turn of the conversation sent as `messages`; the system prompt travels apart from these
/// (as its own field for Anthropic, as the first message for `OpenAiChat`).
#[derive(Debug, Clone)]
pub struct Turn {
    pub role: Role,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
}

impl Role {
    fn as_str(self) -> &'static str {
        match self {
            Role::User => "user",
            Role::Assistant => "assistant",
        }
    }
}

impl Turn {
    pub fn user(content: String) -> Turn {
        Turn {
            role: Role::User,
            content,
        }
    }

    pub fn assistant(content: String) -> Turn {
        Turn {
            role: Role::Assistant,
            content,
        }
    }
}

/// A request ready to send: `Http::post_json`/`get_with` take its parts directly.
pub struct PreparedRequest {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// `headers` as the borrowed pairs `Http` wants.
pub fn header_refs(headers: &[(String, String)]) -> Vec<(&str, &str)> {
    headers
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect()
}

#[derive(Serialize)]
struct OpenAiMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct OpenAiResponseFormat<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
}

#[derive(Serialize)]
struct OpenAiChatRequest<'a> {
    model: &'a str,
    messages: Vec<OpenAiMessage<'a>>,
    response_format: OpenAiResponseFormat<'a>,
}

#[derive(Serialize)]
struct AnthropicMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct AnthropicRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    system: &'a str,
    messages: Vec<AnthropicMessage<'a>>,
    fallbacks: &'a str,
}

/// The request for one answer: `system` is the fixed instructions, `turns` the wish and, on a
/// retry, the previous answer and the validation errors.
pub fn build_chat_request(
    provider: &Provider,
    base_url: &str,
    api_key: Option<&str>,
    model: &str,
    system: &str,
    turns: &[Turn],
) -> PreparedRequest {
    match provider.wire {
        WireFormat::OpenAiChat => {
            let mut messages = vec![OpenAiMessage {
                role: "system",
                content: system,
            }];
            messages.extend(turns.iter().map(|t| OpenAiMessage {
                role: t.role.as_str(),
                content: &t.content,
            }));
            let body = OpenAiChatRequest {
                model,
                messages,
                response_format: OpenAiResponseFormat {
                    kind: "json_object",
                },
            };
            PreparedRequest {
                url: format!("{base_url}/chat/completions"),
                headers: auth_headers(provider, api_key),
                body: serde_json::to_vec(&body).unwrap_or_default(),
            }
        }
        WireFormat::Anthropic => {
            let messages: Vec<AnthropicMessage> = turns
                .iter()
                .map(|t| AnthropicMessage {
                    role: t.role.as_str(),
                    content: &t.content,
                })
                .collect();
            let body = AnthropicRequest {
                model,
                max_tokens: ANTHROPIC_MAX_TOKENS,
                system,
                messages,
                fallbacks: "default",
            };
            PreparedRequest {
                url: format!("{base_url}/messages"),
                headers: auth_headers(provider, api_key),
                body: serde_json::to_vec(&body).unwrap_or_default(),
            }
        }
    }
}

/// The request for the model list: the same auth headers, no body.
pub fn build_models_request(
    provider: &Provider,
    base_url: &str,
    api_key: Option<&str>,
) -> PreparedRequest {
    PreparedRequest {
        url: format!("{base_url}/models"),
        headers: auth_headers(provider, api_key),
        body: Vec::new(),
    }
}

/// The auth header of the provider's wire format, plus its fixed extra headers; none when no
/// key is given (a local server).
fn auth_headers(provider: &Provider, api_key: Option<&str>) -> Vec<(String, String)> {
    let mut headers: Vec<(String, String)> = provider
        .extra_headers
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let Some(key) = api_key.filter(|k| !k.is_empty()) else {
        return headers;
    };
    match provider.wire {
        WireFormat::OpenAiChat => {
            headers.push(("Authorization".to_string(), format!("Bearer {key}")))
        }
        WireFormat::Anthropic => {
            headers.push(("x-api-key".to_string(), key.to_string()));
            headers.push((
                "anthropic-version".to_string(),
                ANTHROPIC_VERSION.to_string(),
            ));
            headers.push(("anthropic-beta".to_string(), ANTHROPIC_BETA.to_string()));
        }
    }
    headers
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WireError {
    #[error("the answer was not the expected shape: {0}")]
    Unreadable(String),
    #[error("the provider declined to answer")]
    Declined,
    #[error("the answer was cut off before it finished (max tokens)")]
    Truncated,
}

impl From<serde_json::Error> for WireError {
    fn from(e: serde_json::Error) -> WireError {
        WireError::Unreadable(e.to_string())
    }
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiAnswerMessage,
}

#[derive(Deserialize)]
struct OpenAiAnswerMessage {
    content: String,
}

#[derive(Deserialize)]
struct OpenAiChatResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Deserialize)]
struct AnthropicContentBlock {
    #[serde(rename = "type")]
    kind: String,
    text: Option<String>,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContentBlock>,
    stop_reason: Option<String>,
}

/// The text of the model's answer, out of the provider's wrapping JSON.
pub fn parse_chat_answer(wire: WireFormat, body: &[u8]) -> Result<String, WireError> {
    match wire {
        WireFormat::OpenAiChat => {
            let answer: OpenAiChatResponse = serde_json::from_slice(body)?;
            answer
                .choices
                .into_iter()
                .next()
                .map(|c| c.message.content)
                .ok_or_else(|| WireError::Unreadable("no choices".to_string()))
        }
        WireFormat::Anthropic => {
            let answer: AnthropicResponse = serde_json::from_slice(body)?;
            match answer.stop_reason.as_deref() {
                Some("refusal") => return Err(WireError::Declined),
                Some("max_tokens") => return Err(WireError::Truncated),
                _ => {}
            }
            let text: String = answer
                .content
                .into_iter()
                .filter(|b| b.kind == "text")
                .filter_map(|b| b.text)
                .collect();
            Ok(text)
        }
    }
}

#[derive(Deserialize)]
struct ModelEntry {
    id: String,
}

#[derive(Deserialize)]
struct ModelList {
    data: Vec<ModelEntry>,
}

/// Model ids, sorted; both wire formats answer `{"data":[{"id":...}]}`.
pub fn parse_model_list(body: &[u8]) -> Result<Vec<String>, WireError> {
    let list: ModelList = serde_json::from_slice(body)?;
    let mut ids: Vec<String> = list.data.into_iter().map(|m| m.id).collect();
    ids.sort_unstable();
    Ok(ids)
}

/// The provider's error message, if the status was one (HttpError::StatusBody): shown next to
/// Kontra's own text.
pub fn provider_message(e: &HttpError) -> Option<&str> {
    match e {
        HttpError::StatusBody(_, text) if !text.is_empty() => Some(text),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::provider::PROVIDERS;
    use serde_json::Value;

    fn openai() -> &'static Provider {
        PROVIDERS.iter().find(|p| p.id == "openai").unwrap()
    }

    fn openrouter() -> &'static Provider {
        PROVIDERS.iter().find(|p| p.id == "openrouter").unwrap()
    }

    fn anthropic() -> &'static Provider {
        PROVIDERS.iter().find(|p| p.id == "anthropic").unwrap()
    }

    fn ollama() -> &'static Provider {
        PROVIDERS.iter().find(|p| p.id == "ollama").unwrap()
    }

    #[test]
    fn openai_request_has_bearer_and_json_object() {
        let request = build_chat_request(
            openai(),
            openai().base_url,
            Some("sk-abc"),
            "gpt-5",
            "system text",
            &[Turn::user("wish text".to_string())],
        );
        assert_eq!(request.url, "https://api.openai.com/v1/chat/completions");
        assert!(request
            .headers
            .contains(&("Authorization".to_string(), "Bearer sk-abc".to_string())));
        let body: Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(body["model"], "gpt-5");
        assert_eq!(body["response_format"]["type"], "json_object");
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["messages"][0]["content"], "system text");
        assert_eq!(body["messages"][1]["role"], "user");
        assert_eq!(body["messages"][1]["content"], "wish text");
    }

    #[test]
    fn openrouter_request_carries_its_extra_headers() {
        let request = build_chat_request(
            openrouter(),
            openrouter().base_url,
            Some("sk-or"),
            "some/model",
            "s",
            &[Turn::user("u".to_string())],
        );
        assert!(request.headers.contains(&(
            "HTTP-Referer".to_string(),
            "https://github.com/shrippen/kontra".to_string()
        )));
        assert!(request
            .headers
            .contains(&("X-Title".to_string(), "Kontra".to_string())));
    }

    #[test]
    fn ollama_request_sends_no_auth_header() {
        let request = build_chat_request(ollama(), ollama().base_url, None, "llama", "s", &[]);
        assert!(request.headers.is_empty());
    }

    #[test]
    fn anthropic_request_has_its_own_headers_and_shape() {
        let request = build_chat_request(
            anthropic(),
            anthropic().base_url,
            Some("sk-ant"),
            "claude-opus-5-5",
            "system text",
            &[
                Turn::user("wish".to_string()),
                Turn::assistant("first answer".to_string()),
                Turn::user("fix these".to_string()),
            ],
        );
        assert_eq!(request.url, "https://api.anthropic.com/v1/messages");
        assert!(request
            .headers
            .contains(&("x-api-key".to_string(), "sk-ant".to_string())));
        assert!(request.headers.contains(&(
            "anthropic-version".to_string(),
            ANTHROPIC_VERSION.to_string()
        )));
        assert!(request
            .headers
            .contains(&("anthropic-beta".to_string(), ANTHROPIC_BETA.to_string())));
        let body: Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(body["system"], "system text");
        assert_eq!(body["max_tokens"], ANTHROPIC_MAX_TOKENS);
        assert_eq!(body["fallbacks"], "default");
        // No system message among `messages`: it travelled in its own field.
        assert_eq!(body["messages"].as_array().unwrap().len(), 3);
        assert_eq!(body["messages"][1]["role"], "assistant");
    }

    #[test]
    fn openai_answer_reads_the_first_choice() {
        let body = br#"{"choices":[{"message":{"content":"{\"title\":\"x\"}"}}]}"#;
        let answer = parse_chat_answer(WireFormat::OpenAiChat, body).unwrap();
        assert_eq!(answer, r#"{"title":"x"}"#);
    }

    #[test]
    fn anthropic_answer_concatenates_text_and_skips_thinking() {
        let body = br#"{"content":[
            {"type":"thinking","text":"reasoning, not the answer"},
            {"type":"text","text":"{\"title\":"},
            {"type":"text","text":"\"x\"}"}
        ],"stop_reason":"end_turn"}"#;
        let answer = parse_chat_answer(WireFormat::Anthropic, body).unwrap();
        assert_eq!(answer, r#"{"title":"x"}"#);
    }

    #[test]
    fn anthropic_refusal_is_its_own_error() {
        let body = br#"{"content":[],"stop_reason":"refusal"}"#;
        assert_eq!(
            parse_chat_answer(WireFormat::Anthropic, body),
            Err(WireError::Declined)
        );
    }

    #[test]
    fn anthropic_max_tokens_is_its_own_error() {
        let body = br#"{"content":[{"type":"text","text":"cut off"}],"stop_reason":"max_tokens"}"#;
        assert_eq!(
            parse_chat_answer(WireFormat::Anthropic, body),
            Err(WireError::Truncated)
        );
    }

    #[test]
    fn model_list_is_sorted() {
        let body = br#"{"data":[{"id":"b-model"},{"id":"a-model"}]}"#;
        assert_eq!(parse_model_list(body).unwrap(), vec!["a-model", "b-model"]);
    }

    #[test]
    fn unreadable_answer_names_the_problem() {
        assert!(matches!(
            parse_chat_answer(WireFormat::OpenAiChat, b"not json"),
            Err(WireError::Unreadable(_))
        ));
    }
}
