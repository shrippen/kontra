//! Strikes the tab expects (R2): the right expectation never tells a note later, a wrong one
//! changes nothing the player sees, neither the first report nor the final pitch.

use kontra_dsp::synth::{bass_tone, render, SynthNote, ToneShape};
use kontra_dsp::{NoteTracker, TrackerConfig};
use kontra_model::{
    DetectedNote, ExpectedStrike, ExpectedTone, InputSource, Instrument, Pitch, A4_HZ,
};

const SR: f64 = 48_000.0;
const BLOCK: usize = 480;
/// Strikes are told this far ahead, as the session does.
const AHEAD_S: f64 = 0.25;
/// Half width of an expected strike's window (the session's loose timing window).
const WINDOW_S: f64 = 0.09;
/// A2, D3, G2, C3, E2, B2.
const NOTES: [u8; 6] = [45, 50, 43, 48, 40, 47];
const SPACING_S: f64 = 0.4;
/// Wrong expectations: a semitone either way, an octave either way, a fourth.
const SHIFTS: [i8; 5] = [1, -1, 12, -12, 5];

/// A note as told: onset, first pitch, when it was first told (sample), final pitch.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Told {
    onset_s: f64,
    first_midi: f64,
    told_at: usize,
    final_midi: f64,
}

fn line() -> Vec<SynthNote> {
    NOTES
        .iter()
        .enumerate()
        .map(|(i, n)| SynthNote {
            onset_s: 0.3 + i as f64 * SPACING_S,
            hz: Pitch(*n).hz(A4_HZ),
            duration_s: SPACING_S * 0.9,
            velocity: 0.7,
        })
        .collect()
}

/// The line's strikes, each moved by `shift` semitones, with extra tones `others` (a double
/// stop).
fn strikes(notes: &[SynthNote], shift: i8, others: &[i8]) -> Vec<ExpectedStrike> {
    notes
        .iter()
        .map(|n| {
            let midi = kontra_model::hz_to_midi(n.hz, A4_HZ).round() as i16;
            let tone = |by: i8| ExpectedTone {
                midi: (midi + i16::from(by)).clamp(0, i16::from(u8::MAX)) as u8,
                string: None,
            };
            let strike = ExpectedStrike::new(n.onset_s, WINDOW_S).with(tone(shift));
            others.iter().fold(strike, |s, by| s.with(tone(*by)))
        })
        .collect()
}

fn track(signal: &[f32], expected: &[ExpectedStrike]) -> Vec<Told> {
    let config = TrackerConfig::for_instrument(&Instrument::default_bass(), InputSource::Line);
    let mut tracker = NoteTracker::new(SR, config);
    let mut told: Vec<Told> = Vec::new();
    let mut next = 0;
    let keep = |told: &mut Vec<Told>, found: &[DetectedNote], at: usize| {
        told.extend(found.iter().map(|n| Told {
            onset_s: n.onset_s,
            first_midi: n.midi,
            told_at: at,
            final_midi: n.midi,
        }));
    };
    for (i, block) in signal.chunks(BLOCK).enumerate() {
        let end = i * BLOCK + block.len();
        while let Some(s) = expected
            .get(next)
            .filter(|s| s.onset_s < end as f64 / SR + AHEAD_S)
        {
            tracker.expect(s);
            next += 1;
        }
        let found = tracker.push(block).to_vec();
        keep(&mut told, &found, end);
        for r in tracker.pitch_revised() {
            if let Some(t) = told
                .iter_mut()
                .find(|t| (t.onset_s - r.onset_s).abs() < 1e-9)
            {
                t.final_midi = r.midi;
            }
        }
    }
    let found = tracker.flush().to_vec();
    keep(&mut told, &found, signal.len());
    told
}

fn semitones(told: &[Told], pick: fn(&Told) -> f64) -> Vec<(i64, i64)> {
    told.iter()
        .map(|t| ((t.onset_s * 1000.0).round() as i64, pick(t).round() as i64))
        .collect()
}

