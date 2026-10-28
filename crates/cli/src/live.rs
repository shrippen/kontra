//! `practice` and `tune`: live loops printing what happens.

use crate::args::{ModeArg, PracticeArgs, TuneArgs};
use crate::{instrument, load_arrangement, print_summary, CliResult};
use kontra_model::InputSource;
use kontra_session::{
    InputChoice, Live, LiveSetup, LiveTuner, LoopRange, Mode, Outcome, PracticeEvent,
    PracticeOptions, ScoreSettings, SessionEvent, Workshop, NULL_DEVICE,
};
use std::time::{Duration, Instant};

const POLL: Duration = Duration::from_millis(10);
const INPUT_END_GRACE: Duration = Duration::from_secs(2);

pub fn practice(a: &PracticeArgs) -> CliResult {
    let arrangement = load_arrangement(&a.score, a.track)?;
    let mode = match a.mode {
        ModeArg::Play => Mode::Play,
        ModeArg::Song => Mode::Song,
        ModeArg::Wait => Mode::Wait,
        ModeArg::Workshop => Mode::Workshop(Workshop {
            target_speed: a.target,
            ..Workshop::default()
        }),
    };
    let source = if a.laptop_jack {
        InputSource::LaptopJack
    } else {
        InputSource::Line
    };
    let mut settings = ScoreSettings::defaults(arrangement.instrument.kind, source);
    settings.latency_ms = a.latency_ms;
    let options = PracticeOptions {
        metronome: !a.no_metronome,
        count_in_bars: a.count_in,
        reference_tones: a.reference,
        backing_offset_s: a.backing_offset,
        transpose_semitones: 0,
    };
    let input = match &a.input_file {
        Some(path) => InputChoice::File {
            path: path.clone(),
            laptop_jack: a.laptop_jack,
        },
        None => InputChoice::Device {
            id: a.input.clone(),
            channel: None,
        },
    };
    let setup = LiveSetup {
        input,
        output_device: output_for(a.input_file.as_deref(), a.output.as_deref()),
        source,
        backing: a.backing.clone(),
        original: None,
    };
    let loop_range = a
        .r#loop
        .as_deref()
        .map(|text| parse_loop(text, &arrangement))
        .transpose()?;
    let mut live = Live::start(arrangement.clone(), mode, settings, options, &setup)
        .map_err(|e| e.to_string())?;
    if live.output_is_silent {
        println!("(no audio output found: running silently)");
    }
    live.practice_mut().set_speed(a.speed);
    if let Some(range) = loop_range {
        live.practice_mut().set_loop(Some(range));
    }
    if a.input_file.is_some() {
        if !live.play_aligned() {
            return Err("the input file did not start".into());
        }
    } else {
        live.practice_mut().play();
    }
    println!(
        "{} — {} ({:?}, {:.0} %)",
        arrangement.title,
        arrangement.artist,
        mode,
        a.speed * 100.0
    );
    let mut ended: Option<Instant> = None;
    loop {
        for event in live.poll() {
            if event == PracticeEvent::InputEnded {
                ended = Some(Instant::now());
            }
            if show(&event, &arrangement) {
                print_summary(&live.practice().session().summary(), &arrangement);
                return Ok(());
            }
        }
        // A recording that ended cannot play the rest (e.g. Wait mode still waiting).
        if ended.is_some_and(|t| t.elapsed() > INPUT_END_GRACE) {
            println!("input ended");
            print_summary(&live.practice().session().summary(), &arrangement);
            return Ok(());
        }
        std::thread::sleep(POLL);
    }
}

