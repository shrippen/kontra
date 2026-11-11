//! A mode takes over a running input (Runde 20, audio engine across modes): `Input::rebase`
//! starts the analysis over, the worker announces it with `Event::Rebased`, and the stamps go
//! on where they were.

use kontra_drivers::{write_wav, AudioBuffer};
use kontra_dsp::synth::{render, SynthNote, ToneShape};
use kontra_engine::{Clock, Event, Input, InputKind, Pace};
use kontra_model::{InputSource, Instrument, Pitch, A4_HZ};
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};

const RATE: f64 = 48_000.0;
/// When the next mode attaches, in file time: between the second and the third note.
const REBASE_AT_S: f64 = 1.3;
/// How far a stamp may be off (the file test allows the same).
const TOLERANCE_S: f64 = 0.006;
/// Ample time for a file's worker to return after it told the end.
const WORKER_END: Duration = Duration::from_millis(200);

#[test]
fn a_rebased_input_goes_on_with_its_stamps() {
    let onsets = [0.3, 0.8, 2.0, 2.5];
    let notes: Vec<SynthNote> = onsets
        .iter()
        .map(|&onset_s| SynthNote {
            onset_s,
            hz: Pitch::parse("A1").unwrap().hz(A4_HZ),
            duration_s: 0.3,
            velocity: 0.7,
        })
        .collect();
    let samples = render(&notes, RATE, 3.0, ToneShape::Rich, 7);
    let path = std::env::temp_dir().join(format!("kontra-rebase-{}.wav", std::process::id()));
    write_wav(
        &path,
        &AudioBuffer {
            sample_rate: RATE as u32,
            channels: vec![samples],
        },
    )
    .unwrap();

    let clock = Clock::new();
    let (tx, rx) = channel();
    let kind = InputKind::File {
        path: path.clone(),
        pace: Pace::RealTime,
        laptop_jack: false,
    };
    let bass = Instrument::default_bass();
    let mut input = Input::start(&kind, &bass, InputSource::Line, &clock, tx, None).unwrap();

    // The next mode attaches while the file plays.
    let wait = Instant::now();
    let start_s = loop {
        if let Some(start) = input.started_s() {
            break start;
        }
        assert!(
            wait.elapsed() < Duration::from_secs(5),
            "the file did not start"
        );
        std::thread::sleep(Duration::from_millis(1));
    };
    while clock.now_s() < start_s + REBASE_AT_S {
        std::thread::sleep(Duration::from_millis(1));
    }
    let epoch = input.rebase(&bass, InputSource::Line).expect("audio input");
    assert_eq!(epoch, 1);

    let mut before = Vec::new();
    let mut after = Vec::new();
    let mut rebased = false;
    for event in rx.iter() {
        match event {
            Event::Note(n) if rebased => after.push(n.onset_s - start_s),
            Event::Note(n) => before.push(n.onset_s - start_s),
            Event::Rebased { epoch: told } => {
                assert_eq!(told, epoch);
                rebased = true;
            }
            Event::Ended => break,
            _ => {}
        }
    }
    drop(input);
    let _ = std::fs::remove_file(path);

    assert!(rebased, "the worker announces the rebase");
    assert_eq!(before.len(), 2, "{before:?}");
    assert_eq!(after.len(), 2, "{after:?}");
    for (heard, want) in before.iter().chain(&after).zip(onsets) {
        assert!((heard - want).abs() < TOLERANCE_S, "{heard} for {want}");
    }
}

/// Review #286: a file that has ended runs no analysis any more; a rebase there must say so
/// instead of promising an `Event::Rebased` that never comes.
#[test]
fn an_ended_file_takes_no_rebase() {
    let samples = vec![0.0f32; RATE as usize / 10];
    let path = std::env::temp_dir().join(format!("kontra-ended-{}.wav", std::process::id()));
    write_wav(
        &path,
        &AudioBuffer {
            sample_rate: RATE as u32,
            channels: vec![samples],
        },
    )
    .unwrap();
    let (tx, rx) = channel();
    let kind = InputKind::File {
        path: path.clone(),
        pace: Pace::Fast,
        laptop_jack: false,
    };
    let bass = Instrument::default_bass();
    let mut input = Input::start(&kind, &bass, InputSource::Line, &Clock::new(), tx, None).unwrap();
    for event in rx.iter() {
        if matches!(event, Event::Ended) {
            break;
        }
    }
    // The worker ends right after it told the end; the first rebase after that is refused
    // (a full ring of four would refuse only the fifth).
    std::thread::sleep(WORKER_END);
    assert_eq!(input.rebase(&bass, InputSource::Line), None);
    drop(input);
    let _ = std::fs::remove_file(path);
}
