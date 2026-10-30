//! Playback: backing track, transpose, loop keys, calibration, recorded runs.

use super::*;

/// QA P2 (F9): a backing file that is gone must not stop the piece; it plays without, and
/// says so in the user's language.
#[test]
fn missing_backing_plays_without_it() {
    let dir = temp_dir("backing");
    let take = dir.join("take.wav");
    std::fs::create_dir_all(&dir).unwrap();
    kontra_session::write_mono_wav(&take, &vec![0.0; 48_000], 48_000).unwrap();
    let mut core = Core::new(
        &dir,
        QaOptions {
            input_file: Some(take),
            laptop_jack: false,
        },
    );
    core.update_settings(|s| s.language = Some("de".into()));
    core.open(&demo()).unwrap();
    core.set_backing(Some(dir.join("weg.wav")), 0.0);
    let events = core.play();
    assert!(
        !events.iter().any(|e| matches!(e, UiEvent::Error(_))),
        "{events:?}"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, UiEvent::Message(m) if m.contains("weg.wav"))),
        "{events:?}"
    );
    core.stop_live();
    let _ = std::fs::remove_dir_all(dir);
}

/// QA 27.10. (J2): the backing track was decoded in the UI thread on every first play after
/// opening. Now a thread prepares it once; play waits for it and then reads the cache.
#[test]
fn backing_is_prepared_in_the_background() {
    let dir = temp_dir("prepare");
    // A recording as input: never the real microphone (CI has none, QA 28.10.).
    let take = dir.join("take.wav");
    kontra_session::write_mono_wav(&take, &vec![0.0; 48_000], 48_000).unwrap();
    let mut core = Core::new(
        &dir,
        QaOptions {
            input_file: Some(take),
            laptop_jack: false,
        },
    );
    core.open(&demo()).unwrap();
    let rate = 44_100u32;
    let audio: Vec<f32> = (0..rate * 3)
        .map(|i| 0.3 * (2.0 * std::f32::consts::PI * 110.0 * i as f32 / rate as f32).sin())
        .collect();
    let wav = dir.join("band.wav");
    kontra_session::write_mono_wav(&wav, &audio, rate).unwrap();
    core.set_backing(Some(wav), 0.0);

    let events = core.tick();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, UiEvent::BackingLoading(true))),
        "{events:?}"
    );
    assert!(core.backing_loading());
    let waits = core.play();
    assert!(
        waits.iter().any(|e| matches!(e, UiEvent::Message(_))) && !core.has_run(),
        "{waits:?}"
    );
    let end = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while core.backing_loading() && std::time::Instant::now() < end {
        core.tick();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(!core.backing_loading());
    let cache = kontra_session::BackingCache::new(&core.backing_cache_dir());
    assert_eq!(cache.usage().0, 1);

    // Opened again: nothing to prepare, it plays at once.
    core.stop_live();
    core.open(&demo()).unwrap();
    core.tick();
    assert!(!core.backing_loading());
    let _ = core.play();
    assert!(core.has_run());
    // Played from the prepared file: no second one at another rate (review #162).
    assert_eq!(cache.usage().0, 1);
    core.stop_live();
    let _ = std::fs::remove_dir_all(dir);
}

