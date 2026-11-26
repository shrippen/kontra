//! The plugin host as the layers above see it: find, probe and scan plugin files, inspect and
//! run one, and the crash marker of a probe. The driver's host is reached only through these
//! functions.
//!
//! The data types (`PluginInfo`, `PluginFormat`, …) are plain values in `kontra_model`.

use kontra_engine::plugins as host;
use std::path::{Path, PathBuf};
use std::time::Duration;
use thiserror::Error;

pub use kontra_model::{
    BrokenFile, ParamInfo, PluginFormat, PluginInfo, PortLayout, ProbeCommand, ScanReport,
};

/// Why a plugin call failed. The editor reasons the page tells apart have a variant each.
#[derive(Debug, Error)]
pub enum PluginError {
    #[error("editor: {0}")]
    Editor(String),
    #[error("the plugin has no editor of its own")]
    NoEditor,
    #[error("the plugin is not loaded now")]
    NotLoaded,
    #[error("the plugin thread is busy")]
    Busy,
    #[error("{0}")]
    Other(String),
}

impl From<host::PluginError> for PluginError {
    fn from(error: host::PluginError) -> PluginError {
        match error {
            host::PluginError::Editor(reason) => PluginError::Editor(reason),
            host::PluginError::NoEditor => PluginError::NoEditor,
            host::PluginError::NotLoaded => PluginError::NotLoaded,
            host::PluginError::Busy => PluginError::Busy,
            other => PluginError::Other(other.to_string()),
        }
    }
}

/// What a plugin file tells about itself once loaded.
#[derive(Debug, Clone)]
pub struct PluginReport {
    pub info: PluginInfo,
    pub latency_samples: u32,
    pub params: Vec<ParamInfo>,
}

/// The plugin files installed for `format`, found in the standard folders.
pub fn installed(format: PluginFormat) -> Vec<PathBuf> {
    host::plugin_files(format)
}

/// The folders `installed` looks in.
pub fn search_dirs(format: PluginFormat) -> Vec<PathBuf> {
    host::search_paths(format)
}

/// What a probe process prints for `path` (the answer `scan` reads).
pub fn probe_answer(path: &Path) -> String {
    host::probe_line(path)
}

/// Loads `path` in this process and lists its plugins; a plugin can crash this process.
pub fn probe_here(format: PluginFormat, path: &Path) -> Result<Vec<PluginInfo>, PluginError> {
    Ok(host::probe_file(format, path)?)
}

/// Loads one plugin of `path` (`id` picks it if the file holds several) and reads its details.
pub fn inspect(
    format: PluginFormat,
    path: &Path,
    id: Option<&str>,
) -> Result<PluginReport, PluginError> {
    let mut plugin = host::LoadedPlugin::open(format, path, id)?;
    Ok(PluginReport {
        info: plugin.info().clone(),
        latency_samples: plugin.latency_samples(),
        params: plugin.params(),
    })
}

/// Runs the audio file `input` through one plugin of `path` and writes the WAV `output`, with
/// `params` (id, value) set. Returns the plugin's latency in samples; it is not compensated.
pub fn render_file(
    format: PluginFormat,
    path: &Path,
    id: Option<&str>,
    input: &Path,
    output: &Path,
    params: &[(u32, f64)],
) -> Result<u32, PluginError> {
    let mut plugin = host::LoadedPlugin::open(format, path, id)?;
    plugin.render_file(input, output, params)?;
    Ok(plugin.latency_samples())
}

/// Probes each file in a process of its own, so a crashing plugin does not take this one down.
pub fn scan(files: &[PathBuf], probe: &ProbeCommand, timeout: Duration) -> ScanReport {
    host::scan_files(files, probe, timeout)
}

/// Waits up to `patience` for plugins still loading to finish; false if they did not.
pub fn settle(patience: Duration) -> bool {
    host::settle_plugins(patience)
}

/// The plugin file and name a previous run died in, if the marker at `marker` says so.
pub fn crashed(marker: &Path) -> Option<(PathBuf, String)> {
    host::crashed_plugin(marker)
}

/// Forgets the crash at `marker`.
pub fn clear_crash(marker: &Path) {
    host::clear_crash_marker(marker)
}
