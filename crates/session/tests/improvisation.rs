//! Improvisation scoring (LH-BEW-13): an open exercise counts each attack by how it fits the
//! bar's chord, not against the written example line.

use kontra_model::{
    Arrangement, DetectedNote, InputSource, Instrument, Judging, Marker, MarkerKind, Note, Pitch,
    TempoMap, TICKS_PER_QUARTER,
};
use kontra_session::{Mode, ScoreSettings, Session};

const BAR: u64 = 4 * TICKS_PER_QUARTER;
/// 120 BPM: a quarter is half a second.
const QUARTER_S: f64 = 0.5;

/// Four bars Am7 | D7 | Gmaj7 | Cmaj7 (key of C), an example line of quarters, open.
fn open_changes(rest_in_bar_two: bool) -> Arrangement {
    let mut a = Arrangement::new("Open", Instrument::default_bass());
    a.tempo = TempoMap::constant(120.0);
    a.judging = Judging::Open;
    for (bar, chord) in ["Am7", "D7", "Gmaj7", "Cmaj7"].iter().enumerate() {
        a.markers.push(Marker {
            tick: bar as u64 * BAR,
            name: chord.to_string(),
            kind: MarkerKind::Chord,
        });
    }
    for beat in 0..16u64 {
        // Bar two of the example rests on beats 3 and 4: a rest window.
        if rest_in_bar_two && (6..8).contains(&beat) {
            continue;
        }
        a.notes.push(Note::new(
            beat * TICKS_PER_QUARTER,
            TICKS_PER_QUARTER,
            Pitch(33),
        ));
    }
    a.normalize();
    a
}

fn played(onset_s: f64, midi: u8) -> DetectedNote {
    DetectedNote {
        onset_s,
        midi: f64::from(midi),
        confidence: 0.9,
        level_db: -12.0,
        source: InputSource::Line,
        technique: None,
        over_background: false,
    }
}

/// Plays `line` (one MIDI note per quarter, `None` a rest) through a song run.
fn run(arrangement: Arrangement, line: &[Option<u8>]) -> kontra_session::Summary {
    let end = arrangement.duration_s();
    let mut session = Session::new(arrangement, Mode::Song, ScoreSettings::default());
    session.play(0.0);
    for (beat, midi) in line.iter().enumerate() {
        let at = beat as f64 * QUARTER_S;
        session.update(at);
        if let Some(midi) = midi {
            session.on_note(&played(at + 0.005, *midi));
        }
    }
    session.update(end + 1.0);
    session.summary()
}

/// Acceptance LH-BEW-13: a walking bass of chord tones only is 100 % chord tones.
#[test]
fn walking_bass_of_chord_tones_is_all_chord_tones() {
    // Am7: A C E G · D7: D F# A C · Gmaj7: G B D F# · Cmaj7: C E G B (none of the example).
    let walk = [
        33, 36, 40, 43, 38, 42, 45, 48, 31, 35, 38, 42, 36, 40, 43, 47,
    ];
    let line: Vec<Option<u8>> = walk.iter().copied().map(Some).collect();
    let summary = run(open_changes(false), &line);
    let open = summary.improvisation.expect("open run");
    assert_eq!(open.chord_tones, 16);
    assert_eq!(open.chord_share(), Some(1.0));
    assert_eq!(summary.hit_rate, 1.0);
    // No written note is missed: the example line is not judged.
    assert_eq!(summary.missed, 0);
    assert_eq!(summary.judged, 0);
}

#[test]
fn tones_of_the_key_fit_and_others_are_foreign() {
    // Over Am7: B (in C major) fits, A# (foreign) does not; then two chord tones.
    let line = [Some(35), Some(34), Some(33), Some(36)];
    let open = run(open_changes(false), &line).improvisation.unwrap();
    assert_eq!(
        (open.chord_tones, open.scale_tones, open.foreign),
        (2, 1, 1)
    );
    assert_eq!(open.fitting_share(), Some(0.75));
    assert_eq!(open.chord_share(), Some(0.5));
}

#[test]
fn rests_of_the_example_ask_for_room() {
    let quarters = |rest: bool| -> Vec<Option<u8>> {
        (0..16)
            .map(|beat| (!(rest && (6..8).contains(&beat))).then_some(45))
            .collect()
    };
    // Left free: kept. Played through: not.
    let kept = run(open_changes(true), &quarters(true))
        .improvisation
        .unwrap();
    assert_eq!((kept.rests, kept.rests_kept), (1, 1));
    let through = run(open_changes(true), &quarters(false))
        .improvisation
        .unwrap();
    assert_eq!((through.rests, through.rests_kept), (1, 0));
    // Without rests in the example there are no windows.
    let none = run(open_changes(false), &quarters(false))
        .improvisation
        .unwrap();
    assert_eq!(none.rests, 0);
}

#[test]
fn written_runs_have_no_improvisation() {
    let mut a = open_changes(false);
    a.judging = Judging::PitchAndTime;
    assert!(run(a, &[Some(33)]).improvisation.is_none());
}

/// Review #226: a silent pass of an open run is no clean pass at 100 %; it would offer more
/// notes, move building blocks on and feed the band. Nothing played is 0 %, not clean.
#[test]
fn silent_open_pass_is_not_clean() {
    use kontra_session::{LoopRange, SessionEvent};
    let mut session = Session::new(open_changes(false), Mode::Song, ScoreSettings::default());
    session.set_loop(
        0.0,
        Some(LoopRange {
            start_s: 0.0,
            end_s: 4.0 * QUARTER_S,
        }),
    );
    session.play(0.0);
    let mut events = Vec::new();
    for step in 0..30 {
        events.extend(session.update(f64::from(step) * 0.1));
    }
    let pass = events.iter().find_map(|e| match e {
        SessionEvent::PassComplete {
            clean, hit_rate, ..
        } => Some((*clean, *hit_rate)),
        _ => None,
    });
    assert_eq!(pass, Some((false, 0.0)), "{events:?}");
}