/// Prints one event; true when the run is over.
fn show(event: &PracticeEvent, arrangement: &kontra_model::Arrangement) -> bool {
    match event {
        PracticeEvent::Session(SessionEvent::Result(r)) => {
            let bar = r
                .bar
                .and_then(|b| arrangement.bars.get(b))
                .map_or(0, |b| b.source_index + 1);
            let what = match r.outcome {
                Outcome::Hit => "✓ hit".to_string(),
                Outcome::WrongPitch { played_midi } => format!(
                    "✗ wrong ({})",
                    kontra_model::Pitch(played_midi.round().clamp(0.0, 127.0) as u8)
                ),
                Outcome::OffPitch => format!("~ off pitch ({:+.0} ct)", r.cents.unwrap_or(0.0)),
                Outcome::Missed => "○ missed".to_string(),
                Outcome::Extra { played_midi } => format!(
                    "+ extra ({})",
                    kontra_model::Pitch(played_midi.round().clamp(0.0, 127.0) as u8)
                ),
            };
            let timing = r
                .offset_ms
                .map_or(String::new(), |o| format!(" {o:+.0} ms"));
            let cents = r.cents.map_or(String::new(), |c| format!(" {c:+.0} ct"));
            println!("bar {bar:>3}  {what}{timing}{cents}");
        }
        PracticeEvent::Session(SessionEvent::Holding { note }) => {
            if let Some(n) = arrangement.notes.get(*note) {
                println!(
                    "waiting for {} (string {}, fret {})",
                    n.pitch,
                    n.string.map_or(0, |s| s + 1),
                    n.fret.unwrap_or(0)
                );
            }
        }
        PracticeEvent::Session(SessionEvent::PassComplete {
            pass,
            clean,
            hit_rate,
        }) => {
            println!(
                "— pass {} {} ({:.0} % hits)",
                pass + 1,
                if *clean { "clean" } else { "with mistakes" },
                hit_rate * 100.0
            );
        }
        PracticeEvent::Session(SessionEvent::SpeedChanged(speed)) => {
            println!("speed {:.0} %", speed * 100.0)
        }
        PracticeEvent::Session(SessionEvent::Finished) => return true,
        PracticeEvent::Error(message) => println!("audio problem: {message}"),
        _ => {}
    }
    false
}

/// The output device: a recording as input (QA) plays silently unless `--output` names one.
fn output_for(input_file: Option<&std::path::Path>, output: Option<&str>) -> Option<String> {
    if output.is_none() && input_file.is_some() {
        return Some(NULL_DEVICE.to_string());
    }
    output.map(str::to_string)
}

/// `5-8` → loop over bars 5 to 8 (1-based, inclusive, original bar numbers of the first pass).
fn parse_loop(text: &str, arrangement: &kontra_model::Arrangement) -> Result<LoopRange, String> {
    let (from, to) = text.split_once('-').ok_or("loop must look like 5-8")?;
    let from: u32 = from
        .trim()
        .parse()
        .map_err(|_| "loop start is not a number")?;
    let to: u32 = to.trim().parse().map_err(|_| "loop end is not a number")?;
    let first = arrangement
        .bars
        .iter()
        .find(|b| b.source_index + 1 == from)
        .ok_or(format!("bar {from} not found"))?;
    let last = arrangement
        .bars
        .iter()
        .find(|b| b.source_index + 1 == to)
        .ok_or(format!("bar {to} not found"))?;
    let start_s = arrangement.tempo.seconds_at(first.start_ticks);
    let end_s = arrangement.tempo.seconds_at(last.end_ticks());
    if end_s <= start_s {
        return Err("loop end must come after its start".into());
    }
    Ok(LoopRange { start_s, end_s })
}

pub fn tune(a: &TuneArgs) -> CliResult {
    let instrument = instrument(a.strings, false)?;
    let input = match &a.input_file {
        Some(path) => InputChoice::File {
            path: path.clone(),
            laptop_jack: false,
        },
        None => InputChoice::Device {
            id: a.input.clone(),
            channel: None,
        },
    };
    let mut tuner =
        LiveTuner::start(&input, &instrument, InputSource::Line).map_err(|e| e.to_string())?;
    let end = Instant::now() + Duration::from_secs_f64(a.seconds);
    while Instant::now() < end {
        for event in tuner.poll() {
            match event {
                PracticeEvent::Tuner { note, cents } => {
                    let hint = if cents < -3.0 {
                        "tune up"
                    } else if cents > 3.0 {
                        "tune down"
                    } else {
                        "in tune"
                    };
                    println!("{note:<4} {cents:+6.1} ct  {hint}");
                }
                PracticeEvent::InputEnded => return Ok(()),
                PracticeEvent::Error(m) => println!("audio problem: {m}"),
                _ => {}
            }
        }
        std::thread::sleep(POLL);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    // QA 27.10.: "practice --input-file … --backing …" played the backing on the owner's speakers.
    #[test]
    fn a_recording_plays_silently() {
        let take = Some(Path::new("take.wav"));
        assert_eq!(output_for(take, None).as_deref(), Some(NULL_DEVICE));
        assert_eq!(output_for(take, Some("hw:1")).as_deref(), Some("hw:1"));
        assert_eq!(output_for(None, None), None);
    }
}
