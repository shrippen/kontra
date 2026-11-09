//! `korpus ends` (E5, LH-BEW-10): where notes end, per recording and in total.

use crate::CliResult;
use kontra_session as session;
use kontra_session::EndTally;
use std::path::PathBuf;

/// LH-BEW-10: the end of a muted note within this.
const TARGET_MS: f64 = 30.0;
/// The generated set's annotations are exact.
const SYNTHETIC_WINDOW_S: f64 = 0.03;

/// Prints the staccato line of `korpus check`; true when every stopped note is told muted
/// within `TARGET_MS` and no note is muted while it sounds.
pub(crate) fn check() -> bool {
    let (item, offsets) = session::generate_staccato();
    let t = session::evaluate_ends(
        item.set,
        &item.samples,
        item.sample_rate,
        &item.notes,
        &offsets,
        SYNTHETIC_WINDOW_S,
    );
    let ok = t.stopped > 0 && t.within(TARGET_MS) >= 1.0 && t.early == 0;
    print_line("note ends (staccato)", &t, if ok { "ok" } else { "FAILED" });
    ok
}

/// Prints one line per recording (or the synthetic staccato set without `dirs`) and the total.
pub(crate) fn measure(dirs: &[PathBuf], window_s: f64) -> CliResult {
    print_header();
    if dirs.is_empty() {
        let (item, offsets) = session::generate_staccato();
        let tally = session::evaluate_ends(
            item.set,
            &item.samples,
            item.sample_rate,
            &item.notes,
            &offsets,
            SYNTHETIC_WINDOW_S,
        );
        print_line("staccato (synthetic)", &tally, "");
        return Ok(());
    }
    let mut files: Vec<PathBuf> = Vec::new();
    for dir in dirs {
        let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        files.extend(
            entries
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "json")),
        );
    }
    files.sort();
    let mut total = EndTally::default();
    for json in &files {
        let item = session::read_korpus_file(json)?;
        let offsets = session::read_korpus_offsets(json)?;
        let tally = session::evaluate_ends(
            item.set,
            &item.samples,
            item.sample_rate,
            &item.notes,
            &offsets,
            window_s,
        );
        let name = json
            .file_stem()
            .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
        print_line(&name, &tally, "");
        total.add(&tally);
    }
    print_line("total", &total, "");
    Ok(())
}

fn print_header() {
    println!(
        "{:<22} {:>7} {:>6} {:>6} {:>6} {:>6} {:>6} {:>8} {:>7} {:>7} {:>7} {:>9}",
        "file",
        "stopped",
        "found",
        "muted",
        "decay",
        "repl.",
        "none",
        "≤30 ms",
        "p50 ms",
        "p95 ms",
        "bias",
        "early"
    );
}

fn print_line(name: &str, t: &EndTally, verdict: &str) {
    let ms = |v: Option<f64>| v.map_or_else(|| "-".into(), |v| format!("{v:.1}"));
    println!(
        "{name:<22} {:>7} {:>6} {:>6} {:>6} {:>6} {:>6} {:>7.1}% {:>7} {:>7} {:>7} {:>8.1}% {verdict}",
        t.stopped,
        t.stopped_found,
        t.muted,
        t.decayed,
        t.replaced,
        t.untold,
        t.within(TARGET_MS) * 100.0,
        ms(t.abs_quantile(0.5)),
        ms(t.abs_quantile(0.95)),
        ms(t.median_ms()),
        t.early_mutes() * 100.0
    );
}
