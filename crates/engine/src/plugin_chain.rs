//! Plugins as slots of the effect chain (LH-EIN-19 to LH-EIN-21): the control side that builds
//! chains with plugin instances, and the `Insert` that runs one in the audio callback.
//!
//! ```text
//!  control thread                  plugin thread                  audio callback
//!  ChainBuilder::fit(spec, bank)
//!    ├─ no plugin slot: Chain built at once
//!    └─ plugin slots: PluginHost::load ─► open, state, activate, params
//!       ChainBuilder::poll (each tick) ◄── PluginLoaded { processor, handle }
//!         all in: PluginInsert { processor, lease } ─► Chain ──► rtrb ──► Shaper (fades over)
//!         Active { PluginHandle, lease } stays here: parameters go through it
//!
//!  retired chain ◄── rtrb ◄── Shaper      dropped on the control thread: PluginInsert, then
//!  Active dropped once the lease is the only holder left (processor gone): the handle closes
//!  the instance on the plugin thread
//! ```
//!
//! - Loading never blocks the control thread. Until every plugin of a new shape has answered,
//!   the old chain keeps running (none yet: the bass passes dry). A shape that changes again
//!   meanwhile drops the loads; their instances close on the plugin thread.
//! - Every new chain gets fresh instances of its plugins, loaded from the slot's saved state and
//!   parameters: the old chain still fades out with its own, and a plugin has one processor.
//! - A plugin that fails to load leaves its slot out of the chain (the signal passes it); the
//!   reason goes to `PluginProblems`, which the settings page reads.
//! - Audio runs in blocks of `BLOCK` frames: the insert collects a block, so it adds `BLOCK`
//!   frames of latency to the plugin's own (reported by `Insert::latency_frames`).

use kontra_drivers::plugins::{
    PendingLoad, PluginError, PluginFormat, PluginHandle, PluginHost, PluginInfo, PluginLoaded,
    PluginProcessor, PluginSetup, CHANNELS,
};
use kontra_dsp::{Chain, Insert};
use kontra_model::{ChainSpec, EffectKind, ParamSpec, PluginSpec};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Frames the plugin gets at once, and the delay this adds.
const BLOCK: usize = 64;
const MS_PER_S: f64 = 1000.0;

/// Why plugin slots are bypassed, by `PluginSpec::id`, and how often that changed.
#[derive(Debug, Default)]
struct ProblemList {
    reasons: BTreeMap<u32, String>,
    revision: u64,
}

/// Why a plugin slot is bypassed, by `PluginSpec::id`. Shared between the control side that
/// builds chains and the page that shows the reasons.
#[derive(Debug, Clone, Default)]
pub struct PluginProblems(Arc<Mutex<ProblemList>>);

impl PluginProblems {
    pub fn get(&self, id: u32) -> Option<String> {
        self.0.lock().ok()?.reasons.get(&id).cloned()
    }

    /// Notes why the plugin `id` could not be loaded; the engine does, a test may.
    pub fn note(&self, id: u32, reason: String) {
        let Ok(mut all) = self.0.lock() else {
            return;
        };
        if all.reasons.get(&id) == Some(&reason) {
            return;
        }
        all.reasons.insert(id, reason);
        all.revision += 1;
    }

    /// Forgets the reason for `id`: it loaded, or its id goes to a new plugin.
    pub fn clear(&self, id: u32) {
        let Ok(mut all) = self.0.lock() else {
            return;
        };
        if all.reasons.remove(&id).is_some() {
            all.revision += 1;
        }
    }

    /// Counts the changes of the list: a page that shows it knows when to look again (loads
    /// now finish after the call that started them).
    pub fn revision(&self) -> u64 {
        self.0.lock().map_or(0, |all| all.revision)
    }
}

/// The plugins of the settings, as the engine needs them.
#[derive(Debug, Clone, Default)]
pub struct PluginBank {
    specs: Arc<Vec<PluginSpec>>,
    problems: PluginProblems,
    crash_marker: Option<PathBuf>,
}

impl PluginBank {
    pub fn new(specs: Vec<PluginSpec>, problems: PluginProblems) -> PluginBank {
        PluginBank {
            specs: Arc::new(specs),
            problems,
            crash_marker: None,
        }
    }

