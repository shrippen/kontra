//! `korpus write` / `korpus check`.

use crate::args::{KorpusArgs, KorpusCommand};
use crate::CliResult;
use kontra_session as session;
use kontra_session::KorpusSet;
use std::path::Path;

/// LH-BEW-05: fretless intonation within 3 cents of the reference, per note.
const FRETLESS_MAX_CENTS: f64 = 3.0;

pub fn run(a: &KorpusArgs) -> CliResult {
    match &a.command {
        KorpusCommand::Write { dir } => write(dir),
        KorpusCommand::Check { dir } => check(dir.as_deref()),
        KorpusCommand::Measure {
            dirs,
            window_ms,
            laptop_jack,
            pitch,
            min_recall,
            min_precision,
            verbose,
        } => {
            let matching = session::Matching {
                window_s: window_ms / 1000.0,
                require_pitch: *pitch,
            };
            let (recall, precision) = measure(dirs, matching, *laptop_jack, *verbose)?;
            let low = |floor: &Option<f64>, value: f64| floor.is_some_and(|m| value < m);
            if low(min_recall, recall) || low(min_precision, precision) {
                return Err(format!(
                    "below the regression floor: recall {recall:.1} %, precision {precision:.1} %"
                ));
            }
            Ok(())
        }
        KorpusCommand::Techniques { dirs, window_ms } => techniques(dirs, window_ms / 1000.0),
    }
}

/// Rows and columns of the technique table: a plain pluck, then Kontra's techniques.
const TOLD: [Option<kontra_model::Technique>; 7] = {
    use kontra_model::Technique as T;
    [
        None,
        Some(T::Slide),
        Some(T::HammerOn),
        Some(T::PullOff),
        Some(T::Slap),
        Some(T::Pop),
        Some(T::Ghost),
    ]
};

fn technique_name(t: Option<kontra_model::Technique>) -> String {
    t.map_or_else(|| "plain".to_string(), |t| format!("{t:?}"))
}

