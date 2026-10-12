//! `check`: the hardware check (round 10). Guides through the measurements Kontra can take
//! itself and writes every result into a report, with the takes as corpus pairs:
//!
//! ```text
//! check-<date>/
//!   report.md, report.json    loopback, calibration, takes, soak, stems, manual checklist
//!   takes/<name>.wav + .json  real bass for `korpus measure` / `korpus techniques`
//! ```

use crate::args::{CheckArgs, CheckStep};
use crate::{load_arrangement, CliResult};
use kontra_model::{Instrument, Technique};
use kontra_session as session;
use serde_json::{json, Map, Value};
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const POLL: Duration = Duration::from_millis(10);
/// Where takes go by default: the gitignored corpus folder for own recordings.
const OWN_KORPUS: &str = "testdata/korpus/local/own";
/// A detection counts for a note within this distance (score-grid annotation, as IDMT).
const MATCH_WINDOW_S: f64 = 0.08;
/// LH-NF-01: stroke to display with an interface; the round trip is an upper bound for it.
const INTERFACE_LIMIT_MS: f64 = 30.0;
/// Below this peak a take is too quiet to judge (gain too low).
const QUIET_DBFS: f32 = -30.0;
/// LH-BIB-09: a 4-minute song separated in under 3 minutes.
const STEMS_LIMIT_S: f64 = 180.0;
/// Soak speed: time-stretched like a slowed-down practice (as `soak`).
const SOAK_SPEED: f64 = 0.8;
const SECONDS_PER_DAY: u64 = 86_400;

/// What only a person can check; listed in the report to be ticked off by hand.
const MANUAL: [&str; 7] = [
    "Sound: EQ and warmth of the reference tones and the backing on the practice amp or headphones",
    "Monitoring latency: own bass through Kontra, playing feels direct (no echo)",
    "Transpose a band recording while playing (J-D1): no clicks, pitch right",
    "Beamer 1280×720 at 150 % (P4 Thomas): tab lane and evaluation readable from the back",
    "Two MIDI keyboards: choose one under Connection, unplug it, Kontra says it took the other",
    "Microphone with speakers (J-E2): band from the speakers, bass notes still recognised",
    "Unplug the interface while playing: Kontra pauses and says so",
];

/// Answer to a prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Answer {
    Go,
    Skip,
}

/// What the report collects: Markdown lines and the same as JSON.
struct Report {
    md: Vec<String>,
    json: Map<String, Value>,
}

impl Report {
    fn line(&mut self, text: String) {
        println!("{text}");
        self.md.push(text);
    }

    fn section(&mut self, title: &str) {
        println!();
        self.md.push(String::new());
        self.line(format!("## {title}"));
    }
}

pub fn run(a: &CheckArgs) -> CliResult {
    let out = a.out.clone().unwrap_or_else(default_dir);
    let takes_dir = out.join("takes");
    std::fs::create_dir_all(&takes_dir).map_err(|e| format!("{}: {e}", takes_dir.display()))?;
    let setup = setup(a);
    let instrument = Instrument::default_bass();
    let steps = if a.steps.is_empty() {
        vec![CheckStep::Loopback, CheckStep::Calibrate, CheckStep::Takes]
    } else {
        a.steps.clone()
    };

    let mut report = Report {
        md: vec![format!("# Kontra hardware check {}", stamp())],
        json: Map::new(),
    };
    devices(&mut report, a);
    let mut latency_ms = a.latency_ms;
    for step in steps {
        match step {
            CheckStep::Loopback => loopback(&mut report, a, &setup, &instrument)?,
            CheckStep::Calibrate => {
                if let Some(measured) = calibrate(&mut report, a, &setup, &instrument)? {
                    latency_ms = measured;
                }
            }
            CheckStep::Takes => takes(&mut report, a, &setup, &instrument, &takes_dir, latency_ms)?,
            CheckStep::Soak => soak(&mut report, a)?,
            CheckStep::Stems => stems(&mut report, a, &out)?,
        }
    }
    manual(&mut report);

    let md = out.join("report.md");
    std::fs::write(&md, report.md.join("\n") + "\n")
        .map_err(|e| format!("{}: {e}", md.display()))?;
    let json = out.join("report.json");
    let text =
        serde_json::to_string_pretty(&Value::Object(report.json)).map_err(|e| e.to_string())?;
    std::fs::write(&json, text).map_err(|e| format!("{}: {e}", json.display()))?;
    println!("\nreport: {}", md.display());
    Ok(())
}

