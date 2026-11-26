//! The fixed choices of the settings, one enum each. Their serde names are the keys in the
//! settings file, in translation keys and in the UI; the UI lists them from `choices_json`.

use crate::Core;
pub use kontra_session::{Choice, Pocket};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// How to practice (LH-MOD).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PracticeMode {
    #[default]
    Play,
    Song,
    Wait,
    Workshop,
}

/// How the notes are shown (LH-ANZ-04).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum View {
    #[default]
    Lane,
    Tab,
    Notation,
    Combined,
    Keys,
}

/// The look of the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Style {
    #[default]
    Kante,
    KanteLight,
    System,
}

/// Note marks in play mode; the other modes always mark fully.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlayMarks {
    #[default]
    Quiet,
    Off,
    Full,
}

/// Motion: follow the desktop's animation setting, less or full.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Motion {
    #[default]
    System,
    Reduce,
    Full,
}

/// Timing window (LH-BEW-02); `Auto` is by input, `Custom` is `Settings::window_ms`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Window {
    #[default]
    Auto,
    Loose,
    Normal,
    Strict,
    Custom,
}

/// Tuning of the profile (LH-TUN-02); `Drop` is the lowest string a whole step down, `Custom`
/// is `Settings::custom_tuning`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TuningPreset {
    #[default]
    Standard,
    Drop,
    HalfDown,
    WholeDown,
    Custom,
}

/// String colours (LH-ANZ-06), Kante's palettes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StringColors {
    #[default]
    Kante,
    Mono,
    Bright,
}

impl Choice for PracticeMode {
    const ALL: &'static [Self] = &[Self::Play, Self::Song, Self::Wait, Self::Workshop];
}

impl Choice for View {
    const ALL: &'static [Self] = &[
        Self::Lane,
        Self::Tab,
        Self::Notation,
        Self::Combined,
        Self::Keys,
    ];
}

impl Choice for Style {
    const ALL: &'static [Self] = &[Self::Kante, Self::KanteLight, Self::System];
}

impl Choice for PlayMarks {
    const ALL: &'static [Self] = &[Self::Quiet, Self::Off, Self::Full];
}

impl Choice for Motion {
    const ALL: &'static [Self] = &[Self::System, Self::Reduce, Self::Full];
}

impl Choice for Window {
    const ALL: &'static [Self] = &[
        Self::Auto,
        Self::Loose,
        Self::Normal,
        Self::Strict,
        Self::Custom,
    ];
}

impl Choice for TuningPreset {
    const ALL: &'static [Self] = &[
        Self::Standard,
        Self::Drop,
        Self::HalfDown,
        Self::WholeDown,
        Self::Custom,
    ];
}

impl Choice for StringColors {
    const ALL: &'static [Self] = &[Self::Kante, Self::Mono, Self::Bright];
}

impl Core {
    /// The choice `key` names; an unknown key (UI and core out of step) is told as a notice.
    pub fn pick<C: Choice>(&mut self, key: &str) -> Option<C> {
        let picked = C::from_key(key);
        if picked.is_none() {
            let text = self.tr("choice.unknown", &[("key", key.to_string())]);
            self.notice.push_back(text);
        }
        picked
    }

    /// `pick` for an optional key, e.g. a command line argument.
    pub fn pick_opt<C: Choice>(&mut self, key: Option<&str>) -> Option<C> {
        key.and_then(|key| self.pick(key))
    }

    /// The keys of every fixed choice by UI property name, as JSON, for the UI's lists.
    pub fn choices_json() -> String {
        json!({
            "mode": PracticeMode::keys(),
            "view": View::keys(),
            "style": Style::keys(),
            "playMarks": PlayMarks::keys(),
            "motion": Motion::keys(),
            "strictness": Window::keys(),
            "pocket": Pocket::keys(),
            "tuning": TuningPreset::keys(),
            "stringColors": StringColors::keys(),
        })
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_keep_the_spelling_of_older_settings_files() {
        assert_eq!(PracticeMode::keys(), ["play", "song", "wait", "workshop"]);
        assert_eq!(Style::keys(), ["kante", "kante-light", "system"]);
        assert_eq!(
            TuningPreset::keys(),
            ["standard", "drop", "half-down", "whole-down", "custom"]
        );
        assert_eq!(Pocket::keys(), ["free", "onbeat", "laidback", "pushed"]);
        assert_eq!(
            Window::keys(),
            ["auto", "loose", "normal", "strict", "custom"]
        );
    }
}
