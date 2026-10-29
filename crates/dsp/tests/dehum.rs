//! Mains hum removed before detection (E8): the hum is followed and subtracted, so noise on it
//! has no pitch, and notes on the hum's frequencies (G1 49 Hz, G2 98 Hz) keep theirs.

use kontra_dsp::synth::{add_noise, render, Noise, SynthNote, ToneShape};
use kontra_dsp::{NoteTracker, TrackerConfig};
use kontra_model::{hz_to_midi, DetectedNote, InputSource, Instrument, Pitch, A4_HZ};
use std::f64::consts::TAU;

const SR: f64 = 48_000.0;
/// Hum partials as (multiple of the mains frequency, level in dBFS): a ground loop, louder than
/// the corpus set idle-line (−50 dBFS).
const PARTIALS: [(f64, f32); 3] = [(1.0, -40.0), (3.0, -48.0), (5.0, -54.0)];
/// Time for the hum alone before anything is played: Kontra listens before the first note.
const LEAD_IN_S: f64 = 3.0;

/// `len_s` of hum whose mains frequency goes from `from_hz` to `to_hz`, with a little noise.
fn hum(len_s: f64, from_hz: f64, to_hz: f64) -> Vec<f32> {
    let len = (len_s * SR) as usize;
    let mut phase = 0.0f64;
    let mut signal = Vec::with_capacity(len);
    for i in 0..len {
        let hz = from_hz + (to_hz - from_hz) * i as f64 / len as f64;
        let x: f64 = PARTIALS
            .iter()
            .map(|(k, db)| 10f64.powf(f64::from(*db) / 20.0) * (k * phase).sin())
            .sum();
        signal.push(x as f32);
        phase = (phase + TAU * hz / SR) % (TAU * 15.0);
    }
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

/// A click (3 ms), a scrape (120 ms of hiss) and a hand on the strings (30 ms, low), from
/// `from_s` on, one every second.
fn add_noises(signal: &mut [f32], from_s: f64) {
    let mut noise = Noise::new(7);
    let mut low = 0.0f32;
    let k = (TAU * 200.0 / SR) as f32;
    // (length s, amplitude, low-passed)
    let sounds = [(0.003, 0.3, false), (0.12, 0.03, false), (0.03, 1.5, true)];
    for (i, (len_s, amplitude, dull)) in sounds.into_iter().cycle().take(6).enumerate() {
        let at = ((from_s + i as f64) * SR) as usize;
        let len = (len_s * SR) as usize;
        for (j, s) in signal[at..at + len].iter_mut().enumerate() {
            let fade = 1.0 - j as f32 / len as f32;
            low += k * (noise.sample() - low);
            let x = if dull { low } else { noise.sample() };
            *s += amplitude * fade * x;
        }
    }
}

/// Noise on hum at `from_hz` → `to_hz`: heard, but without a pitch (the hum is gone).
fn assert_noise_pitchless(from_hz: f64, to_hz: f64) {
    let mut signal = hum(LEAD_IN_S + 7.0, from_hz, to_hz);
    add_noises(&mut signal, LEAD_IN_S + 0.5);

    let found = track(&signal);
    assert!(!found.is_empty(), "the attacks are heard");
    assert!(
        found.iter().all(|n| n.is_pitchless()),
        "hum {from_hz}→{to_hz} Hz: {found:?}"
    );
}

/// Before E8 each came out with the hum's pitch (MIDI 31.35), only marked `over_background`.
#[test]
fn noise_on_50_hz_hum_has_no_pitch() {
    assert_noise_pitchless(50.0, 50.0);
}

#[test]
fn noise_on_60_hz_hum_has_no_pitch() {
    assert_noise_pitchless(60.0, 60.0);
}

/// The grid's frequency wanders within its tolerance (49.8–50.2 Hz).
#[test]
fn noise_on_drifting_hum_has_no_pitch() {
    assert_noise_pitchless(49.8, 50.2);
}

/// G1 (49 Hz, 35 ct below the hum) and G2 (98 Hz, the hum's second harmonic −35 ct), played on
/// hum, soft and loud: each a note of its own with its own pitch.
#[test]
fn g1_and_g2_on_hum_keep_their_pitch() {
    for (from_hz, to_hz) in [(50.0, 50.0), (49.8, 50.2), (60.0, 60.0)] {
        for velocity in [0.7, 0.1] {
            for name in ["G1", "G2"] {
                let hz = Pitch::parse(name).unwrap().hz(A4_HZ);
                note_on_hum(hz, velocity, from_hz, to_hz);
            }
        }
    }
}

fn note_on_hum(hz: f64, velocity: f32, from_hz: f64, to_hz: f64) {
    let len_s = LEAD_IN_S + 4.0;
    let notes: Vec<SynthNote> = [0.5, 2.0]
        .iter()
        .map(|at| SynthNote {
            onset_s: LEAD_IN_S + at,
            hz,
            duration_s: 1.2,
            velocity,
        })
        .collect();
    let tone = render(&notes, SR, len_s, ToneShape::Rich, 4);
    let signal: Vec<f32> = hum(len_s, from_hz, to_hz)
        .iter()
        .zip(&tone)
        .map(|(h, t)| h + t)
        .collect();

    let found = track(&signal);
    let expected = hz_to_midi(hz, A4_HZ);
    for note in &notes {
        let heard = found
            .iter()
            .find(|n| (n.onset_s - note.onset_s).abs() < 0.03);
        assert!(
            heard.is_some_and(|h| (h.midi - expected).abs() < 0.2 && !h.over_background),
            "{hz:.1} Hz at {:.1} s, velocity {velocity}, on {from_hz}→{to_hz} Hz: {found:?}",
            note.onset_s
        );
    }
    let extra: Vec<&DetectedNote> = found
        .iter()
        .filter(|n| notes.iter().all(|m| (n.onset_s - m.onset_s).abs() >= 0.03))
        .filter(|n| n.stands_out())
        .collect();
    assert!(extra.is_empty(), "{hz:.1} Hz: extra notes {extra:?}");
}
