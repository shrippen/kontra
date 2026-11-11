//! Plugins as slots of the effect chain (LH-EIN-19 to LH-EIN-21): the control side that builds
//! chains with plugin instances, and the `Insert` that runs one in the audio callback.
//!
//! ```text
//!  control thread                                        audio callback
//!  ChainBuilder::fit(spec, bank)
//!    ├─ LoadedPlugin::open ─ load_state ─ activate ─ set_param      (off the audio thread)
//!    ├─ PluginInsert { PluginProcessor, lease } ─► Chain ──► rtrb ──► Shaper (fades over)
//!    └─ Active { LoadedPlugin, lease }  stays here: parameters change through it
//!
//!  retired chain ◄── rtrb ◄── Shaper      dropped on the control thread: PluginInsert, then
//!  Active dropped once the lease is the only holder left (processor gone, plugin may close)
//! ```
//!
//! - Every new chain gets fresh instances of its plugins, loaded from the slot's saved state and
//!   parameters: the old chain still fades out with its own, and a plugin has one processor.
//! - A plugin that fails to load leaves its slot out of the chain (the signal passes it); the
//!   reason goes to `PluginProblems`, which the settings page reads.
//! - Audio runs in blocks of `BLOCK` frames: the insert collects a block, so it adds `BLOCK`
//!   frames of latency to the plugin's own (reported by `Insert::latency_frames`).

use kontra_drivers::plugins::{
    LoadedPlugin, PluginError, PluginFormat, PluginInfo, PluginProcessor, CHANNELS,
};
use kontra_dsp::{Chain, Insert};
use kontra_model::{ChainSpec, EffectKind, ParamSpec, PluginSpec};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// Frames the plugin gets at once, and the delay this adds.
const BLOCK: usize = 64;

/// Why a plugin slot is bypassed, by `PluginSpec::id`. Shared between the control side that
/// builds chains and the page that shows the reasons.
#[derive(Debug, Clone, Default)]
pub struct PluginProblems(Arc<Mutex<BTreeMap<u32, String>>>);

impl PluginProblems {
    pub fn get(&self, id: u32) -> Option<String> {
        self.0.lock().ok()?.get(&id).cloned()
    }

    /// Notes why the plugin `id` could not be loaded; the engine does, a test may.
    pub fn note(&self, id: u32, reason: String) {
        if let Ok(mut all) = self.0.lock() {
            all.insert(id, reason);
        }
    }

    /// Forgets the reason for `id`: it loaded, or its id goes to a new plugin.
    pub fn clear(&self, id: u32) {
        if let Ok(mut all) = self.0.lock() {
            all.remove(&id);
        }
    }
}

/// The plugins of the settings, as the engine needs them.
#[derive(Debug, Clone, Default)]
pub struct PluginBank {
    specs: Arc<Vec<PluginSpec>>,
    problems: PluginProblems,
}

impl PluginBank {
    pub fn new(specs: Vec<PluginSpec>, problems: PluginProblems) -> PluginBank {
        PluginBank {
            specs: Arc::new(specs),
            problems,
        }
    }

    fn find(&self, id: u32) -> Option<&PluginSpec> {
        self.specs.iter().find(|s| s.id == id)
    }
}

impl PartialEq for PluginBank {
    fn eq(&self, other: &PluginBank) -> bool {
        self.specs == other.specs
    }
}

/// Loads a plugin to find out what a slot of it needs: parameters with their values and the
/// state it starts with. `id` is 0; the caller gives it its own. Plugins that are no effects
/// fail here, not when the chain is built.
pub fn describe_plugin(info: &PluginInfo) -> Result<PluginSpec, PluginError> {
    let mut plugin = LoadedPlugin::load(info)?;
    drop(plugin.activate(f64::from(DESCRIBE_RATE), BLOCK as u32)?);
    plugin.deactivate()?;
    let params = plugin
        .params()
        .into_iter()
        .map(|p| ParamSpec {
            id: p.id,
            name: p.name,
            min: p.min,
            max: p.max,
            default: p.default,
            value: p.current,
        })
        .collect();
    Ok(PluginSpec {
        id: 0,
        format: info.format.to_string().to_ascii_lowercase(),
        path: info.path.clone(),
        plugin: info.id.clone(),
        name: info.name.clone(),
        vendor: info.vendor.clone(),
        state: plugin.save_state().unwrap_or_default(),
        params,
    })
}

/// The rate a plugin is tried at before it is added; any usual one works.
const DESCRIBE_RATE: u32 = 48_000;

/// A plugin instance in a chain: collects a block, runs it, plays the block before.
struct PluginInsert {
    processor: PluginProcessor,
    input: [[f32; BLOCK]; CHANNELS],
    output: [[f32; BLOCK]; CHANNELS],
    at: usize,
    latency: usize,
    /// Dropped last: when only the builder holds it, the processor is gone.
    _lease: Arc<()>,
}

impl std::fmt::Debug for PluginInsert {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginInsert")
            .field("latency", &self.latency)
            .finish_non_exhaustive()
    }
}

