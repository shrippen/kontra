//! Plugins as slots of the effect chain (LH-EIN-19 to LH-EIN-21): the control side that builds
//! chains with plugin instances, and the `Insert` that runs one in the audio callback.
//!
//! ```text
//!  control thread                  plugin thread                  audio callback
//!  ChainBuilder::fit(spec, bank)
//!    ├─ no new plugin: Chain built at once
//!    └─ new plugin ids: PluginHost::load ─► open, state, activate, params
//!       ChainBuilder::poll (each tick) ◄── PluginLoaded { processor, handle }
//!         all in: PluginInsert { processor } ─► Chain ──► rtrb ──► Shaper (fades over)
//!         Active { PluginHandle, parked } stays here: parameters go through it
//!
//!  retired chain ◄── rtrb ◄── Shaper      dropped on the control thread: its PluginInsert
//!  parks the processor in `Active::parked`; a later chain takes it from there
//!
//!  a kept instance that must move into another chain (reorder, off → on):
//!    running chain  [P W]  holds the processor
//!    1. gap chain   [W]    P bypassed; sent, the builder waits (poll) until [P W] has come
//!                          back and parked the processor
//!    2. target      [W P]  built with the parked processor
//! ```
//!
//! - One instance per plugin slot (`PluginSpec::id`) lives as long as the slot is in the
//!   settings, whatever the chain's shape: reordering, adding or removing another effect and
//!   switching the slot off and on keep it, with its own state (and, later, its open window).
//!   Only a new id loads; only an id that leaves the settings closes its instance (after the
//!   chain that still runs it is gone).
//! - A plugin has one processor, and the chain that fades out still runs it while the next one
//!   fades in. So a kept plugin moves in two steps (see above): the plugin is bypassed for the
//!   two fades the steps take (a few tens of ms plus the wait for the audio thread). The
//!   builder's rate never changes and the block size is fixed, so the processor is reused as it
//!   is: no deactivate or activate.
//! - Loading never blocks the control thread. Until every new plugin of a shape has answered,
//!   the old chain keeps running (none yet: the bass passes dry). A shape that changes again
//!   meanwhile drops the loads; their instances close on the plugin thread.
//! - A plugin that fails to load leaves its slot out of the chain (the signal passes it); the
//!   reason goes to `PluginProblems`, which the settings page reads.
//! - Audio runs in blocks of `BLOCK` frames: the insert collects a block, so it adds `BLOCK`
//!   frames of latency to the plugin's own (reported by `Insert::latency_frames`).
//! - Each `Player` has its own builder and so its own instances; players never run at once.

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
        format: info.format.key().into(),
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

/// Where a plugin's processor waits while no chain runs it.
type Parked = Arc<Mutex<Option<PluginProcessor>>>;

/// A plugin instance in a chain: collects a block, runs it, plays the block before.
struct PluginInsert {
    /// `None` only after the drop gave it back.
    processor: Option<PluginProcessor>,
    input: [[f32; BLOCK]; CHANNELS],
    output: [[f32; BLOCK]; CHANNELS],
    at: usize,
    latency: usize,
    /// Where the processor goes when the chain is dropped (on the control thread).
    parked: Parked,
}

impl PluginInsert {
    fn new(processor: PluginProcessor, latency_samples: u32, parked: Parked) -> PluginInsert {
        PluginInsert {
            processor: Some(processor),
            input: [[0.0; BLOCK]; CHANNELS],
            output: [[0.0; BLOCK]; CHANNELS],
            at: 0,
            latency: latency_samples as usize,
            parked,
        }
    }
}

impl Drop for PluginInsert {
    /// Hands the processor back to its instance, so the next chain can run it. A busy or
    /// poisoned place drops it here instead: the builder then sees the instance as lost.
    fn drop(&mut self) {
        let Some(processor) = self.processor.take() else {
            return;
        };
        if let Ok(mut place) = self.parked.try_lock() {
            *place = Some(processor);
        }
    }
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
        if let Some(processor) = self.processor.as_mut() {
            let _ = processor.process([in_l, in_r], [out_l, out_r]);
        }
        heard
    }

    fn latency_frames(&self) -> usize {
        BLOCK + self.latency
    }
}

/// Where the processor of an instance is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Place {
    /// Back from its chain: a new chain can take it.
    Parked,
    /// In a chain that is not dropped yet.
    Away,
    /// Its chain is gone without handing it back: the instance is of no use.
    Lost,
}

/// The instance of a plugin slot, on the control side. Lives as long as the slot is in the
/// settings.
struct Active {
    /// Dropped first: the processor must be gone before the handle closes the instance.
    parked: Parked,
    id: u32,
    /// Closes the instance on the plugin thread when this is dropped.
    handle: PluginHandle,
    /// What the plugin was last told, by parameter id.
    sent: Vec<(u32, f64)>,
    /// Delay the plugin adds, in samples.
    latency_samples: u32,
}

impl Active {
    fn place(&self) -> Place {
        // The count first: an insert that is gone has parked its processor before.
        let holders = Arc::strong_count(&self.parked);
        let parked = self.parked.lock().is_ok_and(|p| p.is_some());
        match (parked, holders) {
            (true, _) => Place::Parked,
            (false, 1) => Place::Lost,
            (false, _) => Place::Away,
        }
    }

