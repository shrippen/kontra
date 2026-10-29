//! Early report on a growing window (R1): a note goes out before the full YIN window is there
//! when its period is unambiguous; the full window has the last word on its pitch, and a note
//! told early that proves to be no note of its own is taken back.

use kontra_dsp::synth::{bass_tone, render, SynthNote, ToneShape};
use kontra_dsp::{NoteTracker, TrackerConfig};
use kontra_model::{hz_to_midi, InputSource, Instrument, Pitch, A4_HZ};

const SR: f64 = 48_000.0;
const BLOCK: usize = 480;
const LEAD_S: f64 = 0.2;
/// Notes from E2 up are told within this (until R1 80 ms).
const TOLD_WITHIN_MS: f64 = 65.0;

/// A note as the session ends up with it: every revision applied, retracted ones dropped.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Told {
    onset_s: f64,
    first_midi: f64,
    /// Seconds from the onset to the end of the block that told it.
    after_s: f64,
    final_midi: f64,
    level_db: f32,
}

/// Notes told and the onsets taken back.
fn track(signal: &[f32]) -> (Vec<Told>, Vec<f64>) {
    let config = TrackerConfig::for_instrument(&Instrument::default_bass(), InputSource::Line);
    let mut tracker = NoteTracker::new(SR, config);
    let mut told: Vec<Told> = Vec::new();
    let mut retracted = Vec::new();
    let blocks = signal.chunks(BLOCK).map(Some).chain([None]);
    for (i, block) in blocks.enumerate() {
        let end_s = ((i * BLOCK) as f64 + block.map_or(0, <[f32]>::len) as f64) / SR;
        let found = match block {
            Some(b) => tracker.push(b).to_vec(),
            None => tracker.flush().to_vec(),
        };
        told.extend(found.iter().map(|n| Told {
            onset_s: n.onset_s,
            first_midi: n.midi,
            after_s: end_s - n.onset_s,
            final_midi: n.midi,
            level_db: n.level_db,
        }));
        for r in tracker.pitch_revised() {
            if let Some(t) = told.iter_mut().find(|t| t.onset_s == r.onset_s) {
                t.final_midi = r.midi;
            }
        }
        for r in tracker.retracted() {
            retracted.push(r.onset_s);
            told.retain(|t| t.onset_s != r.onset_s);
        }
    }
    (told, retracted)
}

fn midi(name: &str) -> f64 {
    Pitch::parse(name).map_or(0.0, |p| hz_to_midi(p.hz(A4_HZ), A4_HZ))
}

#[test]
fn reports_before_the_full_window() {
    // A 4-string bass waits for two periods of D1 (55 ms) plus the attack skip and a second
    // estimate: 80 ms before a G2 went out (blocks of 10 ms). From E2 up the period shows
    // sooner: E2 needs 37 ms of audio.
    let names = ["G2", "D3", "A2", "C3", "E2"];
    let notes: Vec<SynthNote> = names
        .iter()
        .enumerate()
        .map(|(i, n)| SynthNote {
            onset_s: LEAD_S + i as f64 * 0.4,
            hz: Pitch::parse(n).unwrap().hz(A4_HZ),
            duration_s: 0.36,
            velocity: 0.7,
        })
        .collect();
    let signal = render(&notes, SR, 2.8, ToneShape::Rich, 21);
    let (told, retracted) = track(&signal);
    assert!(retracted.is_empty(), "{retracted:?}");
    assert_eq!(told.len(), notes.len(), "{told:?}");
    for (t, name) in told.iter().zip(names) {
        let ms = t.after_s * 1000.0;
        assert!(ms <= TOLD_WITHIN_MS, "{name} told after {ms:.0} ms");
        let cents = (t.final_midi - midi(name)) * 100.0;
        assert!(cents.abs() < 3.0, "{name}: {cents:.1} ct");
    }
}

#[test]
fn early_tap_before_the_pluck_is_taken_back() {
    // The fretting finger taps the string 60 ms before the pluck, 30 dB below it (hardware
    // check 13.10.): long enough to be told early on its own pitch. The pluck shows it was no
    // note: it is taken back, and the note starts at the pluck with the pluck's level.
    let hz = Pitch::parse("G2").unwrap().hz(A4_HZ);
    let tap = bass_tone(hz, SR, 0.06, ToneShape::Rich, 10);
    let pluck = bass_tone(hz, SR, 0.8, ToneShape::Rich, 11);
    let mut signal = vec![0.0; (LEAD_S * SR) as usize];
    signal.extend(tap.iter().map(|x| x * 0.03));
    signal.extend(pluck.iter().map(|x| x * 0.9));
    signal.extend(std::iter::repeat_n(0.0, (0.3 * SR) as usize));
    let (told, retracted) = track(&signal);
    assert_eq!(retracted.len(), 1, "the tap was told early: {retracted:?}");
    assert!((retracted[0] - LEAD_S).abs() < 0.005, "{retracted:?}");
    assert_eq!(told.len(), 1, "{told:?}");
    assert!((told[0].onset_s - 0.26).abs() < 0.005, "{told:?}");
    assert!(told[0].level_db > -6.0, "{told:?}");
}

#[test]
fn soft_note_stays_before_a_later_accent() {
    // A soft note told early, then an accent 300 ms or 1 s later: no precursor, it stays.
    // The full window's search ends at the pitch timeout (150 ms) with or without a pitch;
    // only an attack before that can take the note back. The short soft note gives the full
    // window little to hear.
    let hz = Pitch::parse("G2").map_or(0.0, |p| p.hz(A4_HZ));
    for soft_s in [0.045, 0.25] {
        for gap_s in [0.3, 1.0] {
            let soft = bass_tone(hz, SR, soft_s, ToneShape::Rich, 10);
            let accent = bass_tone(hz, SR, 0.5, ToneShape::Rich, 11);
            let mut signal = vec![0.0; (LEAD_S * SR) as usize];
            signal.extend(soft.iter().map(|x| x * 0.03));
            signal.resize(((LEAD_S + gap_s) * SR) as usize, 0.0);
            signal.extend(accent.iter().map(|x| x * 0.9));
            signal.extend(std::iter::repeat_n(0.0, (0.3 * SR) as usize));
            let (told, retracted) = track(&signal);
            let at = format!("{soft_s} s soft, accent after {gap_s} s");
            assert!(retracted.is_empty(), "{at}: {retracted:?}");
            assert_eq!(told.len(), 2, "{at}: {told:?}");
            assert!((told[0].onset_s - LEAD_S).abs() < 0.005, "{at}: {told:?}");
        }
    }
}
