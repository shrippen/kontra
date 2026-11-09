//! Real recordings against a frozen baseline (E7): `korpus freeze` writes the results of a
//! recording group once, `korpus check --real` measures every frozen group again and fails
//! when one falls below it. Groups not in the baseline (new recordings, e.g. the owner's next
//! takes) are measured and reported on their own; they never move a frozen group.
//!
//! ```text
//! --real conv ─┬─ conv/idmt-st   in baseline ─► measured ≥ frozen − 0.5 points? ─► ok / FAILED
//!              ├─ conv/filobass  in baseline ─► …
//!              └─ conv/new-set   not in it   ─► "new", reported only
//! baseline group missing on disk (not converted) ─► skipped
//! ```

use crate::CliResult;
use kontra_session as session;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// As the CI's IDMT floor: pitch-aware, ±80 ms (score-grid annotations).
const WINDOW_S: f64 = 0.08;
/// A frozen value may drop this much (percentage points) before the check fails: the same
/// recordings measured on another machine differ in the last digit at most.
const TOLERANCE_POINTS: f64 = 0.5;
const PERCENT: f64 = 100.0;

/// Whether `freeze` may overwrite a group the baseline already holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Replace {
    No,
    Allowed,
}

/// The totals of one recording group.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Measured {
    files: usize,
    notes: usize,
    detected: usize,
    matched: usize,
}

impl Measured {
    fn recall(&self) -> f64 {
        percent(self.matched, self.notes)
    }

    fn precision(&self) -> f64 {
        percent(self.matched, self.detected)
    }

    fn to_json(self, name: &str) -> Value {
        json!({
            "group": name,
            "files": self.files,
            "notes": self.notes,
            "recall": round2(self.recall()),
            "precision": round2(self.precision()),
        })
    }
}

fn percent(part: usize, whole: usize) -> f64 {
    match whole {
        0 => 0.0,
        w => part as f64 / w as f64 * PERCENT,
    }
}

fn round2(v: f64) -> f64 {
    (v * PERCENT).round() / PERCENT
}

/// A group: its name (root's last part / path below the root) and directory.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Group {
    name: String,
    dir: PathBuf,
}

/// Every directory under the roots that holds WAV+JSON pairs, sorted by name.
fn groups(roots: &[PathBuf]) -> Result<Vec<Group>, String> {
    let mut found = Vec::new();
    for root in roots {
        let base = root
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        collect(root, &base, &mut found)?;
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(found)
}

fn collect(dir: &Path, name: &str, found: &mut Vec<Group>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut has_pairs = false;
    for path in entries.filter_map(Result::ok).map(|e| e.path()) {
        if path.is_dir() {
            let child = path
                .file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
            collect(&path, &format!("{name}/{child}"), found)?;
            continue;
        }
        has_pairs |= is_pair(&path);
    }
    if has_pairs {
        found.push(Group {
            name: name.to_string(),
            dir: dir.to_path_buf(),
        });
    }
    Ok(())
}

/// A JSON annotation with its WAV.
fn is_pair(path: &Path) -> bool {
    path.extension().is_some_and(|x| x == "json") && path.with_extension("wav").exists()
}

fn measure(dir: &Path) -> Result<Measured, String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| is_pair(p))
        .collect();
    files.sort();
    let matching = session::Matching {
        window_s: WINDOW_S,
        require_pitch: true,
    };
    let mut total = Measured::default();
    for json in &files {
        let item = session::read_korpus_file(json)?;
        let (e, _) = session::evaluate_korpus_with(
            item.set,
            &item.samples,
            item.sample_rate,
            &item.notes,
            matching,
        );
        total.files += 1;
        total.notes += e.annotated;
        total.detected += e.detected;
        total.matched += e.matched;
    }
    Ok(total)
}

/// The frozen groups of `baseline` (empty if the file does not exist yet).
fn read(baseline: &Path) -> Result<Vec<Value>, String> {
    if !baseline.exists() {
        return Ok(Vec::new());
    }
    let text =
        std::fs::read_to_string(baseline).map_err(|e| format!("{}: {e}", baseline.display()))?;
    let file: Value =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", baseline.display()))?;
    Ok(file["groups"].as_array().cloned().unwrap_or_default())
}

fn frozen<'a>(groups: &'a [Value], name: &str) -> Option<&'a Value> {
    groups.iter().find(|g| g["group"] == name)
}

