//! Fixed choices of the settings: one enum each, its serde name is the key everywhere
//! (settings file, translation key, UI).

use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize};

/// An enum whose variants are the allowed values of one setting.
pub trait Choice: Copy + Serialize + DeserializeOwned + 'static {
    /// Every value, in the order the UI lists them.
    const ALL: &'static [Self];

    /// The serde name, e.g. "onbeat".
    fn key(self) -> String {
        match serde_json::to_value(self) {
            Ok(serde_json::Value::String(key)) => key,
            _ => String::new(),
        }
    }

    fn from_key(key: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|c| c.key() == key)
    }

    /// All keys, for the UI's lists.
    fn keys() -> Vec<String> {
        Self::ALL.iter().map(|c| c.key()).collect()
    }
}

/// For `#[serde(deserialize_with)]`: a stored value that is not a choice any more falls back to
/// the default of that field; the rest of the file stays.
pub fn or_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned + Default,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(serde_json::from_value(value).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    enum Sample {
        #[default]
        One,
        TwoWords,
    }

    impl Choice for Sample {
        const ALL: &'static [Sample] = &[Sample::One, Sample::TwoWords];
    }

    #[derive(Deserialize)]
    struct Holder {
        #[serde(deserialize_with = "or_default")]
        sample: Sample,
        other: u32,
    }

    #[test]
    fn keys_are_the_serde_names() {
        assert_eq!(Sample::keys(), ["one", "two-words"]);
        assert_eq!(Sample::from_key("two-words"), Some(Sample::TwoWords));
        assert_eq!(Sample::from_key("TwoWords"), None);
    }

    #[test]
    fn an_unknown_stored_value_takes_the_default_and_keeps_the_rest() {
        let held: Holder = serde_json::from_str(r#"{"sample": "gone", "other": 7}"#).unwrap();
        assert_eq!((held.sample, held.other), (Sample::One, 7));
        let held: Holder = serde_json::from_str(r#"{"sample": 3, "other": 7}"#).unwrap();
        assert_eq!((held.sample, held.other), (Sample::One, 7));
    }
}
