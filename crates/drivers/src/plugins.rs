//! Audio plugins as effects (LH-EIN-20, LH-EIN-21; FX2, FX3): find them, load them, run audio
//! through them. CLAP and VST3 sit behind the same types.
//!
//! ```text
//!  main thread                                      audio thread
//!  LoadedPlugin ── set_param ─► rtrb ring ────────► PluginProcessor::process(in, out)
//!       │  params, state, latency                        no allocation, no lock
//!       └─ activate(rate, max block) ──────────────────► PluginProcessor
//!
//!  scan:  kontra-cli plugins scan ─► one `plugins probe <file>` process per file ─► JSON line
//!         (a crash or hang of a plugin ends only its probe; the file is marked broken)
//! ```
//!
//! Audio is stereo at this API. A plugin with a mono port gets the mean of both channels and
//! its output goes to both; so the mono bass is passed on both channels and comes out on both.
//! Only the first input and the first output port of a plugin are used (no side chain).

mod clap;
mod host;
mod paths;
mod scan;
#[allow(unsafe_code)]
mod vst3;

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub use host::{
    clear_crash_marker, crashed_plugin, settle_plugins, PendingLoad, PluginDescription,
    PluginHandle, PluginHost, PluginLoaded, PluginSetup,
};
pub use paths::{plugin_files, search_paths};
pub use scan::{probe_file, probe_line, scan_files, BrokenFile, ProbeCommand, ScanReport};

/// Channels of the audio at the API.
pub const CHANNELS: usize = 2;
/// Parameter changes that fit in the ring between two blocks.
const PARAM_RING_SLOTS: usize = 256;
/// One entry per id, in the order the ids first appear; the last value of an id wins.
fn coalesce(changes: impl Iterator<Item = (u32, f64)>) -> Vec<(u32, f64)> {
    let mut merged: Vec<(u32, f64)> = Vec::new();
    for (id, value) in changes {
        match merged.iter_mut().find(|(known, _)| *known == id) {
            Some(entry) => entry.1 = value,
            None => merged.push((id, value)),
        }
    }
    merged
}

/// Block size of the offline render.
const RENDER_BLOCK: u32 = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginFormat {
    Clap,
    Vst3,
}

impl PluginFormat {
    /// The format a file belongs to, by its extension.
    pub fn of_file(path: &Path) -> Option<PluginFormat> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "clap" => Some(PluginFormat::Clap),
            "vst3" => Some(PluginFormat::Vst3),
            _ => None,
        }
    }

    /// The name saved in chains and settings ("clap", "vst3"); never shown.
    pub fn key(self) -> &'static str {
        match self {
            PluginFormat::Clap => "clap",
            PluginFormat::Vst3 => "vst3",
        }
    }

    /// The format a saved [`PluginFormat::key`] names.
    pub fn from_key(key: &str) -> Option<PluginFormat> {
        [PluginFormat::Clap, PluginFormat::Vst3]
            .into_iter()
            .find(|f| f.key() == key)
    }
}

/// The name shown to the player. "VST 3" is spelled as Steinberg's trademark rules ask.
impl fmt::Display for PluginFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginFormat::Clap => f.write_str("CLAP"),
            PluginFormat::Vst3 => f.write_str("VST 3"),
        }
    }
}

/// Channels of every audio input and output port of a plugin.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortLayout {
    pub inputs: Vec<u32>,
    pub outputs: Vec<u32>,
}

/// One plugin as the scan found it. A file can hold several.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginInfo {
    pub format: PluginFormat,
    pub id: String,
    pub name: String,
    pub vendor: String,
    pub path: PathBuf,
    pub ports: PortLayout,
}

/// A parameter with its range and current value, in the plugin's own units.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParamInfo {
    pub id: u32,
    pub name: String,
    pub min: f64,
    pub max: f64,
    pub default: f64,
    pub current: f64,
}

