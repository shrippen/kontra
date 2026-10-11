//! J-D1 Original-Song als Backing Track (P2 Jens): couple a band recording to the score, detect
//! its beats, nudge two bars by hand, play at 75 %, then a whole step higher, and open it again.
//! The band is rendered as clicks (no committed audio); "no audible offset over four minutes"
//! needs ears and a device and stays a manual step.

use crate::common::{judged, run_until, write_midi, Profile, COUNT_IN_S, RATE};
use kontra_appcore::UiEvent;
use kontra_model::Arrangement;
use std::path::Path;
use std::time::Duration;

const RIFF: [u8; 8] = [28, 33, 38, 43, 28, 33, 38, 43];
const BAND_BPM: f64 = 120.0;
/// The band's first downbeat in its recording.
const BAND_START_S: f64 = 1.0;
const SLOW: f64 = 0.75;
const NUDGE_MS: f64 = 20.0;
const UP: i32 = 2;

/// Noise clicks on every beat of the band, as in the beat tracker's tests.
fn band(path: &Path) {
    let mut audio = vec![0.0f32; RATE as usize * 14];
    let mut noise = 1u32;
    for beat in 0..24 {
        let start = ((BAND_START_S + f64::from(beat) * 60.0 / BAND_BPM) * f64::from(RATE)) as usize;
        for i in 0..1500 {
            noise = noise.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let n = (noise >> 9) as f32 / (1u32 << 23) as f32 - 0.5;
            if let Some(s) = audio.get_mut(start + i) {
                *s += n * (-(i as f32) / 600.0).exp();
            }
        }
    }
    kontra_session::write_mono_wav(path, &audio, RATE).unwrap();
}

/// The bass played along `a` at `speed`, after the count-in bar.
fn take(path: &Path, a: &Arrangement, speed: f64) {
    let samples =
        kontra_session::render_performance(a, speed, COUNT_IN_S / speed, Default::default(), RATE);
    kontra_session::write_mono_wav(path, &samples, RATE).unwrap();
}

fn play_all_hit(core: &mut kontra_appcore::Core, seconds: u64) {
    core.set_mode("song");
    let start = core.play();
    assert!(
        !start.iter().any(|e| matches!(e, UiEvent::Error(_))),
        "{start:?}"
    );
    let events = run_until(core, Duration::from_secs(seconds), |e| {
        matches!(e, UiEvent::Finished { .. })
    });
    let states = judged(&events);
    assert_eq!(states.len(), RIFF.len(), "{states:?}");
    assert!(states.iter().all(|s| *s == "hit"), "{states:?}");
}

#[test]
fn backing_beats_slow_and_transposed() {
    let profile = Profile::new("d1");
    let score = profile.dir.join("Riff.mid");
    write_midi(&score, "Riff", &RIFF);
    let wav = profile.dir.join("band.wav");
    band(&wav);
    let slow_take = profile.dir.join("slow.wav");
    let up_take = profile.dir.join("up.wav");

    // Step 1: couple and detect: the bars follow the band.
    let mut setup = profile.core(None);
    setup.open(&score).unwrap();
    setup.set_backing(Some(wav.clone()), 0.0);
    setup.backing_analyse().unwrap();
    let info: serde_json::Value = serde_json::from_str(&setup.backing_json()).unwrap();
    assert!(
        (info["bpm"].as_f64().unwrap() - BAND_BPM).abs() < 2.0,
        "{info}"
    );
    assert!(
        (info["firstS"].as_f64().unwrap() - BAND_START_S).abs() < 0.02,
        "{info}"
    );

    // Step 2: two bar starts moved by hand.
    let bar_s = |core: &kontra_appcore::Core, bar: usize| {
        let a = core.arrangement().unwrap();
        a.tempo.seconds_at(a.bars[bar].start_ticks)
    };
    let before = bar_s(&setup, 1);
    setup.backing_adjust(0, 0.0, Some((1, NUDGE_MS))).unwrap();
    let nudged = bar_s(&setup, 1) - before;
    assert!(
        (nudged - NUDGE_MS / 1000.0).abs() < 0.002,
        "moved {nudged} s"
    );
    setup.backing_adjust(0, 0.0, Some((1, -NUDGE_MS))).unwrap();
    assert!(
        (bar_s(&setup, 1) - before).abs() < 0.002,
        "back where it was"
    );
    take(&slow_take, setup.arrangement().unwrap(), SLOW);

    // Step 3: at 75 % every note on time.
    let mut core = profile.core(Some(&slow_take));
    core.open(&score).unwrap();
    core.set_speed(SLOW);
    play_all_hit(&mut core, 12);

    // Step 4: a whole step up; tab and notes move with the backing.
    let original: Vec<u8> = core
        .arrangement()
        .unwrap()
        .notes
        .iter()
        .map(|n| n.pitch.0)
        .collect();
    core.set_transpose(UP).unwrap();
    let up = core.arrangement().unwrap().clone();
    assert!(
        up.notes.iter().all(|n| n.fret.is_some()),
        "every note has a place"
    );
    take(&up_take, &up, 1.0);

    let mut again = profile.core(Some(&up_take));
    again.open(&score).unwrap();
    assert_eq!(again.transpose(), UP, "kept with the piece");
    let moved: Vec<u8> = again
        .arrangement()
        .unwrap()
        .notes
        .iter()
        .map(|n| n.pitch.0)
        .collect();
    assert!(
        original
            .iter()
            .zip(&moved)
            .all(|(o, m)| i32::from(*m) - i32::from(*o) == UP),
        "{original:?} → {moved:?}"
    );
    play_all_hit(&mut again, 9);
}
