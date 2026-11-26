//! `kontra-cli plugins`: find, inspect and run CLAP plugins (FX2).

use crate::args::{PluginsArgs, PluginsCommand};
use crate::CliResult;
use kontra_session::plugin_host::{self, PluginFormat, ProbeCommand, ScanReport};
use std::path::Path;
use std::time::Duration;

const FORMATS: [PluginFormat; 2] = [PluginFormat::Clap, PluginFormat::Vst3];

pub fn run(a: &PluginsArgs) -> CliResult {
    match &a.command {
        PluginsCommand::Scan { json, timeout_s } => {
            let listing = if *json { Listing::Json } else { Listing::Text };
            scan(listing, *timeout_s)
        }
        PluginsCommand::Probe { path } => {
            println!("{}", plugin_host::probe_answer(path));
            Ok(())
        }
        PluginsCommand::Params { path, id } => params(path, id.as_deref()),
        PluginsCommand::Run {
            path,
            input,
            output,
            id,
            param,
        } => render(path, id.as_deref(), input, output, param),
    }
}

fn format_of(path: &Path) -> Result<PluginFormat, String> {
    PluginFormat::of_file(path).ok_or_else(|| "not a plugin file (.clap, .vst3)".to_string())
}

/// How `scan` prints what it found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Listing {
    Text,
    Json,
}

/// Every probe is a process of this program, so a crashing plugin ends only its probe.
fn scan(listing: Listing, timeout_s: f32) -> CliResult {
    let program = std::env::current_exe().map_err(|e| format!("cannot find myself: {e}"))?;
    let probe = ProbeCommand {
        program,
        args: vec!["plugins".into(), "probe".into()],
    };
    let timeout = Duration::try_from_secs_f32(timeout_s)
        .ok()
        .filter(|t| !t.is_zero())
        .ok_or_else(|| {
            format!("--timeout-s must be a positive number of seconds, not {timeout_s}")
        })?;

    let mut report = ScanReport {
        categories_read: true,
        ..ScanReport::default()
    };
    let mut searched = Vec::new();
    for format in FORMATS {
        searched.extend(plugin_host::search_dirs(format));
        let mut part = plugin_host::scan(&plugin_host::installed(format), &probe, timeout);
        report.plugins.append(&mut part.plugins);
        report.broken.append(&mut part.broken);
    }

    if listing == Listing::Json {
        let text = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
        println!("{text}");
        return Ok(());
    }
    print_report(&report, &searched);
    Ok(())
}

fn print_report(report: &ScanReport, searched: &[std::path::PathBuf]) {
    for p in &report.plugins {
        println!(
            "{}  {} — {}  [{}]  in {}  out {}",
            p.format,
            p.name,
            p.vendor,
            p.id,
            channels(&p.ports.inputs),
            channels(&p.ports.outputs),
        );
        println!("    {}", p.path.display());
    }
    for b in &report.broken {
        println!("{}  BROKEN {} — {}", b.format, b.path.display(), b.reason);
    }
    println!(
        "{} plugins, {} broken file(s); searched: {}",
        report.plugins.len(),
        report.broken.len(),
        searched
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
}

fn channels(ports: &[u32]) -> String {
    if ports.is_empty() {
        return "-".into();
    }
    ports
        .iter()
        .map(|c| format!("{c}ch"))
        .collect::<Vec<_>>()
        .join("+")
}

fn params(path: &Path, id: Option<&str>) -> CliResult {
    let plugin = plugin_host::inspect(format_of(path)?, path, id).map_err(|e| e.to_string())?;
    let info = &plugin.info;
    println!("{} — {}  [{}]", info.name, info.vendor, info.id);
    println!("latency: {} samples", plugin.latency_samples);
    for p in &plugin.params {
        println!(
            "{:>6}  {}  min {}  max {}  default {}  current {}",
            p.id, p.name, p.min, p.max, p.default, p.current
        );
    }
    Ok(())
}

fn render(
    path: &Path,
    id: Option<&str>,
    input: &Path,
    output: &Path,
    params: &[String],
) -> CliResult {
    let mut parsed = Vec::new();
    for text in params {
        parsed.push(parse_param(text)?);
    }
    let latency_samples =
        plugin_host::render_file(format_of(path)?, path, id, input, output, &parsed)
            .map_err(|e| e.to_string())?;
    println!(
        "written: {} (latency {latency_samples} samples, not compensated)",
        output.display()
    );
    Ok(())
}

/// `id=value`, e.g. `7=0.5`.
fn parse_param(text: &str) -> Result<(u32, f64), String> {
    let bad = || format!("--param {text}: expected id=value, e.g. 7=0.5");
    let (id, value) = text.split_once('=').ok_or_else(bad)?;
    Ok((
        id.trim().parse().map_err(|_| bad())?,
        value.trim().parse().map_err(|_| bad())?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn param_syntax() {
        assert_eq!(parse_param("7=0.5").unwrap(), (7, 0.5));
        assert_eq!(parse_param(" 3 = -2 ").unwrap(), (3, -2.0));
        assert!(parse_param("x=1").is_err());
        assert!(parse_param("7").is_err());
    }
}
