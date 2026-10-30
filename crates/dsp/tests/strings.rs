//! Which string a note was played on (R7): the tracker tells it after a note the tab puts on a
//! string, from the stiffness of synthetic strings.

use kontra_dsp::{NoteTracker, TrackerConfig, LOG_B_PER_FRET};
use kontra_model::{
    midi_to_hz, ExpectedStrike, ExpectedTone, InputSource, Instrument, InstrumentKind, StringHeard,
    StringVerdict, Unclear, A4_HZ,
};

const SR: f64 = 48_000.0;
const BLOCK: usize = 480;
const ONSET_S: f64 = 0.3;
/// Amplitude decay per second of a plucked note (≈ 9 dB/s) and of a palm-muted one (52).
const RING: f64 = 1.0;
const MUTED: f64 = 6.0;

/// A stiff string's note at `ONSET_S` (partials at `k·f0·√(1 + B·k²)`, falling 1/k), and
/// silence from `stop_s` on, if given.
fn stiff_note(midi: u8, log_b: f64, decay: f64, stop_s: Option<f64>) -> Vec<f32> {
    let hz = midi_to_hz(f64::from(midi), A4_HZ);
    let b = 10f64.powf(log_b);
    let len = (1.5 * SR) as usize;
    (0..len)
        .map(|i| {
            let t = i as f64 / SR - ONSET_S;
            if t < 0.0 || stop_s.is_some_and(|s| t + ONSET_S >= s) {
                return 0.0;
            }
            let tone: f64 = (1..=30)
                .map(|k| {
                    let k = f64::from(k);
                    let f = k * hz * (1.0 + b * k * k).sqrt();
                    if f > SR / 2.0 {
                        return 0.0;
                    }
                    (std::f64::consts::TAU * f * t).sin() / k
                })
                .sum();
            (0.3 * tone * (-t * decay).exp()) as f32
        })
        .collect()
}

/// log10 B of `midi` on `string` of the default bass, from its profile.
fn on_string(string: u8, midi: u8) -> f64 {
    let bass = Instrument::default_bass();
    let open = bass.tuning.strings[usize::from(string)].0;
    bass.string_log_b(string).unwrap_or(f64::NAN) + f64::from(midi - open) * LOG_B_PER_FRET
}

/// The verdicts for `signal` with one strike expected at `ONSET_S`.
fn told(signal: &[f32], instrument: &Instrument, tone: ExpectedTone) -> Vec<StringVerdict> {
    let config = TrackerConfig::for_instrument(instrument, InputSource::Line);
    let mut tracker = NoteTracker::new(SR, config);
    tracker.expect(&ExpectedStrike::new(ONSET_S, 0.08).with(tone));
    let mut verdicts = Vec::new();
    for block in signal.chunks(BLOCK) {
        tracker.push(block);
        verdicts.extend_from_slice(tracker.string_verdicts());
    }
    tracker.flush();
    verdicts.extend_from_slice(tracker.string_verdicts());
    verdicts
}

const G2: u8 = 43;
const A_STRING: u8 = 1;
const D_STRING: u8 = 2;

#[test]
fn tells_the_string_a_note_was_played_on() {
    // G2 on the A string (fret 10): expected there, then expected on the D string.
    let signal = stiff_note(G2, on_string(A_STRING, G2), RING, None);
    let bass = Instrument::default_bass();
    let tone = |string| ExpectedTone {
        midi: G2,
        string: Some(string),
    };
    let same = told(&signal, &bass, tone(A_STRING));
    assert_eq!(same.len(), 1, "{same:?}");
    assert_eq!(same[0].heard, StringHeard::Expected);
    assert!((same[0].onset_s - ONSET_S).abs() < 0.02, "{same:?}");
    let other = told(&signal, &bass, tone(D_STRING));
    assert_eq!(other[0].heard, StringHeard::Other(A_STRING), "{other:?}");
}

#[test]
fn a_note_cut_short_is_not_told() {
    // Stopped after 80 ms, the next (louder) attack 90 ms after the first: too little of the
    // first tone to measure.
    let mut cut = stiff_note(G2, on_string(A_STRING, G2), RING, Some(ONSET_S + 0.08));
    let next = stiff_note(G2 + 2, on_string(A_STRING, G2 + 2), RING, None);
    let shift = (0.09 * SR) as usize;
    for (i, x) in next.iter().enumerate() {
        if let Some(slot) = cut.get_mut(i + shift) {
            *slot += 2.0 * x;
        }
    }
    let tone = ExpectedTone {
        midi: G2,
        string: Some(A_STRING),
    };
    let verdicts = told(&cut, &Instrument::default_bass(), tone);
    assert_eq!(verdicts.len(), 1, "{verdicts:?}");
    assert_eq!(verdicts[0].heard, StringHeard::Unclear(Unclear::ShortNote));
}

