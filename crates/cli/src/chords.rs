//! `korpus chords` and the chord line of `korpus check` (M2): chords scored as whole sets.

use crate::CliResult;
use kontra_session as session;
use kontra_session::ChordTally;
use std::path::PathBuf;

/// A rolled chord's tones lie up to 25 ms apart (`generate_midi_chords`).
const GENERATED_WINDOW_S: f64 = 0.03;

/// Prints the generated MIDI set's line of `korpus check`; true when every chord is complete
/// (MIDI tells each key exactly).
pub(crate) fn check() -> bool {
    let (notes, detected) = session::generate_midi_chords();
    let tally = session::score_chords(&notes, &detected, GENERATED_WINDOW_S);
    let ok = tally.chords > 0 && tally.complete == tally.chords;
    print_line("chords (MIDI)", &tally, if ok { "ok" } else { "FAILED" });
    ok
}

/// Per take (`.json` + `.mid`) and in total, or the generated set without `dirs`; reports,
/// does not fail.
pub(crate) fn measure(dirs: &[PathBuf], window_s: f64) -> CliResult {
    if dirs.is_empty() {
        let (notes, detected) = session::generate_midi_chords();
        print_line(
            "generated (MIDI)",
            &session::score_chords(&notes, &detected, window_s),
            "",
        );
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
    let mut total = ChordTally::default();
    for json in files.iter().filter(|j| j.with_extension("mid").exists()) {
        let (notes, detected) = session::read_midi_korpus(json)?;
        let tally = session::score_chords(&notes, &detected, window_s);
        let name = json
            .file_stem()
            .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
        print_line(&name, &tally, "");
        total.add(&tally);
    }
    print_line("total", &total, "");
    Ok(())
}

fn print_line(name: &str, t: &ChordTally, verdict: &str) {
    println!(
        "{name:<22} chords {:>4} tones {:>5} complete {:>5.1}%  missing {:>4} extra {:>4} {verdict}",
        t.chords,
        t.tones,
        t.complete_share() * 100.0,
        t.missing,
        t.extra
    );
}
