//! LH-EIN-09 / LH-NF-14: a new instrument is a new profile, not a rebuild. A dummy six-string
//! guitar (E2–E4, an octave above the bass) runs through tab assignment, detection and scoring
//! without any change to session or scoring: only the profile differs.

use kontra_model::{
    Arrangement, BassKind, InputSource, Instrument, Note, Pitch, TempoMap, Tuning,
    TICKS_PER_QUARTER,
};
use kontra_session::{
    assign_tab, render_performance, write_mono_wav, Mode, Outcome, ScoreSettings, Session,
    SessionEvent,
};

const RATE: u32 = 48_000;
const LEAD_IN_S: f64 = 0.5;
/// A riff across all six strings: E2 G2 A2 C3 D3 F3 G3 A3 B3 D4 E4 G4.
const RIFF: [u8; 12] = [40, 43, 45, 48, 50, 53, 55, 57, 59, 62, 64, 67];

fn guitar() -> Instrument {
    let tuning = Tuning::from_names("guitar", &["E2", "A2", "D3", "G3", "B3", "E4"]).unwrap();
    Instrument::new(BassKind::Fretted, tuning, 22).unwrap()
}

fn riff(instrument: Instrument) -> Arrangement {
    let mut a = Arrangement::new("Riff", instrument);
    a.tempo = TempoMap::constant(120.0);
    for (i, midi) in RIFF.iter().enumerate() {
        a.notes.push(Note::new(
            i as u64 * TICKS_PER_QUARTER,
            TICKS_PER_QUARTER,
            Pitch(*midi),
        ));
    }
    a.normalize();
    a
}

#[test]
fn a_guitar_profile_needs_no_change_in_session_or_scoring() {
    let mut a = riff(guitar());

    // Tab: every note gets a string and fret of the guitar.
    assign_tab(&mut a);
    for n in &a.notes {
        let (string, fret) = (n.string.unwrap(), n.fret.unwrap());
        assert_eq!(a.instrument.pitch_at(string, fret), Some(n.pitch), "{n:?}");
    }

    // Detection: the profile's range, not the bass's, decides what is heard.
    let samples = render_performance(&a, 1.0, LEAD_IN_S, Default::default(), RATE);
    let dir = std::env::temp_dir().join(format!("kontra-profiles-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let take = dir.join("guitar.wav");
    write_mono_wav(&take, &samples, RATE).unwrap();
    let heard =
        kontra_session::analyze_file(&take, &a.instrument, InputSource::Line, false).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(heard.len(), RIFF.len(), "{heard:?}");

    // Scoring: the same session as for the bass.
    let mut session = Session::new(a, Mode::Song, ScoreSettings::default());
    session.play(LEAD_IN_S);
    let mut outcomes = Vec::new();
    for note in &heard {
        session.on_note(note);
    }
    for step in 0..=1000 {
        for event in session.update(f64::from(step) * 0.01) {
            if let SessionEvent::Result(r) = event {
                outcomes.push(r.outcome);
            }
        }
    }
    assert_eq!(outcomes.len(), RIFF.len(), "{outcomes:?}");
    assert!(outcomes.iter().all(|o| *o == Outcome::Hit), "{outcomes:?}");
}
