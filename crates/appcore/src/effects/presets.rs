//! Named effect chains (LH-EIN-19, Runde 19): the player saves the chain under a name and calls
//! it up later. A preset holds a copy of the chain and of the plugins its slots name, so loading
//! one replaces both at once.

use crate::Settings;
use kontra_model::{ChainSpec, PluginSpec};
use serde::{Deserialize, Serialize};

/// One saved chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChainPreset {
    pub name: String,
    pub chain: ChainSpec,
    pub plugins: Vec<PluginSpec>,
}

/// More presets than fit a choice list help nobody; replacing one by name still works.
pub(super) const MAX_PRESETS: usize = 32;

/// Saves the chain as `name` (trimmed), replacing one of the same name. False for an empty
/// name or a full list.
pub(super) fn save_preset(settings: &mut Settings, name: &str) -> bool {
    let name = name.trim();
    if name.is_empty() {
        return false;
    }
    let preset = ChainPreset {
        name: name.to_string(),
        chain: settings.monitor_chain,
        plugins: settings.plugins.clone(),
    };
    let presets = &mut settings.chain_presets;
    if let Some(same) = presets.iter_mut().find(|p| p.name == name) {
        *same = preset;
        return true;
    }
    if presets.len() >= MAX_PRESETS {
        return false;
    }
    presets.push(preset);
    true
}

/// Makes preset `index` the chain, plugins included. False when there is none.
pub(super) fn load_preset(settings: &mut Settings, index: usize) -> bool {
    let Some(preset) = settings.chain_presets.get(index) else {
        return false;
    };
    settings.monitor_chain = preset.chain;
    settings.plugins = preset.plugins.clone();
    true
}

/// Forgets preset `index`. False when there is none.
pub(super) fn delete_preset(settings: &mut Settings, index: usize) -> bool {
    if index >= settings.chain_presets.len() {
        return false;
    }
    settings.chain_presets.remove(index);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use kontra_model::{EffectKind, SlotSpec};

    fn gain(id: u32) -> PluginSpec {
        PluginSpec {
            id,
            format: "clap".into(),
            path: "/a.clap".into(),
            plugin: "gain".into(),
            name: "Gain".into(),
            vendor: "V".into(),
            state: vec![1, 2],
            params: vec![],
        }
    }

    #[test]
    fn a_preset_brings_back_chain_and_plugins() {
        let mut s = Settings::default();
        s.monitor_chain.push(SlotSpec::new(EffectKind::Delay));
        s.monitor_chain.push(SlotSpec::for_plugin(1));
        s.plugins.push(gain(1));
        let saved = (s.monitor_chain, s.plugins.clone());

        assert!(save_preset(&mut s, "  Slap  "));
        s.monitor_chain = ChainSpec::EMPTY;
        s.plugins.clear();
        assert!(load_preset(&mut s, 0));
        assert_eq!((s.monitor_chain, s.plugins.clone()), saved);
        assert_eq!(s.chain_presets[0].name, "Slap");

        // Same name: replaced, not doubled. No name: refused.
        s.monitor_chain.remove(0);
        assert!(save_preset(&mut s, "Slap"));
        assert_eq!(s.chain_presets.len(), 1);
        assert_eq!(s.chain_presets[0].chain.slots().len(), 1);
        assert!(!save_preset(&mut s, " "));

        // Out of range changes nothing; delete drops it.
        assert!(!load_preset(&mut s, 5));
        assert!(!delete_preset(&mut s, 5));
        assert!(delete_preset(&mut s, 0));
        assert!(s.chain_presets.is_empty());
    }

    #[test]
    fn the_list_has_an_end() {
        let mut s = Settings::default();
        for n in 0..MAX_PRESETS {
            assert!(save_preset(&mut s, &n.to_string()));
        }
        assert!(!save_preset(&mut s, "one more"));
        assert!(save_preset(&mut s, "0"), "replacing still works");
    }
}
