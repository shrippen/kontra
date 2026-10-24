//! J-A2 Stimmen ohne Vorwissen (P1 Lea): the E string 30 cents flat. The recording planung/QA.md names
//! (`stimmen-e-30c.wav`) is rendered here, not committed.

use crate::common::{run_until, Profile, BPM, RATE};
use kontra_appcore::UiEvent;
use kontra_model::{Arrangement, Instrument, Note, Pitch, TempoMap, TICKS_PER_QUARTER};
use std::time::Duration;

const E1: u8 = 28;
const FLAT_CENTS: f64 = -30.0;

#[test]
fn e_string_30_cents_flat_reads_minus_30() {
    let profile = Profile::new("a2");
    let take = profile.dir.join("stimmen-e-30c.wav");
    let mut a = Arrangement::new("E", Instrument::default_bass());
    a.tempo = TempoMap::constant(BPM);
    a.notes.push(Note::new(0, 4 * TICKS_PER_QUARTER, Pitch(E1)));
    a.normalize();
    let mistakes = kontra_session::Mistakes {
        detune_cents: FLAT_CENTS,
        ..Default::default()
    };
    let samples = kontra_session::render_performance(&a, 1.0, 0.2, mistakes, RATE);
    kontra_session::write_mono_wav(&take, &samples, RATE).unwrap();

    let mut core = profile.core(Some(&take));
    core.start_tuner().unwrap();
    let events = run_until(&mut core, Duration::from_secs(3), |_| false);
    let readings: Vec<(String, f64)> = events
        .into_iter()
        .filter_map(|e| match e {
            UiEvent::Tuner { note, cents, .. } => Some((note, cents)),
            _ => None,
        })
        .collect();
    assert!(readings.len() >= 5, "{readings:?}");
    let mut cents: Vec<f64> = readings
        .iter()
        .filter(|(n, _)| n == "E1")
        .map(|(_, c)| *c)
        .collect();
    assert!(cents.len() * 2 > readings.len(), "{readings:?}");
    cents.sort_by(f64::total_cmp);
    let median = cents[cents.len() / 2];
    assert!((median - FLAT_CENTS).abs() <= 3.0, "{median} {readings:?}");
}