/// Since R1 a clear tone goes out as early without expectation (the rise check after the onset,
/// 40 ms, bounds both); the expectation must not make it later or change its pitch.
#[test]
fn right_expectation_never_later() {
    let notes = line();
    let signal = render(&notes, SR, 2.8, ToneShape::Rich, 11);
    let free = track(&signal, &[]);
    let expecting = track(&signal, &strikes(&notes, 0, &[]));
    assert_eq!(free.len(), notes.len(), "{free:?}");
    assert_eq!(
        semitones(&expecting, |t| t.final_midi),
        semitones(&free, |t| t.final_midi),
        "same final pitches"
    );
    assert_eq!(
        semitones(&expecting, |t| t.first_midi),
        semitones(&expecting, |t| t.final_midi),
        "the early pitch stands"
    );
    assert!(expecting
        .iter()
        .zip(&free)
        .all(|(e, f)| e.told_at <= f.told_at));
}

/// The wrong-expectation guard: a tab that expects other notes than played changes neither
/// the first report nor the final pitch.
#[test]
fn wrong_expectation_changes_nothing() {
    let notes = line();
    let signal = render(&notes, SR, 2.8, ToneShape::Rich, 12);
    let free = track(&signal, &[]);
    for shift in SHIFTS {
        let expecting = track(&signal, &strikes(&notes, shift, &[]));
        assert_eq!(
            semitones(&expecting, |t| t.first_midi),
            semitones(&free, |t| t.first_midi),
            "first reports, expected {shift:+}"
        );
        assert_eq!(
            semitones(&expecting, |t| t.final_midi),
            semitones(&free, |t| t.final_midi),
            "final pitches, expected {shift:+}"
        );
    }
}

/// A double stop expects two tones; either one heard is enough.
#[test]
fn either_tone_of_a_double_stop() {
    let notes = line();
    let signal = render(&notes, SR, 2.8, ToneShape::Rich, 13);
    let free = track(&signal, &[]);
    let fifth_above = track(&signal, &strikes(&notes, 7, &[0]));
    assert_eq!(
        semitones(&fifth_above, |t| t.final_midi),
        semitones(&free, |t| t.final_midi)
    );
    assert!(fifth_above
        .iter()
        .zip(&free)
        .all(|(e, f)| e.told_at <= f.told_at));
}

/// After `forget_expected` the tracker reports as without expectation.
#[test]
fn forgotten_strikes_do_not_count() {
    let notes = line();
    let signal = render(&notes, SR, 2.8, ToneShape::Rich, 14);
    let config = TrackerConfig::for_instrument(&Instrument::default_bass(), InputSource::Line);
    let mut tracker = NoteTracker::new(SR, config);
    for s in strikes(&notes, 0, &[]) {
        tracker.expect(&s);
    }
    tracker.forget_expected();
    let mut told_at = Vec::new();
    for (i, block) in signal.chunks(BLOCK).enumerate() {
        let count = tracker.push(block).len();
        told_at.extend(std::iter::repeat_n(i * BLOCK + block.len(), count));
    }
    let free = track(&signal, &[]);
    let free_at: Vec<usize> = free.iter().map(|t| t.told_at).collect();
    assert_eq!(told_at, free_at);
}

/// Where the period is too unclear to tell early on its own (R1), the expected pitch goes out
/// sooner, and right: A2 with an inharmonic tone (2.71×, −8 dB) over it.
#[test]
fn expectation_helps_an_unclear_period() {
    let a2 = Pitch(45).hz(A4_HZ);
    let mut signal = vec![0.0f32; (0.3 * SR) as usize];
    let tone = bass_tone(a2, SR, 0.5, ToneShape::Rich, 3);
    let other = bass_tone(a2 * 2.71, SR, 0.5, ToneShape::Rich, 4);
    signal.extend(tone.iter().zip(&other).map(|(a, b)| a + 0.4 * b));
    let note = SynthNote {
        onset_s: 0.3,
        hz: a2,
        duration_s: 0.5,
        velocity: 0.7,
    };
    let free = track(&signal, &[]);
    let expecting = track(&signal, &strikes(&[note], 0, &[]));
    assert_eq!(expecting.len(), 1, "{expecting:?}");
    assert!(
        expecting[0].told_at < free[0].told_at,
        "{expecting:?} {free:?}"
    );
    assert_eq!(expecting[0].first_midi.round(), 45.0, "{expecting:?}");
}
