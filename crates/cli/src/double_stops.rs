//! Double stops in `korpus check` and `korpus double-stops` (R8, LH-BEW-11): how often an
//! announced double stop is told as both tones, and a single note never is.

use crate::CliResult;
use kontra_session as session;
use kontra_session::{DoubleStopTally, KorpusSet};
use std::path::PathBuf;

/// Targets on the synthetic sets (experiment: 99 % told, 0 of 4774 single notes).
const DETECTED_MIN: f64 = 0.9;
const FALSE_ALARMS_MAX: f64 = 0.01;
/// Single notes announced with a second tone this far above (positive) or below: fourth, fifth
/// both ways, tenth, octave (a run each; the experiment's eight gave 0 alarms on every one).
const SECOND_TONES: [i8; 5] = [5, 7, -7, -16, 12];
/// The single-note sets heard with a second tone announced.
const SINGLE_SETS: [KorpusSet; 5] = [
    KorpusSet::Line4,
    KorpusSet::Line5,
    KorpusSet::LaptopJack4,
    KorpusSet::Fretless5,
    KorpusSet::RoomDouble,
];
/// The generated sets' annotations are exact.
const SYNTHETIC_WINDOW_S: f64 = 0.03;

/// Prints the double-stop lines of `korpus check`; returns whether every target is met.
pub(crate) fn check() -> bool {
    let set = session::generate_double_stops();
    let dyads = session::evaluate_double_stops(
        KorpusSet::Line4,
        &set.samples,
        set.sample_rate,
        &set.strikes,
        SYNTHETIC_WINDOW_S,
    );
    let mut ok = dyads.detected() >= DETECTED_MIN && dyads.octaves_told == 0;
    print_line("double stops", &dyads, Target::of(ok));
    for single in SINGLE_SETS {
        let item = session::generate_korpus(single);
        let tally = second_tones(single, &item, SYNTHETIC_WINDOW_S);
        let fine = single_ok(&tally);
        ok &= fine;
        print_line(&format!("+2nd {}", single.name()), &tally, Target::of(fine));
    }
    ok
}

/// Single notes: hardly ever both, never the other tone, octaves without a statement.
fn single_ok(tally: &DoubleStopTally) -> bool {
    tally.false_alarms() <= FALSE_ALARMS_MAX && tally.singles_other == 0 && tally.octaves_told == 0
}

/// Every note of `item` announced with each of `SECOND_TONES`.
fn second_tones(set: KorpusSet, item: &session::KorpusItem, window_s: f64) -> DoubleStopTally {
    let mut tally = DoubleStopTally::default();
    for semitones in SECOND_TONES {
        let strikes = session::announce_second_tone(&item.notes, semitones);
        tally.add(&session::evaluate_double_stops(
            set,
            &item.samples,
            item.sample_rate,
            &strikes,
            window_s,
        ));
    }
    tally
}

/// Whether a line is held to the targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    Met,
    Missed,
    /// Real recordings: reported only.
    None,
}

impl Target {
    fn of(ok: bool) -> Target {
        if ok {
            Target::Met
        } else {
            Target::Missed
        }
    }

    fn label(self) -> &'static str {
        match self {
            Target::Met => "ok",
            Target::Missed => "FAILED",
            Target::None => "",
        }
    }
}

fn print_line(name: &str, t: &DoubleStopTally, target: Target) {
    let pct = |v: f64| v * 100.0;
    println!(
        "{name:<22} dyads {:>4} both {:>5.1}%  singles {:>5} both {:>4.1}% other {:>3}  \
         octaves {:>4} told {:>2}  untold {:>3} {}",
        t.dyads,
        pct(t.detected()),
        t.singles,
        pct(t.false_alarms()),
        t.singles_other,
        t.octaves,
        t.octaves_told,
        t.untold,
        target.label()
    );
}

/// `korpus double-stops`: per recording, its notes mixed in pairs (semi-real double stops) and
/// every note announced with a second tone (false alarms); reports, does not fail.
pub(crate) fn measure(dirs: &[PathBuf], window_ms: f64, per_interval: usize) -> CliResult {
    let window_s = window_ms / 1000.0;
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
    let mut total = DoubleStopTally::default();
    for (i, json) in files.iter().enumerate() {
        let item = session::read_korpus_file(json)?;
        let mixed = session::double_stop_mixtures(&item, per_interval, i as u64 + 1);
        let mut tally = session::evaluate_double_stops(
            item.set,
            &mixed.samples,
            mixed.sample_rate,
            &mixed.strikes,
            window_s,
        );
        tally.add(&second_tones(item.set, &item, window_s));
        let name = json
            .file_stem()
            .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
        print_line(&name, &tally, Target::None);
        total.add(&tally);
    }
    print_line("total", &total, Target::None);
    Ok(())
}
