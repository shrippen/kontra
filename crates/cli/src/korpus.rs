//! `korpus write` / `korpus check`.

use crate::args::{KorpusArgs, KorpusCommand};
use crate::CliResult;
use kontra_session as session;
use kontra_session::KorpusSet;
use std::path::Path;

pub fn run(a: &KorpusArgs) -> CliResult {
    match &a.command {
        KorpusCommand::Write { dir } => write(dir),
        KorpusCommand::Check { dir } => check(dir.as_deref()),
    }
}

fn write(dir: &Path) -> CliResult {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for set in KorpusSet::ALL {
        let item = session::generate_korpus(set);
        let wav = dir.join(format!("{}.wav", set.name()));
        let json = dir.join(format!("{}.json", set.name()));
        session::write_mono_wav(&wav, &item.samples, item.sample_rate)
            .map_err(|e| e.to_string())?;
        let text = serde_json::to_string_pretty(&item).map_err(|e| e.to_string())?;
        std::fs::write(&json, text).map_err(|e| format!("{}: {e}", json.display()))?;
        println!(
            "{}: {} notes, {:.0} s",
            set.name(),
            item.notes.len(),
            item.samples.len() as f64 / f64::from(item.sample_rate)
        );
    }
    Ok(())
}

fn check(dir: Option<&Path>) -> CliResult {
    println!(
        "{:<14} {:>6} {:>8} {:>9} {:>7} {:>9} {:>8}",
        "set", "notes", "recall", "precision", "pitch", "p95 ms", "cents"
    );
    let mut failed = false;
    for set in KorpusSet::ALL {
        let item = match dir {
            Some(d) => session::read_korpus(d, set).map_err(|e| e.to_string())?,
            None => session::generate_korpus(set),
        };
        let e = session::evaluate_korpus(set, &item.samples, item.sample_rate, &item.notes);
        let needed = if set == KorpusSet::LaptopJack4 {
            0.95
        } else {
            0.98
        };
        let ok = e.recall() >= 0.99
            && e.precision() >= 0.99
            && e.pitch_accuracy() >= needed
            && e.p95_abs_onset_ms < 10.0;
        failed |= !ok;
        println!(
            "{:<14} {:>6} {:>7.1}% {:>8.1}% {:>6.1}% {:>9.2} {:>8.2} {}",
            set.name(),
            e.annotated,
            e.recall() * 100.0,
            e.precision() * 100.0,
            e.pitch_accuracy() * 100.0,
            e.p95_abs_onset_ms,
            e.mean_abs_cents,
            if ok { "ok" } else { "FAILED" }
        );
    }
    if failed {
        return Err("corpus below LH-NF-02/03 targets".into());
    }
    Ok(())
}
