//! The plugin host as the layers above see it: find, probe and scan plugin files, and the
//! crash marker of a probe. The driver's host is reached only through these functions.
//!
//! The data types are the driver's own (`PluginInfo`, `PluginFormat`, …): plain values the
//! app shows and stores, so they pass through unchanged.

use kontra_engine::plugins as host;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub use kontra_engine::plugins::{
    LoadedPlugin, PluginError, PluginFormat, PluginInfo, ProbeCommand, ScanReport,
};

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
    host::probe_file(format, path)
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