    /// Loads of this bank leave a marker at `path` while a plugin's code runs (see
    /// `kontra_drivers::plugins::crashed_plugin`).
    pub fn with_crash_marker(mut self, path: PathBuf) -> PluginBank {
        self.crash_marker = Some(path);
        self
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
/// fail here, not when the chain is built. The plugin thread does the work; this call waits
/// for it (user-initiated, once), so never call it where a frame is due.
pub fn describe_plugin(
    info: &PluginInfo,
    crash_marker: Option<&Path>,
) -> Result<PluginSpec, PluginError> {
    let setup = PluginSetup {
        format: info.format,
        path: info.path.clone(),
        id: info.id.clone(),
        state: Vec::new(),
        params: Vec::new(),
        sample_rate: f64::from(DESCRIBE_RATE),
        max_block: BLOCK as u32,
        crash_marker: crash_marker.map(Path::to_path_buf),
    };
    let described = PluginHost::global().describe(setup)?;
    let params = described
        .params
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
        state: described.state,
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
    /// Closes the instance on the plugin thread when this is dropped.
    handle: PluginHandle,
    /// What the plugin was last told, by parameter id.
    sent: Vec<(u32, f64)>,
    /// Shared with the `PluginInsert`; alone here, the processor is gone.
    lease: Arc<()>,
}

/// One slot of a chain that waits to be built.
enum Wait {
    /// Not a plugin, off, or failed: the signal passes.
    Bypass,
    Loading(u32, PendingLoad),
    Ready(u32, Box<PluginLoaded>),
}

/// A chain of a new shape whose plugins load on the plugin thread.
struct Pending {
    spec: ChainSpec,
    /// The newest values: they may have moved while the plugins loaded.
    bank: PluginBank,
    /// One per slot of `spec`.
    slots: Vec<Wait>,
}

/// Builds the effect chain of the monitored bass with its plugins and keeps their instances.
pub struct ChainBuilder {
    rate: f64,
    shape: ChainSpec,
    current: Vec<Active>,
    /// Of chains that fade out or wait to be dropped; closed when the lease says so.
    retiring: Vec<Active>,
    /// A new shape that waits for its plugins.
    pending: Option<Pending>,
    /// What the chain built last delays the signal by.
    latency_frames: usize,
}

impl std::fmt::Debug for ChainBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChainBuilder")
            .field("plugins", &self.current.len())
            .field("retiring", &self.retiring.len())
            .field("loading", &self.pending.is_some())
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
            pending: None,
            latency_frames: 0,
        }
    }

    /// What the chain built last delays the bass by, in ms (0 before the first).
    pub fn latency_ms(&self) -> f64 {
        self.latency_frames as f64 / self.rate * MS_PER_S
    }

    /// Closes the instances of chains that are gone. Call after dropping retired chains.
    pub fn reap(&mut self) {
        self.retiring.retain(|a| Arc::strong_count(&a.lease) > 1);
    }

    /// Instances still held for chains that have not come back yet.
    pub fn retiring(&self) -> usize {
        self.retiring.len()
    }

    /// Whether a new shape still waits for its plugins (`poll` finishes it).
    pub fn is_loading(&self) -> bool {
        self.pending.is_some()
    }

    /// A new chain when effects, their order or on/off differ from the last one and all its
    /// plugins are ready (a chain without plugins is, at once), else `None`; then `poll` brings
    /// it. Parameter changes of plugins go to their instances.
    pub fn fit(&mut self, spec: &ChainSpec, bank: &PluginBank) -> Option<Box<Chain>> {
        self.reap();
        if let Some(pending) = self.pending.as_mut() {
            if pending.spec.same_shape(spec) {
                pending.bank = bank.clone();
                return self.poll();
            }
            // Changed again: the loads end, their instances close on the plugin thread.
            self.pending = None;
        }
        if self.shape.same_shape(spec) {
            self.send_params(bank);
            return None;
        }
        self.pending = Some(self.start(spec, bank));
        self.poll()
    }

    /// The chain whose plugins have all answered since `fit`, if any. Not blocking; call it
    /// from the control thread's tick. A plugin that failed leaves its slot out and notes why.
    pub fn poll(&mut self) -> Option<Box<Chain>> {
        self.reap();
        let pending = self.pending.as_mut()?;
        let problems = pending.bank.problems.clone();
        let mut waiting = false;
        for wait in &mut pending.slots {
            let Wait::Loading(id, load) = wait else {
                continue;
            };
            let id = *id;
            let Some(result) = load.poll() else {
                waiting = true;
                continue;
            };
            *wait = match result {
                Ok(loaded) => {
                    problems.clear(id);
                    Wait::Ready(id, Box::new(loaded))
                }
                Err(e) => {
                    problems.note(id, e.to_string());
                    Wait::Bypass
                }
            };
        }
        if waiting {
            return None;
        }
        let pending = self.pending.take()?;
        Some(self.build(pending))
    }

    /// Sends the loads of the plugin slots of `spec`.
    fn start(&self, spec: &ChainSpec, bank: &PluginBank) -> Pending {
        let mut slots = Vec::new();
        for slot in spec.slots() {
            if !(slot.on && slot.kind == EffectKind::Plugin) {
                slots.push(Wait::Bypass);
                continue;
            }
            let setup = bank
                .find(slot.plugin)
                .ok_or_else(|| "not in the plugin list".to_string())
                .and_then(|spec| setup_of(spec, self.rate, bank.crash_marker.clone()));
            match setup {
                Ok(setup) => {
                    let load = PluginHost::global().load(setup);
                    slots.push(Wait::Loading(slot.plugin, load));
                }
                Err(reason) => {
                    bank.problems.note(slot.plugin, reason);
                    slots.push(Wait::Bypass);
                }
            }
        }
        Pending {
            spec: *spec,
            bank: bank.clone(),
            slots,
        }
    }

    /// The chain of a pending shape whose loads are all answered; the old instances retire.
    fn build(&mut self, pending: Pending) -> Box<Chain> {
        self.shape = pending.spec;
        self.retiring.append(&mut self.current);

        let mut inserts: Vec<Option<Box<dyn Insert>>> = Vec::new();
        for wait in pending.slots {
            let Wait::Ready(id, loaded) = wait else {
                inserts.push(None);
                continue;
            };
            let (active, insert) = join(id, *loaded);
            self.current.push(active);
            inserts.push(Some(insert));
        }
        let chain = Chain::with_plugins(&pending.spec, self.rate, &mut |at| {
            inserts.get_mut(at)?.take()
        });
        self.latency_frames = chain.latency_frames();
        // Values may have moved while the plugins loaded.
        self.send_params(&pending.bank);
        Box::new(chain)
    }

    /// Changes the plugins made themselves since the last call, as (`PluginSpec::id`, parameter
    /// id, value in the plugin's units).
    pub fn take_param_changes(&mut self) -> Vec<(u32, u32, f64)> {
        let mut all = Vec::new();
        for active in &mut self.current {
            for (param, value) in active.handle.take_changes() {
                // The plugin's value is what it was last told: the settings that take it over
                // are not sent back.
                match active.sent.iter_mut().find(|(id, _)| *id == param) {
                    Some(entry) => entry.1 = value,
                    None => active.sent.push((param, value)),
                }
                all.push((active.id, param, value));
            }
        }
        all
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
                active.handle.set_param(p.id, p.value);
                match known {
                    Some(i) => active.sent[i].1 = p.value,
                    None => active.sent.push((p.id, p.value)),
                }
            }
        }
    }
}

