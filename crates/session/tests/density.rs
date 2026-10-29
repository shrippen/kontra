//! Adaptive difficulty (LH-MOD-06): thinning by density and suggestions between runs.

use kontra_model::{
    Arrangement, DetectedNote, InputSource, Instrument, Note, Pitch, TempoMap, TICKS_PER_QUARTER,
};
use kontra_session::{Density, Mode, ScoreSettings, Session};

const EIGHTH: u64 = TICKS_PER_QUARTER / 2;

/// Two bars of eighth notes at 120 BPM (0.25 s apart), pitches rising from E1.
fn eighths() -> Arrangement {
    let mut a = Arrangement::new("Eighths", Instrument::default_bass());
    a.tempo = TempoMap::constant(120.0);
    for i in 0..16u64 {
        a.notes
            .push(Note::new(i * EIGHTH, EIGHTH, Pitch(28 + (i % 5) as u8)));
    }
    a.normalize();
    a
}

fn starts(a: &Arrangement) -> Vec<u64> {
    a.notes.iter().map(|n| n.start_ticks / EIGHTH).collect()
}

#[test]
fn thinning_keeps_beats_or_strong_beats() {
    let a = eighths();
    assert_eq!(Density::Full.thin(&a).notes.len(), 16);
    assert_eq!(
        starts(&Density::Beats.thin(&a)),
        [0, 2, 4, 6, 8, 10, 12, 14]
    );
    assert_eq!(starts(&Density::Sparse.thin(&a)), [0, 4, 8, 12]);
}

#[test]
fn every_bar_keeps_its_first_attack_and_ties_follow() {
    // Bar 1: an offbeat eighth tied into the next; bar 2: a quarter on beat 2.
    let mut a = Arrangement::new("Offbeat", Instrument::default_bass());
    let mut tied = Note::new(2 * EIGHTH, EIGHTH, Pitch(33));
    tied.tied = true;
    a.notes.push(Note::new(EIGHTH, EIGHTH, Pitch(33)));
    a.notes.push(tied);
    a.notes.push(Note::new(5 * EIGHTH, EIGHTH, Pitch(35)));
    a.notes.push(Note::new(
        8 * EIGHTH + TICKS_PER_QUARTER,
        TICKS_PER_QUARTER,
        Pitch(28),
    ));
    a.normalize();
    // The offbeat note opens bar 1 and keeps its tie; the other offbeat goes.
    assert_eq!(starts(&Density::Sparse.thin(&a)), [1, 2, 10]);
}

#[test]
fn suggestions_step_once_and_stop_at_the_ends() {
    assert_eq!(Density::Sparse.suggest(0.95), Some(Density::Beats));
    assert_eq!(Density::Full.suggest(0.95), None);
    assert_eq!(Density::Full.suggest(0.4), Some(Density::Beats));
    assert_eq!(Density::Sparse.suggest(0.4), None);
    assert_eq!(Density::Beats.suggest(0.75), None);
    assert_eq!(Density::from_level(2), Density::Beats);
    assert_eq!(Density::Beats.level(), 2);
}

fn played(onset_s: f64, midi: f64) -> DetectedNote {
    DetectedNote {
        onset_s,
        midi,
        confidence: 0.9,
        level_db: -12.0,
        source: InputSource::Line,
        technique: None,
        over_background: false,
    }
}

/// A player who misses one note in twenty (95 %) on a full song, as hit rate of a run.
fn run_at_95(arrangement: Arrangement) -> f64 {
    let mut session = Session::new(arrangement.clone(), Mode::Song, ScoreSettings::default());
    session.play(0.0);
    let mut t = 0.0;
    for (i, note) in arrangement.notes.iter().enumerate() {
        let onset = arrangement.tempo.seconds_at(note.start_ticks);
        while t + 0.01 < onset + 0.08 {
            session.update(t);
            t += 0.01;
        }
        if i % 20 != 19 {
            session.on_note(&played(onset, f64::from(note.pitch.0)));
        }
    }
    session.update(arrangement.duration_s() + 1.0);
    session.summary().hit_rate
}

/// Acceptance LH-MOD-06: 95 % hits reach full density in at most five runs, accepting each
/// suggestion.
#[test]
fn player_at_95_percent_reaches_full_density_within_five_runs() {
    let mut song = eighths();
    for i in 16..40u64 {
        song.notes
            .push(Note::new(i * EIGHTH, EIGHTH, Pitch(28 + (i % 5) as u8)));
    }
    song.normalize();

    let mut density = Density::Sparse;
    let mut runs = 0;
    while density != Density::Full && runs < 5 {
        let rate = run_at_95(density.thin(&song));
        runs += 1;
        if let Some(next) = density.suggest(rate) {
            density = next;
        }
    }
    assert_eq!(density, Density::Full, "after {runs} runs");
}
