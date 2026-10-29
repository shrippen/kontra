//! `korpus write` / `korpus check` / `measure` / `techniques` / `confusion`.

use crate::args::{KorpusArgs, KorpusCommand};
use crate::CliResult;
use kontra_session as session;
use kontra_session::KorpusSet;
use std::path::Path;

/// LH-BEW-05: fretless intonation within 3 cents of the reference, per note.
const FRETLESS_MAX_CENTS: f64 = 3.0;

const CENTS_PER_SEMITONE: f64 = 100.0;

/// LH-NF-16: at most one false note per minute where nothing is played. Not met yet (29.10.:
/// 41 and 26 per minute, the hum's pitch at every crackle; ROADMAP F3): until the fix a guard
/// against more.
const IDLE_TARGET_PER_MINUTE: f64 = 1.0;
const IDLE_GUARD_PER_MINUTE: f64 = 45.0;
const S_PER_MINUTE: f64 = 60.0;

/// LH-NF-15, line sets: p95 from attack to report. A guard 10 ms over the first measurement
/// (29.10.: 81 and 100 ms) until the owner confirms the target (70 ms).
const REPORT_GUARD_MS: f64 = 110.0;

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
        KorpusCommand::Confusion { dirs, window_ms } => confusion(dirs, window_ms / 1000.0),
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
        Some(T::Muted),
    ]
};

fn technique_name(t: Option<kontra_model::Technique>) -> String {
    t.map_or_else(|| "plain".to_string(), |t| format!("{t:?}"))
}

/// Per annotated technique (rows) how the nearest detection within `window_s` was told
/// (columns), or missed. The diagonal is right; the rest is what the technique evaluation would
/// count as "another technique".
fn techniques(dirs: &[std::path::PathBuf], window_s: f64) -> CliResult {
    let mut table: Table = vec![[0; TOLD.len() + 1]; TOLD.len()];
    each_recording(dirs, window_s, |notes, found| {
        tally(notes, found, window_s, &mut table)
    })?;
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

/// Runs `f` with the annotation and the detections of every WAV+JSON pair in `dirs`.
fn each_recording(
    dirs: &[std::path::PathBuf],
    window_s: f64,
    mut f: impl FnMut(&[session::Annotation], &[kontra_model::DetectedNote]),
) -> CliResult {
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
            f(&item.notes, &found);
        }
    }
    Ok(())
}

/// What the tracker heard against the annotated pitch: the columns of `korpus confusion`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Heard {
    Right,
    /// One semitone off: a neighbouring fret.
    Semitone,
    OctaveDown,
    OctaveUp,
    /// A fifth or a twelfth up: the third partial, or the second heard as the root.
    Fifth,
    TwoOctaves,
    Other,
    Missed,
}

impl Heard {
    const ALL: [Heard; 8] = [
        Heard::Right,
        Heard::Semitone,
        Heard::OctaveDown,
        Heard::OctaveUp,
        Heard::Fifth,
        Heard::TwoOctaves,
        Heard::Other,
        Heard::Missed,
    ];

    /// The class of `heard_midi` against `annotated_midi`, by the rounded interval. Example:
    /// annotated E1 (28), heard B2 (47) → +19, a twelfth: `Fifth`.
    fn of(annotated_midi: f64, heard_midi: f64) -> Heard {
        match (heard_midi - annotated_midi).round() as i32 {
            0 => Heard::Right,
            -1 | 1 => Heard::Semitone,
            -12 => Heard::OctaveDown,
            12 => Heard::OctaveUp,
            7 | 19 => Heard::Fifth,
            -24 | 24 => Heard::TwoOctaves,
            _ => Heard::Other,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Heard::Right => "right",
            Heard::Semitone => "±1",
            Heard::OctaveDown => "-12",
            Heard::OctaveUp => "+12",
            Heard::Fifth => "+7/+19",
            Heard::TwoOctaves => "±24",
            Heard::Other => "other",
            Heard::Missed => "missed",
        }
    }
}

/// Counts per register (octave of the annotated note, `BTreeMap` key) and `Heard` class.
type Confusion = std::collections::BTreeMap<i32, [usize; Heard::ALL.len()]>;

/// Per register what the tracker made of each annotated note (Harmonicon A5): where octave
/// errors sit tells whether the low strings or the high frets need work.
fn confusion(dirs: &[std::path::PathBuf], window_s: f64) -> CliResult {
    let mut table = Confusion::new();
    each_recording(dirs, window_s, |notes, found| {
        confuse(notes, found, window_s, &mut table)
    })?;
    print!("{:<8} {:>6}", "register", "notes");
    for h in Heard::ALL {
        print!(" {:>7}", h.name());
    }
    println!(" {:>7}", "right");
    for (octave, counts) in &table {
        let total: usize = counts.iter().sum();
        print!("{:<8} {total:>6}", format!("C{octave}–B{octave}"));
        for count in counts {
            print!(" {count:>7}");
        }
        let right = counts[0] as f64 / total.max(1) as f64 * 100.0;
        println!(" {right:>6.1}%");
    }
    Ok(())
}

