//! `kontra-cli`: Kontra's functions on the command line, for development, QA and scripting.
//! Talks only to `kontra-session` (the layer below the UI).

mod args;
mod korpus;
mod live;
mod tab;

use args::{Cli, Command};
use clap::Parser;
use kontra_model::{InputSource, Instrument, InstrumentKind, Tuning};
use kontra_session as session;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Devices => devices(),
        Command::Import(a) => import(&a),
        Command::Analyze(a) => analyze(&a),
        Command::Score(a) => score(&a),
        Command::Korpus(a) => korpus::run(&a),
        Command::Practice(a) => live::practice(&a),
        Command::Tune(a) => live::tune(&a),
        Command::Render(a) => render(&a),
        Command::Soak(a) => soak(&a),
        Command::Exercises(a) => exercises(&a),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("kontra-cli: {message}");
            ExitCode::FAILURE
        }
    }
}

/// Errors are shown to people, so they are plain strings.
pub(crate) type CliResult = Result<(), String>;

fn devices() -> CliResult {
    for direction in [session::Direction::Input, session::Direction::Output] {
        println!("{direction:?}:");
        let list = session::list_devices(direction);
        if list.is_empty() {
            println!("  (none found)");
        }
        for d in list {
            let mark = if d.is_default { "*" } else { " " };
            println!(
                "{mark} {} — {:?}, {} Hz, {} ch  [{}]",
                d.name, d.connection, d.sample_rate, d.channels, d.id
            );
        }
    }
    Ok(())
}

pub(crate) fn instrument(strings: u8, fretless: bool) -> Result<Instrument, String> {
    let tuning = match strings {
        4 => Tuning::bass4_standard(),
        5 => Tuning::bass5_standard(),
        6 => Tuning::bass6_standard(),
        other => return Err(format!("{other} strings are not supported (4, 5 or 6)")),
    };
    let kind = if fretless {
        InstrumentKind::Fretless
    } else {
        InstrumentKind::Fretted
    };
    Instrument::new(kind, tuning, 24).map_err(|e| e.to_string())
}

pub(crate) fn source(laptop_jack: bool) -> InputSource {
    if laptop_jack {
        InputSource::LaptopJack
    } else {
        InputSource::Line
    }
}

pub(crate) fn load_arrangement(
    path: &std::path::Path,
    track: Option<usize>,
) -> Result<kontra_model::Arrangement, String> {
    let imported = session::import_file(path).map_err(|e| e.to_string())?;
    let track = track
        .or_else(|| imported.suggested())
        .ok_or("no bass track found; choose one with --track (see `import`)")?;
    imported.arrangement(track).map_err(|e| e.to_string())
}

fn import(a: &args::ImportArgs) -> CliResult {
    let imported = session::import_file(&a.file).map_err(|e| e.to_string())?;
    println!("{} — {}", imported.title(), imported.artist());
    for (i, t) in imported.tracks().iter().enumerate() {
        let bass = if t.is_bass() { "bass" } else { "" };
        let pick = if imported.suggested() == Some(i) {
            "→"
        } else {
            " "
        };
        println!("{pick} [{i}] {} ({} strings) {bass}", t.name(), t.strings());
    }
    let Some(track) = a.track.or_else(|| imported.suggested()) else {
        return Err("no bass track recognized; choose one with --track".into());
    };
    let arrangement = imported.arrangement(track).map_err(|e| e.to_string())?;
    println!(
        "track {track}: {} notes, {} bars, {:.1} s, tempo {:.0} BPM, tuning {}",
        arrangement.notes.len(),
        arrangement.bars.len(),
        arrangement.duration_s(),
        arrangement.tempo.bpm_at(0),
        arrangement
            .instrument
            .tuning
            .strings
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    );
    print!("{}", tab::render(&arrangement, a.bars));
    if let Some(out) = &a.json {
        let text = serde_json::to_string_pretty(&arrangement).map_err(|e| e.to_string())?;
        std::fs::write(out, text).map_err(|e| format!("{}: {e}", out.display()))?;
        println!("written: {}", out.display());
    }
    Ok(())
}

/// Writes the bundled exercise set (LH-BIB-08); `testdata/exercises` is its checked-in output.
fn exercises(a: &args::ExercisesArgs) -> CliResult {
    for (level, file, exercise) in kontra_session::exercise::bundled() {
        let dir = a.dir.join(level);
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let path = dir.join(file);
        std::fs::write(&path, &exercise.midi).map_err(|e| format!("{}: {e}", path.display()))?;
        println!("{}", path.display());
    }
    Ok(())
}

fn render(a: &args::RenderArgs) -> CliResult {
    let arrangement = load_arrangement(&a.score, a.track)?;
    let mistakes = session::Mistakes {
        offset_ms: a.offset_ms,
        wrong_every: a.wrong_every,
        miss_every: a.miss_every,
        detune_cents: a.detune_cents,
        playing: if a.techniques {
            session::Playing::Articulated {
                plain_every: a.plain_every,
            }
        } else {
            session::Playing::Plucked
        },
        ..Default::default()
    };
    let samples = session::render_performance(&arrangement, a.speed, a.lead_in, mistakes, 48_000);
    session::write_mono_wav(&a.out, &samples, 48_000).map_err(|e| e.to_string())?;
    println!(
        "written: {} ({:.1} s)",
        a.out.display(),
        samples.len() as f64 / 48_000.0
    );
    Ok(())
}

