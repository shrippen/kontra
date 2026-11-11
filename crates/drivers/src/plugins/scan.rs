//! Scanning with crash isolation. Loading a plugin file runs foreign code, which can crash or
//! hang. So the scan never loads a file itself: it starts a probe process per file
//! (`kontra-cli plugins probe <file>`), waits for its answer with a time limit and marks the file
//! broken when the process crashes, hangs or answers nonsense. The scan goes on with the next
//! file.
//!
//! ```text
//! scan_files ─► probe process ─► probe_file(file) ─► one JSON line on stdout
//!                  │ killed after the timeout, or crashed
//!                  └─► BrokenFile { path, reason }
//! ```

use super::{clap, vst3, PluginError, PluginFormat, PluginInfo};
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const POLL: Duration = Duration::from_millis(10);
/// After the probe ended, its output is read this long at most (a child it left behind may keep
/// the pipe open).
const READER_GRACE: Duration = Duration::from_secs(2);

/// A plugin file the scan could not read, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrokenFile {
    pub format: PluginFormat,
    pub path: PathBuf,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanReport {
    pub plugins: Vec<PluginInfo>,
    pub broken: Vec<BrokenFile>,
}

/// The program that probes one file: `program args… <file>`.
#[derive(Debug, Clone)]
pub struct ProbeCommand {
    pub program: PathBuf,
    pub args: Vec<OsString>,
}

/// What the probe process writes as its last line.
#[derive(Debug, Serialize, Deserialize)]
enum Answer {
    Plugins(Vec<PluginInfo>),
    Failed(String),
}

/// Loads a file in this process and lists its plugins. A plugin can crash this process.
pub fn probe_file(format: PluginFormat, path: &Path) -> Result<Vec<PluginInfo>, PluginError> {
    match format {
        PluginFormat::Clap => clap::probe(path),
        PluginFormat::Vst3 => vst3::probe(path),
    }
}

/// The body of the probe process: the answer for `path` as one line of JSON.
pub fn probe_line(path: &Path) -> String {
    let answer = match PluginFormat::of_file(path) {
        None => Answer::Failed("not a plugin file (.clap, .vst3)".into()),
        Some(format) => match probe_file(format, path) {
            Ok(plugins) => Answer::Plugins(plugins),
            Err(e) => Answer::Failed(e.to_string()),
        },
    };
    serde_json::to_string(&answer).unwrap_or_else(|e| format!("{{\"Failed\":\"{e}\"}}"))
}

/// Probes every file in its own process; each gets `timeout`.
pub fn scan_files(files: &[PathBuf], probe: &ProbeCommand, timeout: Duration) -> ScanReport {
    let mut report = ScanReport::default();
    for file in files {
        let Some(format) = PluginFormat::of_file(file) else {
            continue;
        };
        match run_probe(probe, file, timeout) {
            Ok(mut plugins) => report.plugins.append(&mut plugins),
            Err(reason) => report.broken.push(BrokenFile {
                format,
                path: file.clone(),
                reason,
            }),
        }
    }
    report
}

fn run_probe(
    probe: &ProbeCommand,
    file: &Path,
    timeout: Duration,
) -> Result<Vec<PluginInfo>, String> {
    let mut child = Command::new(&probe.program)
        .args(&probe.args)
        .arg(file)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("cannot start {}: {e}", probe.program.display()))?;

    // Read on a thread so a full pipe never blocks the probe.
    let mut stdout = child.stdout.take();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut text = String::new();
        if let Some(out) = stdout.as_mut() {
            let mut bytes = Vec::new();
            let _ = out.read_to_end(&mut bytes);
            text = String::from_utf8_lossy(&bytes).into_owned();
        }
        let _ = tx.send(text);
    });

    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(e) => return Err(format!("cannot wait for the probe: {e}")),
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("no answer within {} s", timeout.as_secs_f32()));
        }
        std::thread::sleep(POLL);
    };

    if !status.success() {
        return Err(format!("crashed ({})", describe(&status)));
    }
    let text = rx.recv_timeout(READER_GRACE).unwrap_or_default();
    parse(&text)
}

/// Plugins may print on stdout; the answer is the last line.
fn parse(text: &str) -> Result<Vec<PluginInfo>, String> {
    let line = text.lines().rev().find(|l| !l.trim().is_empty());
    let Some(line) = line else {
        return Err("no answer".into());
    };
    match serde_json::from_str::<Answer>(line) {
        Ok(Answer::Plugins(plugins)) => Ok(plugins),
        Ok(Answer::Failed(reason)) => Err(reason),
        Err(_) => Err("unreadable answer".into()),
    }
}

#[cfg(unix)]
fn describe(status: &std::process::ExitStatus) -> String {
    use std::os::unix::process::ExitStatusExt;
    match status.signal() {
        Some(signal) => format!("signal {signal}"),
        None => status.to_string(),
    }
}

#[cfg(not(unix))]
fn describe(status: &std::process::ExitStatus) -> String {
    status.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answer_is_the_last_line() {
        let ok = serde_json::to_string(&Answer::Plugins(Vec::new())).unwrap();
        let text = format!("the plugin says hello\n{ok}\n");
        assert_eq!(parse(&text).unwrap(), Vec::new());
    }

    #[test]
    fn failure_and_silence_are_reasons() {
        let failed = serde_json::to_string(&Answer::Failed("bad".into())).unwrap();
        assert_eq!(parse(&failed).unwrap_err(), "bad");
        assert_eq!(parse("").unwrap_err(), "no answer");
        assert_eq!(parse("hello").unwrap_err(), "unreadable answer");
    }
}
