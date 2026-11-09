//! Where notes end (E5, LH-BEW-10): the tracker tells a muted note's end, a note the next one
//! replaces, and no mute while a note rings.

use kontra_dsp::synth::{render, SynthNote, ToneShape};
use kontra_dsp::{NoteTracker, TrackerConfig};
use kontra_model::{midi_to_hz, Ending, InputSource, Instrument, NoteEnd, A4_HZ};

const SR: f64 = 48_000.0;
const BLOCK: usize = 480;
/// The synthetic mute fades over the note's last 15 ms (`synth::render`).
const FADE_S: f64 = 0.015;
/// LH-BEW-10: a muted note's end within this.
const WITHIN_S: f64 = 0.03;

fn note(onset_s: f64, midi: u8, duration_s: f64) -> SynthNote {
    SynthNote {
        onset_s,
        hz: midi_to_hz(f64::from(midi), A4_HZ),
        duration_s,
        velocity: 0.8,
    }
}

/// The ends told for `notes`, rendered over `total_s`.
fn ends(notes: &[SynthNote], total_s: f64) -> Vec<NoteEnd> {
    let samples = render(notes, SR, total_s, ToneShape::Rich, 3);
    let config = TrackerConfig::for_instrument(&Instrument::default_bass(), InputSource::Line);
    let mut tracker = NoteTracker::new(SR, config);
    let mut told = Vec::new();
    for block in samples.chunks(BLOCK) {
        tracker.push(block);
        told.extend_from_slice(tracker.ends());
    }
    tracker.flush();
    told.extend_from_slice(tracker.ends());
    told
}

#[test]
fn a_muted_note_ends_where_the_hand_stops_it() {
    for midi in [28, 33, 40, 50] {
        let told = ends(&[note(0.3, midi, 0.4)], 1.5);
        let [end] = told[..] else {
            panic!("midi {midi}: {told:?}");
        };
        assert_eq!(end.ending, Ending::Muted, "midi {midi}");
        let stopped_s = 0.3 + 0.4 - FADE_S;
        assert!(
            (end.end_s - stopped_s).abs() <= WITHIN_S,
            "midi {midi}: {end:?}"
        );
    }
}

#[test]
fn the_next_note_replaces_a_ringing_one() {
    let told = ends(&[note(0.3, 33, 2.0), note(0.8, 38, 0.4)], 2.0);
    let first = told.iter().find(|e| e.onset_s < 0.5);
    let Some(first) = first else {
        panic!("{told:?}");
    };
    assert_eq!(first.ending, Ending::Replaced);
    assert!((first.end_s - 0.8).abs() <= WITHIN_S, "{first:?}");
}

#[test]
fn a_held_note_is_not_muted() {
    let told = ends(&[note(0.3, 28, 3.0)], 3.5);
    let muted = told
        .iter()
        .find(|e| e.ending == Ending::Muted && e.end_s < 3.3 - 0.1);
    assert!(muted.is_none(), "{told:?}");
}
