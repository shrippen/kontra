//! The effect chain as the mixer shows and edits it (LH-EIN-19): a JSON list for the page and
//! one edit call; the page holds no logic.
//!
//! ```text
//! page ── effects_json() ──► {"slots": [{name, on, values: [{label, shown, value}]}], "kinds": …}
//! page ── edit_effects("add" | "remove" | "up" | "down" | "on" | "set", …) ──► Settings.monitor_chain
//! ```

use crate::Core;
use kontra_model::{ChainSpec, EffectKind, SlotSpec, CHAIN_SLOTS};
use serde_json::json;

/// What a page may add: the built-in effects. Plugins (LH-EIN-20, -21) join here later.
const ADDABLE: [EffectKind; 8] = [
    EffectKind::Overdrive,
    EffectKind::Fuzz,
    EffectKind::Octaver,
    EffectKind::Chorus,
    EffectKind::EnvelopeFilter,
    EffectKind::Delay,
    EffectKind::Reverb,
    EffectKind::Widener,
];

/// The key of a kind in the texts and in the page's calls.
fn key(kind: EffectKind) -> &'static str {
    match kind {
        EffectKind::Overdrive => "overdrive",
        EffectKind::Widener => "widener",
        EffectKind::Chorus => "chorus",
        EffectKind::EnvelopeFilter => "envelope",
        EffectKind::Octaver => "octaver",
        EffectKind::Delay => "delay",
        EffectKind::Reverb => "reverb",
        EffectKind::Fuzz => "fuzz",
        EffectKind::Plugin => "plugin",
    }
}

/// How many of `a`, `b` a kind uses, and whether the mix means something to the player.
fn values_of(kind: EffectKind) -> (usize, bool) {
    match kind {
        EffectKind::Plugin => (0, false),
        EffectKind::Overdrive | EffectKind::Widener => (1, false),
        EffectKind::Fuzz | EffectKind::EnvelopeFilter => (2, false),
        EffectKind::Chorus | EffectKind::Octaver | EffectKind::Delay | EffectKind::Reverb => {
            (2, true)
        }
    }
}

fn kind_of(name: &str) -> Option<EffectKind> {
    ADDABLE.into_iter().find(|k| key(*k) == name)
}

fn value_mut<'a>(slot: &'a mut SlotSpec, name: &str) -> Option<&'a mut f32> {
    let (count, has_mix) = values_of(slot.kind);
    match name {
        "a" if count >= 1 => Some(&mut slot.a),
        "b" if count >= 2 => Some(&mut slot.b),
        "mix" if has_mix => Some(&mut slot.mix),
        _ => None,
    }
}

impl Core {
    /// The chain for the mixer: `{"max", "full", "empty", "fullText", "kinds": [{"key",
    /// "name"}], "slots": [{"index", "kind", "name", "hint", "on", "first", "last",
    /// "values": [{"key", "label", "value", "shown"}]}]}`.
    pub fn effects_json(&self) -> String {
        let chain = &self.settings().monitor_chain;
        let name = |k: EffectKind| self.tr(&format!("fx.kind.{}", key(k)), &[]);
        let slots: Vec<_> = chain
            .slots()
            .iter()
            .enumerate()
            .map(|(index, slot)| self.slot_json(index, slot, chain.slots().len()))
            .collect();
        let kinds: Vec<_> = ADDABLE
            .iter()
            .map(|&k| json!({ "key": key(k), "name": name(k) }))
            .collect();
        json!({
            "max": CHAIN_SLOTS,
            "full": chain.slots().len() >= CHAIN_SLOTS,
            "empty": chain.is_empty(),
            "fullText": self.tr("fx.full", &[("max", CHAIN_SLOTS.to_string())]),
            "kinds": kinds,
            "slots": slots,
        })
        .to_string()
    }

    fn slot_json(&self, index: usize, slot: &SlotSpec, len: usize) -> serde_json::Value {
        let kind = key(slot.kind);
        let (count, has_mix) = values_of(slot.kind);
        let shown = |v: f32| {
            let percent = (v * 100.0).round() as i32;
            self.tr("mixer.percent", &[("percent", percent.to_string())])
        };
        let mut values = Vec::new();
        let named = [("a", slot.a), ("b", slot.b)];
        for (name, value) in named.into_iter().take(count) {
            let label = self.tr(&format!("fx.value.{kind}.{name}"), &[]);
            values.push(
                json!({ "key": name, "label": label, "value": value, "shown": shown(value) }),
            );
        }
        if has_mix {
            let label = self.tr("fx.value.mix", &[]);
            values.push(json!({ "key": "mix", "label": label, "value": slot.mix, "shown": shown(slot.mix) }));
        }
        let hint = match slot.kind {
            EffectKind::Plugin => String::new(),
            _ => self.tr(&format!("fx.kind.{kind}.hint"), &[]),
        };
        json!({
            "index": index,
            "kind": kind,
            "name": self.tr(&format!("fx.kind.{kind}"), &[]),
            "hint": hint,
            "on": slot.on,
            "first": index == 0,
            "last": index + 1 == len,
            "values": values,
        })
    }

    /// Edits the chain and applies it at once (the audio crossfades). `op`: "add" (`name` is
    /// the kind), "remove", "up", "down", "on" (`value` 0 or 1) or "set" (`name` is "a", "b"
    /// or "mix"). Returns a hint when nothing changed for a reason the player should know
    /// (full chain) or monitoring cannot work, as `remix` does.
    pub fn edit_effects(
        &mut self,
        op: &str,
        index: usize,
        name: &str,
        value: f64,
    ) -> Option<String> {
        let len = self.settings().monitor_chain.slots().len();
        let value = if value.is_finite() { value } else { 0.0 };
        let apply: Box<dyn FnOnce(&mut ChainSpec)> = match op {
            "add" => {
                let kind = kind_of(name)?;
                if len >= CHAIN_SLOTS {
                    return Some(self.tr("fx.full", &[("max", CHAIN_SLOTS.to_string())]));
                }
                Box::new(move |c| {
                    c.push(SlotSpec::new(kind));
                })
            }
            "remove" if index < len => Box::new(move |c| {
                c.remove(index);
            }),
            "up" if (1..len).contains(&index) => Box::new(move |c| {
                c.move_slot(index, index - 1);
            }),
            "down" if index + 1 < len => Box::new(move |c| {
                c.move_slot(index, index + 1);
            }),
            "on" if index < len => Box::new(move |c| c.slots_mut()[index].on = value >= 0.5),
            "set" if index < len => {
                let name = name.to_string();
                Box::new(move |c| {
                    if let Some(v) = value_mut(&mut c.slots_mut()[index], &name) {
                        *v = value.clamp(0.0, 1.0) as f32;
                    }
                })
            }
            _ => return None,
        };
        self.remix(|s| apply(&mut s.monitor_chain))
    }
}