#[test]
fn a_palm_muted_note_is_not_told() {
    let signal = stiff_note(G2, on_string(A_STRING, G2), MUTED, None);
    let tone = ExpectedTone {
        midi: G2,
        string: Some(A_STRING),
    };
    let verdicts = told(&signal, &Instrument::default_bass(), tone);
    assert_eq!(verdicts[0].heard, StringHeard::Unclear(Unclear::Muted));
}

#[test]
fn only_strikes_on_a_string_with_a_choice_are_checked() {
    let bass = Instrument::default_bass();
    let signal = stiff_note(G2, on_string(A_STRING, G2), RING, None);
    let unplaced = ExpectedTone {
        midi: G2,
        string: None,
    };
    assert!(told(&signal, &bass, unplaced).is_empty());

    // E1 only sounds on the E string: nothing to tell.
    let low = stiff_note(28, on_string(0, 28), RING, None);
    let e1 = ExpectedTone {
        midi: 28,
        string: Some(0),
    };
    assert!(told(&low, &bass, e1).is_empty());
}

#[test]
fn fretless_says_it_is_not_supported() {
    let mut fretless = Instrument::default_bass();
    fretless.kind = InstrumentKind::Fretless;
    let signal = stiff_note(G2, on_string(A_STRING, G2), RING, None);
    let tone = ExpectedTone {
        midi: G2,
        string: Some(A_STRING),
    };
    let verdicts = told(&signal, &fretless, tone);
    assert_eq!(verdicts.len(), 1, "{verdicts:?}");
    assert_eq!(
        verdicts[0].heard,
        StringHeard::Unclear(Unclear::NotSupported)
    );
    assert_eq!(verdicts[0].log_b, None);
}

#[test]
fn a_retracted_note_gets_no_string() {
    // The fretting finger taps the string 60 ms before the pluck, 30 dB below it: told early,
    // then taken back (R1). Only the pluck may get a string.
    let tap = stiff_note(G2, on_string(A_STRING, G2), RING, Some(ONSET_S + 0.06));
    let pluck = stiff_note(G2, on_string(A_STRING, G2), RING, None);
    let shift = (0.06 * SR) as usize;
    let mut signal: Vec<f32> = tap.iter().map(|x| x * 0.03).collect();
    for (i, x) in pluck.iter().enumerate() {
        if let Some(slot) = signal.get_mut(i + shift) {
            *slot += 0.9 * x;
        }
    }
    let tone = ExpectedTone {
        midi: G2,
        string: Some(A_STRING),
    };
    let verdicts = told(&signal, &Instrument::default_bass(), tone);
    let tapped = verdicts
        .iter()
        .filter(|v| (v.onset_s - ONSET_S).abs() < 0.005)
        .count();
    assert_eq!(tapped, 0, "{verdicts:?}");
    assert_eq!(verdicts.len(), 1, "{verdicts:?}");
}

/// `stiff_note` starting `flat_cents` low and rising to its pitch over `rise_s` (a string that
/// settles, J-E3).
fn settling_note(midi: u8, log_b: f64, flat_cents: f64, rise_s: f64) -> Vec<f32> {
    let hz = midi_to_hz(f64::from(midi), A4_HZ);
    let b = 10f64.powf(log_b);
    let len = (1.5 * SR) as usize;
    let mut phase = 0.0;
    (0..len)
        .map(|i| {
            let t = i as f64 / SR - ONSET_S;
            if t < 0.0 {
                return 0.0;
            }
            let cents = flat_cents * (1.0 - t / rise_s).max(0.0);
            phase += std::f64::consts::TAU * hz * 2f64.powf(cents / 1200.0) / SR;
            let tone: f64 = (1..=30)
                .map(|k| {
                    let k = f64::from(k);
                    let stretch = k * (1.0 + b * k * k).sqrt();
                    if stretch * hz > SR / 2.0 {
                        return 0.0;
                    }
                    (stretch * phase).sin() / k
                })
                .sum();
            (0.3 * tone * (-t * RING).exp()) as f32
        })
        .collect()
}

#[test]
fn a_settled_pitch_gets_its_string() {
    // Review #207: G2 on the A string starting 140 cents flat, told early as F#2. Once the
    // pitch is revised to G2, its string is checked as for any G2.
    let signal = settling_note(G2, on_string(A_STRING, G2), -140.0, 0.12);
    let tone = ExpectedTone {
        midi: G2,
        string: Some(A_STRING),
    };
    let verdicts = told(&signal, &Instrument::default_bass(), tone);
    assert_eq!(verdicts.len(), 1, "{verdicts:?}");
    assert_eq!(verdicts[0].heard, StringHeard::Expected, "{verdicts:?}");
}
