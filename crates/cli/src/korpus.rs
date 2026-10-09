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
        KorpusCommand::Measure {
            dirs,
            window_ms,
            laptop_jack,
            verbose,
        } => measure(dirs, *window_ms, *laptop_jack, *verbose),
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

fn measure(
    dirs: &[std::path::PathBuf],
    window_ms: f64,
    laptop_jack: bool,
    verbose: bool,
) -> CliResult {
    let mut files: Vec<std::path::PathBuf> = Vec::new();
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
    println!(
        "{:<44} {:>6} {:>6} {:>8} {:>9} {:>7} {:>8} {:>7}",
        "file", "notes", "found", "recall", "precision", "pitch", "mean ms", "cents"
    );
    let (mut annotated, mut detected, mut matched, mut pitch) = (0, 0, 0, 0);
    for json in &files {
        let mut item = session::read_korpus_file(json)?;
        if laptop_jack {
            item.samples = session::simulate_laptop_jack(&item.samples, item.sample_rate);
        }
        let (e, found) = session::evaluate_korpus_window(
            item.set,
            &item.samples,
            item.sample_rate,
            &item.notes,
            window_ms / 1000.0,
        );
        annotated += e.annotated;
        detected += e.detected;
        matched += e.matched;
        pitch += e.pitch_correct;
        let name = json
            .file_stem()
            .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
        println!(
            "{:<44} {:>6} {:>6} {:>7.1}% {:>8.1}% {:>6.1}% {:>8.2} {:>7.1}",
            name,
            e.annotated,
            e.detected,
            e.recall() * 100.0,
            e.precision() * 100.0,
            e.pitch_accuracy() * 100.0,
            e.mean_abs_onset_ms,
            e.mean_abs_cents
        );
        if verbose {
            report(&item.notes, &found, window_ms / 1000.0);
        }
    }
    let pct = |a: usize, b: usize| {
        if b == 0 {
            0.0
        } else {
            a as f64 / b as f64 * 100.0
        }
    };
    println!(
        "{:<44} {:>6} {:>6} {:>7.1}% {:>8.1}% {:>6.1}%",
        "total",
        annotated,
        detected,
        pct(matched, annotated),
        pct(matched, detected),
        pct(pitch, matched)
    );
    Ok(())
}

/// Misses, wrong pitches and extra detections of one file (greedy nearest match, as evaluated).
fn report(notes: &[session::Annotation], found: &[kontra_model::DetectedNote], window_s: f64) {
    let mut used = vec![false; found.len()];
    for n in notes {
        let best = found
            .iter()
            .enumerate()
            .filter(|(i, d)| !used[*i] && (d.onset_s - n.onset_s).abs() <= window_s)
            .min_by(|a, b| {
                (a.1.onset_s - n.onset_s)
                    .abs()
                    .total_cmp(&(b.1.onset_s - n.onset_s).abs())
            });
        match best {
            None => println!("    miss   {:>9.3} s  midi {:>6.2}", n.onset_s, n.midi),
            Some((i, d)) => {
                used[i] = true;
                if d.midi.round() != n.midi.round() {
                    println!(
                        "    pitch  {:>9.3} s  midi {:>6.2} -> {:>6.2}",
                        n.onset_s, n.midi, d.midi
                    );
                }
            }
        }
    }
    for (d, _) in found.iter().zip(&used).filter(|(_, u)| !**u) {
        println!("    extra  {:>9.3} s  midi {:>6.2}", d.onset_s, d.midi);
    }
}