/// LH-BIB-06: the beats of a backing track become the piece's tempo map and offset; bar 1
/// can be moved by a beat.
#[test]
fn backing_beats_set_the_tempo() {
    let dir = temp_dir("beats");
    let mut core = Core::new(&dir, QaOptions::default());
    core.open(&demo()).unwrap();
    // Clicks at 105 BPM from 1.0 s: a band a little faster than the score's 100 BPM.
    let rate = 44_100u32;
    let mut audio = vec![0.0f32; rate as usize * 22];
    let mut noise = 1u32;
    for beat in 0..36 {
        let start = ((1.0 + f64::from(beat) * 60.0 / 105.0) * f64::from(rate)) as usize;
        for i in 0..1500 {
            noise = noise.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let n = (noise >> 9) as f32 / (1u32 << 23) as f32 - 0.5;
            if let Some(s) = audio.get_mut(start + i) {
                *s += n * (-(i as f32) / 600.0).exp();
            }
        }
    }
    let wav = dir.join("band.wav");
    kontra_session::write_mono_wav(&wav, &audio, rate).unwrap();
    core.set_backing(Some(wav), 0.0);
    core.backing_analyse().unwrap();
    let info: serde_json::Value = serde_json::from_str(&core.backing_json()).unwrap();
    assert!(
        (info["bpm"].as_f64().unwrap() - 105.0).abs() < 2.0,
        "{info}"
    );
    assert!(
        (info["firstS"].as_f64().unwrap() - 1.0).abs() < 0.02,
        "{info}"
    );
    let bpm = core.arrangement().unwrap().tempo.bpm_at(0);
    assert!((bpm - 105.0).abs() < 2.0, "tempo follows the band: {bpm}");
    core.backing_adjust(1, 0.0, None).unwrap();
    let info: serde_json::Value = serde_json::from_str(&core.backing_json()).unwrap();
    assert!(
        (info["firstS"].as_f64().unwrap() - (1.0 + 60.0 / 105.0)).abs() < 0.02,
        "{info}"
    );
    core.backing_clear().unwrap();
    assert_eq!(core.arrangement().unwrap().tempo.bpm_at(0), 100.0);
    let _ = std::fs::remove_dir_all(dir);
}

/// QA 27.10. (J3): hi-hats alone cannot tell beat from offbeat; the sheet must say so instead
/// of "8 of 8 bars follow the recording".
#[test]
fn backing_without_low_beats_is_unsure() {
    let dir = temp_dir("beats-unsure");
    let mut core = Core::new(&dir, QaOptions::default());
    core.update_settings(|s| s.language = Some("en".into()));
    core.open(&demo()).unwrap();
    let rate = 44_100u32;
    let mut audio = vec![0.0f32; rate as usize * 22];
    let mut noise = 1u32;
    for beat in 0..36 {
        let start = ((1.0 + f64::from(beat) * 60.0 / 105.0) * f64::from(rate)) as usize;
        let mut last = 0.0f32;
        for i in 0..1500 {
            noise = noise.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let n = (noise >> 9) as f32 / (1u32 << 23) as f32 - 0.5;
            if let Some(s) = audio.get_mut(start + i) {
                *s += (n - last) * (-(i as f32) / 600.0).exp();
            }
            last = n;
        }
    }
    let wav = dir.join("hats.wav");
    kontra_session::write_mono_wav(&wav, &audio, rate).unwrap();
    core.set_backing(Some(wav), 0.0);
    core.backing_analyse().unwrap();
    let info: serde_json::Value = serde_json::from_str(&core.backing_json()).unwrap();
    assert_eq!(info["sure"], false, "{info}");
    assert!(!info["text"].as_str().unwrap().contains("follow"), "{info}");

    // Half a beat by hand (PR-Agent #155): the warning goes, bar 1 moves half a beat; a
    // second click takes it back instead of moving a whole beat.
    let first = info["firstS"].as_f64().unwrap();
    let half_s = 30.0 / info["bpm"].as_f64().unwrap();
    core.backing_half_beat().unwrap();
    let moved: serde_json::Value = serde_json::from_str(&core.backing_json()).unwrap();
    assert_eq!(moved["sure"], true, "{moved}");
    assert_eq!(moved["halfBeat"], true, "{moved}");
    let at = moved["firstS"].as_f64().unwrap();
    assert!((at - first - half_s).abs() < 0.02, "{moved}");
    core.backing_half_beat().unwrap();
    let back: serde_json::Value = serde_json::from_str(&core.backing_json()).unwrap();
    assert_eq!(back["firstS"], info["firstS"], "{back}");
    assert_eq!(back["sure"], false, "{back}");
    let _ = std::fs::remove_dir_all(dir);
}