#[derive(Debug, Error)]
pub enum PluginError {
    #[error("{path}: cannot load: {message}")]
    Load { path: PathBuf, message: String },
    #[error("{0} plugins are not supported yet")]
    Unsupported(PluginFormat),
    #[error("{path}: plugin {id} not found")]
    NotFound { path: PathBuf, id: String },
    #[error("{path} holds several plugins, pick one: {ids}")]
    Ambiguous { path: PathBuf, ids: String },
    #[error("{name}: not an effect (needs an audio input and an audio output)")]
    NotAnEffect { name: String },
    #[error("{name}: audio ports with more than {CHANNELS} channels are not supported")]
    TooManyChannels { name: String },
    #[error("plugin: {0}")]
    Plugin(String),
    #[error("no parameter {0}")]
    NoParam(u32),
    #[error("the parameter queue is full")]
    QueueFull,
    #[error("plugin has no state")]
    NoState,
    #[error("plugin state: {0}")]
    State(String),
    #[error("block of {0} frames is empty or too large")]
    Block(u32),
    #[error("the audio side is still in use")]
    InUse,
    #[error(transparent)]
    File(#[from] crate::FileError),
}

/// Why `PluginProcessor::process` failed. The output then holds the input (dry).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ProcessError {
    #[error("the plugin cannot start processing")]
    Start,
    #[error("the plugin failed to process")]
    Process,
}

/// A plugin on the main thread: parameters, state, latency. `activate` hands the audio side to
/// the audio thread.
pub struct LoadedPlugin {
    inner: Backend,
}

/// The format a plugin was loaded with.
enum Backend {
    Clap(clap::ClapPlugin),
    Vst3(vst3::Vst3Plugin),
}

/// The same call on whichever backend is behind `$backend`.
macro_rules! each {
    ($backend:expr, $p:ident => $call:expr) => {
        match $backend {
            Backend::Clap($p) => $call,
            Backend::Vst3($p) => $call,
        }
    };
}

impl fmt::Debug for LoadedPlugin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LoadedPlugin")
            .field("info", each!(&self.inner, p => p.info()))
            .finish_non_exhaustive()
    }
}

impl LoadedPlugin {
    /// Loads the plugin a scan found.
    pub fn load(info: &PluginInfo) -> Result<LoadedPlugin, PluginError> {
        LoadedPlugin::open(info.format, &info.path, Some(&info.id))
    }

    /// Loads a plugin from a file; without `id` the file must hold exactly one.
    pub fn open(
        format: PluginFormat,
        path: &Path,
        id: Option<&str>,
    ) -> Result<LoadedPlugin, PluginError> {
        let inner = match format {
            PluginFormat::Clap => Backend::Clap(clap::ClapPlugin::open(path, id)?),
            PluginFormat::Vst3 => Backend::Vst3(vst3::Vst3Plugin::open(path, id)?),
        };
        Ok(LoadedPlugin { inner })
    }

    pub fn info(&self) -> &PluginInfo {
        each!(&self.inner, p => p.info())
    }

    /// Parameters with their current values.
    pub fn params(&mut self) -> Vec<ParamInfo> {
        each!(&mut self.inner, p => p.params())
    }

    /// Sets a parameter, clamped to its range. While active the change goes through a ring
    /// to the audio thread and takes effect with the next block.
    pub fn set_param(&mut self, id: u32, value: f64) -> Result<(), PluginError> {
        each!(&mut self.inner, p => p.set_param(id, value))
    }

    /// Delay the plugin adds, in samples.
    pub fn latency_samples(&mut self) -> u32 {
        each!(&mut self.inner, p => p.latency_samples())
    }

    /// Parameter changes the plugin made itself since the last call (its own editor, a preset,
    /// an automation of its own), as (parameter id, value in the plugin's units): one entry per
    /// parameter, the last value. Not echoed: they are not sent back with `set_param`. Main thread.
    pub fn take_changes(&mut self) -> Vec<(u32, f64)> {
        each!(&mut self.inner, p => p.take_changes())
    }

    pub fn save_state(&mut self) -> Result<Vec<u8>, PluginError> {
        each!(&mut self.inner, p => p.save_state())
    }

    pub fn load_state(&mut self, bytes: &[u8]) -> Result<(), PluginError> {
        each!(&mut self.inner, p => p.load_state(bytes))
    }

