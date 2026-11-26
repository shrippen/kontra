//! What the layers above hand the audio side to mix and shape the sound: bus volumes, tone
//! shaping and the plugins of the effect chain. Each type owns its engine counterpart.

use kontra_engine as engine;
use kontra_model::{PluginInfo, PluginSpec, Shaping};
use std::path::{Path, PathBuf};

/// Why plugin slots are bypassed, by `PluginSpec::id`. Shared between the audio side that
/// builds chains and the page that shows the reasons.
#[derive(Debug, Clone, Default)]
pub struct PluginProblems(engine::PluginProblems);

impl PluginProblems {
    pub fn get(&self, id: u32) -> Option<String> {
        self.0.get(id)
    }

    /// Notes why the plugin `id` could not be loaded; the audio side does, a test may.
    pub fn note(&self, id: u32, reason: String) {
        self.0.note(id, reason);
    }

    /// Forgets the reason for `id`: it loaded, or its id goes to a new plugin.
    pub fn clear(&self, id: u32) {
        self.0.clear(id);
    }

    /// Counts the changes of the list: a page that shows it knows when to look again.
    pub fn revision(&self) -> u64 {
        self.0.revision()
    }
}

/// The plugin instances every player shares: one per plugin slot of the settings.
#[derive(Debug, Clone, Default)]
pub struct PluginRack(engine::PluginRack);

/// The plugins of the settings with the rack their instances live in.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PluginBank(engine::PluginBank);

impl PluginBank {
    pub fn new(specs: Vec<PluginSpec>, problems: PluginProblems) -> PluginBank {
        PluginBank(engine::PluginBank::new(specs, problems.0))
    }

    /// The instances of this bank's plugins live in `rack`: give every bank of the settings the
    /// same one, so that every player finds the instances already loaded.
    pub fn with_rack(self, rack: PluginRack) -> PluginBank {
        PluginBank(self.0.with_rack(rack.0))
    }

    /// Loads of this bank leave a marker at `path` while a plugin's code runs.
    pub fn with_crash_marker(self, path: PathBuf) -> PluginBank {
        PluginBank(self.0.with_crash_marker(path))
    }
}

/// A `describe_plugin_later` in progress.
#[derive(Debug)]
pub struct PendingSpec(engine::PendingSpec);

impl PendingSpec {
    /// The spec once the plugin is described, `None` before. Not blocking.
    pub fn poll(&self) -> Option<Result<PluginSpec, crate::plugin_host::PluginError>> {
        Some(self.0.poll()?.map_err(Into::into))
    }

    /// The plugin being described.
    pub fn info(&self) -> &PluginInfo {
        self.0.info()
    }
}

/// Loads a plugin to find out what a slot of it needs (parameters, start state), without
/// waiting for the plugin thread: the spec comes through `PendingSpec::poll`.
pub fn describe_plugin_later(info: &PluginInfo, crash_marker: Option<&Path>) -> PendingSpec {
    PendingSpec(engine::describe_plugin_later(info, crash_marker))
}

/// Bus volumes, 0..=1, with the tone shaping and plugins of the monitored bass.
#[derive(Debug, Clone, PartialEq)]
pub struct Gains {
    pub click: f32,
    pub tone: f32,
    pub backing: f32,
    /// Synthesised drum accompaniment (LH-LRN-01).
    pub drums: f32,
    /// The original bass mixed back into a backing track without it (LH-BIB-09); 0 = removed.
    pub bass: f32,
    /// The bass input played back (input monitoring); 0 = off.
    pub monitor: f32,
    /// Tone shaping of the monitored bass (LH-EIN-08); detection hears the dry input.
    pub shaping: Shaping,
    /// The plugins the chain of `shaping` names (LH-EIN-20, -21).
    pub plugins: Option<PluginBank>,
}

impl Default for Gains {
    fn default() -> Self {
        engine::Gains::default().into()
    }
}

impl From<engine::Gains> for Gains {
    fn from(gains: engine::Gains) -> Gains {
        Gains {
            click: gains.click,
            tone: gains.tone,
            backing: gains.backing,
            drums: gains.drums,
            bass: gains.bass,
            monitor: gains.monitor,
            shaping: gains.shaping,
            plugins: gains.plugins.map(PluginBank),
        }
    }
}

impl From<Gains> for engine::Gains {
    fn from(gains: Gains) -> engine::Gains {
        engine::Gains {
            click: gains.click,
            tone: gains.tone,
            backing: gains.backing,
            drums: gains.drums,
            bass: gains.bass,
            monitor: gains.monitor,
            shaping: gains.shaping,
            plugins: gains.plugins.map(|bank| bank.0),
        }
    }
}