/// `korpus check --real`: every frozen group on disk against its values, new groups apart.
pub(crate) fn check(roots: &[PathBuf], baseline: &Path) -> CliResult {
    let frozen_groups = read(baseline)?;
    let on_disk = groups(roots)?;
    println!();
    println!(
        "{:<32} {:>5} {:>6} {:>8} {:>9}   baseline {}",
        "real group",
        "files",
        "notes",
        "recall",
        "precision",
        baseline.display()
    );
    let mut failed = false;
    for group in &on_disk {
        let m = measure(&group.dir)?;
        let verdict = match frozen(&frozen_groups, &group.name) {
            None => "new, not in the baseline".to_string(),
            Some(f) => {
                let (verdict, ok) = compare(&m, f);
                failed |= !ok;
                verdict
            }
        };
        println!(
            "{:<32} {:>5} {:>6} {:>7.2}% {:>8.2}%   {verdict}",
            group.name,
            m.files,
            m.notes,
            m.recall(),
            m.precision()
        );
    }
    for f in &frozen_groups {
        let name = f["group"].as_str().unwrap_or_default();
        if !on_disk.iter().any(|g| g.name == name) {
            println!("{name:<32} skipped: not on disk (not converted here)");
        }
    }
    if failed {
        return Err("real recordings below their frozen baseline (E7)".into());
    }
    Ok(())
}

/// A frozen group against its measurement: the verdict and whether it holds.
fn compare(m: &Measured, f: &Value) -> (String, bool) {
    let same_data =
        f["files"].as_u64() == Some(m.files as u64) && f["notes"].as_u64() == Some(m.notes as u64);
    if !same_data {
        return (
            format!(
                "FAILED: data changed (frozen {} files, {} notes); reconvert or freeze again",
                f["files"], f["notes"]
            ),
            false,
        );
    }
    let low = |key: &str, value: f64| {
        f[key]
            .as_f64()
            .is_some_and(|v| value < v - TOLERANCE_POINTS)
    };
    let worse = low("recall", m.recall()) || low("precision", m.precision());
    let text = format!(
        "frozen {}% / {}% {}",
        f["recall"],
        f["precision"],
        if worse { "FAILED" } else { "ok" }
    );
    (text, !worse)
}

/// `korpus freeze`: measures `names` and writes them into the baseline.
pub(crate) fn freeze(
    names: &[String],
    roots: &[PathBuf],
    baseline: &Path,
    replace: Replace,
) -> CliResult {
    let mut frozen_groups = read(baseline)?;
    let on_disk = groups(roots)?;
    for name in names {
        let Some(group) = on_disk.iter().find(|g| &g.name == name) else {
            return Err(format!("{name}: no such group under the --real roots"));
        };
        let held = frozen_groups
            .iter()
            .position(|g| g["group"] == name.as_str());
        if held.is_some() && replace == Replace::No {
            return Err(format!(
                "{name}: already frozen; --replace to freeze it again"
            ));
        }
        let m = measure(&group.dir)?;
        let entry = m.to_json(name);
        match held {
            Some(i) => frozen_groups[i] = entry,
            None => frozen_groups.push(entry),
        }
        println!(
            "{name}: {} files, {} notes, recall {:.2} %, precision {:.2} %",
            m.files,
            m.notes,
            m.recall(),
            m.precision()
        );
    }
    frozen_groups.sort_by(|a, b| a["group"].as_str().cmp(&b["group"].as_str()));
    let file = json!({
        "about": "Frozen results of real recordings (E7, kontra-cli korpus freeze); \
                  pitch-aware, ±80 ms. korpus check --real fails below them.",
        "groups": frozen_groups,
    });
    let text = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
    std::fs::write(baseline, text + "\n").map_err(|e| format!("{}: {e}", baseline.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn measured(notes: usize, matched: usize) -> Measured {
        Measured {
            files: 2,
            notes,
            detected: notes,
            matched,
        }
    }

    #[test]
    fn a_frozen_group_holds_within_the_tolerance() {
        let frozen = measured(1000, 900).to_json("conv/x");
        assert!(compare(&measured(1000, 896), &frozen).1);
        assert!(!compare(&measured(1000, 890), &frozen).1);
        // Better passes.
        assert!(compare(&measured(1000, 950), &frozen).1);
    }

    #[test]
    fn changed_data_fails() {
        let frozen = measured(1000, 900).to_json("conv/x");
        let (text, ok) = compare(&measured(999, 900), &frozen);
        assert!(!ok);
        assert!(text.contains("data changed"));
    }
}
