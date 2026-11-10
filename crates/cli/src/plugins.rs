//! `kontra-cli plugins`: find, inspect and run CLAP plugins (FX2).

use crate::args::{PluginsArgs, PluginsCommand};
use crate::CliResult;
use kontra_session::plugins::{
    plugin_files, probe_line, scan_files, search_paths, LoadedPlugin, PluginFormat, PluginInfo,
    ProbeCommand, ScanReport,
};
use std::path::Path;
use std::time::Duration;

const FORMATS: [PluginFormat; 2] = [PluginFormat::Clap, PluginFormat::Vst3];

pub fn run(a: &PluginsArgs) -> CliResult {
    match &a.command {
        PluginsCommand::Scan { json, timeout_s } => scan(*json, *timeout_s),
        PluginsCommand::Probe { path } => {
            println!("{}", probe_line(path));
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

fn open(path: &Path, id: Option<&str>) -> Result<LoadedPlugin, String> {
    let format = PluginFormat::of_file(path).ok_or("not a plugin file (.clap, .vst3)")?;
    LoadedPlugin::open(format, path, id).map_err(|e| e.to_string())
}

/// Every probe is a process of this program, so a crashing plugin ends only its probe.
fn scan(json: bool, timeout_s: f32) -> CliResult {
    let program = std::env::current_exe().map_err(|e| format!("cannot find myself: {e}"))?;
    let probe = ProbeCommand {
        program,
        args: vec!["plugins".into(), "probe".into()],
    };
    let timeout = Duration::from_secs_f32(timeout_s);

    let mut report = ScanReport::default();
    let mut searched = Vec::new();
    for format in FORMATS {
        searched.extend(search_paths(format));
        let mut part = scan_files(&plugin_files(format), &probe, timeout);
        report.plugins.append(&mut part.plugins);
        report.broken.append(&mut part.broken);
    }

    if json {
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
    let mut plugin = open(path, id)?;
    let info: PluginInfo = plugin.info().clone();
    println!("{} — {}  [{}]", info.name, info.vendor, info.id);
    println!("latency: {} samples", plugin.latency_samples());
    for p in plugin.params() {
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
    let mut plugin = open(path, id)?;
    plugin
        .render_file(input, output, &parsed)
        .map_err(|e| e.to_string())?;
    println!(
        "written: {} (latency {} samples, not compensated)",
        output.display(),
        plugin.latency_samples()
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
