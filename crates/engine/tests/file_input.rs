//! A rendered riff written to WAV and fed through the file input finds every note.

use kontra_drivers::{write_wav, AudioBuffer};
use kontra_dsp::synth::{render, SynthNote, ToneShape};
use kontra_engine::{Clock, Event, Input, InputKind, Pace};
use kontra_model::{InputSource, Instrument, Pitch, A4_HZ};
use std::sync::mpsc::channel;

#[test]
fn file_input_detects_riff() {
    let sr = 48_000.0;
    let names = ["E1", "G1", "A1", "E1", "D2", "C2"];
    let notes: Vec<SynthNote> = names
        .iter()
        .enumerate()
        .map(|(i, n)| SynthNote {
            onset_s: 0.3 + i as f64 * 0.25,
            hz: Pitch::parse(n).unwrap().hz(A4_HZ),
            duration_s: 0.24,
            velocity: 0.7,
        })
        .collect();
    let samples = render(&notes, sr, 2.2, ToneShape::Rich, 11);
    let path = std::env::temp_dir().join(format!("kontra-engine-{}.wav", std::process::id()));
    write_wav(
        &path,
        &AudioBuffer {
            sample_rate: 48_000,
            channels: vec![samples],
        },
    )
    .unwrap();

    let clock = Clock::new();
    let (tx, rx) = channel();
    let kind = InputKind::File {
        path: path.clone(),
        pace: Pace::Fast,
        laptop_jack: false,
    };
    let input = Input::start(
        &kind,
        &Instrument::default_bass(),
        InputSource::Line,
        &clock,
        tx,
    )
    .unwrap();
    let mut found = Vec::new();
    let mut levels = 0;
    for event in rx.iter() {
        match event {
            Event::Note(n) => found.push(n),
            Event::Level { .. } => levels += 1,
            Event::Ended => break,
            _ => {}
        }
    }
    drop(input);
    std::fs::remove_file(path).unwrap();
    assert!(levels > 30);
    assert_eq!(found.len(), names.len());
    let start = found[0].onset_s - 0.3;
    for (f, n) in found.iter().zip(&notes) {
        assert!(((f.onset_s - start) - n.onset_s).abs() < 0.006);
        assert_eq!(
            f.midi.round() as u8,
            Pitch::parse(&format!("{}", Pitch::nearest(n.hz, A4_HZ).unwrap().0))
                .unwrap()
                .0
        );
    }
}