impl Insert for PluginInsert {
    fn process(&mut self, frame: (f32, f32)) -> (f32, f32) {
        let heard = (self.output[0][self.at], self.output[1][self.at]);
        self.input[0][self.at] = frame.0;
        self.input[1][self.at] = frame.1;
        self.at += 1;
        if self.at < BLOCK {
            return heard;
        }
        self.at = 0;
        let [in_l, in_r] = &self.input;
        let [out_l, out_r] = &mut self.output;
        // A failing plugin leaves its output as the input (dry), so the bass keeps sounding.
        let _ = self.processor.process([in_l, in_r], [out_l, out_r]);
        heard
    }

    fn latency_frames(&self) -> usize {
        BLOCK + self.latency
    }
}

/// A plugin of the current chain, on the control side.
struct Active {
    id: u32,
    plugin: LoadedPlugin,
    /// What the plugin was last told, by parameter id.
    sent: Vec<(u32, f64)>,
    /// Shared with the `PluginInsert`; alone here, the processor is gone.
    lease: Arc<()>,
}

/// Builds the effect chain of the monitored bass with its plugins and keeps their instances.
pub struct ChainBuilder {
    rate: f64,
    shape: ChainSpec,
    current: Vec<Active>,
    /// Of chains that fade out or wait to be dropped; closed when the lease says so.
    retiring: Vec<Active>,
}

impl std::fmt::Debug for ChainBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChainBuilder")
            .field("plugins", &self.current.len())
            .field("retiring", &self.retiring.len())
            .finish_non_exhaustive()
    }
}

impl ChainBuilder {
    pub fn new(sample_rate: f64) -> ChainBuilder {
        ChainBuilder {
            rate: sample_rate,
            shape: ChainSpec::EMPTY,
            current: Vec::new(),
            retiring: Vec::new(),
        }
    }

    /// Closes the instances of chains that are gone. Call after dropping retired chains.
    pub fn reap(&mut self) {
        self.retiring.retain(|a| Arc::strong_count(&a.lease) > 1);
    }

    /// Instances still held for chains that have not come back yet.
    pub fn retiring(&self) -> usize {
        self.retiring.len()
    }

    /// A new chain when effects, their order or on/off differ from the last one, else `None`;
    /// then parameter changes of plugins go to their instances.
    pub fn fit(&mut self, spec: &ChainSpec, bank: &PluginBank) -> Option<Box<Chain>> {
        self.reap();
        if self.shape.same_shape(spec) {
            self.send_params(bank);
            return None;
        }
        self.shape = *spec;
        self.retiring.append(&mut self.current);

        let mut inserts: Vec<Option<Box<dyn Insert>>> = Vec::new();
        for slot in spec.slots() {
            let mut insert = None;
            if slot.on && slot.kind == EffectKind::Plugin {
                insert = self.open(slot.plugin, bank);
            }
            inserts.push(insert);
        }
        let chain = Chain::with_plugins(spec, self.rate, &mut |at| inserts.get_mut(at)?.take());
        Some(Box::new(chain))
    }

    /// Loads the plugin of a slot and activates it; on failure notes why in the bank.
    fn open(&mut self, id: u32, bank: &PluginBank) -> Option<Box<dyn Insert>> {
        let opened = match bank.find(id) {
            Some(spec) => load(spec, self.rate),
            None => Err("not in the plugin list".to_string()),
        };
        match opened {
            Ok((active, insert)) => {
                bank.problems.clear(id);
                self.current.push(Active { id, ..active });
                Some(insert)
            }
            Err(reason) => {
                bank.problems.note(id, reason);
                None
            }
        }
    }

    fn send_params(&mut self, bank: &PluginBank) {
        for active in &mut self.current {
            let Some(spec) = bank.find(active.id) else {
                continue;
            };
            for p in &spec.params {
                let known = active.sent.iter().position(|(id, _)| *id == p.id);
                if known.is_some_and(|i| active.sent[i].1 == p.value) {
                    continue;
                }
                if active.plugin.set_param(p.id, p.value).is_err() {
                    continue;
                }
                match known {
                    Some(i) => active.sent[i].1 = p.value,
                    None => active.sent.push((p.id, p.value)),
                }
            }
        }
    }
}

/// An activated instance from a slot's saved state and parameters.
fn load(spec: &PluginSpec, rate: f64) -> Result<(Active, Box<dyn Insert>), String> {
    let format = match spec.format.as_str() {
        "clap" => PluginFormat::Clap,
        "vst3" => PluginFormat::Vst3,
        other => return Err(format!("unknown format {other}")),
    };
    let fail = |e: PluginError| e.to_string();
    let mut plugin = LoadedPlugin::open(format, &spec.path, Some(&spec.plugin)).map_err(fail)?;
    if !spec.state.is_empty() {
        // A state that does not fit (another version) leaves the plugin at its defaults.
        let _ = plugin.load_state(&spec.state);
    }
    let processor = plugin.activate(rate, BLOCK as u32).map_err(fail)?;
    let latency = plugin.latency_samples() as usize;

    let mut sent = Vec::new();
    for p in &spec.params {
        if plugin.set_param(p.id, p.value).is_ok() {
            sent.push((p.id, p.value));
        }
    }
    let lease = Arc::new(());
    let insert = PluginInsert {
        processor,
        input: [[0.0; BLOCK]; CHANNELS],
        output: [[0.0; BLOCK]; CHANNELS],
        at: 0,
        latency,
        _lease: Arc::clone(&lease),
    };
    let active = Active {
        id: spec.id,
        plugin,
        sent,
        lease,
    };
    Ok((active, Box::new(insert)))
}
