//! J-D2 Übung an Schüler geben (P4 Thomas, Runde 15): the teacher aligns a band recording, sets
//! two loops and a target tempo, practises once and shares the piece with its backing. The
//! student opens the package on a second profile: same loops, tempo and bars, the backing plays
//! from the student's own data folder without aligning, and their progress starts at zero.

use crate::common::{band, judged, run_until, write_midi, Profile, COUNT_IN_S, RATE};
use kontra_appcore::{Core, Opened, PackageAudio, UiEvent};
use kontra_model::Arrangement;
use std::path::Path;
use std::time::Duration;

/// Four bars, so two loops of two bars each.
const RIFF: [u8; 16] = [
    28, 33, 38, 43, 28, 33, 38, 43, 40, 38, 36, 33, 28, 33, 38, 43,
];
const BAND_BPM: f64 = 120.0;
const BAND_START_S: f64 = 1.0;
const TARGET_SPEED: f64 = 0.8;
const LOOPS: [(usize, usize); 2] = [(0, 1), (2, 3)];

/// The bass played along `a`, after the count-in bar.
fn take(path: &Path, a: &Arrangement) {
    let samples = kontra_session::render_performance(a, 1.0, COUNT_IN_S, Default::default(), RATE);
    kontra_session::write_mono_wav(path, &samples, RATE).unwrap();
}

/// One run in song mode, every note a hit.
fn play_song(core: &mut Core) {
    core.set_mode("song");
    let start = core.play();
    assert!(
        !start.iter().any(|e| matches!(e, UiEvent::Error(_))),
        "{start:?}"
    );
    let events = run_until(core, Duration::from_secs(16), |e| {
        matches!(e, UiEvent::Finished { .. })
    });
    let states = judged(&events);
    assert_eq!(states.len(), RIFF.len(), "{states:?}");
    assert!(states.iter().all(|s| *s == "hit"), "{states:?}");
}

/// Bar starts in ms, rounded: where the bars fall in the backing.
fn bar_times(core: &Core) -> Vec<i64> {
    let a = core.arrangement().unwrap();
    a.bars
        .iter()
        .map(|b| (a.tempo.seconds_at(b.start_ticks) * 1000.0).round() as i64)
        .collect()
}

fn pieces(core: &Core) -> Vec<serde_json::Value> {
    let p: serde_json::Value = serde_json::from_str(&core.progress_json()).unwrap();
    p["pieces"].as_array().unwrap().clone()
}

#[test]
fn package_carries_loops_tempo_backing() {
    let thomas = Profile::new("d2-thomas");
    let score = thomas.dir.join("Riff.mid");
    write_midi(&score, "Riff", &RIFF);
    let wav = thomas.dir.join("band.wav");
    band(&wav, BAND_BPM, BAND_START_S);

    // Step 1: backing aligned, loops and target tempo set, one run practised.
    let mut setup = thomas.core(None);
    setup.open(&score).unwrap();
    setup.set_backing(Some(wav.clone()), 0.0);
    setup.backing_analyse().unwrap();
    for (from, to) in LOOPS {
        setup.set_loop(from, to);
        setup.save_loop();
    }
    setup.clear_loop();
    setup.set_target_speed(TARGET_SPEED);
    let aligned: serde_json::Value = serde_json::from_str(&setup.backing_json()).unwrap();
    let bars = bar_times(&setup);
    let thomas_take = thomas.dir.join("take.wav");
    take(&thomas_take, setup.arrangement().unwrap());
    drop(setup);

    let mut teacher = thomas.core(Some(&thomas_take));
    teacher.open(&score).unwrap();
    play_song(&mut teacher);
    assert_eq!(pieces(&teacher).len(), 1, "the teacher practised");
    let package = thomas.dir.join("Riff.kontra");
    teacher
        .export_package(&score, &package, PackageAudio::Embed)
        .unwrap();

    // Step 2: on the student's profile; the teacher's file is out of reach.
    let lena = Profile::new("d2-schueler");
    let shared = lena.dir.join("Riff.kontra");
    std::fs::copy(&package, &shared).unwrap();
    drop(thomas);

    let mut student = lena.core(None);
    assert_eq!(student.open(&shared).unwrap(), Opened::Ready);
    assert_eq!(student.saved_loops(), LOOPS.to_vec());
    assert_eq!(student.target_speed(), TARGET_SPEED);
    // No aligning again: the same beat map (JSON keeps floats to the last bit or so).
    let got: serde_json::Value = serde_json::from_str(&student.backing_json()).unwrap();
    assert_eq!(got["text"], aligned["text"], "{got}");
    for key in ["bpm", "firstS"] {
        let (a, b) = (got[key].as_f64().unwrap(), aligned[key].as_f64().unwrap());
        assert!((a - b).abs() < 1e-9, "{key}: {a} vs {b}");
    }
    assert_eq!(bar_times(&student), bars);
    assert_eq!(student.backing_name(), "band.wav");
    assert!(lena.dir.join("packages/Riff/band.wav").exists());
    assert!(pieces(&student).is_empty(), "progress starts at zero");
    let lena_take = lena.dir.join("take.wav");
    take(&lena_take, student.arrangement().unwrap());
    drop(student);

    // The backing plays with the student's run, which is the only one in their progress.
    let mut student = lena.core(Some(&lena_take));
    student.open(&shared).unwrap();
    play_song(&mut student);
    let own = pieces(&student);
    assert_eq!(own.len(), 1);
    assert_eq!(own[0]["runs"], 1);
}
