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

/// Hum that stops (a charger unplugged), then a click 300 ms later: no note with the hum's pitch
/// (PR-Agent #198: the old model was subtracted on for 1 s, a hum in anti-phase from silence).
#[test]
fn click_after_hum_stops_has_no_pitch() {
    let stop_s = LEAD_IN_S + 1.0;
    let mut signal = hum(stop_s, 50.0, 50.0);
    signal.extend(std::iter::repeat_n(0.0, (3.0 * SR) as usize));
    add_noise(&mut signal, -66.0, 4);
    for (i, s) in signal[((stop_s + 0.3) * SR) as usize..]
        .iter_mut()
        .take((0.003 * SR) as usize)
        .enumerate()
    {
        *s += 0.3 * (1.0 - i as f32 / (0.003 * SR) as f32) * if i % 2 == 0 { 1.0 } else { -1.0 };
    }

    let found = track(&signal);
    let after: Vec<&DetectedNote> = found.iter().filter(|n| n.onset_s > stop_s).collect();
    assert!(!after.is_empty(), "the click is heard: {found:?}");
    assert!(after.iter().all(|n| !n.stands_out()), "{after:?}");
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

/// Whether `midi` lies on a partial (1–8) of the grid `grid_hz`, within 50 ct.
fn on_grid(midi: f64, grid_hz: f64) -> bool {
    (1..=8).any(|k| (midi - hz_to_midi(k as f64 * grid_hz, A4_HZ)).abs() < 0.5)
}

/// The hum's level changes (unplugged, a hand on a passive bass, a charger plugged in) at 4 s
/// and 8 phase positions: no note with the hum's pitch stands out, from the change on (review
/// #198, round 2: the subtraction stopped 40–60 ms late, the jump came out as G1 or A#1).
#[test]
fn hum_level_change_is_no_note() {
    for grid_hz in [50.0, 60.0] {
        for factor in [0.0f32, 0.1, 0.25, 0.5, 1.5, 2.0] {
            for i in 0..8 {
                let change_s = 4.0 + f64::from(i) * 0.0027;
                let mut signal = hum(7.0, grid_hz, grid_hz);
                let change = (change_s * SR) as usize;
                signal[change..].iter_mut().for_each(|x| *x *= factor);
                add_noise(&mut signal, -66.0, 9);

                let found = track(&signal);
                let false_notes: Vec<&DetectedNote> = found
                    .iter()
                    .filter(|n| n.onset_s > change_s - 0.1)
                    .filter(|n| n.stands_out() && on_grid(n.midi, grid_hz))
                    .collect();
                assert!(
                    false_notes.is_empty(),
                    "{grid_hz} Hz × {factor} at {change_s:.4} s: {false_notes:?}"
                );
            }
        }
    }
}

/// Notes on a hum whose grid drifts fast (49.8→50.2 Hz in 7 s): after each note fades, no hum
/// comes out as a note (review #198, round 2: an extra at MIDI 31.38 after 6.2 s).
#[test]
fn drifting_hum_after_notes_is_no_note() {
    for name in ["A1", "A#1", "B1", "A#2", "D3", "D#3", "F#3"] {
        let hz = Pitch::parse(name).unwrap().hz(A4_HZ);
        note_on_hum(hz, 0.3, 49.8, 50.2);
    }
}

/// G1 played 50 ms after the hum changed (a charger plugged in: hum from silence, or twice as
/// loud) is still a note of its own: it is much louder at its pitch than the hum was.
#[test]
fn g1_right_after_a_hum_change_stands_out() {
    let hz = Pitch::parse("G1").unwrap().hz(A4_HZ);
    let expected = hz_to_midi(hz, A4_HZ);
    for (before, after) in [(0.0f32, 1.0f32), (1.0, 2.0)] {
        for velocity in [0.7, 0.3] {
            let mut signal = hum(6.0, 50.0, 50.0);
            let change = (4.0 * SR) as usize;
            signal[..change].iter_mut().for_each(|x| *x *= before);
            signal[change..].iter_mut().for_each(|x| *x *= after);
            let note = SynthNote {
                onset_s: 4.05,
                hz,
                duration_s: 1.0,
                velocity,
            };
            let tone = render(&[note], SR, 6.0, ToneShape::Rich, 4);
            signal.iter_mut().zip(&tone).for_each(|(s, t)| *s += t);

            let found = track(&signal);
            assert!(
                found.iter().any(|n| (n.onset_s - 4.05).abs() < 0.03
                    && (n.midi - expected).abs() < 0.2
                    && n.stands_out()),
                "hum ×{before}→×{after}, velocity {velocity}: {found:?}"
            );
        }
    }
}
