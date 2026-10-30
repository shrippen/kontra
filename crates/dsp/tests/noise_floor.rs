//! Onset gate from the noise floor at the count-in (LH-EIN-16): above the noise, no false notes
//! in it, plucks still found; in a quiet room soft plucks at a microphone found.

use kontra_dsp::synth::{add_noise, render, SynthNote, ToneShape};
use kontra_dsp::{gate_over_floor, NoteTracker, TrackerConfig};
use kontra_model::{InputSource, Instrument};

const SR: f64 = 48_000.0;
const BLOCK: usize = 480;
/// The count-in measured: from the input's start to the first measured click.
const COUNT_IN_S: f64 = 3.0;
/// `add_noise` is uniform: its RMS lies this far below its amplitude.
const UNIFORM_RMS_DB: f32 = 4.77;
/// White noise in the onset band (200 Hz – 5 kHz of 24 kHz, Hann) reads about this far below
/// its RMS.
const BAND_BELOW_RMS_DB: f32 = 8.2;

/// Whether the gate comes from the count-in or stays the default.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Gate {
    Default,
    Calibrated,
}

/// White noise of `rms_db` dBFS, `len_s` long.
fn noise(rms_db: f32, len_s: f64, seed: u64) -> Vec<f32> {
    let mut signal = vec![0.0; (len_s * SR) as usize];
    add_noise(&mut signal, rms_db + UNIFORM_RMS_DB, seed);
    signal
}

/// Twelve plucks every 0.3 s, the loudest peaking at `peak_db` dBFS, softer ones 10 dB below.
fn plucks(peak_db: f32) -> (Vec<f32>, usize) {
    let hz = [41.2, 55.0, 73.4, 98.0, 61.7, 46.2];
    let notes: Vec<SynthNote> = (0..12)
        .map(|i| SynthNote {
            onset_s: 0.2 + i as f64 * 0.3,
            hz: hz[i % hz.len()],
            duration_s: 0.28,
            velocity: if i % 2 == 0 { 0.9 } else { 0.3 },
        })
        .collect();
    let mut signal = render(&notes, SR, 4.0, ToneShape::Rich, 1);
    let peak = signal.iter().fold(0f32, |m, v| m.max(v.abs()));
    let gain = 10f32.powf(peak_db / 20.0) / peak;
    signal.iter_mut().for_each(|v| *v *= gain);
    (signal, notes.len())
}

/// A tracker that heard `COUNT_IN_S` of `count_in` and took its gate from it (or not), and
/// that gate.
fn tracker(source: InputSource, count_in: &[f32], gate: Gate) -> (NoteTracker, f32) {
    let config = TrackerConfig::for_instrument(&Instrument::default_bass(), source);
    let mut tracker = NoteTracker::new(SR, config);
    tracker.measure_floor(0..(COUNT_IN_S * SR) as u64);
    for block in count_in.chunks(BLOCK) {
        tracker.push(block);
    }
    if gate == Gate::Calibrated {
        // Not measured: a NaN gate passes no attack, and every check below fails.
        let floor = tracker.noise_floor_db().unwrap_or(f32::NAN);
        tracker.set_onset_gate(gate_over_floor(floor));
    }
    let gate_db = tracker.config().onset.gate_db;
    (tracker, gate_db)
}

/// Notes that stand out in `signal` after a count-in of its own noise.
fn heard(source: InputSource, signal: &[f32], noise_db: f32, gate: Gate) -> usize {
    let count_in = noise(noise_db, COUNT_IN_S + 0.1, 9);
    let (mut tracker, _) = tracker(source, &count_in, gate);
    let mut found = 0;
    for block in signal.chunks(BLOCK) {
        found += tracker
            .push(block)
            .iter()
            .filter(|n| n.stands_out())
            .count();
    }
    found + tracker.flush().iter().filter(|n| n.stands_out()).count()
}

/// Plucks at `peak_db` on noise at `noise_db`.
fn on_noise(peak_db: f32, noise_db: f32) -> (Vec<f32>, usize) {
    let (mut signal, count) = plucks(peak_db);
    let rumble = noise(noise_db, signal.len() as f64 / SR, 5);
    signal.iter_mut().zip(&rumble).for_each(|(s, n)| *s += n);
    (signal, count)
}

/// LH-EIN-16 at a line input with noise at −50 and −40 dBFS: the gate lies above the noise,
/// a minute of it gives no note, and no pluck is lost against the default gate.
#[test]
fn gate_sits_above_line_noise() {
    for noise_db in [-50.0f32, -40.0] {
        let count_in = noise(noise_db, COUNT_IN_S + 0.1, 9);
        let (_, gate_db) = tracker(InputSource::Line, &count_in, Gate::Calibrated);
        let band_db = noise_db - BAND_BELOW_RMS_DB;
        assert!(
            gate_db > band_db + 3.0,
            "noise {noise_db}: gate {gate_db} not above the band's {band_db}"
        );

        let idle = noise(noise_db, 60.0, 11);
        let false_notes = heard(InputSource::Line, &idle, noise_db, Gate::Calibrated);
        assert_eq!(false_notes, 0, "noise {noise_db}: false notes");

        for peak_db in [-6.0f32, -20.0, -30.0] {
            let (signal, _) = on_noise(peak_db, noise_db);
            let before = heard(InputSource::Line, &signal, noise_db, Gate::Default);
            let after = heard(InputSource::Line, &signal, noise_db, Gate::Calibrated);
            assert!(
                after >= before,
                "noise {noise_db}, plucks {peak_db}: {after} found, {before} before"
            );
        }
    }
}

/// P6 in a quiet room: noise at −80 dBFS at the microphone. The fixed gate (−48 dB) lost the
/// soft plucks peaking at −30 dBFS; the gate from the count-in finds them.
#[test]
fn quiet_room_finds_soft_plucks() {
    let noise_db = -80.0;
    let (signal, count) = on_noise(-30.0, noise_db);
    let before = heard(InputSource::Microphone, &signal, noise_db, Gate::Default);
    let after = heard(InputSource::Microphone, &signal, noise_db, Gate::Calibrated);
    assert_eq!(
        after, count,
        "found {after} of {count} (default gate: {before})"
    );
}

/// A loud room at the microphone (noise −45 dBFS): the gate rises above the noise but not
/// beyond `GATE_MAX_DB`, and the noise gives no note.
#[test]
fn loud_room_raises_the_gate() {
    let noise_db = -45.0;
    let count_in = noise(noise_db, COUNT_IN_S + 0.1, 9);
    let (_, gate_db) = tracker(InputSource::Microphone, &count_in, Gate::Calibrated);
    assert!(gate_db > noise_db - BAND_BELOW_RMS_DB, "gate {gate_db}");
    assert!(gate_db <= -40.0, "gate {gate_db}");
    let idle = noise(noise_db, 30.0, 13);
    assert_eq!(
        heard(InputSource::Microphone, &idle, noise_db, Gate::Calibrated),
        0
    );
}