/// LH-NF-04: plays for `minutes` and fails above `max_xruns` dropouts.
fn soak(a: &args::SoakArgs) -> CliResult {
    const RATE: u32 = 48_000;
    let arrangement = load_arrangement(&a.score, None)?;
    let generated = std::env::temp_dir().join(format!("kontra-soak-{}.wav", std::process::id()));
    let backing = match &a.backing {
        Some(path) => path.clone(),
        None => {
            let samples = session::render_performance(
                &arrangement,
                1.0,
                0.0,
                session::Mistakes::default(),
                RATE,
            );
            session::write_mono_wav(&generated, &samples, RATE).map_err(|e| e.to_string())?;
            generated.clone()
        }
    };
    let output = match a.output.as_str() {
        "null" => session::NULL_DEVICE,
        id => id,
    };
    println!(
        "soak: {:.1} min at {:.0} % on {output} …",
        a.minutes,
        a.speed * 100.0
    );
    let report = session::soak(
        arrangement,
        Some(&backing),
        a.speed,
        a.minutes * 60.0,
        Some(output),
    );
    let _ = std::fs::remove_file(&generated);
    let report = report.map_err(|e| e.to_string())?;
    println!(
        "xruns: {} in {:.1} min ({} passes)",
        report.xruns,
        report.seconds / 60.0,
        report.passes
    );
    if report.xruns > a.max_xruns {
        return Err(format!("{} xruns (allowed {})", report.xruns, a.max_xruns));
    }
    Ok(())
}

fn analyze(a: &args::AnalyzeArgs) -> CliResult {
    let instrument = instrument(a.strings, a.fretless)?;
    let notes = session::analyze_file(&a.file, &instrument, source(a.laptop_jack), a.laptop_jack)
        .map_err(|e| e.to_string())?;
    if a.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&notes).map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    println!(
        "{:>9}  {:<5} {:>7}  {:>5}  {:>7}",
        "onset s", "note", "cents", "conf", "level"
    );
    for n in &notes {
        let pitch = kontra_model::Pitch(n.midi.round().clamp(0.0, 127.0) as u8);
        let cents = (n.midi - n.midi.round()) * 100.0;
        println!(
            "{:>9.3}  {:<5} {:>+7.1}  {:>5.2}  {:>5.1} dB",
            n.onset_s,
            pitch.to_string(),
            cents,
            n.confidence,
            n.level_db
        );
    }
    println!("{} notes", notes.len());
    Ok(())
}

fn score(a: &args::ScoreArgs) -> CliResult {
    let arrangement = load_arrangement(&a.score, a.track)?;
    let notes = session::analyze_file(
        &a.recording,
        &arrangement.instrument,
        source(a.laptop_jack),
        a.laptop_jack,
    )
    .map_err(|e| e.to_string())?;
    let mut settings =
        session::ScoreSettings::defaults(arrangement.instrument.kind, source(a.laptop_jack));
    settings.strictness = a.strictness.into();
    settings.latency_ms = a.latency_ms;
    let scored = session::score_notes(
        &arrangement,
        &notes,
        session::Mode::Song,
        settings,
        a.speed,
        a.offset,
    );
    print_summary(&scored.summary, &arrangement);
    Ok(())
}

pub(crate) fn print_summary(s: &session::Summary, arrangement: &kontra_model::Arrangement) {
    println!(
        "hit rate {:.0} %  ({} of {}; {} wrong, {} missed, {} extra)",
        s.hit_rate * 100.0,
        s.hits,
        s.judged,
        s.wrong,
        s.missed,
        s.extra
    );
    if let (Some(mean), Some(spread)) = (s.mean_offset_ms, s.spread_ms) {
        let tendency = if mean < -5.0 {
            format!("mostly {:.0} ms early", -mean)
        } else if mean > 5.0 {
            format!("mostly {mean:.0} ms late")
        } else {
            "on time".to_string()
        };
        println!("timing: {tendency}, spread ±{spread:.0} ms");
    }
    if let Some(cents) = s.mean_abs_cents {
        println!("intonation: {cents:.1} cents average deviation");
    }
    let max = s.histogram.counts.iter().copied().max().unwrap_or(0).max(1);
    for (i, count) in s.histogram.counts.iter().enumerate() {
        let from = s.histogram.first_bin_ms + i as f64 * s.histogram.bin_ms;
        let bar = "█".repeat((*count as usize * 30).div_ceil(max as usize));
        println!("  {from:>+5.0} ms {bar} {count}");
    }
    if !s.hardest_bars.is_empty() {
        let bars: Vec<String> = s
            .hardest_bars
            .iter()
            .map(|b| {
                arrangement
                    .bars
                    .get(*b)
                    .map_or(b + 1, |bar| bar.source_index as usize + 1)
                    .to_string()
            })
            .collect();
        println!("hardest bars: {}", bars.join(", "));
    }
}
