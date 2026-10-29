//! Mains hum under a line input (LH-NF-16, F3): noise on it is no note with the hum's pitch, a
//! note played on it keeps its own.

use kontra_dsp::synth::{add_noise, render, Noise, SynthNote, ToneShape};
use kontra_dsp::{NoteTracker, TrackerConfig};
use kontra_model::{hz_to_midi, DetectedNote, InputSource, Instrument, Pitch, A4_HZ};

const SR: f64 = 48_000.0;
const HUM_HZ: f64 = 50.0;
/// Hum at −50 dBFS, as in the corpus set idle-line.
const HUM_AMPLITUDE: f32 = 0.003;

/// `len_s` of hum with a little noise.
fn hum(len_s: f64) -> Vec<f32> {
    let mut signal: Vec<f32> = (0..(len_s * SR) as usize)
        .map(|i| {
            let t = i as f64 / SR;
            HUM_AMPLITUDE * (std::f64::consts::TAU * HUM_HZ * t).sin() as f32
        })
        .collect();
    add_noise(&mut signal, -66.0, 3);
    signal
}

fn track(signal: &[f32]) -> Vec<DetectedNote> {
    let config = TrackerConfig::for_instrument(&Instrument::default_bass(), InputSource::Line);
    let mut tracker = NoteTracker::new(SR, config);
    let mut found = Vec::new();
    for block in signal.chunks(480) {
        found.extend_from_slice(tracker.push(block));
    }
    found.extend_from_slice(tracker.flush());
    found
}

/// A click (3 ms), a scrape (120 ms of hiss) and a hand on the strings (30 ms, low): before the
/// fix each came out as a note at MIDI 31.35, the hum's pitch.
#[test]
fn noise_on_hum_has_no_pitch() {
    let mut signal = hum(4.0);
    let mut noise = Noise::new(7);
    let mut low = 0.0f32;
    let k = (std::f64::consts::TAU * 200.0 / SR) as f32;
    // (at s, length s, amplitude, low-passed)
    let sounds = [
        (1.0, 0.003, 0.3, false),
        (2.0, 0.12, 0.03, false),
        (3.0, 0.03, 1.5, true),
    ];
    for (at_s, len_s, amplitude, dull) in sounds {
        let at = (at_s * SR) as usize;
        let len = (len_s * SR) as usize;
        for (i, s) in signal[at..at + len].iter_mut().enumerate() {
            let fade = 1.0 - i as f32 / len as f32;
            low += k * (noise.sample() - low);
            let x = if dull { low } else { noise.sample() };
            *s += amplitude * fade * x;
        }
    }

    let found = track(&signal);
    assert!(!found.is_empty(), "the attacks are heard");
    assert!(found.iter().all(|n| n.midi == 0.0), "{found:?}");
}

/// G1 (49 Hz) lies 35 ct below the hum: played on it, it is a note with its own pitch.
#[test]
fn note_on_hum_keeps_its_pitch() {
    let hz = Pitch::parse("G1").unwrap().hz(A4_HZ);
    let note = SynthNote {
        onset_s: 1.0,
        hz,
        duration_s: 0.8,
        velocity: 0.7,
    };
    let tone = render(&[note], SR, 2.0, ToneShape::Rich, 4);
    let signal: Vec<f32> = hum(2.0).iter().zip(&tone).map(|(h, t)| h + t).collect();

    let found = track(&signal);
    let expected = hz_to_midi(hz, A4_HZ);
    assert!(
        found.iter().any(|n| (n.midi - expected).abs() < 0.3),
        "G1 at {expected:.2}: {found:?}"
    );
}