/// Per annotated technique (rows) how the nearest detection within `window_s` was told
/// (columns), or missed. The diagonal is right; the rest is what the technique evaluation would
/// count as "another technique".
fn techniques(dirs: &[std::path::PathBuf], window_s: f64) -> CliResult {
    let mut table = vec![[0usize; TOLD.len() + 1]; TOLD.len()];
    for dir in dirs {
        let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let mut files: Vec<_> = entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .collect();
        files.sort();
        for json in files {
            let item = session::read_korpus_file(&json)?;
            let matching = session::Matching {
                window_s,
                require_pitch: false,
            };
            let (_, found) = session::evaluate_korpus_with(
                item.set,
                &item.samples,
                item.sample_rate,
                &item.notes,
                matching,
            );
            let mut used = vec![false; found.len()];
            for note in &item.notes {
                let Some(row) = TOLD.iter().position(|t| *t == note.technique) else {
                    continue;
                };
                let best = found
                    .iter()
                    .enumerate()
                    .filter(|(i, d)| !used[*i] && (d.onset_s - note.onset_s).abs() <= window_s)
                    .min_by(|a, b| {
                        (a.1.onset_s - note.onset_s)
                            .abs()
                            .total_cmp(&(b.1.onset_s - note.onset_s).abs())
                    });
                let column = match best {
                    None => TOLD.len(),
                    Some((i, d)) => {
                        used[i] = true;
                        TOLD.iter().position(|t| *t == d.technique).unwrap_or(0)
                    }
                };
                table[row][column] += 1;
            }
        }
    }
    print!("{:<10} {:>6}", "annotated", "notes");
    for t in TOLD {
        print!(" {:>8}", technique_name(t));
    }
    println!(" {:>8} {:>7}", "missed", "right");
    for (row, counts) in table.iter().enumerate() {
        let total: usize = counts.iter().sum();
        if total == 0 {
            continue;
        }
        print!("{:<10} {:>6}", technique_name(TOLD[row]), total);
        for count in counts {
            print!(" {count:>8}");
        }
        let right = counts[row] as f64 / total as f64 * 100.0;
        println!(" {right:>6.1}%");
    }
    Ok(())
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
        "{:<14} {:>6} {:>8} {:>9} {:>7} {:>7} {:>9} {:>8} {:>8} {:>8}",
        "set",
        "notes",
        "recall",
        "precision",
        "pitch",
        "techn.",
        "p95 ms",
        "cents",
        "p95 ct",
        "max ct"
    );
    let mut failed = false;
    for set in KorpusSet::ALL {
        let item = match dir {
            Some(d) => session::read_korpus(d, set).map_err(|e| e.to_string())?,
            None => session::generate_korpus(set),
        };
        let e = session::evaluate_korpus(set, &item.samples, item.sample_rate, &item.notes);
        // LH-EIN-11: a room through a microphone needs ≥ 90 % (acceptance of J-E2).
        let room = matches!(set, KorpusSet::RoomDouble | KorpusSet::RoomDoubleBacking);
        let needed = match set {
            KorpusSet::LaptopJack4 => 0.95,
            KorpusSet::RoomDouble | KorpusSet::RoomDoubleBacking => 0.9,
            _ => 0.98,
        };
        let found = if room { 0.9 } else { 0.99 };
        // Legato notes have no attack: their time comes from the pitch window (±30 ms).
        let onset_ms = if room || set == KorpusSet::Techniques {
            30.0
        } else {
            10.0
        };
        // A mean hides one note that is always off (B0 +8 ct, J-B4): check p95 and max too.
        let cents_ok = set != KorpusSet::Fretless5
            || (e.p95_abs_cents <= FRETLESS_MAX_CENTS && e.max_abs_cents <= FRETLESS_MAX_CENTS);
        // LH-BEW-06: ≥ 85 % of the technique set recognised; elsewhere no technique invented.
        let technique_needed = if set == KorpusSet::Techniques {
            0.85
        } else {
            0.95
        };
        let ok = e.recall() >= found
            && e.precision() >= found
            && e.pitch_accuracy() >= needed
            && e.technique_accuracy() >= technique_needed
            && e.p95_abs_onset_ms < onset_ms
            && cents_ok;
        failed |= !ok;
        println!(
            "{:<14} {:>6} {:>7.1}% {:>8.1}% {:>6.1}% {:>6.1}% {:>9.2} {:>8.2} {:>8.2} {:>8.2} {}",
            set.name(),
            e.annotated,
            e.recall() * 100.0,
            e.precision() * 100.0,
            e.pitch_accuracy() * 100.0,
            e.technique_accuracy() * 100.0,
            e.p95_abs_onset_ms,
            e.mean_abs_cents,
            e.p95_abs_cents,
            e.max_abs_cents,
            if ok { "ok" } else { "FAILED" }
        );
    }
    if failed {
        return Err("corpus below LH-NF-02/03 or LH-BEW-05 targets".into());
    }
    Ok(())
}

/// Prints one line per file and the total; returns total recall and precision in percent.
fn measure(
    dirs: &[std::path::PathBuf],
    matching: session::Matching,
    laptop_jack: bool,
    verbose: bool,
) -> Result<(f64, f64), String> {
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
        let (e, found) = session::evaluate_korpus_with(
            item.set,
            &item.samples,
            item.sample_rate,
            &item.notes,
            matching,
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
            report(&item.notes, &found, matching);
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
    Ok((pct(matched, annotated), pct(matched, detected)))
}

/// Misses, wrong pitches and extra detections of one file (greedy nearest match, as evaluated).
fn report(
    notes: &[session::Annotation],
    found: &[kontra_model::DetectedNote],
    matching: session::Matching,
) {
    let window_s = matching.window_s;
    let mut used = vec![false; found.len()];
    for n in notes {
        let best = found
            .iter()
            .enumerate()
            .filter(|(i, d)| {
                !used[*i]
                    && (d.onset_s - n.onset_s).abs() <= window_s
                    && (!matching.require_pitch || d.midi.round() == n.midi.round())
            })
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
