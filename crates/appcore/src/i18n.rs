//! Texts in German and English (LH-ALL-01). Keys like Andon's `t(key)`; placeholders `{name}`.

use serde_json::Value;
use std::collections::HashMap;

const DE: &str = include_str!("../i18n/de.json");
const EN: &str = include_str!("../i18n/en.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    German,
    English,
}

impl Language {
    /// From `LC_ALL`, `LC_MESSAGES` or `LANG`; German for `de*`, else English.
    pub fn from_environment() -> Language {
        let value = ["LC_ALL", "LC_MESSAGES", "LANG"]
            .iter()
            .filter_map(|k| std::env::var(k).ok())
            .find(|v| !v.is_empty() && v != "C" && v != "C.UTF-8" && v != "POSIX")
            .unwrap_or_default();
        Language::from_code(&value)
    }

    pub fn from_code(code: &str) -> Language {
        if code.to_ascii_lowercase().starts_with("de") {
            Language::German
        } else {
            Language::English
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Language::German => "de",
            Language::English => "en",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Catalog {
    language: Language,
    texts: HashMap<String, String>,
}

impl Catalog {
    pub fn new(language: Language) -> Catalog {
        let source = match language {
            Language::German => DE,
            Language::English => EN,
        };
        let texts = match serde_json::from_str::<Value>(source) {
            Ok(Value::Object(map)) => map
                .into_iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k, s.to_string())))
                .collect(),
            _ => HashMap::new(),
        };
        Catalog { language, texts }
    }

    pub fn language(&self) -> Language {
        self.language
    }

    /// Text for `key` with `{name}` placeholders filled; the key itself if missing (visible in QA).
    pub fn tr(&self, key: &str, args: &[(&str, String)]) -> String {
        let mut text = self
            .texts
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.to_string());
        for (name, value) in args {
            text = text.replace(&format!("{{{name}}}"), value);
        }
        text
    }

    /// All texts as JSON, for QML.
    pub fn json(&self) -> String {
        serde_json::to_string(&self.texts).unwrap_or_else(|_| "{}".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placeholders(text: &str) -> Vec<String> {
        let mut found: Vec<String> = text
            .split('{')
            .skip(1)
            .filter_map(|p| p.split_once('}'))
            .map(|(n, _)| n.to_string())
            .collect();
        found.sort();
        found
    }

    #[test]
    fn catalogs_have_same_keys_and_placeholders() {
        let de = Catalog::new(Language::German);
        let en = Catalog::new(Language::English);
        assert!(de.texts.len() > 50);
        assert_eq!(de.texts.len(), en.texts.len());
        for (key, text) in &de.texts {
            let other = en
                .texts
                .get(key)
                .unwrap_or_else(|| panic!("missing in en: {key}"));
            assert_eq!(placeholders(text), placeholders(other), "{key}");
        }
    }

    #[test]
    fn fills_placeholders() {
        let de = Catalog::new(Language::German);
        assert_eq!(
            de.tr("summary.hitrate", &[("percent", "87".into())]),
            "87 % getroffen"
        );
        assert_eq!(de.tr("no.such.key", &[]), "no.such.key");
        assert_eq!(Language::from_code("de_DE.UTF-8"), Language::German);
        assert_eq!(Language::from_code("en_US"), Language::English);
    }
}
