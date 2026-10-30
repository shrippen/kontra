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
    assert_eq!(
        core.latency_note(),
        "auf den Klick gemessen, enthält deine Neigung",
        "without a cable the value holds the lean (R4)"
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

/// The loopback plan of `kontra_session::loopback_plan`: twelve E2 tones 0.6 s apart from one
/// second after the input starts, 0.25 s each.
const LOOPBACK_LEAD_S: f64 = 1.0;
const LOOPBACK_TONES: usize = 12;
const LOOPBACK_SPACING_S: f64 = 0.6;
const LOOPBACK_TONE_S: f64 = 0.25;
const E2_HZ: f64 = 82.41;
/// System latency of the simulated interface and the player's lean (timing report 29.10.).
const SYSTEM_MS: f64 = 20.0;
const LEAN_MS: f64 = -15.0;

/// What the input hears with a cable from output to input: the tones `delay_ms` late.
fn loopback_take(path: &std::path::Path, delay_ms: f64) {
    let length_s = LOOPBACK_LEAD_S + LOOPBACK_TONES as f64 * LOOPBACK_SPACING_S + 1.0;
    let mut samples = vec![0.0f32; (length_s * f64::from(RATE)) as usize];
    for i in 0..LOOPBACK_TONES {
        let at = LOOPBACK_LEAD_S + i as f64 * LOOPBACK_SPACING_S + delay_ms / 1000.0;
        let first = (at * f64::from(RATE)) as usize;
        let n = (LOOPBACK_TONE_S * f64::from(RATE)) as usize;
        for k in 0..n {
            let t = k as f64 / f64::from(RATE);
            let fade = (k.min(n - k) as f64 / 48.0).min(1.0);
            samples[first + k] =
                (0.5 * fade * (2.0 * std::f64::consts::PI * E2_HZ * t).sin()) as f32;
        }
    }
    kontra_session::write_mono_wav(path, &samples, RATE).unwrap();
}

/// Runs a measurement on `core` until it ends; the new latency (if any) and the text.
fn measure(core: &mut kontra_appcore::Core) -> (Option<f64>, String) {
    let events = run_until(core, Duration::from_secs(15), |e| {
        matches!(e, UiEvent::Calibrated { .. })
    });
    events
        .iter()
        .find_map(|e| match e {
            UiEvent::Calibrated { latency_ms, text } => Some((*latency_ms, text.clone())),
            _ => None,
        })
        .expect("measurement finished")
}

/// Eight plucks on the clicks, `offset_ms` after each.
fn click_take(path: &std::path::Path, offset_ms: f64) {
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
        kontra_session::render_performance(&clicks, 1.0, CLICK_LEAD_S, late(offset_ms), RATE);
    kontra_session::write_mono_wav(path, &samples, RATE).unwrap();
}

/// A song run with notes `offset_ms` late at the input, aim `pocket`; the summary.
fn song_summary(
    profile: &Profile,
    data: &kontra_appcore::Core,
    offset_ms: f64,
    pocket: &str,
) -> serde_json::Value {
    let score = profile.dir.join("Riff.mid");
    write_midi(&score, "Riff", &RIFF);
    let take = profile.dir.join("song.wav");
    let samples = kontra_session::render_performance(
        &piece("Riff", &RIFF),
        1.0,
        COUNT_IN_S,
        late(offset_ms),
        RATE,
    );
    kontra_session::write_mono_wav(&take, &samples, RATE).unwrap();

    let mut song = profile.core(Some(&take));
    assert_eq!(song.settings().latency_ms, data.settings().latency_ms);
    song.update_settings(|s| s.pocket = pocket.into());
    song.open(&score).unwrap();
    song.set_mode("song");
    song.play();
    let events = run_until(&mut song, Duration::from_secs(12), |e| {
        matches!(e, UiEvent::Finished { .. })
    });
    let summary = events
        .iter()
        .find_map(|e| match e {
            UiEvent::Finished { summary_json } => Some(summary_json.clone()),
            _ => None,
        })
        .expect("finished");
    serde_json::from_str(&summary).unwrap()
}

// Timing report 29.10. (R4): "play on the click" took the player's lean for latency, so 15 ms
// early showed "on the point". With a cable value first, the clicks measure the lean instead.
#[test]
fn loopback_keeps_the_players_lean() {
    let profile = Profile::new("a3-lean");

    // Cable: the tones come back 20 ms late.
    let cable = profile.dir.join("kabel.wav");
    loopback_take(&cable, SYSTEM_MS);
    let mut core = profile.core(Some(&cable));
    core.start_loopback().unwrap();
    let (latency, text) = measure(&mut core);
    let latency = latency.expect("measured by cable");
    assert!((latency - SYSTEM_MS).abs() <= 3.0, "{latency} ms: {text}");
    assert_eq!(core.latency_note(), "mit Kabel gemessen");

    // Clicks: the player plucks 15 ms early, heard 5 ms after each click.
    let plucks = profile.dir.join("klicks.wav");
    click_take(&plucks, latency + LEAN_MS);
    let mut core = profile.core(Some(&plucks));
    core.start_calibration().unwrap();
    let (_, text) = measure(&mut core);
    assert_eq!(
        core.settings().latency_ms,
        latency,
        "cable value kept: {text}"
    );
    let lean = core.lean_ms().expect("lean measured");
    assert!((lean - LEAN_MS).abs() <= 3.0, "{lean} ms");
    assert!(text.contains("vor dem Klick"), "{text}");
    assert!(
        core.latency_note().contains("deine Neigung: \u{2212}1"),
        "{}",
        core.latency_note()
    );

    // Song with the same lean, aim "Auf den Punkt": missed, 15 ms early, no "recalibrate".
    let s = song_summary(&profile, &core, latency + LEAN_MS, "onbeat");
    let mean = s["meanOffsetMs"].as_f64().unwrap();
    assert!((mean - LEAN_MS).abs() <= 3.0, "{mean} ms: {s}");
    let pocket = s["pocketText"].as_str().unwrap();
    assert!(pocket.contains("verfehlt"), "{pocket}");
    assert_eq!(s["recalibrateText"], "", "{s}");
}

// LH-EIN-14: a cable value holds no lean; a steady lean in the run is the player's.
#[test]
fn loopback_latency_never_asks_to_recalibrate() {
    let profile = Profile::new("a3-hint");
    let cable = profile.dir.join("kabel.wav");
    loopback_take(&cable, SYSTEM_MS);
    let mut core = profile.core(Some(&cable));
    core.start_loopback().unwrap();
    let latency = measure(&mut core).0.expect("measured by cable");

    let s = song_summary(&profile, &core, latency + LEAN_MS, "free");
    assert_eq!(s["recalibrateText"], "", "{s}");
    let observation = s["observation"]["text"].as_str().unwrap_or_default();
    assert!(observation.contains("vor"), "the lean is named: {s}");
}

// P1 at the laptop jack has no cable: the tones do not come back, the latency stays.
#[test]
fn loopback_without_cable_says_so() {
    let profile = Profile::new("a3-nocable");
    let silence = profile.dir.join("still.wav");
    kontra_session::write_mono_wav(&silence, &vec![0.0; 10 * RATE as usize], RATE).unwrap();
    let mut core = profile.core(Some(&silence));
    core.update_settings(|s| s.latency_ms = 30.0);
    core.start_loopback().unwrap();
    let (latency, text) = measure(&mut core);
    assert_eq!(latency, None);
    assert!(text.contains("Ohne Kabel"), "{text}");
    assert_eq!(core.settings().latency_ms, 30.0);
}