/// Adds one recording to `table`: each annotated note against its nearest unused detection.
fn confuse(
    notes: &[session::Annotation],
    found: &[kontra_model::DetectedNote],
    window_s: f64,
    table: &mut Confusion,
) {
    let mut used = vec![false; found.len()];
    for note in notes {
        let heard = match nearest(found, &used, note.onset_s, window_s) {
            Some(k) => {
                used[k] = true;
                Heard::of(note.midi, found[k].midi)
            }
            None => Heard::Missed,
        };
        // MIDI 12 is C0.
        let octave = (note.midi.round() as i32).div_euclid(12) - 1;
        let column = Heard::ALL.iter().position(|h| *h == heard).unwrap_or(0);
        table.entry(octave).or_default()[column] += 1;
    }
}

/// The unused detection nearest to `onset_s` within `window_s`.
fn nearest(
    found: &[kontra_model::DetectedNote],
    used: &[bool],
    onset_s: f64,
    window_s: f64,
) -> Option<usize> {
    found
        .iter()
        .enumerate()
        .filter(|(k, d)| !used[*k] && (d.onset_s - onset_s).abs() <= window_s)
        .min_by(|a, b| {
            (a.1.onset_s - onset_s)
                .abs()
                .total_cmp(&(b.1.onset_s - onset_s).abs())
        })
        .map(|(k, _)| k)
}

/// Counts per annotated technique (rows) and told technique (columns, the last: missed).
type Table = Vec<[usize; TOLD.len() + 1]>;