fn setup(a: &CheckArgs) -> session::LiveSetup {
    let source = crate::source(a.laptop_jack);
    let input = match &a.input_file {
        Some(path) => session::InputChoice::File {
            path: path.clone(),
            laptop_jack: a.laptop_jack,
        },
        None => session::InputChoice::Device {
            id: a.input.clone(),
            channel: None,
        },
    };
    session::LiveSetup {
        input,
        output_device: a.output.clone(),
        source,
        backing: None,
    }
}

/// Waits for Enter; "s" skips. `--yes` never waits.
fn ask(a: &CheckArgs, prompt: &str) -> Answer {
    println!("{prompt}");
    if a.yes {
        return Answer::Go;
    }
    println!("  Enter: go, s: skip");
    let mut line = String::new();
    let _ = std::io::stdin().lock().read_line(&mut line);
    if line.trim().eq_ignore_ascii_case("s") {
        return Answer::Skip;
    }
    Answer::Go
}

/// The devices the check runs on.
fn devices(report: &mut Report, a: &CheckArgs) {
    report.section("Devices");
    let name = |direction: session::Direction, id: &Option<String>| -> String {
        let list = session::list_devices(direction);
        let found = list.iter().find(|d| match id {
            Some(id) => &d.id == id,
            None => d.is_default,
        });
        found.map_or_else(
            || id.clone().unwrap_or_else(|| "default".into()),
            |d| format!("{} ({:?}, {} Hz)", d.name, d.connection, d.sample_rate),
        )
    };
    let input = match &a.input_file {
        Some(path) => format!("file {}", path.display()),
        None => name(session::Direction::Input, &a.input),
    };
    let output = name(session::Direction::Output, &a.output);
    report.line(format!("- input: {input}"));
    report.line(format!("- output: {output}"));
    report.line(format!("- laptop jack: {}", a.laptop_jack));
    report.line(format!("- Kontra {}", env!("CARGO_PKG_VERSION")));
    report.json.insert(
        "devices".into(),
        json!({ "input": input, "output": output, "laptopJack": a.laptop_jack }),
    );
}

/// Runs a take to its end.
fn record(
    setup: &session::LiveSetup,
    instrument: &Instrument,
    plan: Vec<(f64, session::TakeSound)>,
    length_s: f64,
) -> Result<session::Take, String> {
    let mut live =
        session::LiveTake::start(setup, instrument, plan, length_s).map_err(|e| e.to_string())?;
    if live.output_is_silent() {
        println!("  (no audio output found: nothing is heard)");
    }
    while !live.poll() {
        std::thread::sleep(POLL);
    }
    Ok(live.finish())
}

fn loopback(
    report: &mut Report,
    a: &CheckArgs,
    setup: &session::LiveSetup,
    instrument: &Instrument,
) -> CliResult {
    report.section("Loopback latency");
    let prompt = "Connect the output to the input with a cable (monitoring off, volume half).";
    if ask(a, prompt) == Answer::Skip {
        report.line("skipped".into());
        return Ok(());
    }
    let plan = session::loopback_plan();
    let take = record(
        setup,
        instrument,
        plan.clone(),
        session::loopback_length_s(),
    )?;
    let block = take
        .block_frames
        .map_or("unknown".into(), |b| b.to_string());
    let Some(trip) = session::round_trip(&plan, &take) else {
        report.line("no tone came back: is the cable in, the output loud enough?".into());
        report.json.insert("loopback".into(), Value::Null);
        return Ok(());
    };
    let verdict = if trip.median_ms < INTERFACE_LIMIT_MS {
        "under"
    } else {
        "over"
    };
    report.line(format!(
        "- round trip {:.1} ms (±{:.1} ms, {} of {} tones), {verdict} the {INTERFACE_LIMIT_MS:.0} ms of LH-NF-01",
        trip.median_ms, trip.spread_ms, trip.heard, trip.sent
    ));
    report.line(format!("- block {block} frames, {} xruns", take.xruns));
    report.json.insert(
        "loopback".into(),
        json!({
            "roundTripMs": trip.median_ms,
            "spreadMs": trip.spread_ms,
            "heard": trip.heard,
            "sent": trip.sent,
            "blockFrames": take.block_frames,
            "xruns": take.xruns,
        }),
    );
    Ok(())
}

