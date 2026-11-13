//! LLM providers Kontra can ask for a piece plan (LH-BIB-16): a fixed preset, or "custom" with
//! the user's own base URL. Each speaks one of two wire formats (`wire.rs`).

/// Whether a provider needs an API key at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyNeed {
    Required,
    /// A key helps (a hosted model, a higher rate limit) but is not required (e.g. "custom").
    Optional,
    /// A local server: no key, ever (ollama, lmstudio).
    None,
}

/// The shape of the request and the answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireFormat {
    /// `POST {base}/chat/completions`, OpenAI's message/answer shape. Most providers, including
    /// local servers, speak this.
    OpenAiChat,
    /// `POST {base}/messages`, Anthropic's own shape.
    Anthropic,
}

/// One provider preset, or the fixed shape of "custom" (its `base_url` is a placeholder; the
/// user's own base URL is held in settings, not here).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Provider {
    pub id: &'static str,
    pub name: &'static str,
    pub base_url: &'static str,
    pub wire: WireFormat,
    pub key: KeyNeed,
    /// The environment variable that overrides a stored key; empty when `key` is `None`.
    pub key_env: &'static str,
    /// Where to read about getting a key; empty when `key` is `None`.
    pub docs_url: &'static str,
    /// Sent with every request besides the usual auth header (openrouter asks for these).
    pub extra_headers: &'static [(&'static str, &'static str)],
    /// Picked when the user has not chosen one; `None` outside Anthropic (the user picks).
    pub default_model: Option<&'static str>,
}

/// The id of the "custom" preset: an OpenAI-shaped server at a base URL the user types in.
pub const CUSTOM: &str = "custom";

const OPENROUTER_HEADERS: [(&str, &str); 2] = [
    ("HTTP-Referer", "https://github.com/shrippen/kontra"),
    ("X-Title", "Kontra"),
];

/// Every provider Kontra offers, in the order the settings show them.
pub const PROVIDERS: [Provider; 12] = [
    Provider {
        id: "openrouter",
        name: "OpenRouter",
        base_url: "https://openrouter.ai/api/v1",
        wire: WireFormat::OpenAiChat,
        key: KeyNeed::Required,
        key_env: "OPENROUTER_API_KEY",
        docs_url: "https://openrouter.ai/keys",
        extra_headers: &OPENROUTER_HEADERS,
        default_model: None,
    },
    Provider {
        id: "openai",
        name: "OpenAI",
        base_url: "https://api.openai.com/v1",
        wire: WireFormat::OpenAiChat,
        key: KeyNeed::Required,
        key_env: "OPENAI_API_KEY",
        docs_url: "https://platform.openai.com/api-keys",
        extra_headers: &[],
        default_model: None,
    },
    Provider {
        id: "anthropic",
        name: "Anthropic",
        base_url: "https://api.anthropic.com/v1",
        wire: WireFormat::Anthropic,
        key: KeyNeed::Required,
        key_env: "ANTHROPIC_API_KEY",
        docs_url: "https://console.anthropic.com/settings/keys",
        extra_headers: &[],
        default_model: Some("claude-opus-5-5"),
    },
    Provider {
        id: "google",
        name: "Google Gemini",
        base_url: "https://generativelanguage.googleapis.com/v1beta/openai",
        wire: WireFormat::OpenAiChat,
        key: KeyNeed::Required,
        key_env: "GEMINI_API_KEY",
        docs_url: "https://aistudio.google.com/apikey",
        extra_headers: &[],
        default_model: None,
    },
    Provider {
        id: "mistral",
        name: "Mistral",
        base_url: "https://api.mistral.ai/v1",
        wire: WireFormat::OpenAiChat,
        key: KeyNeed::Required,
        key_env: "MISTRAL_API_KEY",
        docs_url: "https://console.mistral.ai/api-keys",
        extra_headers: &[],
        default_model: None,
    },
    Provider {
        id: "groq",
        name: "Groq",
        base_url: "https://api.groq.com/openai/v1",
        wire: WireFormat::OpenAiChat,
        key: KeyNeed::Required,
        key_env: "GROQ_API_KEY",
        docs_url: "https://console.groq.com/keys",
        extra_headers: &[],
        default_model: None,
    },
    Provider {
        id: "deepseek",
        name: "DeepSeek",
        base_url: "https://api.deepseek.com/v1",
        wire: WireFormat::OpenAiChat,
        key: KeyNeed::Required,
        key_env: "DEEPSEEK_API_KEY",
        docs_url: "https://platform.deepseek.com/api_keys",
        extra_headers: &[],
        default_model: None,
    },
    Provider {
        id: "xai",
        name: "xAI",
        base_url: "https://api.x.ai/v1",
        wire: WireFormat::OpenAiChat,
        key: KeyNeed::Required,
        key_env: "XAI_API_KEY",
        docs_url: "https://console.x.ai",
        extra_headers: &[],
        default_model: None,
    },
    Provider {
        id: "together",
        name: "Together AI",
        base_url: "https://api.together.xyz/v1",
        wire: WireFormat::OpenAiChat,
        key: KeyNeed::Required,
        key_env: "TOGETHER_API_KEY",
        docs_url: "https://api.together.ai/settings/api-keys",
        extra_headers: &[],
        default_model: None,
    },
    Provider {
        id: "ollama",
        name: "Ollama",
        base_url: "http://localhost:11434/v1",
        wire: WireFormat::OpenAiChat,
        key: KeyNeed::None,
        key_env: "",
        docs_url: "",
        extra_headers: &[],
        default_model: None,
    },
    Provider {
        id: "lmstudio",
        name: "LM Studio",
        base_url: "http://localhost:1234/v1",
        wire: WireFormat::OpenAiChat,
        key: KeyNeed::None,
        key_env: "",
        docs_url: "",
        extra_headers: &[],
        default_model: None,
    },
    Provider {
        id: CUSTOM,
        name: "Benutzerdefiniert",
        // The user's own base URL, held in settings; this preset carries none of its own.
        base_url: "",
        wire: WireFormat::OpenAiChat,
        key: KeyNeed::Optional,
        key_env: "",
        docs_url: "",
        extra_headers: &[],
        default_model: None,
    },
];

/// The preset with this id, if Kontra offers one.
pub fn provider(id: &str) -> Option<&'static Provider> {
    PROVIDERS.iter().find(|p| p.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_id_is_unique() {
        let mut ids: Vec<&str> = PROVIDERS.iter().map(|p| p.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), PROVIDERS.len());
    }

    #[test]
    fn a_key_need_names_its_variable() {
        for p in &PROVIDERS {
            match p.key {
                KeyNeed::Required => assert!(!p.key_env.is_empty(), "{}", p.id),
                KeyNeed::Optional | KeyNeed::None => {}
            }
        }
    }

    #[test]
    fn only_anthropic_has_a_default_model() {
        for p in &PROVIDERS {
            let expected = p.id == "anthropic";
            assert_eq!(p.default_model.is_some(), expected, "{}", p.id);
        }
    }

    #[test]
    fn custom_is_found_by_id() {
        assert_eq!(provider(CUSTOM).unwrap().id, CUSTOM);
        assert!(provider("nope").is_none());
    }
}
