//! Double stops the tab announces (R8): the tracker tells which tones sounded, once per
//! announced double stop, matched to the note by its onset.

use kontra_dsp::synth::{render, SynthNote, ToneShape};
use kontra_dsp::{NoteTracker, TrackerConfig};
use kontra_model::{
    DetectedNote, DoubleStopHeard, DoubleStopVerdict, ExpectedStrike, ExpectedTone, InputSource,
    Instrument, Pitch, A4_HZ,
};

const SR: f64 = 48_000.0;
const BLOCK: usize = 480;
const AHEAD_S: f64 = 0.25;
const WINDOW_S: f64 = 0.09;
const SPACING_S: f64 = 0.5;
/// A verdict comes this soon after the attack (frame 20 + 140 ms, a block and a step later).
const VERDICT_MAX_S: f64 = 0.18;

/// One strike: the tones played and the two the tab expects.
struct Strike {
    played: &'static [u8],
    expected: (u8, u8),
}

/// Plays `strikes` one after another; returns the notes and the verdicts with the time each
/// verdict was told.
fn run(strikes: &[Strike], shape: ToneShape) -> (Vec<DetectedNote>, Vec<(DoubleStopVerdict, f64)>) {
    let onset = |i: usize| 0.3 + i as f64 * SPACING_S;
    let total_s = onset(strikes.len()) + 0.3;

    // `render` is one string: each tone of a strike on its own, then mixed.
    let mut signal = vec![0.0f32; (total_s * SR).ceil() as usize];
    for voice in 0..2 {
        let notes: Vec<SynthNote> = strikes
            .iter()
            .enumerate()
            .filter_map(|(i, s)| {
                s.played.get(voice).map(|m| SynthNote {
                    onset_s: onset(i),
                    hz: Pitch(*m).hz(A4_HZ),
                    duration_s: SPACING_S * 0.9,
                    velocity: 0.4,
                })
            })
            .collect();
        let rendered = render(&notes, SR, total_s, shape, 11 + voice as u64);
        for (out, x) in signal.iter_mut().zip(rendered) {
            *out += x;
        }
    }
    let expected: Vec<ExpectedStrike> = strikes
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let tone = |midi| ExpectedTone { midi, string: None };
            ExpectedStrike::new(onset(i), WINDOW_S)
                .with(tone(s.expected.0))
                .with(tone(s.expected.1))
        })
        .collect();

    let config = TrackerConfig::for_instrument(&Instrument::default_bass(), InputSource::Line);
    let mut tracker = NoteTracker::new(SR, config);
    let (mut found, mut verdicts) = (Vec::new(), Vec::new());
    let mut next = 0;
    for (i, block) in signal.chunks(BLOCK).enumerate() {
        let end_s = ((i + 1) * BLOCK) as f64 / SR;
        while let Some(s) = expected.get(next).filter(|s| s.onset_s < end_s + AHEAD_S) {
            tracker.expect(s);
            next += 1;
        }
        found.extend_from_slice(tracker.push(block));
        verdicts.extend(tracker.double_stops().iter().map(|v| (*v, end_s)));
    }
    found.extend_from_slice(tracker.flush());
    verdicts.extend(tracker.double_stops().iter().map(|v| (*v, total_s)));
    (found, verdicts)
}

/// The verdict for the note at `onset_s`, if one was told.
fn verdict_at(
    found: &[DetectedNote],
    verdicts: &[(DoubleStopVerdict, f64)],
    onset_s: f64,
) -> Option<(DoubleStopHeard, f64)> {
    let note = found.iter().find(|n| (n.onset_s - onset_s).abs() < 0.03)?;
    verdicts
        .iter()
        .find(|(v, _)| v.onset_s == note.onset_s)
        .map(|(v, at)| (v.heard, at - onset_s))
}

/// Fifths, fourths, thirds, a tenth: both heard, about 160 ms after the attack.
#[test]
fn hears_both_tones() {
    let strikes = [
        Strike {
            played: &[33, 40],
            expected: (33, 40),
        },
        Strike {
            played: &[38, 43],
            expected: (38, 43),
        },
        Strike {
            played: &[43, 47],
            expected: (43, 47),
        },
        Strike {
            played: &[28, 44],
            expected: (28, 44),
        },
        Strike {
            played: &[45, 48],
            expected: (45, 48),
        },
    ];
    for shape in [ToneShape::Rich, ToneShape::Dull] {
        let (found, verdicts) = run(&strikes, shape);
        assert_eq!(verdicts.len(), strikes.len(), "{verdicts:?}");
        for i in 0..strikes.len() {
            let onset_s = 0.3 + i as f64 * SPACING_S;
            let (heard, after_s) = verdict_at(&found, &verdicts, onset_s).expect("verdict");
            assert_eq!(heard, DoubleStopHeard::Both, "strike {i}, {shape:?}");
            assert!(after_s < VERDICT_MAX_S, "strike {i}: {after_s:.3} s");
        }
    }
}

/// One tone played where the tab has two: the verdict names it.
#[test]
fn names_the_one_tone_played() {
    let strikes = [
        Strike {
            played: &[33],
            expected: (33, 40),
        },
        Strike {
            played: &[40],
            expected: (33, 40),
        },
        Strike {
            played: &[38],
            expected: (38, 45),
        },
        Strike {
            played: &[47],
            expected: (43, 47),
        },
    ];
    let wanted = [
        DoubleStopHeard::Lower,
        DoubleStopHeard::Upper,
        DoubleStopHeard::Lower,
        DoubleStopHeard::Upper,
    ];
    let (found, verdicts) = run(&strikes, ToneShape::Rich);
    for (i, want) in wanted.iter().enumerate() {
        let onset_s = 0.3 + i as f64 * SPACING_S;
        let heard = verdict_at(&found, &verdicts, onset_s).map(|v| v.0);
        assert_eq!(heard, Some(*want), "strike {i}");
    }
}

/// An octave cannot be told (the upper tone's partials are the lower's): no statement, at once,
/// whatever was played.
#[test]
fn octave_tells_nothing() {
    let strikes = [
        Strike {
            played: &[33, 45],
            expected: (33, 45),
        },
        Strike {
            played: &[33],
            expected: (33, 45),
        },
    ];
    let (found, verdicts) = run(&strikes, ToneShape::Rich);
    for i in 0..strikes.len() {
        let onset_s = 0.3 + i as f64 * SPACING_S;
        let (heard, after_s) = verdict_at(&found, &verdicts, onset_s).expect("verdict");
        assert_eq!(heard, DoubleStopHeard::Unknown);
        assert!(after_s < 0.1, "told at once, not after {after_s:.3} s");
    }
}

/// Single notes expected as single notes get no verdict.
#[test]
fn single_strikes_get_no_verdict() {
    let config = TrackerConfig::for_instrument(&Instrument::default_bass(), InputSource::Line);
    let mut tracker = NoteTracker::new(SR, config);
    let note = SynthNote {
        onset_s: 0.3,
        hz: Pitch(40).hz(A4_HZ),
        duration_s: 0.4,
        velocity: 0.6,
    };
    tracker.expect(&ExpectedStrike::new(0.3, WINDOW_S).with(ExpectedTone {
        midi: 40,
        string: None,
    }));
    let signal = render(&[note], SR, 1.0, ToneShape::Rich, 3);
    let mut told = 0;
    for block in signal.chunks(BLOCK) {
        tracker.push(block);
        told += tracker.double_stops().len();
    }
    tracker.flush();
    told += tracker.double_stops().len();
    assert_eq!(told, 0);
}