/// "Play on the click" with the bass; the measured latency places the takes' annotations.
fn calibrate(
    report: &mut Report,
    a: &CheckArgs,
    setup: &session::LiveSetup,
    instrument: &Instrument,
) -> Result<Option<f64>, String> {
    report.section("Play on the click (LH-EIN-05)");
    let prompt = "Bass into the input. Play an open string on each of the 8 clicks.";
    if ask(a, prompt) == Answer::Skip {
        report.line("skipped".into());
        return Ok(None);
    }
    let mut live =
        session::LiveCalibration::start(setup, instrument, 1.0).map_err(|e| e.to_string())?;
    let result = loop {
        if let Some(result) = live.poll() {
            break result;
        }
        std::thread::sleep(POLL);
    };
    report.json.insert("calibration".into(), json!(result));
    match result {
        session::CalibrationResult::Measured {
            latency_ms,
            spread_ms,
        } => {
            report.line(format!("- latency {latency_ms:.1} ms (±{spread_ms:.1} ms)"));
            Ok(Some(latency_ms))
        }
        session::CalibrationResult::TooFew { matched } => {
            report.line(format!("- too few plucks heard ({matched} of 8)"));
            Ok(None)
        }
        session::CalibrationResult::Unsteady { spread_ms } => {
            report.line(format!("- too uneven to measure (±{spread_ms:.0} ms)"));
            Ok(None)
        }
    }
}

fn takes(
    report: &mut Report,
    a: &CheckArgs,
    setup: &session::LiveSetup,
    instrument: &Instrument,
    dir: &Path,
    latency_ms: f64,
) -> CliResult {
    report.section("Bass takes (corpus, LH-BEW-06)");
    report.line(format!(
        "{:.0} BPM, 4 clicks count-in; annotations at the beat + {latency_ms:.1} ms; ±{:.0} ms",
        session::CHECK_BPM,
        MATCH_WINDOW_S * 1000.0
    ));
    report.line(String::new());
    report.line("| take | notes | found | pitch | technique | peak | file |".into());
    report.line("|---|---|---|---|---|---|---|".into());
    let mut rows = Vec::new();
    for take in session::check_takes() {
        if !a.takes.is_empty() && !a.takes.iter().any(|t| t == take.name) {
            continue;
        }
        let prompt = format!("\n{}: {}", take.name, describe(&take));
        if ask(a, &prompt) == Answer::Skip {
            report.line(format!("| {} | skipped | | | | | |", take.name));
            continue;
        }
        let heard = record(setup, instrument, take.plan(), take.length_s())?;
        rows.push(judge(report, &take, &heard, dir, latency_ms)?);
    }
    report.json.insert("takes".into(), Value::Array(rows));
    Ok(())
}

/// Writes a take as a corpus pair and measures detection on it, as `korpus measure` does.
fn judge(
    report: &mut Report,
    take: &session::CheckTake,
    heard: &session::Take,
    dir: &Path,
    latency_ms: f64,
) -> Result<Value, String> {
    let notes = take.annotate(heard, latency_ms);
    let wav = dir.join(format!("{}.wav", take.name));
    session::write_mono_wav(&wav, &heard.samples, heard.sample_rate).map_err(|e| e.to_string())?;
    let item = session::KorpusItem {
        set: session::KorpusSet::Line4,
        sample_rate: heard.sample_rate,
        notes: notes.clone(),
        samples: Vec::new(),
    };
    let json_path = wav.with_extension("json");
    let text = serde_json::to_string_pretty(&item).map_err(|e| e.to_string())?;
    std::fs::write(&json_path, text).map_err(|e| format!("{}: {e}", json_path.display()))?;

    let matching = session::Matching {
        window_s: MATCH_WINDOW_S,
        require_pitch: false,
    };
    let (e, _) = session::evaluate_korpus_with(
        item.set,
        &heard.samples,
        heard.sample_rate,
        &notes,
        matching,
    );
    let peak = peak_dbfs(&heard.samples);
    let quiet = if peak < QUIET_DBFS {
        " (too quiet)"
    } else {
        ""
    };
    let technique = if take.notes.iter().any(|n| n.technique.is_some()) {
        format!("{}/{}", e.technique_correct, e.matched)
    } else {
        "–".into()
    };
    report.line(format!(
        "| {} | {} | {} | {}/{} | {technique} | {peak:.0} dBFS{quiet} | {} |",
        take.name,
        e.annotated,
        e.matched,
        e.pitch_correct,
        e.matched,
        wav.display()
    ));
    Ok(json!({
        "name": take.name,
        "annotated": e.annotated,
        "detected": e.detected,
        "matched": e.matched,
        "pitchCorrect": e.pitch_correct,
        "techniqueCorrect": e.technique_correct,
        "meanAbsOnsetMs": e.mean_abs_onset_ms,
        "peakDbfs": peak,
        "xruns": heard.xruns,
        "file": wav,
    }))
}

