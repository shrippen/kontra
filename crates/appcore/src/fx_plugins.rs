//! Plugins for the effect chain (LH-EIN-20, LH-EIN-21): finding them, what the mixer lists of
//! them, and what the engine gets.
//!
//! ```text
//! "Plugins suchen" ─► thread ─► one `kontra --probe-plugin <file>` process per file ─► report
//!                                                      (crash or hang: file marked broken)
//! report ─► plugins.json (data folder) ─► "Effekt hinzufügen" lists the plugins found
//! add    ─► describe_plugin (loads it once: parameters, state) ─► Settings.plugins + a slot
//! Settings.plugins ─► PluginBank in the Gains ─► engine builds the instances
//! ```
//!
//! The probe is this program itself: packages ship only `kontra`, and `main` answers
//! `--probe-plugin <file>` before Qt starts (`probe_plugin_line`).

use crate::store::{self, Settings};
use kontra_model::{EffectKind, ParamSpec, PluginSpec};
use kontra_session::plugins::{clear_crash_marker, crashed_plugin, settle_plugins};
use kontra_session::plugins::{
    plugin_files, probe_line, scan_files, PluginFormat, PluginInfo, ProbeCommand, ScanReport,
};
use kontra_session::{PluginBank, PluginProblems};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

/// The argument that makes `kontra` probe one plugin file and print the answer.
pub const PROBE_FLAG: &str = "--probe-plugin";

/// A plugin file that does not answer within this is broken.
const PROBE_TIMEOUT: Duration = Duration::from_secs(10);

/// The last scan, in the data folder.
const CACHE_FILE: &str = "plugins.json";

/// Left in the data folder while a plugin's code runs (loading, describing): if it is still
/// there at the next start, Kontra died inside that plugin (LH-EIN-20).
const CRASH_MARKER_FILE: &str = "plugin-loading.marker";
/// How long Kontra waits at its end for loads to finish and plugins to close.
const SETTLE_PATIENCE: Duration = Duration::from_secs(2);

const FORMATS: [PluginFormat; 2] = [PluginFormat::Clap, PluginFormat::Vst3];

/// The body of the probe process: what `PROBE_FLAG` prints for `path` (one line of JSON).
pub fn probe_plugin_line(path: &Path) -> String {
    probe_line(path)
}

/// What the mixer knows about plugins: the last scan, a scan that runs, and why slots are
/// bypassed.
#[derive(Debug, Default)]
pub(crate) struct Plugins {
    /// Why plugin slots sound dry; filled by the engine, shown in the mixer.
    pub(crate) problems: PluginProblems,
    /// How often `problems` had changed when the page last looked.
    seen_problems: u64,
    /// Plugins (by `PluginSpec::id`) that took Kontra down while loading and were switched off.
    pub(crate) crashed: BTreeSet<u32>,
    crash_marker: PathBuf,
    report: ScanReport,
    running: Option<mpsc::Receiver<ScanReport>>,
    cache: PathBuf,
    /// The program that probes a file; `None`: this program.
    probe: Option<ProbeCommand>,
    /// The chain latency the page shows, in tenths of a ms; `None` while nothing plays.
    shown_latency: Option<i64>,
}

// Kontra ends (Core drops `plugins` after its players): let the plugin thread finish a load, so
// no crash marker stays, and close the plugins before the program goes.
impl Drop for Plugins {
    fn drop(&mut self) {
        let _ = settle_plugins(SETTLE_PATIENCE);
    }
}

impl Plugins {
    /// With the last scan of `data_dir`, if any.
    pub(crate) fn new(data_dir: &Path) -> Plugins {
        let cache = data_dir.join(CACHE_FILE);
        Plugins {
            problems: PluginProblems::default(),
            seen_problems: 0,
            crashed: BTreeSet::new(),
            crash_marker: data_dir.join(CRASH_MARKER_FILE),
            report: store::load(&cache),
            running: None,
            cache,
            probe: None,
            shown_latency: None,
        }
    }