/// LH-WIE-02: transposing moves every note, is kept for the piece and keeps its progress.
#[test]
fn transpose_moves_notes_and_is_kept() {
    let dir = temp_dir("transpose");
    let mut core = Core::new(&dir, QaOptions::default());
    core.open(&demo()).unwrap();
    let pitches = |core: &Core| -> Vec<u8> {
        core.arrangement()
            .unwrap()
            .notes
            .iter()
            .map(|n| n.pitch.0)
            .collect()
    };
    let before = pitches(&core);
    let song = core.library.entry(&demo()).unwrap().song_id.clone();

    core.set_transpose(2).unwrap();
    assert_eq!(core.transpose(), 2);
    let after = pitches(&core);
    assert!(
        before.iter().zip(&after).all(|(b, a)| {
            let d = i32::from(*a) - i32::from(*b);
            d == 2 || d == 2 - 12
        }),
        "{before:?} → {after:?}"
    );
    assert!(core
        .arrangement()
        .unwrap()
        .notes
        .iter()
        .all(|n| n.fret.is_some()));
    assert_eq!(core.library.entry(&demo()).unwrap().song_id, song);

    // A new start opens it transposed; 0 goes back.
    let mut again = Core::new(&dir, QaOptions::default());
    again.open(&demo()).unwrap();
    assert_eq!(pitches(&again), after);
    again.set_transpose(0).unwrap();
    assert_eq!(pitches(&again), before);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn loop_keys_set_edges_and_rewind_to_loop_start() {
    let dir = temp_dir("loopkeys");
    let mut core = Core::new(&dir, QaOptions::default());
    core.open(&demo()).unwrap();
    let bars: serde_json::Value = serde_json::from_str(&core.bars_json()).unwrap();
    let start = |i: usize| bars[i]["time"].as_f64().unwrap();
    assert_eq!(core.rewind(), 0.0);

    // `[` in bar 3, `]` in bar 5 → bars 3–5; rewind goes to bar 3.
    core.set_loop_edge(LoopEdge::Start, start(2) + 0.1);
    core.set_loop_edge(LoopEdge::End, start(4) + 0.1);
    assert_eq!(core.loop_seconds(), (start(2), start(5)));
    assert_eq!(core.rewind(), start(2));

    // An end before the start starts over in that bar.
    core.set_loop_edge(LoopEdge::End, start(1));
    assert_eq!(core.loop_seconds(), (start(1), start(2)));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn calibration_measures_recorded_latency() {
    let dir = temp_dir("calibrate");
    std::fs::create_dir_all(&dir).unwrap();
    let take = dir.join("clicks.wav");
    // Eight quarter notes at 100 BPM (0.6 s), plucked 45 ms after each click.
    let mut a = Arrangement::new("clicks", kontra_model::Instrument::default_bass());
    a.tempo = kontra_model::TempoMap::constant(100.0);
    for i in 0..8 {
        let q = kontra_model::TICKS_PER_QUARTER;
        a.notes.push(kontra_model::Note::new(
            i * q,
            q / 2,
            kontra_model::Pitch(33),
        ));
    }
    a.normalize();
    let mistakes = kontra_session::Mistakes {
        offset_ms: 45.0,
        ..Default::default()
    };
    let samples = kontra_session::render_performance(&a, 1.0, CALIBRATION_LEAD_S, mistakes, 48_000);
    kontra_session::write_mono_wav(&take, &samples, 48_000).unwrap();
    let mut core = Core::new(
        &dir,
        QaOptions {
            input_file: Some(take),
            laptop_jack: false,
        },
    );
    core.start_calibration().unwrap();
    let end = std::time::Instant::now() + std::time::Duration::from_secs(12);
    let mut result = None;
    while std::time::Instant::now() < end && result.is_none() {
        result = core.tick().into_iter().find_map(|e| match e {
            UiEvent::Calibrated { latency_ms, .. } => Some(latency_ms),
            _ => None,
        });
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let latency = result.flatten().expect("measured");
    assert!((latency - 45.0).abs() <= 3.0, "{latency}");
    assert_eq!(core.settings().latency_ms, latency);
    // LH-EIN-16: a silent count-in gives the lowest gate.
    assert_eq!(core.settings().onset_gate_db, Some(-62.0));
    std::fs::remove_dir_all(dir).unwrap();
}

/// LH-WIE-02 while playing: a whole step up mid-run, the run goes on and the notes after it
/// are judged against the new pitches (the take changes key at the same moment).
#[test]
fn transposing_mid_run_keeps_playing() {
    let dir = temp_dir("transpose-live");
    let take = dir.join("take.wav");
    std::fs::create_dir_all(&dir).unwrap();
    let mut core = Core::new(
        &dir,
        QaOptions {
            input_file: Some(take.clone()),
            laptop_jack: false,
        },
    );
    core.open(&demo()).unwrap();
    let mut played = core.arrangement().unwrap().clone();
    // Halfway between the fourth and fifth note.
    let times: Vec<f64> = played
        .notes
        .iter()
        .map(|n| played.tempo.seconds_at(n.start_ticks))
        .collect();
    let switch_s = (times[3] + times[4]) / 2.0;
    for (n, t) in played.notes.iter_mut().zip(&times) {
        if *t > switch_s {
            n.pitch = kontra_model::Pitch(n.pitch.0 + 2);
        }
    }
    let samples = kontra_session::render_performance(
        &played,
        1.0,
        2.4,
        kontra_session::Mistakes::default(),
        48_000,
    );
    kontra_session::write_mono_wav(&take, &samples[..(48_000.0 * 6.0) as usize], 48_000).unwrap();
    core.play();
    let (mut hits, mut wrong, mut switched) = (0, 0, false);
    let end = std::time::Instant::now() + std::time::Duration::from_secs(7);
    while std::time::Instant::now() < end {
        if !switched && core.position_s() >= switch_s {
            core.set_transpose(2).unwrap();
            assert!(core.is_playing(), "the run goes on");
            switched = true;
        }
        for e in core.tick() {
            match e {
                UiEvent::Judged { state: "hit", .. } => hits += 1,
                UiEvent::Judged { state: "wrong", .. } => wrong += 1,
                _ => {}
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(switched);
    assert!(hits >= 10 && wrong == 0, "hits {hits}, wrong {wrong}");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn plays_a_recording_and_judges_notes() {
    let dir = temp_dir("play");
    let take = dir.join("take.wav");
    std::fs::create_dir_all(&dir).unwrap();
    let mut core = Core::new(
        &dir,
        QaOptions {
            input_file: Some(take.clone()),
            laptop_jack: false,
        },
    );
    core.open(&demo()).unwrap();
    // One count-in bar at 100 BPM = 2.4 s lead-in; render 4 bars at double speed for a short test.
    let arrangement = core.arrangement().unwrap().clone();
    let samples = kontra_session::render_performance(
        &arrangement,
        1.0,
        2.4,
        kontra_session::Mistakes::default(),
        48_000,
    );
    kontra_session::write_mono_wav(&take, &samples[..(48_000.0 * 6.0) as usize], 48_000).unwrap();
    let events = core.play();
    assert!(
        !events.iter().any(|e| matches!(e, UiEvent::Error(_))),
        "{events:?}"
    );
    let mut hits = 0;
    let end = std::time::Instant::now() + std::time::Duration::from_secs(7);
    while std::time::Instant::now() < end {
        for e in core.tick() {
            if let UiEvent::Judged { state: "hit", .. } = e {
                hits += 1;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    // 3.6 s of playing at 0.3 s per note: about 12 hits.
    assert!(hits >= 10, "hits {hits}");

    // The evaluation lists every judged note with its timing (LH-BEW-01).
    let session = core.live.as_ref().unwrap().practice().session();
    let summary: serde_json::Value = serde_json::from_str(&core.summary_json(
        &session.summary(),
        session.results(),
        session.technique_results(),
    ))
    .unwrap();
    let notes = summary["notes"].as_array().unwrap();
    assert!(notes.len() >= hits, "{}", notes.len());
    assert!(notes[0]["offsetMs"].is_f64());
    let bar = core.tr("summary.note.bar", &[("bar", "1".into())]);
    assert!(notes[0]["text"].as_str().unwrap().starts_with(&bar));
    std::fs::remove_dir_all(dir).unwrap();
}
