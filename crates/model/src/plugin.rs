//! What a plugin slot of the effect chain keeps (LH-EIN-19 to LH-EIN-21): which plugin, its
//! saved state and its parameter values. `SlotSpec` is `Copy` with a fixed size, so the slot
//! only names its plugin by `SlotSpec::plugin`; the details live here, in a list next to the
//! chain (`Settings.plugins`).
//!
//! ```text
//! SlotSpec { kind: Plugin, plugin: 3 } ──► PluginSpec { id: 3, format, path, plugin, state, params }
//! ```
//!
//! The state is written as base64 text, so the settings stay a readable JSON file.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::path::PathBuf;

/// A parameter of a plugin with the range the page needs for a slider, in the plugin's units.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParamSpec {
    pub id: u32,
    pub name: String,
    pub min: f64,
    pub max: f64,
    pub default: f64,
    /// Where the player left it.
    pub value: f64,
}

/// One plugin instance of the chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginSpec {
    /// What `SlotSpec::plugin` names. Never 0, never reused within a list.
    pub id: u32,
    /// "clap" or "vst3".
    pub format: String,
    pub path: PathBuf,
    /// The plugin's own id inside the file (a file can hold several).
    pub plugin: String,
    pub name: String,
    pub vendor: String,
    /// The plugin's own saved state (from when it was added); the parameters are applied over it.
    #[serde(with = "base64_bytes")]
    pub state: Vec<u8>,
    pub params: Vec<ParamSpec>,
}

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const PAD: char = '=';

mod base64_bytes {
    use super::*;

    pub fn serialize<S: Serializer>(bytes: &[u8], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&encode(bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let text = String::deserialize(d)?;
        decode(&text).ok_or_else(|| serde::de::Error::custom("state is not base64"))
    }
}

/// Standard base64 with padding. Example: `[0xff, 0xfe]` → `"//4="`.
fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, &b)| n | u32::from(b) << (16 - 8 * i));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(char::from(ALPHABET[(n >> (18 - 6 * i) & 63) as usize]));
                continue;
            }
            out.push(PAD);
        }
    }
    out
}

/// Inverse of `encode`; `None` for anything that is not padded base64.
fn decode(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(4) {
        return None;
    }
    let body = text.trim_end_matches(PAD);
    if text.len() - body.len() > 2 {
        return None;
    }
    let mut out = Vec::with_capacity(body.len() * 3 / 4);
    let (mut bits, mut have) = (0u32, 0u32);
    for c in body.bytes() {
        let value = ALPHABET.iter().position(|&a| a == c)? as u32;
        bits = bits << 6 | value;
        have += 6;
        if have >= 8 {
            have -= 8;
            out.push((bits >> have & 0xff) as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_roundtrip() {
        for len in 0..9u8 {
            let bytes: Vec<u8> = (0..len)
                .map(|i| i.wrapping_mul(97).wrapping_add(250))
                .collect();
            assert_eq!(decode(&encode(&bytes)).as_deref(), Some(bytes.as_slice()));
        }
        assert_eq!(encode(&[0xff, 0xfe]), "//4=");
        assert_eq!(encode(b"Man"), "TWFu");
        assert_eq!(decode("TWF"), None);
        assert_eq!(decode("TW!u"), None);
    }

    #[test]
    fn state_is_text_in_json() {
        let spec = PluginSpec {
            id: 1,
            format: "clap".into(),
            path: "/a.clap".into(),
            plugin: "x".into(),
            name: "X".into(),
            vendor: "V".into(),
            state: vec![0, 0, 0, 63],
            params: vec![],
        };
        let text = serde_json::to_string(&spec).unwrap();
        assert!(text.contains("\"AAAAPw==\""), "{text}");
        assert_eq!(serde_json::from_str::<PluginSpec>(&text).unwrap(), spec);
    }
}