/// What the plugin thread needs to load a slot's plugin from its saved state and parameters.
fn setup_of(
    spec: &PluginSpec,
    rate: f64,
    crash_marker: Option<PathBuf>,
) -> Result<PluginSetup, String> {
    let format = match spec.format.as_str() {
        "clap" => PluginFormat::Clap,
        "vst3" => PluginFormat::Vst3,
        other => return Err(format!("unknown format {other}")),
    };
    Ok(PluginSetup {
        format,
        path: spec.path.clone(),
        id: spec.plugin.clone(),
        state: spec.state.clone(),
        params: spec.params.iter().map(|p| (p.id, p.value)).collect(),
        sample_rate: rate,
        max_block: BLOCK as u32,
        crash_marker,
    })
}

/// Splits a loaded plugin into the insert for the chain and the control side that stays.
fn join(id: u32, loaded: PluginLoaded) -> (Active, Box<dyn Insert>) {
    let PluginLoaded {
        processor,
        handle,
        latency_samples,
        applied,
    } = loaded;
    let lease = Arc::new(());
    let insert = PluginInsert {
        processor,
        input: [[0.0; BLOCK]; CHANNELS],
        output: [[0.0; BLOCK]; CHANNELS],
        at: 0,
        latency: latency_samples as usize,
        _lease: Arc::clone(&lease),
    };
    let active = Active {
        id,
        handle,
        sent: applied,
        lease,
    };
    (active, Box::new(insert))
}
