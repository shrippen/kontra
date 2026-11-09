//! Call and response (LH-LRN-19): Kontra calls two bars, the player answers the same motif in
//! the next two. The answer is judged, the call never, even when a microphone hears it.

use kontra_model::{
    Arrangement, Chord, DetectedNote, InputSource, Instrument, Marker, MarkerKind, Note, Pitch,
    TempoMap, TICKS_PER_QUARTER,
};
use kontra_session::{call_and_response, Mode, ScoreSettings, Session};

const BAR: u64 = 4 * TICKS_PER_QUARTER;
const FIFTH: u8 = 7;

/// Four bars Am | D | Am | D in quarters at 120 BPM, with a fifth bar left over.
fn groove(chords: bool) -> Arrangement {
    let mut a = Arrangement::new("Groove", Instrument::default_bass());
    a.tempo = TempoMap::constant(120.0);
    if chords {
        for (bar, name) in ["Am", "D", "Am", "D", "Am"].iter().enumerate() {
            a.markers.push(Marker {
                tick: bar as u64 * BAR,
                name: name.to_string(),
                kind: MarkerKind::Chord,
            });
        }
    }
    for beat in 0..20u64 {
        a.notes.push(Note::new(
            beat * TICKS_PER_QUARTER,
            TICKS_PER_QUARTER,
            Pitch(33),
        ));
    }
    a.normalize();
    a
}

fn heard(onset_s: f64, midi: u8) -> DetectedNote {
    DetectedNote {
        onset_s,
        midi: f64::from(midi),
        confidence: 0.9,
        level_db: -12.0,
        source: InputSource::Microphone,
        technique: None,
        over_background: false,
    }
}

#[test]
fn the_motif_is_chord_tones_within_a_fifth_and_repeats() {
    let a = call_and_response(&groove(true)).expect("chords");
    assert_eq!(a.bars.len(), 4, "the bar left over goes");
    assert_eq!(a.calls, [(0, 2 * BAR)]);
    assert_eq!(a.notes.len(), 16);
    let (call, answer) = a.notes.split_at(8);
    let low = call.iter().map(|n| n.pitch.0).min().unwrap();
    let high = call.iter().map(|n| n.pitch.0).max().unwrap();
    assert!(high - low <= FIFTH, "{low}..{high}");
    for n in call {
        let bar = (n.start_ticks / BAR) as usize;
        let chord = Chord::parse(["Am", "D"][bar]).unwrap();
        assert!(chord.has(n.pitch.0), "{} in bar {bar}", n.pitch.0);
    }
    // The answer is the call, two bars later, with its tab.
    for (c, r) in call.iter().zip(answer) {
        assert_eq!(r.pitch, c.pitch);
        assert_eq!(r.start_ticks, c.start_ticks + 2 * BAR);
        assert!(r.string.is_some() && r.fret.is_some());
    }
}

#[test]
fn without_chords_there_is_no_call() {
    assert!(call_and_response(&groove(false)).is_none());
}

/// Acceptance LH-LRN-19: the answer in time is judged; the call, heard through a microphone,
/// is neither a hit nor an extra note.
#[test]
fn the_answer_is_judged_and_the_call_never() {
    let a = call_and_response(&groove(true)).unwrap();
    let end = a.duration_s();
    let mut session = Session::new(a.clone(), Mode::Song, ScoreSettings::default());
    session.play(0.0);
    let mut t = 0.0;
    for n in &a.notes {
        let onset = a.tempo.seconds_at(n.start_ticks) + 0.01;
        while t < onset {
            session.update(t);
            t += 0.01;
        }
        // The microphone hears Kontra's call as well as the player's answer.
        session.on_note(&heard(onset, n.pitch.0));
    }
    session.update(end + 1.0);
    let s = session.summary();
    assert_eq!((s.judged, s.hits, s.extra), (8, 8, 0), "{s:?}");
}
