//! Kontra's own groups for effects: what the "add effect" list filters by. A plugin names
//! itself with free categories (VST 3 sub-categories, CLAP features); this maps them onto a
//! small fixed set.

use kontra_model::EffectKind;
use serde::{Deserialize, Serialize};

/// The groups, in display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PluginGroup {
    Dynamics,
    EqFilter,
    Distortion,
    Modulation,
    Space,
    Pitch,
    Restoration,
    Analysis,
    Instrument,
    Other,
}

impl PluginGroup {
    pub const ALL: [PluginGroup; 10] = [
        PluginGroup::Dynamics,
        PluginGroup::EqFilter,
        PluginGroup::Distortion,
        PluginGroup::Modulation,
        PluginGroup::Space,
        PluginGroup::Pitch,
        PluginGroup::Restoration,
        PluginGroup::Analysis,
        PluginGroup::Instrument,
        PluginGroup::Other,
    ];

    /// Stable name for translation keys and saved data.
    pub fn key(self) -> &'static str {
        match self {
            PluginGroup::Dynamics => "dynamics",
            PluginGroup::EqFilter => "eq-filter",
            PluginGroup::Distortion => "distortion",
            PluginGroup::Modulation => "modulation",
            PluginGroup::Space => "space",
            PluginGroup::Pitch => "pitch",
            PluginGroup::Restoration => "restoration",
            PluginGroup::Analysis => "analysis",
            PluginGroup::Instrument => "instrument",
            PluginGroup::Other => "other",
        }
    }
}

/// Category words (lower case, spaces and underscores as `-`) and their group. VST 3 names are
/// Steinberg's PlugType strings, the rest CLAP features. "Spatial" (panners, spatializers) is
/// Space; "Surround", "Ambisonics" and "Up-Downmix" only name a channel format and stay unmapped,
/// like "Mastering", "Tools", "Network" and "Generator" (Other).
const WORDS: &[(&str, PluginGroup)] = &[
    ("dynamics", PluginGroup::Dynamics),
    ("compressor", PluginGroup::Dynamics),
    ("limiter", PluginGroup::Dynamics),
    ("gate", PluginGroup::Dynamics),
    ("expander", PluginGroup::Dynamics),
    ("transient-shaper", PluginGroup::Dynamics),
    ("de-esser", PluginGroup::Dynamics),
    ("eq", PluginGroup::EqFilter),
    ("equalizer", PluginGroup::EqFilter),
    ("filter", PluginGroup::EqFilter),
    ("distortion", PluginGroup::Distortion),
    ("modulation", PluginGroup::Modulation),
    ("chorus", PluginGroup::Modulation),
    ("flanger", PluginGroup::Modulation),
    ("phaser", PluginGroup::Modulation),
    ("tremolo", PluginGroup::Modulation),
    ("frequency-shifter", PluginGroup::Modulation),
    ("ring-modulator", PluginGroup::Modulation),
    ("reverb", PluginGroup::Space),
    ("delay", PluginGroup::Space),
    ("echo", PluginGroup::Space),
    ("spatial", PluginGroup::Space),
    ("pitch-shift", PluginGroup::Pitch),
    ("pitch-shifter", PluginGroup::Pitch),
    ("pitch-correction", PluginGroup::Pitch),
    ("restoration", PluginGroup::Restoration),
    ("noise-reduction", PluginGroup::Restoration),
    ("denoiser", PluginGroup::Restoration),
    ("declipper", PluginGroup::Restoration),
    ("analyzer", PluginGroup::Analysis),
    ("instrument", PluginGroup::Instrument),
    ("synth", PluginGroup::Instrument),
    ("synthesizer", PluginGroup::Instrument),
    ("sampler", PluginGroup::Instrument),
    ("drum", PluginGroup::Instrument),
    ("drum-machine", PluginGroup::Instrument),
];

fn word_of(category: &str) -> String {
    category.trim().to_lowercase().replace([' ', '_'], "-")
}

/// The group of a plugin from its categories, case-insensitive. Generic words ("Fx",
/// "audio-effect", "utility", "stereo") are not in `WORDS`, so any specific word beats them;
/// among specific ones the plugin's own order decides.
///
/// ```text
/// ["Fx", "Dynamics", "Tools"]         ─► Dynamics
/// ["audio-effect", "reverb", "delay"] ─► Space
/// ["Fx", "Network"]                   ─► Other
/// ```
pub fn group_of_plugin(categories: &[String]) -> PluginGroup {
    categories
        .iter()
        .find_map(|category| {
            let word = word_of(category);
            WORDS.iter().find(|(w, _)| *w == word).map(|(_, g)| *g)
        })
        .unwrap_or(PluginGroup::Other)
}