    /// As `new`, and if the last run died while a plugin loaded (a marker is left), switches
    /// that plugin's slots off in `settings` and saves them, so it stays off until the player
    /// switches it on.
    pub(crate) fn start(data_dir: &Path, settings: &mut Settings) -> Plugins {
        let mut plugins = Plugins::new(data_dir);
        let Some((path, plugin)) = crashed_plugin(&plugins.crash_marker) else {
            return plugins;
        };
        clear_crash_marker(&plugins.crash_marker);

        let ids: Vec<u32> = settings
            .plugins
            .iter()
            .filter(|p| p.path == path && p.plugin == plugin)
            .map(|p| p.id)
            .collect();
        for slot in settings.monitor_chain.slots_mut() {
            if slot.kind != EffectKind::Plugin || !ids.contains(&slot.plugin) {
                continue;
            }
            slot.on = false;
            plugins.crashed.insert(slot.plugin);
        }
        if !plugins.crashed.is_empty() {
            let _ = store::save(&data_dir.join("settings.json"), settings);
        }
        plugins
    }

    /// Whether the chain latency changed in the digit the page shows (0.1 ms), or appeared or
    /// went away; remembers it.
    pub(crate) fn latency_moved(&mut self, latency_ms: Option<f64>) -> bool {
        // Below a tenth the page shows no line, like nothing playing.
        let tenths = latency_ms
            .map(|ms| (ms * TENTHS_PER_MS).round() as i64)
            .filter(|t| *t > 0);
        let moved = tenths != self.shown_latency;
        self.shown_latency = tenths;
        moved
    }

    /// The plugins of the settings for the engine, which loads them with a crash marker.
    pub(crate) fn bank(&self, settings: &Settings) -> PluginBank {
        PluginBank::new(settings.plugins.clone(), self.problems.clone())
            .with_crash_marker(self.crash_marker.clone())
    }

    pub(crate) fn crash_marker(&self) -> &Path {
        &self.crash_marker
    }

    /// Whether the reasons for bypassed slots changed since the last call (a plugin finished
    /// loading, or failed to).
    pub(crate) fn problems_moved(&mut self) -> bool {
        let now = self.problems.revision();
        let moved = now != self.seen_problems;
        self.seen_problems = now;
        moved
    }

    pub(crate) fn report(&self) -> &ScanReport {
        &self.report
    }

    pub(crate) fn is_scanning(&self) -> bool {
        self.running.is_some()
    }

    /// Starts a scan in the background; false if one runs already.
    pub(crate) fn scan(&mut self) -> bool {
        if self.running.is_some() {
            return false;
        }
        let probe = self.probe.clone().or_else(own_probe);
        let (tx, rx) = mpsc::channel();
        self.running = Some(rx);
        std::thread::spawn(move || {
            let mut report = ScanReport::default();
            let Some(probe) = probe else {
                let _ = tx.send(report);
                return;
            };
            for format in FORMATS {
                let mut part = scan_files(&plugin_files(format), &probe, PROBE_TIMEOUT);
                report.plugins.append(&mut part.plugins);
                report.broken.append(&mut part.broken);
            }
            let _ = tx.send(report);
        });
        true
    }

    /// True once when a scan has finished; its result is kept and saved.
    pub(crate) fn poll(&mut self) -> bool {
        let Some(running) = self.running.as_ref() else {
            return false;
        };
        let done = match running.try_recv() {
            Ok(report) => Some(report),
            Err(mpsc::TryRecvError::Empty) => return false,
            Err(mpsc::TryRecvError::Disconnected) => Some(ScanReport::default()),
        };
        self.running = None;
        if let Some(report) = done {
            let _ = store::save(&self.cache, &report);
            self.report = report;
        }
        true
    }

    /// Uses `report` as the result of a scan (a test, or a scan made elsewhere).
    #[cfg(test)]
    pub(crate) fn set_report(&mut self, report: ScanReport) {
        self.report = report;
    }

    #[cfg(test)]
    pub(crate) fn set_probe(&mut self, probe: ProbeCommand) {
        self.probe = Some(probe);
    }
}

/// This program as the probe; `None` if it cannot find itself.
fn own_probe() -> Option<ProbeCommand> {
    Some(ProbeCommand {
        program: std::env::current_exe().ok()?,
        args: vec![PROBE_FLAG.into()],
    })
}

/// The plugin of a list entry `plugin:<n>` as the mixer offers it (`n` counts the found ones).
pub(crate) fn found_at<'a>(report: &'a ScanReport, key: &str) -> Option<&'a PluginInfo> {
    let n: usize = key.strip_prefix(PLUGIN_KEY)?.parse().ok()?;
    report.plugins.get(n)
}

/// The latency line shows one decimal.
const TENTHS_PER_MS: f64 = 10.0;

/// What the key of an addable plugin starts with, then its number in the scan.
pub(crate) const PLUGIN_KEY: &str = "plugin:";