/// What to play, as tab: "A0 hA2 A0 hA2 …" (string letter, fret; h, p, sl, S, P, x).
fn describe(take: &session::CheckTake) -> String {
    const STRINGS: [&str; 4] = ["E", "A", "D", "G"];
    let notes: Vec<String> = take
        .notes
        .iter()
        .map(|n| {
            let sign = match n.technique {
                Some(Technique::HammerOn) => "h",
                Some(Technique::PullOff) => "p",
                Some(Technique::Slide) => "sl",
                Some(Technique::Slap) => "S",
                Some(Technique::Pop) => "P",
                Some(Technique::Muted) => "x",
                _ => "",
            };
            let string = STRINGS.get(usize::from(n.string)).copied().unwrap_or("?");
            format!("{sign}{string}{}", n.fret)
        })
        .collect();
    let eighths = take.notes.iter().any(|n| n.beat.fract() != 0.0);
    let rhythm = if eighths { "eighths" } else { "quarters" };
    format!("{} ({rhythm} after 4 clicks)", notes.join(" "))
}

fn soak(report: &mut Report, a: &CheckArgs) -> CliResult {
    report.section("Soak (LH-NF-04)");
    let prompt = format!(
        "{:.0} minutes of playback on the output; nothing to do but listen for dropouts.",
        a.soak_minutes
    );
    if ask(a, &prompt) == Answer::Skip {
        report.line("skipped".into());
        return Ok(());
    }
    let arrangement = load_arrangement(&a.soak_score, None)?;
    let output = a.output.as_deref();
    let result = crate::run_soak(arrangement, None, SOAK_SPEED, a.soak_minutes, output)?;
    report.line(format!(
        "- {} xruns in {:.1} min ({} passes, {:.0} %)",
        result.xruns,
        result.seconds / 60.0,
        result.passes,
        SOAK_SPEED * 100.0
    ));
    report.json.insert(
        "soak".into(),
        json!({ "xruns": result.xruns, "seconds": result.seconds, "passes": result.passes }),
    );
    Ok(())
}

fn stems(report: &mut Report, a: &CheckArgs, out: &Path) -> CliResult {
    report.section("Stem separation (LH-BIB-09)");
    let Some(song) = &a.song else {
        report.line("skipped: no --song".into());
        return Ok(());
    };
    let removal = session::BassRemoval::new(&out.join("stems-cache"));
    let Some(started) = removal.start(song) else {
        report.line("Demucs not found".into());
        return Ok(());
    };
    let mut job = started.map_err(|e| e.to_string())?;
    let clock = Instant::now();
    let result = loop {
        if let Some(result) = job.poll() {
            break result;
        }
        std::thread::sleep(Duration::from_millis(500));
    };
    let seconds = clock.elapsed().as_secs_f64();
    match result {
        Ok(path) => {
            let verdict = if seconds < STEMS_LIMIT_S {
                "under"
            } else {
                "over"
            };
            report.line(format!(
                "- {seconds:.0} s for {}, {verdict} the 3 min of a 4-minute song ({})",
                song.display(),
                path.display()
            ));
        }
        Err(e) => report.line(format!("- failed after {seconds:.0} s: {e}")),
    }
    report
        .json
        .insert("stems".into(), json!({ "seconds": seconds, "song": song }));
    Ok(())
}

/// Peak level in dBFS; silence reads very low, not minus infinity.
fn peak_dbfs(samples: &[f32]) -> f32 {
    let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    20.0 * peak.max(1e-9).log10()
}

fn manual(report: &mut Report) {
    report.section("By hand");
    for item in MANUAL {
        report.line(format!("- [ ] {item}"));
    }
    report.json.insert("manual".into(), json!(MANUAL));
}

fn default_dir() -> PathBuf {
    PathBuf::from(OWN_KORPUS).join(format!("check-{}", stamp().replace([' ', ':'], "-")))
}

/// "2026-10-12 07:41" (UTC) without a date library.
fn stamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let (y, m, d) = civil_from_days((secs / SECONDS_PER_DAY) as i64);
    let rest = secs % SECONDS_PER_DAY;
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}",
        rest / 3600,
        rest % 3600 / 60
    )
}

/// Days since 1970-01-01 to (year, month, day), proleptic Gregorian (H. Hinnant's algorithm).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_from_days() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        // 12 Oct 2026.
        assert_eq!(civil_from_days(20_738), (2026, 10, 12));
        assert_eq!(civil_from_days(11_016), (2000, 2, 29));
    }

    #[test]
    fn takes_read_as_tab() {
        let takes = session::check_takes();
        let hammer = takes.iter().find(|t| t.name == "hammer-on").unwrap();
        assert!(
            describe(hammer).starts_with("A0 hA2 A0 hA2"),
            "{}",
            describe(hammer)
        );
        let dead = takes.iter().find(|t| t.name == "dead").unwrap();
        assert!(describe(dead).starts_with("xA0 xA0"), "{}", describe(dead));
    }
}
