//! The effect chain as the mixer shows and edits it (LH-EIN-19): a JSON list for the page and
//! one edit call; the page holds no logic.
//!
//! ```text
//! page ── effects_json() ──► {"slots": [{name, on, values: [{label, shown, value}]}], "kinds": …,
//!                             "plugins": found ones, "scan": {…}}
//! page ── edit_effects("add" | "remove" | "up" | "down" | "on" | "set" | "scan", …)
//!                                      ──► Settings.monitor_chain (+ Settings.plugins)
//! ```
//!
//! A plugin slot lists the plugin's parameters like the built-in values: sliders from 0 to 1 that
//! `fx_plugins::from_unit` maps onto the plugin's own range.

use crate::fx_plugins::{self, found_at, param_key, param_mut, PLUGIN_KEY};
use crate::{Core, Settings};
use kontra_model::{EffectKind, PluginSpec, SlotSpec, CHAIN_SLOTS};
use kontra_session::describe_plugin;
use serde_json::json;

/// What a page may add besides the plugins found: the built-in effects.
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
    /// "name"}], "plugins": [{"key", "name", "vendor", "format"}], "scan": {"running", "text",
    /// "broken": [{"file", "reason"}]}, "slots": [{"index", "kind", "name", "hint", "problem",
    /// "on", "first", "last", "values": [{"key", "label", "value", "shown"}]}]}`.
    pub fn effects_json(&self) -> String {
        let settings = self.settings();
        let chain = &settings.monitor_chain;
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
            "plugins": self.found_plugins_json(),
            "scan": self.scan_json(),
            "slots": slots,
        })
        .to_string()
    }

    /// The plugins the last scan found, as `edit_effects("add", …)` takes them.
    fn found_plugins_json(&self) -> Vec<serde_json::Value> {
        let found = self.plugins.report().plugins.iter().enumerate();
        found
            .map(|(n, p)| {
                json!({
                    "key": format!("{PLUGIN_KEY}{n}"),
                    "name": p.name,
                    "vendor": p.vendor,
                    "format": p.format.to_string(),
                })
            })
            .collect()
    }

    /// State of the plugin search for the mixer: a line of text and the broken files.
    fn scan_json(&self) -> serde_json::Value {
        let report = self.plugins.report();
        let text = if self.plugins.is_scanning() {
            self.tr("fx.scan.running", &[])
        } else if report.plugins.is_empty() && report.broken.is_empty() {
            self.tr("fx.scan.none", &[])
        } else {
            let args = [
                ("found", report.plugins.len().to_string()),
                ("broken", report.broken.len().to_string()),
            ];
            self.tr("fx.scan.result", &args)
        };
        let broken: Vec<_> = report
            .broken
            .iter()
            .map(|b| {
                let file = b.path.file_name().map_or_else(
                    || b.path.display().to_string(),
                    |f| f.to_string_lossy().into_owned(),
                );
                json!({ "file": file, "reason": b.reason })
            })
            .collect();
        json!({ "running": self.plugins.is_scanning(), "text": text, "broken": broken })
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
        let (name, hint, problem) = match slot.kind {
            EffectKind::Plugin => self.plugin_slot_json(slot, &mut values),
            _ => (
                self.tr(&format!("fx.kind.{kind}"), &[]),
                self.tr(&format!("fx.kind.{kind}.hint"), &[]),
                String::new(),
            ),
        };
        json!({
            "index": index,
            "kind": kind,
            "name": name,
            "hint": hint,
            "problem": problem,
            "on": slot.on,
            "first": index == 0,
            "last": index + 1 == len,
            "values": values,
        })
    }

    /// Name, hint and problem of a plugin slot; its parameters go to `values`. A plugin the
    /// settings lost, a file that is gone and one the engine could not load say so.
    fn plugin_slot_json(
        &self,
        slot: &SlotSpec,
        values: &mut Vec<serde_json::Value>,
    ) -> (String, String, String) {
        let Some(spec) = self.settings().plugins.iter().find(|p| p.id == slot.plugin) else {
            return (
                self.tr("fx.kind.plugin", &[]),
                String::new(),
                self.tr("fx.plugin.lost", &[]),
            );
        };
        for p in &spec.params {
            values.push(json!({
                "key": param_key(p.id),
                "label": p.name,
                "value": fx_plugins::to_unit(p),
                "shown": format!("{:.2}", p.value),
            }));
        }
        let hint = format!("{} · {}", spec.vendor, spec.format.to_ascii_uppercase());
        let problem = if !spec.path.exists() {
            self.tr("fx.plugin.gone", &[])
        } else if let Some(reason) = self.plugins.problems.get(spec.id) {
            self.tr("fx.plugin.failed", &[("reason", reason)])
        } else {
            String::new()
        };
        (spec.name.clone(), hint, problem)
    }

    /// Edits the chain and applies it at once (the audio crossfades). `op`: "add" (`name` is
    /// the kind, or `plugin:<n>` for the n-th plugin found), "remove", "up", "down", "on"
    /// (`value` 0 or 1), "set" (`name` is "a", "b", "mix", or `p<id>` for a plugin parameter)
    /// or "scan" (looks for plugins). Returns a hint when nothing changed for a reason the
    /// player should know (full chain, a plugin that does not load) or monitoring cannot work,
    /// as `remix` does.
    pub fn edit_effects(
        &mut self,
        op: &str,
        index: usize,
        name: &str,
        value: f64,
    ) -> Option<String> {
        let len = self.settings().monitor_chain.slots().len();
        let value = if value.is_finite() { value } else { 0.0 };
        let apply: Box<dyn FnOnce(&mut Settings)> = match op {
            "scan" => {
                self.plugins.scan();
                return None;
            }
            "add" if len >= CHAIN_SLOTS => {
                return Some(self.tr("fx.full", &[("max", CHAIN_SLOTS.to_string())]));
            }
            "add" if name.starts_with(PLUGIN_KEY) => {
                let spec = match self.describe_found(name)? {
                    Ok(spec) => spec,
                    Err(hint) => return Some(hint),
                };
                Box::new(move |s| {
                    let id = fx_plugins::next_id(&s.plugins);
                    s.plugins.push(PluginSpec { id, ..spec });
                    s.monitor_chain.push(SlotSpec::for_plugin(id));
                })
            }
            "add" => {
                let kind = kind_of(name)?;
                Box::new(move |s| {
                    s.monitor_chain.push(SlotSpec::new(kind));
                })
            }
            "remove" if index < len => Box::new(move |s| {
                s.monitor_chain.remove(index);
            }),
            "up" if (1..len).contains(&index) => Box::new(move |s| {
                s.monitor_chain.move_slot(index, index - 1);
            }),
            "down" if index + 1 < len => Box::new(move |s| {
                s.monitor_chain.move_slot(index, index + 1);
            }),
            "on" if index < len => {
                Box::new(move |s| s.monitor_chain.slots_mut()[index].on = value >= 0.5)
            }
            "set" if index < len => {
                let name = name.to_string();
                Box::new(move |s| set_value(s, index, &name, value))
            }
            _ => return None,
        };
        self.remix(|s| {
            apply(s);
            s.prune_plugins();
        })
    }

    /// Loads the plugin the page chose from the scan to learn its parameters and state. `None`
    /// for a key that names nothing; `Err` with the player's hint when it does not load.
    fn describe_found(&self, key: &str) -> Option<Result<PluginSpec, String>> {
        let info = found_at(self.plugins.report(), key)?;
        let described = describe_plugin(info);
        Some(described.map_err(|e| {
            self.tr(
                "fx.plugin.cannot",
                &[("name", info.name.clone()), ("reason", e.to_string())],
            )
        }))
    }
}

/// One value of a slot: `a`, `b`, `mix`, or a parameter of its plugin.
fn set_value(settings: &mut Settings, index: usize, name: &str, value: f64) {
    let slot = &mut settings.monitor_chain.slots_mut()[index];
    if slot.kind != EffectKind::Plugin {
        if let Some(v) = value_mut(slot, name) {
            *v = value.clamp(0.0, 1.0) as f32;
        }
        return;
    }
    let id = slot.plugin;
    let spec = settings.plugins.iter_mut().find(|p| p.id == id);
    if let Some(param) = spec.and_then(|s| param_mut(s, name)) {
        param.value = fx_plugins::from_unit(param, value);
    }
}