/// The group of a built-in effect.
pub fn group_of_effect(kind: EffectKind) -> PluginGroup {
    match kind {
        EffectKind::Overdrive | EffectKind::Fuzz => PluginGroup::Distortion,
        EffectKind::Widener | EffectKind::Delay | EffectKind::Reverb => PluginGroup::Space,
        EffectKind::Chorus => PluginGroup::Modulation,
        EffectKind::EnvelopeFilter => PluginGroup::EqFilter,
        EffectKind::Octaver => PluginGroup::Pitch,
        EffectKind::Plugin => PluginGroup::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cats(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn vst3_sub_categories() {
        let cases = [
            (&["Fx", "Dynamics"][..], PluginGroup::Dynamics),
            (&["Fx", "EQ"], PluginGroup::EqFilter),
            (&["Fx", "Filter"], PluginGroup::EqFilter),
            (&["Fx", "Distortion"], PluginGroup::Distortion),
            (&["Fx", "Modulation"], PluginGroup::Modulation),
            (&["Fx", "Reverb"], PluginGroup::Space),
            (&["Fx", "Delay"], PluginGroup::Space),
            (&["Fx", "Spatial"], PluginGroup::Space),
            (&["Fx", "Pitch Shift"], PluginGroup::Pitch),
            (&["Fx", "Restoration"], PluginGroup::Restoration),
            (&["Fx", "Analyzer"], PluginGroup::Analysis),
            (&["Instrument", "Synth"], PluginGroup::Instrument),
            (&["Instrument", "Drum"], PluginGroup::Instrument),
            (&["Fx", "Tools"], PluginGroup::Other),
            (&["Fx", "Mastering"], PluginGroup::Other),
            (&["Fx", "Surround"], PluginGroup::Other),
        ];
        for (list, group) in cases {
            assert_eq!(group_of_plugin(&cats(list)), group, "{list:?}");
        }
    }

    #[test]
    fn clap_features() {
        let cases = [
            ("compressor", PluginGroup::Dynamics),
            ("limiter", PluginGroup::Dynamics),
            ("gate", PluginGroup::Dynamics),
            ("expander", PluginGroup::Dynamics),
            ("transient-shaper", PluginGroup::Dynamics),
            ("equalizer", PluginGroup::EqFilter),
            ("filter", PluginGroup::EqFilter),
            ("distortion", PluginGroup::Distortion),
            ("chorus", PluginGroup::Modulation),
            ("flanger", PluginGroup::Modulation),
            ("phaser", PluginGroup::Modulation),
            ("tremolo", PluginGroup::Modulation),
            ("frequency-shifter", PluginGroup::Modulation),
            ("ring-modulator", PluginGroup::Modulation),
            ("reverb", PluginGroup::Space),
            ("delay", PluginGroup::Space),
            ("pitch-shifter", PluginGroup::Pitch),
            ("pitch-correction", PluginGroup::Pitch),
            ("restoration", PluginGroup::Restoration),
            ("noise-reduction", PluginGroup::Restoration),
            ("analyzer", PluginGroup::Analysis),
            ("instrument", PluginGroup::Instrument),
            ("synthesizer", PluginGroup::Instrument),
            ("sampler", PluginGroup::Instrument),
            ("drum", PluginGroup::Instrument),
        ];
        for (feature, group) in cases {
            let list = cats(&["audio-effect", feature, "stereo"]);
            assert_eq!(group_of_plugin(&list), group, "{feature}");
        }
    }

    #[test]
    fn unknown_and_empty_are_other() {
        assert_eq!(group_of_plugin(&[]), PluginGroup::Other);
        assert_eq!(
            group_of_plugin(&cats(&["Fx", "Wibble"])),
            PluginGroup::Other
        );
        assert_eq!(
            group_of_plugin(&cats(&["audio-effect", "utility"])),
            PluginGroup::Other
        );
    }

    #[test]
    fn specific_beats_generic_and_case_does_not_matter() {
        assert_eq!(
            group_of_plugin(&cats(&["Fx", "Tools", "REVERB", "Mastering"])),
            PluginGroup::Space
        );
        assert_eq!(
            group_of_plugin(&cats(&["Audio Effect", "Pitch Shift"])),
            PluginGroup::Pitch
        );
    }

    #[test]
    fn first_specific_wins_among_specifics() {
        assert_eq!(
            group_of_plugin(&cats(&["Fx", "Distortion", "Filter"])),
            PluginGroup::Distortion
        );
    }

    #[test]
    fn built_in_effects() {
        let cases = [
            (EffectKind::Overdrive, PluginGroup::Distortion),
            (EffectKind::Fuzz, PluginGroup::Distortion),
            (EffectKind::Widener, PluginGroup::Space),
            (EffectKind::Chorus, PluginGroup::Modulation),
            (EffectKind::EnvelopeFilter, PluginGroup::EqFilter),
            (EffectKind::Octaver, PluginGroup::Pitch),
            (EffectKind::Delay, PluginGroup::Space),
            (EffectKind::Reverb, PluginGroup::Space),
            (EffectKind::Plugin, PluginGroup::Other),
        ];
        for (kind, group) in cases {
            assert_eq!(group_of_effect(kind), group, "{kind:?}");
        }
    }

    #[test]
    fn keys_are_unique() {
        let mut keys: Vec<_> = PluginGroup::ALL.iter().map(|g| g.key()).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), PluginGroup::ALL.len());
    }
}
