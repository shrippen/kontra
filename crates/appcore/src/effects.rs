//! The effect chain as the mixer shows and edits it (LH-EIN-19): a JSON list for the page and
//! one edit call; the page holds no logic.
//!
//! ```text
//! page ── effects_json() ──► {"slots": [{name, on, values: [{label, shown, value}]}], "kinds": …,
//!                             "plugins": found ones, "scan": {…}}
//! page ── edit_effects("add" | "remove" | "up" | "down" | "on" | "set" | "scan"
//!                       | "save" | "load" | "delete", …)
//!                                      ──► Settings.monitor_chain (+ Settings.plugins)
//! ```
//!
//! A plugin slot lists the plugin's parameters like the built-in values: sliders from 0 to 1 that
//! `fx_plugins::from_unit` maps onto the plugin's own range.

mod presets;
pub use presets::ChainPreset;

use crate::fx_plugins::{self, found_at, param_key, param_mut, PLUGIN_KEY};
use crate::{Core, Settings};
use kontra_model::{EffectKind, PluginSpec, SlotSpec, CHAIN_SLOTS};
use kontra_session::describe_plugin_later;
use kontra_session::plugins::{PluginError, PluginFormat};
use serde_json::json;

/// A delay below this rounds to 0.0 ms and is not worth a line.
const MIN_SHOWN_LATENCY_MS: f64 = 0.05;

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
    /// The chain for the mixer: `{"max", "full", "empty", "fullText", "latencyText", "kinds":
    /// [{"key", "name"}], "plugins": [{"key", "name", "vendor", "format"}], "presets": [name], "scan": {"running", "text",
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
            "latencyText": self.latency_text(self.chain_latency_ms()),
            "kinds": kinds,
            "plugins": self.found_plugins_json(),
            "scan": self.scan_json(),
            "slots": slots,
            "adding": self.plugins.adding.as_ref().map_or_else(String::new, |p| {
                self.tr("fx.plugin.adding", &[("name", p.info().name.clone())])
            }),
            "presets": self.settings().chain_presets.iter().map(|p| &p.name).collect::<Vec<_>>(),
        })
        .to_string()
    }

    /// The line under the chain: what it delays the bass by. Empty when nothing runs or the chain
    /// adds nothing.
    pub(crate) fn latency_text(&self, latency_ms: Option<f64>) -> String {
        let Some(ms) = latency_ms.filter(|ms| *ms >= MIN_SHOWN_LATENCY_MS) else {
            return String::new();
        };
        let mark = self.tr("number.decimal_mark", &[]);
        let shown = format!("{ms:.1}").replace('.', &mark);
        self.tr("fx.latency", &[("ms", shown)])
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
        let mark = self.tr("number.decimal_mark", &[]);
        for p in &spec.params {
            values.push(json!({
                "key": param_key(p.id),
                "label": p.name,
                "value": fx_plugins::to_unit(p),
                "shown": self.plugins.text_of(spec.id, p.id, p.value).map_or_else(
                    || fx_plugins::shown_value(p.value, &mark),
                    str::to_string,
                ),
            }));
        }
        let shown = PluginFormat::from_key(&spec.format).map(|f| f.to_string());
        let hint = format!("{} · {}", spec.vendor, shown.unwrap_or_default());
        let problem = if !spec.path.exists() {
            self.tr("fx.plugin.gone", &[])
        } else if self.plugins.crashed.contains(&spec.id) {
            self.tr("fx.plugin.crashed", &[])
        } else if let Some(reason) = self.plugins.problems.get(spec.id) {
            self.tr("fx.plugin.failed", &[("reason", reason)])
        } else {
            String::new()
        };
        (spec.name.clone(), hint, problem)
    }

    /// Edits the chain and applies it at once (the audio crossfades). `op`: "add" (`name` is
    /// the kind, or `plugin:<n>` for the n-th plugin found), "remove", "up", "down", "on"
    /// (`value` 0 or 1), "set" (`name` is "a", "b", "mix", or `p<id>` for a plugin parameter),
    /// "scan" (looks for plugins), "save" (the chain as preset `name`), "load" or "delete"
    /// (preset `index`), "window" (the own window of the plugin in slot `index`). Returns a hint when nothing changed for a reason the
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
        if op == "on" && value >= 0.5 && index < len {
            // Switched on by the player: the crash notice has done its job.
            let plugin = self.settings().monitor_chain.slots()[index].plugin;
            self.plugins.crashed.remove(&plugin);
        }
        let apply: Box<dyn FnOnce(&mut Settings)> = match op {
            "scan" => {
                self.plugins.scan();
                return None;
            }
            "save" | "delete" => return self.edit_presets(op, index, name),
            "window" if index < len => return self.open_slot_window(index),
            "load" => {
                let (chain, plugins) = presets::fresh_copy(self.settings(), index)?;
                // Fresh ids may have named removed plugins before: their old notes do not apply.
                for p in &plugins {
                    self.plugins.problems.clear(p.id);
                    self.plugins.forget_texts(p.id);
                    self.plugins.crashed.remove(&p.id);
                }
                Box::new(move |s| {
                    s.monitor_chain = chain;
                    s.plugins = plugins;
                })
            }
            "add" if len >= CHAIN_SLOTS => {
                return Some(self.tr("fx.full", &[("max", CHAIN_SLOTS.to_string())]));
            }
            "add" if name.starts_with(PLUGIN_KEY) => return self.start_adding(name),
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

    /// Opens the own window of the plugin in slot `index` (FX5). Says why when it cannot.
    fn open_slot_window(&mut self, index: usize) -> Option<String> {
        let plugin = self.settings().monitor_chain.slots()[index].plugin;
        let spec = self.settings().plugins.iter().find(|p| p.id == plugin)?;
        let name = spec.name.clone();
        let Some(opened) = self.open_plugin_editor(plugin, &name) else {
            return Some(self.tr("fx.window.off", &[]));
        };
        match opened {
            Ok(()) => None,
            Err(PluginError::Editor(reason)) => {
                Some(self.tr("fx.window.failed", &[("name", name), ("reason", reason)]))
            }
            Err(PluginError::NotLoaded) => Some(self.tr("fx.window.notloaded", &[("name", name)])),
            Err(PluginError::Busy) => Some(self.tr("fx.window.busy", &[("name", name)])),
            Err(_) => Some(self.tr("fx.window.none", &[("name", name)])),
        }
    }

    /// Saves the chain as `name` or deletes preset `index`; neither touches the sound. A full
    /// list says so.
    fn edit_presets(&mut self, op: &str, index: usize, name: &str) -> Option<String> {
        let mut done = false;
        self.update_settings(|s| {
            done = match op {
                "save" => presets::save_preset(s, name),
                _ => presets::delete_preset(s, index),
            };
        });
        let full = op == "save" && !done && !name.trim().is_empty();
        let max = presets::MAX_PRESETS.to_string();
        full.then(|| self.tr("fx.preset.full", &[("max", max)]))
    }

    /// Starts loading the plugin the page chose from the scan, to learn its parameters and
    /// state, without waiting for it (FX4): `finish_adding` puts it in the chain. One at a time.
    fn start_adding(&mut self, key: &str) -> Option<String> {
        if let Some(busy) = &self.plugins.adding {
            let name = busy.info().name.clone();
            return Some(self.tr("fx.plugin.adding", &[("name", name)]));
        }
        let info = found_at(self.plugins.report(), key)?;
        let pending = describe_plugin_later(info, Some(self.plugins.crash_marker()));
        self.plugins.adding = Some(pending);
        None
    }

    /// The plugin being added, once described: into the settings and the chain, or the hint
    /// why not. `None` while it loads or when nothing is added.
    pub(crate) fn finish_adding(&mut self) -> Option<Option<String>> {
        let result = self.plugins.adding.as_ref()?.poll()?;
        let pending = self.plugins.adding.take()?;
        let name = pending.info().name.clone();
        let spec = match result {
            Ok(spec) => spec,
            Err(e) => {
                let args = [("name", name), ("reason", e.to_string())];
                return Some(Some(self.tr("fx.plugin.cannot", &args)));
            }
        };
        if self.settings().monitor_chain.slots().len() >= CHAIN_SLOTS {
            return Some(Some(
                self.tr("fx.full", &[("max", CHAIN_SLOTS.to_string())]),
            ));
        }
        // Ids come back after a removal: forget what the last holder could not load.
        let id = fx_plugins::next_id(&self.settings().plugins);
        self.plugins.problems.clear(id);
        self.plugins.forget_texts(id);
        Some(self.remix(|s| {
            s.plugins.push(PluginSpec { id, ..spec });
            s.monitor_chain.push(SlotSpec::for_plugin(id));
            s.prune_plugins();
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