/// Adds one recording to `table`. A note that slides later (`slide_cents`, IDMT) also counts a
/// `Slide` row: Kontra reports a slide where it arrives, so the first detection after the
/// note's own one, before the next note, `slide_cents` away from it.
///
/// ```text
/// annotated:  A2 (slides +100 ct) ─────────────────── next note
/// detected:   A2 plain ✓          A#2 Slide ✓ (row Slide)
/// ```
fn tally(
    notes: &[session::Annotation],
    found: &[kontra_model::DetectedNote],
    window_s: f64,
    table: &mut Table,
) {
    let mut used = vec![false; found.len()];
    let column = |d: Option<&kontra_model::DetectedNote>| match d {
        None => TOLD.len(),
        Some(d) => TOLD.iter().position(|t| *t == d.technique).unwrap_or(0),
    };
    let slide = Some(kontra_model::Technique::Slide);
    let slide_row = TOLD.iter().position(|t| *t == slide).unwrap_or(0);
    for (i, note) in notes.iter().enumerate() {
        // Harmonics and others Kontra does not tell are left out.
        let Some(row) = TOLD.iter().position(|t| *t == note.technique) else {
            continue;
        };
        let best = nearest(found, &used, note.onset_s, window_s);
        if let Some(k) = best {
            used[k] = true;
        }
        table[row][column(best.map(|k| &found[k]))] += 1;

        // The slide's arrival, between this note and the next.
        let Some(cents) = note.slide_cents else {
            continue;
        };
        let until = notes
            .get(i + 1)
            .map_or(f64::INFINITY, |n| n.onset_s - window_s);
        let arrival = found.iter().enumerate().find(|(k, d)| {
            !used[*k]
                && d.onset_s > note.onset_s + window_s
                && d.onset_s < until
                && ((d.midi - note.midi).abs() - cents / CENTS_PER_SEMITONE).abs() < 0.5
        });
        if let Some((k, _)) = arrival {
            used[k] = true;
        }
        table[slide_row][column(arrival.map(|(_, d)| d))] += 1;
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
        "{:<14} {:>6} {:>8} {:>9} {:>7} {:>7} {:>9} {:>8} {:>8} {:>8} {:>8}",
        "set",
        "notes",
        "recall",
        "precision",
        "pitch",
        "techn.",
        "p95 ms",
        "cents",
        "p95 ct",
        "max ct",
        "p95 rep"
    );
    let mut failed = false;
    for set in KorpusSet::ALL {
        let item = match dir {
            Some(d) => session::read_korpus(d, set).map_err(|e| e.to_string())?,
            None => session::generate_korpus(set),
        };
        let e = session::evaluate_korpus(set, &item.samples, item.sample_rate, &item.notes);
        // Nothing played: only false notes count (LH-NF-16).
        if set.is_idle() {
            let minutes = item.samples.len() as f64 / f64::from(item.sample_rate) / S_PER_MINUTE;
            let per_minute = e.detected as f64 / minutes;
            let ok = per_minute <= IDLE_GUARD_PER_MINUTE;
            failed |= !ok;
            // The guard decides; the target is told apart, so "ok" never reads as target met.
            // Example: "41.0 per minute (target 1 missed, guard 45) ok".
            let target = if per_minute <= IDLE_TARGET_PER_MINUTE {
                "met"
            } else {
                "missed"
            };
            println!(
                "{:<14} {:>6} {:>6} false notes, {per_minute:.1} per minute (target {:.0} {target}, guard {:.0}) {}",
                set.name(),
                0,
                e.detected,
                IDLE_TARGET_PER_MINUTE,
                IDLE_GUARD_PER_MINUTE,
                if ok { "ok" } else { "FAILED" }
            );
            continue;
        }
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
        let report_ok = !matches!(set, KorpusSet::Line4 | KorpusSet::Line5)
            || e.p95_report_ms <= REPORT_GUARD_MS;
        let ok = e.recall() >= found
            && e.precision() >= found
            && e.pitch_accuracy() >= needed
            && e.technique_accuracy() >= technique_needed
            && e.p95_abs_onset_ms < onset_ms
            && cents_ok
            && report_ok;
        failed |= !ok;
        println!(
            "{:<14} {:>6} {:>7.1}% {:>8.1}% {:>6.1}% {:>6.1}% {:>9.2} {:>8.2} {:>8.2} {:>8.2} {:>8.1} {}",
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
            e.p95_report_ms,
            if ok { "ok" } else { "FAILED" }
        );
    }
    if failed {
        return Err("corpus below LH-NF-02/03/15/16 or LH-BEW-05 targets".into());
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
        let item = session::read_korpus_file(json)?;
        let evaluate = match laptop_jack {
            true => session::evaluate_korpus_laptop_jack,
            false => session::evaluate_korpus_with,
        };
        let (e, found) = evaluate(
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
                    && (!matching.require_pitch || session::same_pitch(n, d.midi))
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
                if !session::same_pitch(n, d.midi) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use kontra_model::{DetectedNote, InputSource, Technique};

    fn heard(onset_s: f64, midi: f64, technique: Option<Technique>) -> DetectedNote {
        DetectedNote {
            onset_s,
            midi,
            confidence: 1.0,
            level_db: -20.0,
            source: InputSource::Line,
            technique,
        }
    }

    #[test]
    fn slide_counts_where_it_arrives() {
        // IDMT: A2 plucked, slides a fret up 250 ms later, then the next note at 0.6 s.
        let note = |onset_s, midi, slide_cents| session::Annotation {
            onset_s,
            midi,
            technique: None,
            slide_cents,
        };
        let notes = [note(0.0, 45.0, Some(100.0)), note(0.6, 40.0, None)];
        let found = [
            heard(0.01, 45.0, None),
            heard(0.25, 46.0, Some(Technique::Slide)),
            heard(0.6, 40.0, None),
        ];
        let mut table: Table = vec![[0; TOLD.len() + 1]; TOLD.len()];
        tally(&notes, &found, 0.08, &mut table);
        let plain = 0;
        let slide = TOLD
            .iter()
            .position(|t| *t == Some(Technique::Slide))
            .unwrap();
        assert_eq!(table[plain][plain], 2);
        assert_eq!(table[slide][slide], 1);
    }

    /// Harmonicon A5: E1 heard right, A1 an octave up, D2 a twelfth up, G2 missed; per
    /// octave of the annotated note.
    #[test]
    fn confusion_sorts_by_interval_and_register() {
        let note = |onset_s, midi| session::Annotation {
            onset_s,
            midi,
            technique: None,
            slide_cents: None,
        };
        let notes = [
            note(0.0, 28.0),
            note(0.5, 33.0),
            note(1.0, 38.0),
            note(1.5, 43.0),
        ];
        let found = [
            heard(0.01, 28.1, None),
            heard(0.5, 45.0, None),
            heard(1.02, 57.0, None),
        ];
        let mut table = Confusion::new();
        confuse(&notes, &found, 0.08, &mut table);
        let column = |h: Heard| Heard::ALL.iter().position(|x| *x == h).unwrap();
        assert_eq!(table[&1][column(Heard::Right)], 1, "{table:?}");
        assert_eq!(table[&1][column(Heard::OctaveUp)], 1, "{table:?}");
        assert_eq!(table[&2][column(Heard::Fifth)], 1, "{table:?}");
        assert_eq!(table[&2][column(Heard::Missed)], 1, "{table:?}");
        assert_eq!(Heard::of(40.0, 28.0), Heard::OctaveDown);
        assert_eq!(Heard::of(40.0, 41.4), Heard::Semitone);
    }
}