/// The key of a parameter slider of a plugin slot (`p<id>`).
pub(crate) fn param_key(id: u32) -> String {
    format!("p{id}")
}

/// A new plugin id: one more than the largest.
pub(crate) fn next_id(plugins: &[PluginSpec]) -> u32 {
    plugins.iter().map(|p| p.id).max().unwrap_or(0) + 1
}

/// A plugin value to read: up to two decimals, no trailing zeros, the language's decimal mark
/// (2.0 → "2", 1.5 → "1,5" in German).
pub(crate) fn shown_value(value: f64, mark: &str) -> String {
    let text = format!("{value:.2}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    // A value just below zero rounds to "-0".
    let text = if text == "-0" { "0" } else { text };
    text.replace('.', mark)
}

/// The parameter named by a slider key, if the plugin has it.
pub(crate) fn param_mut<'a>(spec: &'a mut PluginSpec, key: &str) -> Option<&'a mut ParamSpec> {
    let id: u32 = key.strip_prefix('p')?.parse().ok()?;
    spec.params.iter_mut().find(|p| p.id == id)
}

/// Where a slider at `unit` (0..=1) puts a parameter.
pub(crate) fn from_unit(param: &ParamSpec, unit: f64) -> f64 {
    param.min + unit.clamp(0.0, 1.0) * (param.max - param.min)
}

/// Where a parameter stands on its slider, 0..=1.
pub(crate) fn to_unit(param: &ParamSpec) -> f64 {
    let span = param.max - param.min;
    if span <= 0.0 {
        return 0.0;
    }
    ((param.value - param.min) / span).clamp(0.0, 1.0)
}

impl crate::Core {
    /// Once when a plugin search has finished, the chain's latency changed (the standby built
    /// its chain, a run started or ended, the output went away) or a plugin changed a parameter
    /// by itself.
    pub(crate) fn poll_effects(&mut self) -> Option<crate::UiEvent> {
        let scanned = self.plugins.poll();
        let latency = self.chain_latency_ms();
        let moved = self.plugins.latency_moved(latency);
        let changes = self.plugin_changes();
        let edited = self.take_plugin_changes(&changes);
        let reasons = self.plugins.problems_moved();
        (scanned || moved || edited || reasons).then_some(crate::UiEvent::EffectsChanged)
    }

    /// Writes what plugins changed by themselves (their own editor) into the settings and saves
    /// them, so the values survive a restart. No remix: the engine's instance already has the
    /// value (it made it), and it does not get it back. True if a value differs from the
    /// settings.
    pub(crate) fn take_plugin_changes(&mut self, changes: &[(u32, u32, f64)]) -> bool {
        let mut changed = false;
        for &(plugin, param, value) in changes {
            let spec = self.settings.plugins.iter_mut().find(|p| p.id == plugin);
            let Some(param) = spec.and_then(|s| s.params.iter_mut().find(|p| p.id == param)) else {
                continue;
            };
            if param.value == value {
                continue;
            }
            param.value = value;
            changed = true;
        }
        if changed {
            let _ = store::save(&self.data_dir.join("settings.json"), &self.settings);
        }
        changed
    }
}

#[cfg(test)]
mod shown {
    use super::shown_value;

    /// Review #265: the latency line stayed empty after the standby built its chain, and kept
    /// an old figure after the output went away.
    #[test]
    fn a_new_latency_is_news_once() {
        let dir = std::env::temp_dir().join(format!("kontra-fx-lat-{}", std::process::id()));
        let mut plugins = super::Plugins::new(&dir);
        assert!(
            !plugins.latency_moved(None),
            "nothing plays, as at the start"
        );
        assert!(
            plugins.latency_moved(Some(2.67)),
            "the standby built its chain"
        );
        assert!(!plugins.latency_moved(Some(2.67)));
        assert!(
            !plugins.latency_moved(Some(2.671)),
            "the same to the shown digit"
        );
        assert!(plugins.latency_moved(None), "the output went away");
        assert!(
            !plugins.latency_moved(Some(0.0)),
            "an empty chain shows no line either"
        );
    }

    /// Review #264: a value just below zero read "-0".
    #[test]
    fn no_negative_zero() {
        assert_eq!(shown_value(-0.001, ","), "0");
        assert_eq!(shown_value(-1.5, ","), "-1,5");
        assert_eq!(shown_value(10.0, "."), "10");
    }
}
