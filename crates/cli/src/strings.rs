//! `korpus strings`: which string the notes of converted recordings were heard on (R7).
//!
//! ```text
//! dir = one bass ─► every note expected on its annotated string ─► verdict + log10 B
//!   --held-out: the string values (a_string) of each dir fitted on the other dirs,
//!               the measured B picked again with them (leave one bass out)
//! ```

use crate::CliResult;
use kontra_model::{StringHeard, Technique, Unclear};
use kontra_session as session;
use session::{Pick, StringModel, LOG_B_PER_FRET};
use std::path::PathBuf;

/// Pitch regions by the strings that can play them (four-string bass, frets 0–12).
const REGIONS: [(&str, u8); 4] = [("E1-C#2", 38), ("D2-F#2", 43), ("G2-A#2", 47), ("B2+", 255)];
/// Note lengths (s) to the next annotated attack.
const LENGTHS: [(&str, f64); 3] = [("<0.2 s", 0.2), ("0.2-0.5 s", 0.5), (">0.5 s", f64::MAX)];

/// One annotated note whose string was checked.
struct Row {
    group: usize,
    truth: u8,
    midi: u8,
    heard: StringHeard,
    log_b: Option<f64>,
    length_s: f64,
    technique: Option<Technique>,
}

impl Row {
    fn stated(&self) -> Option<f64> {
        match self.heard {
            StringHeard::Unclear(_) => None,
            _ => self.log_b,
        }
    }
}

/// Correct, stated and planned notes of one slice.
#[derive(Default, Clone, Copy)]
struct Count {
    planned: usize,
    stated: usize,
    right: usize,
    lowest_right: usize,
    chance: f64,
}

impl Count {
    fn line(&self, title: &str) {
        let pct = |a: usize, b: usize| {
            if b == 0 {
                0.0
            } else {
                a as f64 / b as f64 * 100.0
            }
        };
        println!(
            "  {title:<16} n {:>5}  told {:>5.1} %  right {:>5.1} %  (chance {:>4.1} %, lowest fret {:>5.1} %)",
            self.planned,
            pct(self.stated, self.planned),
            pct(self.right, self.stated),
            self.chance / self.stated.max(1) as f64 * 100.0,
            pct(self.lowest_right, self.stated),
        );
    }
}

pub fn run(dirs: &[PathBuf], window_s: f64, held_out: bool) -> CliResult {
    let (rows, base) = collect(dirs, window_s)?;
    let Some(base) = base else {
        return Err("no recordings".into());
    };
    let fitted = fit(&rows, &base, None);
    print!("a_string (log10 B open, all dirs):");
    for s in 0..4u8 {
        print!(" {s}: {:.3}", fitted.1[usize::from(s)].unwrap_or(f64::NAN));
    }
    println!();
    for (g, dir) in dirs.iter().enumerate() {
        let model = match held_out {
            true => fit(&rows, &base, Some(g)).0,
            false => base,
        };
        let mine: Vec<&Row> = rows.iter().filter(|r| r.group == g).collect();
        println!(
            "{} ({})",
            dir.display(),
            if held_out {
                "held out"
            } else {
                "profile values"
            }
        );
        report(&mine, &model);
    }
    Ok(())
}

/// Every checked note of every recording in `dirs`, and the profile model of the first.
fn collect(dirs: &[PathBuf], window_s: f64) -> Result<(Vec<Row>, Option<StringModel>), String> {
    let mut rows = Vec::new();
    let mut base = None;
    for (group, dir) in dirs.iter().enumerate() {
        let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let mut files: Vec<_> = entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .collect();
        files.sort();
        for json in files {
            let item = session::read_korpus_file(&json)?;
            let strings = session::read_korpus_strings(&json)?;
            base = base.or_else(|| StringModel::for_instrument(&item.set.instrument()));
            for told in session::tell_korpus_strings(&item, &strings, window_s) {
                let note = &item.notes[told.note];
                let next = item.notes.get(told.note + 1).map(|n| n.onset_s);
                let Some(truth) = strings[told.note] else {
                    continue;
                };
                rows.push(Row {
                    group,
                    truth,
                    midi: note.midi.round() as u8,
                    heard: told.verdict.heard,
                    log_b: told.verdict.log_b,
                    length_s: next.map_or(f64::MAX, |n| n - note.onset_s),
                    technique: note.technique,
                });
            }
        }
    }
    Ok((rows, base))
}

