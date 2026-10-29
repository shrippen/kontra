//! J-A3 Latenz kalibrieren (P3 Mira): "play on the click" with plucks 40 ms late, then a song run
//! whose notes reach Kontra with the same delay is judged on time. Step 4 (PipeWire quantum
//! 2048) needs the audio server and stays a manual step.

use crate::common::{last_judged, piece, run_until, write_midi, Profile, COUNT_IN_S, RATE};
use kontra_appcore::UiEvent;
use kontra_model::{Arrangement, Instrument, Note, Pitch, TempoMap, TICKS_PER_QUARTER};
use std::time::Duration;

const LATENCY_MS: f64 = 40.0;
/// The calibration's measured clicks: eight quarters at 100 BPM, 3 s after the input starts
/// (after the count-in).
const CLICK_BPM: f64 = 100.0;
const CLICKS: u64 = 8;
const CLICK_LEAD_S: f64 = 3.0;
const RIFF: [u8; 8] = [28, 33, 38, 43, 28, 33, 38, 43];

fn late(ms: f64) -> kontra_session::Mistakes {
    kontra_session::Mistakes {
        offset_ms: ms,
        ..Default::default()
    }
}

#[test]
fn calibrated_latency_makes_late_notes_on_time() {
    let profile = Profile::new("a3");

    // Steps 1–2: eight plucks, each 40 ms after its click.
    let plucks = profile.dir.join("klicks.wav");
    let mut clicks = Arrangement::new("Klicks", Instrument::default_bass());
    clicks.tempo = TempoMap::constant(CLICK_BPM);
    for i in 0..CLICKS {
        clicks.notes.push(Note::new(
            i * TICKS_PER_QUARTER,
            TICKS_PER_QUARTER / 2,
            Pitch(33),
        ));
    }
    clicks.normalize();
    let samples =
        kontra_session::render_performance(&clicks, 1.0, CLICK_LEAD_S, late(LATENCY_MS), RATE);
    kontra_session::write_mono_wav(&plucks, &samples, RATE).unwrap();

    let mut core = profile.core(Some(&plucks));
    core.start_calibration().unwrap();
    let events = run_until(&mut core, Duration::from_secs(12), |e| {
        matches!(e, UiEvent::Calibrated { .. })
    });
    let measured = events.iter().find_map(|e| match e {
        UiEvent::Calibrated { latency_ms, text } => Some((*latency_ms, text.clone())),
        _ => None,
    });
    let (latency, text) = measured.expect("calibration finished");
    let latency = latency.expect("a latency was measured");
    assert!((latency - LATENCY_MS).abs() <= 3.0, "{latency} ms");
    assert!(
        text.contains(&format!("{latency:.0}")),
        "shown in words: {text}"
    );

    // Step 3: a run whose notes arrive 40 ms late is judged on time (±5 ms).
    let score = profile.dir.join("Riff.mid");
    write_midi(&score, "Riff", &RIFF);
    let take = profile.dir.join("song.wav");
    let samples = kontra_session::render_performance(
        &piece("Riff", &RIFF),
        1.0,
        COUNT_IN_S,
        late(LATENCY_MS),
        RATE,
    );
    kontra_session::write_mono_wav(&take, &samples, RATE).unwrap();

    let mut song = profile.core(Some(&take));
    assert_eq!(song.settings().latency_ms, latency, "kept in the profile");
    song.open(&score).unwrap();
    song.set_mode("song");
    song.play();
    let events = run_until(&mut song, Duration::from_secs(9), |e| {
        matches!(e, UiEvent::Finished { .. })
    });
    let offsets: Vec<f64> = last_judged(&events)
        .into_values()
        .map(|(state, offset, _)| {
            assert_eq!(state, "hit");
            offset
        })
        .collect();
    assert_eq!(offsets.len(), RIFF.len(), "{offsets:?}");
    assert!(offsets.iter().all(|o| o.abs() <= 5.0), "{offsets:?}");
}