    /// Prepares the plugin for blocks of up to `max_block` frames. Allocates; not for the audio
    /// thread. Drop the processor before `deactivate`.
    pub fn activate(
        &mut self,
        sample_rate: f64,
        max_block: u32,
    ) -> Result<PluginProcessor, PluginError> {
        let inner = match &mut self.inner {
            Backend::Clap(p) => Processor::Clap(p.activate(sample_rate, max_block)?),
            Backend::Vst3(p) => Processor::Vst3(p.activate(sample_rate, max_block)?),
        };
        Ok(PluginProcessor { inner })
    }

    /// Offline: runs a whole audio file through the plugin and writes a WAV of the same length
    /// (the plugin's latency is not compensated). A mono file goes to both channels. `params`
    /// are set after the activation, the way a live session sets them.
    pub fn render_file(
        &mut self,
        input: &Path,
        output: &Path,
        params: &[(u32, f64)],
    ) -> Result<(), PluginError> {
        let audio = crate::read_audio(input)?;
        let left = &audio.channels[0];
        let right = audio.channels.get(1).unwrap_or(left);
        let mut out = [vec![0.0; left.len()], vec![0.0; left.len()]];

        let mut processor = self.activate(f64::from(audio.sample_rate), RENDER_BLOCK)?;
        for &(id, value) in params {
            self.set_param(id, value)?;
        }
        let [out_l, out_r] = &mut out;
        let blocks = left
            .chunks(RENDER_BLOCK as usize)
            .zip(right.chunks(RENDER_BLOCK as usize))
            .zip(out_l.chunks_mut(RENDER_BLOCK as usize))
            .zip(out_r.chunks_mut(RENDER_BLOCK as usize));
        for (((l, r), o_l), o_r) in blocks {
            processor
                .process([l, r], [o_l, o_r])
                .map_err(|e| PluginError::Plugin(e.to_string()))?;
        }
        drop(processor);
        self.deactivate()?;

        let rendered = crate::AudioBuffer {
            sample_rate: audio.sample_rate,
            channels: out.into(),
        };
        crate::write_wav(output, &rendered)?;
        Ok(())
    }

    /// Ends the activation (fails while the processor still exists).
    pub fn deactivate(&mut self) -> Result<(), PluginError> {
        each!(&mut self.inner, p => p.deactivate())
    }
}

/// The audio side of an activated plugin. `Send`, for the audio thread.
pub struct PluginProcessor {
    inner: Processor,
}

enum Processor {
    Clap(clap::ClapProcessor),
    Vst3(vst3::Vst3Processor),
}

impl fmt::Debug for PluginProcessor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PluginProcessor").finish_non_exhaustive()
    }
}

impl PluginProcessor {
    /// Runs one block through the plugin. All slices are as long as the block (the shortest
    /// counts); blocks beyond `max_block` are cut into pieces. No allocation, no lock.
    pub fn process(
        &mut self,
        input: [&[f32]; CHANNELS],
        output: [&mut [f32]; CHANNELS],
    ) -> Result<(), ProcessError> {
        match &mut self.inner {
            Processor::Clap(p) => p.process(input, output),
            Processor::Vst3(p) => p.process(input, output),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn processor_moves_to_the_audio_thread() {
        fn is_send<T: Send>() {}
        is_send::<PluginProcessor>();
    }

    #[test]
    fn format_by_extension() {
        assert_eq!(
            PluginFormat::of_file(Path::new("/a/b.CLAP")),
            Some(PluginFormat::Clap)
        );
        assert_eq!(
            PluginFormat::of_file(Path::new("x.vst3")),
            Some(PluginFormat::Vst3)
        );
        assert_eq!(PluginFormat::of_file(Path::new("x.so")), None);
    }

    #[test]
    fn shown_names_follow_steinberg() {
        // Steinberg's rules: "VST 3" with a space; saved chains keep the key.
        assert_eq!(PluginFormat::Vst3.to_string(), "VST 3");
        assert_eq!(PluginFormat::Clap.to_string(), "CLAP");
        for format in [PluginFormat::Clap, PluginFormat::Vst3] {
            assert_eq!(PluginFormat::from_key(format.key()), Some(format));
            // The scan's JSON (serde) and saved chains (key) name a format alike.
            let json = serde_json::to_value(format).expect("format as JSON");
            assert_eq!(json, format.key());
        }
        assert_eq!(PluginFormat::Vst3.key(), "vst3");
        assert_eq!(PluginFormat::from_key("vst"), None);
    }
}