/// The profile with a_string fitted (median of log10 B − fret·slope) on the rows outside
/// `skip`, from stated plain notes; strings without notes keep the profile's value.
fn fit(rows: &[Row], base: &StringModel, skip: Option<usize>) -> (StringModel, [Option<f64>; 4]) {
    let mut model = *base;
    let mut values = [None; 4];
    for s in 0..4u8 {
        let mut a: Vec<f64> = rows
            .iter()
            .filter(|r| Some(r.group) != skip && r.truth == s)
            .filter(|r| matches!(r.technique, None | Some(Technique::Slap | Technique::Pop)))
            .filter_map(|r| Some(r.stated()? - f64::from(base.fret(s, r.midi)?) * LOG_B_PER_FRET))
            .collect();
        if a.is_empty() {
            continue;
        }
        a.sort_by(f64::total_cmp);
        let median = a[a.len() / 2];
        values[usize::from(s)] = Some(median);
        model = model.with_log_b(s, Some(median));
    }
    (model, values)
}

fn report(rows: &[&Row], model: &StringModel) {
    let candidates = |midi: u8| (0..4u8).filter(|s| model.fret(*s, midi).is_some()).count();
    let lowest = |midi: u8| (0..4u8).rev().find(|s| model.fret(*s, midi).is_some());
    let add = |c: &mut Count, r: &Row| {
        c.planned += 1;
        let Some(log_b) = r.stated() else {
            return;
        };
        c.stated += 1;
        c.chance += 1.0 / candidates(r.midi) as f64;
        if model.pick(r.midi, log_b) == Pick::String(r.truth) {
            c.right += 1;
        }
        if lowest(r.midi) == Some(r.truth) {
            c.lowest_right += 1;
        }
    };
    let plain = |r: &&&Row| matches!(r.technique, None | Some(Technique::Slap | Technique::Pop));
    let mut all = Count::default();
    rows.iter().filter(plain).for_each(|r| add(&mut all, r));
    all.line("plain/slap/pop");
    for (name, below) in REGIONS {
        let mut c = Count::default();
        let from = REGIONS
            .iter()
            .take_while(|(_, b)| *b < below)
            .last()
            .map_or(0, |(_, b)| *b);
        rows.iter()
            .filter(plain)
            .filter(|r| r.midi >= from && r.midi < below)
            .for_each(|r| add(&mut c, r));
        c.line(name);
    }
    for (i, (name, below)) in LENGTHS.iter().enumerate() {
        let from = if i == 0 { 0.0 } else { LENGTHS[i - 1].1 };
        let mut c = Count::default();
        rows.iter()
            .filter(plain)
            .filter(|r| r.length_s >= from && r.length_s < *below)
            .for_each(|r| add(&mut c, r));
        c.line(name);
    }
    let mut techniques: Vec<Option<Technique>> = rows.iter().map(|r| r.technique).collect();
    techniques.sort_by_key(|t| format!("{t:?}"));
    techniques.dedup();
    for t in techniques {
        let mut c = Count::default();
        rows.iter()
            .filter(|r| r.technique == t)
            .for_each(|r| add(&mut c, r));
        c.line(&t.map_or_else(|| "plain".to_string(), |t| format!("{t:?}")));
    }
    let unclear = |why: Unclear| {
        rows.iter()
            .filter(|r| r.heard == StringHeard::Unclear(why))
            .count()
    };
    println!(
        "  unclear: short {}, muted {}, few partials {}, not supported {}",
        unclear(Unclear::ShortNote),
        unclear(Unclear::Muted),
        unclear(Unclear::FewPartials),
        unclear(Unclear::NotSupported),
    );
}