    /// The insert that runs the instance, if its processor is here.
    fn take_insert(&self) -> Option<Box<dyn Insert>> {
        let processor = self.parked.lock().ok()?.take()?;
        let parked = Arc::clone(&self.parked);
        Some(Box::new(PluginInsert::new(
            processor,
            self.latency_samples,
            parked,
        )))
    }
}

/// One slot of a chain that waits to be built.
enum Wait {
    /// Not a plugin, off, or failed: the signal passes.
    Bypass,
    Loading(u32, PendingLoad),
    Ready(u32, Box<PluginLoaded>),
    /// The kept instance of this plugin; it may have to come back from another chain first.
    Held(u32),
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
    /// The instance of every plugin slot of the settings that loaded.
    held: Vec<Active>,
    /// The plugins in the chain handed over last: their processors are not here.
    in_chain: Vec<u32>,
    /// Instances of slots that left the settings; closed when their chain is gone.
    retiring: Vec<Active>,
    /// A new shape that waits for its plugins.
    pending: Option<Pending>,
    /// What the chain built last delays the signal by.
    latency_frames: usize,
}

impl std::fmt::Debug for ChainBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChainBuilder")
            .field("plugins", &self.held.len())
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
            held: Vec::new(),
            in_chain: Vec::new(),
            retiring: Vec::new(),
            pending: None,
            latency_frames: 0,
        }
    }

    /// What the chain built last delays the bass by, in ms (0 before the first).
    pub fn latency_ms(&self) -> f64 {
        self.latency_frames as f64 / self.rate * MS_PER_S
    }

    /// Closes the instances of removed slots whose chain is gone. Call after dropping retired
    /// chains.
    pub fn reap(&mut self) {
        self.retiring.retain(|a| Arc::strong_count(&a.parked) > 1);
    }

    /// Instances of removed slots still held for chains that have not come back yet.
    pub fn retiring(&self) -> usize {
        self.retiring.len()
    }

    /// Whether a new shape still waits (for plugins to load, or for a chain to come back with a
    /// processor); `poll` finishes it.
    pub fn is_loading(&self) -> bool {
        self.pending.is_some()
    }

    /// A new chain at once when effects, their order or on/off differ from the last one and it
    /// has no plugin to load; with new plugins `None`, and `poll` brings it once they are
    /// loaded (never `fit`, so whether it comes now or later does not depend on how fast they
    /// load). Parameter changes of plugins go to their instances.
    pub fn fit(&mut self, spec: &ChainSpec, bank: &PluginBank) -> Option<Box<Chain>> {
        self.reap();
        self.prune(bank);
        if let Some(pending) = self.pending.as_mut() {
            if pending.spec.same_shape(spec) {
                pending.bank = bank.clone();
                // The running chain follows the sliders while the new one loads.
                self.send_params(bank);
                return None;
            }
            // Changed again: the loads end, their instances close on the plugin thread.
            self.pending = None;
        }
        if self.shape.same_shape(spec) {
            self.send_params(bank);
            return None;
        }
        let pending = self.start(spec, bank);
        let loads = pending.slots.iter().any(|w| matches!(w, Wait::Loading(..)));
        self.pending = Some(pending);
        if loads {
            return None;
        }
        self.poll()
    }

    /// The next chain of a pending shape, if it is ready: the final one once every plugin has
    /// answered and every kept one is back; before that, when a kept plugin still runs in the
    /// chain handed over last, a chain without plugins that frees it. Not blocking; call it
    /// from the control thread's tick (it needs the retired chains dropped, see `reap`). A
    /// plugin that failed leaves its slot out and notes why.
    pub fn poll(&mut self) -> Option<Box<Chain>> {
        self.reap();
        let mut pending = self.pending.take()?;
        if self.answered(&mut pending) {
            if self.holds_wanted(&pending) {
                let chain = self.gap_chain(&pending.spec);
                self.pending = Some(pending);
                return Some(chain);
            }
            if self.all_parked(&pending) {
                return Some(self.build(pending));
            }
        }
        self.pending = Some(pending);
        None
    }

    /// Forgets the instances of plugins that left the bank: they close once their chain is
    /// gone.
    fn prune(&mut self, bank: &PluginBank) {
        let (kept, gone): (Vec<Active>, Vec<Active>) = std::mem::take(&mut self.held)
            .into_iter()
            .partition(|a| bank.find(a.id).is_some());
        self.held = kept;
        self.in_chain
            .retain(|id| self.held.iter().any(|a| a.id == *id));
        self.retiring.extend(gone);
    }

    /// Sends the loads of the plugin slots of `spec` that have no instance yet.
    fn start(&self, spec: &ChainSpec, bank: &PluginBank) -> Pending {
        let mut slots = Vec::new();
        for slot in spec.slots() {
            if !(slot.on && slot.kind == EffectKind::Plugin) {
                slots.push(Wait::Bypass);
                continue;
            }
            if self.held.iter().any(|a| a.id == slot.plugin) {
                slots.push(Wait::Held(slot.plugin));
                continue;
            }
            slots.push(self.load(slot.plugin, bank));
        }
        Pending {
            spec: *spec,
            bank: bank.clone(),
            slots,
        }
    }

    /// Sends the load of the plugin `id` from the bank's saved state and values.
    fn load(&self, id: u32, bank: &PluginBank) -> Wait {
        let setup = bank
            .find(id)
            .ok_or_else(|| "not in the plugin list".to_string())
            .and_then(|spec| setup_of(spec, self.rate, bank.crash_marker.clone()));
        match setup {
            Ok(setup) => Wait::Loading(id, PluginHost::global().load(setup)),
            Err(reason) => {
                bank.problems.note(id, reason);
                Wait::Bypass
            }
        }
    }

    /// Collects the loads that finished; a kept instance that was lost loads again. True when
    /// nothing loads any more.
    fn answered(&mut self, pending: &mut Pending) -> bool {
        let problems = pending.bank.problems.clone();
        let mut waiting = false;
        for wait in &mut pending.slots {
            if let Wait::Held(id) = wait {
                let id = *id;
                let place = self.held.iter().position(|a| a.id == id);
                *wait = match place.map(|i| (i, self.held[i].place())) {
                    Some((i, Place::Lost)) => {
                        self.held.remove(i);
                        self.load(id, &pending.bank)
                    }
                    Some(_) => continue,
                    None => {
                        problems.note(id, "not in the plugin list".to_string());
                        Wait::Bypass
                    }
                };
            }
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
        !waiting
    }

    /// Whether the chain handed over last runs a plugin the pending shape wants.
    fn holds_wanted(&self, pending: &Pending) -> bool {
        pending
            .slots
            .iter()
            .any(|w| matches!(w, Wait::Held(id) if self.in_chain.contains(id)))
    }

    /// Whether every kept plugin of the pending shape has its processor back.
    fn all_parked(&self, pending: &Pending) -> bool {
        pending.slots.iter().all(|w| match w {
            Wait::Held(id) => self
                .held
                .iter()
                .any(|a| a.id == *id && a.place() == Place::Parked),
            _ => true,
        })
    }

    /// The shape of `spec` without its plugins: it frees their processors. It becomes the
    /// shape the builder knows, so a request for the one before it still builds.
    fn gap_chain(&mut self, spec: &ChainSpec) -> Box<Chain> {
        let mut bare = *spec;
        for slot in bare.slots_mut() {
            slot.on &= slot.kind != EffectKind::Plugin;
        }
        self.shape = bare;
        self.in_chain.clear();
        let chain = Chain::new(&bare, self.rate);
        self.latency_frames = chain.latency_frames();
        Box::new(chain)
    }

    /// The chain of a pending shape whose plugins are all answered and back.
    fn build(&mut self, pending: Pending) -> Box<Chain> {
        self.shape = pending.spec;
        self.in_chain.clear();

        let mut inserts: Vec<Option<Box<dyn Insert>>> = Vec::new();
        for wait in pending.slots {
            let insert = match wait {
                Wait::Ready(id, loaded) => Some(self.adopt(id, *loaded)),
                Wait::Held(id) => self.held.iter().find(|a| a.id == id).and_then(|a| {
                    self.in_chain.push(id);
                    a.take_insert()
                }),
                Wait::Bypass | Wait::Loading(..) => None,
            };
            inserts.push(insert);
        }
        let chain = Chain::with_plugins(&pending.spec, self.rate, &mut |at| {
            inserts.get_mut(at)?.take()
        });
        self.latency_frames = chain.latency_frames();
        // Values may have moved while the plugins loaded.
        self.send_params(&pending.bank);
        Box::new(chain)
    }

    /// Takes a loaded plugin as the instance of its slot; returns the insert that runs it.
    fn adopt(&mut self, id: u32, loaded: PluginLoaded) -> Box<dyn Insert> {
        let PluginLoaded {
            processor,
            handle,
            latency_samples,
            applied,
        } = loaded;
        let parked = Parked::default();
        let insert = PluginInsert::new(processor, latency_samples, Arc::clone(&parked));
        self.held.push(Active {
            parked,
            id,
            handle,
            sent: applied,
            latency_samples,
        });
        self.in_chain.push(id);
        Box::new(insert)
    }

    /// Changes the plugins made themselves since the last call, as (`PluginSpec::id`, parameter
    /// id, value in the plugin's units).
    pub fn take_param_changes(&mut self) -> Vec<(u32, u32, f64)> {
        let mut all = Vec::new();
        for active in &mut self.held {
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

    /// Sends the bank's values to the plugins that run in the chain. The parameter ring of a
    /// plugin is only drained while it processes: for the others the values wait (`sent` stays
    /// as it is) and go out when their chain is built.
    fn send_params(&mut self, bank: &PluginBank) {
        for active in &mut self.held {
            if !self.in_chain.contains(&active.id) {
                continue;
            }
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
    let Some(format) = PluginFormat::from_key(&spec.format) else {
        return Err(format!("unknown format {}", spec.format));
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
