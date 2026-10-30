use super::*;
use kontra_drivers::{write_wav, AudioBuffer};

/// LH-EIN-15: 0.8 s without data → one Stalled after 250 ms, then what was lost where
/// the new samples really start.
#[test]
fn stall_is_told_with_what_was_lost() {
    let mut watch = Watch::new(0.0);
    assert!(watch.data(0.01, None, 0.0).is_none(), "no stall yet");
    assert!(watch.idle(0.2, Some(1.0)).is_none(), "under 250 ms");
    assert!(matches!(watch.idle(1.3, Some(1.0)), Some(Event::Stalled { at_s }) if at_s == 1.0));
    assert!(watch.idle(1.5, Some(1.0)).is_none(), "told once");

    // Data again at 1.8 s; the device skipped 0.79 s meanwhile.
    let lost = watch.data(1.8, Some(1.0), 0.79).expect("resumed");
    assert_eq!(lost.from_s, 1.0);
    assert!((lost.lost_s - 0.79).abs() < 1e-9, "{lost:?}");
    assert!(watch.data(1.81, Some(1.8), 0.79).is_none());
}

/// A device slow to start is no stall (PR-Agent #186); one that delivers late but all of
/// it (a burst) lost nothing.
#[test]
fn late_start_or_burst_loses_nothing() {
    let mut watch = Watch::new(0.0);
    assert!(
        watch.idle(0.3, None).is_none(),
        "not armed before the first data"
    );

    watch.data(0.31, None, 0.0);
    watch.idle(1.4, Some(1.0));
    let lost = watch.data(1.5, Some(1.0), 0.0).expect("resumed");
    assert_eq!(lost.lost_s, 0.0);
}

/// Review #186, round 3: the device captured on, but delivers the 0.8 s late and in pieces
/// of 0.1 s. Nothing was lost, though the first piece alone shows a small backlog.
#[test]
fn late_pieces_lose_nothing() {
    let rate = 1000.0;
    let (mut capture, mut drain) = input_ring(1000);
    let mut out = [0.0f32; 100];
    let mut watch = Watch::new(0.0);
    capture.push(&[0.0; 100], 0.0);
    drain.read(&mut out);
    watch.data(0.1, Some(0.1), drain.skipped_s(rate));
    assert!(watch.idle(0.5, Some(0.1)).is_some(), "stalled");

    // Captured on from 0.1 s, delivered from 0.9 s in pieces.
    let mut lost = None;
    for piece in 0..8 {
        let at_s = f64::from(piece) * 0.1;
        capture.push(&[0.0; 100], 0.1 + at_s);
        drain.read(&mut out);
        let skipped_s = drain.skipped_s(rate);
        lost = lost.or(watch.data(0.9 + at_s, Some(0.1 + at_s), skipped_s));
    }
    let lost = lost.expect("resumed");
    assert!(lost.lost_s.abs() < 1e-9, "{lost:?}");
}

/// PR-Agent #186: a note struck 60 ms before the device stalls for 0.8 s is told after the
/// stall; it must keep its own time, not move by the lost 0.8 s.
#[test]
fn note_before_a_stall_keeps_its_time() {
    use kontra_dsp::synth::{bass_tone, ToneShape};

    let rate = 48_000.0;
    let mut analysis = Analysis::new(rate, &Instrument::default_bass(), InputSource::Line);
    let mut watch = Watch::new(0.0);
    let (tx, rx) = std::sync::mpsc::channel();
    let tone = bass_tone(55.0, rate, 2.0, ToneShape::Rich, 1);
    // A block captured at `at_s`, the device having skipped `skipped_s` so far.
    let take = |analysis: &mut Analysis, watch: &mut Watch, at_s: f64, skipped_s, block| {
        let arrival = Arrival {
            now_s: at_s + <[f32]>::len(block) as f64 / rate,
            start_s: Some(0.0),
            skipped_s,
        };
        deliver(analysis, watch, arrival, block, &tx, |a, b| {
            a.process(b, false, &tx)
        });
    };

    // Silence, then the string from 1.0 s; the device stops at 1.06 s.
    let mut signal = vec![0.0f32; rate as usize];
    signal.extend_from_slice(&tone[..(0.06 * rate) as usize]);
    for (i, block) in signal.chunks(BLOCK).enumerate() {
        take(
            &mut analysis,
            &mut watch,
            (i * BLOCK) as f64 / rate,
            0.0,
            block,
        );
    }
    assert!(watch.idle(1.4, analysis.heard_s()).is_some(), "stalled");

    // Back at 1.86 s, 0.8 s skipped; the string still rings for 0.5 s, then a new note (D)
    // at 2.36 s.
    let mut after = tone[(0.06 * rate) as usize..(0.56 * rate) as usize].to_vec();
    after.extend_from_slice(&bass_tone(73.4, rate, 1.0, ToneShape::Rich, 2));
    for (i, block) in after.chunks(BLOCK).enumerate() {
        take(
            &mut analysis,
            &mut watch,
            1.86 + (i * BLOCK) as f64 / rate,
            0.8,
            block,
        );
    }

    let onsets: Vec<f64> = rx
        .try_iter()
        .filter_map(|e| match e {
            Event::Note(n) => Some(n.onset_s),
            _ => None,
        })
        .collect();
    assert!(
        onsets.iter().any(|o| (o - 1.0).abs() < 0.03),
        "the note at 1.0 s: {onsets:?}"
    );
    // Review #186, round 4: the string ringing on after the silence is no new note either.
    assert!(
        !onsets.iter().any(|o| (1.1..2.3).contains(o)),
        "nothing inside the stall or at the resume (1.86 s): {onsets:?}"
    );
    assert!(
        onsets.iter().any(|o| (o - 2.36).abs() < 0.03),
        "the note after the stall at 2.36 s: {onsets:?}"
    );
}

/// A stall that loses nothing (the backlog arrives late, in one piece) bridges no silence:
/// a note struck right where the data stopped keeps its place (review #186, round 6).
#[test]
fn note_at_a_lossless_stall_is_kept() {
    use kontra_dsp::synth::{bass_tone, ToneShape};

    let rate = 48_000.0;
    let mut analysis = Analysis::new(rate, &Instrument::default_bass(), InputSource::Line);
    let mut watch = Watch::new(0.0);
    let (tx, rx) = std::sync::mpsc::channel();
    let take = |analysis: &mut Analysis, watch: &mut Watch, now_s: f64, block| {
        let arrival = Arrival {
            now_s,
            start_s: Some(0.0),
            skipped_s: 0.0,
        };
        deliver(analysis, watch, arrival, block, &tx, |a, b| {
            a.process(b, false, &tx)
        });
    };

    // Silence up to 1.0 s, then nothing for 0.4 s.
    let silence = vec![0.0f32; rate as usize];
    for (i, block) in silence.chunks(BLOCK).enumerate() {
        take(
            &mut analysis,
            &mut watch,
            ((i + 1) * BLOCK) as f64 / rate,
            block,
        );
    }
    assert!(watch.idle(1.4, analysis.heard_s()).is_some(), "stalled");

    // The backlog from 1.0 s arrives at once, nothing skipped: a note struck at 1.02 s.
    let mut late = vec![0.0f32; (0.02 * rate) as usize];
    late.extend_from_slice(&bass_tone(55.0, rate, 1.0, ToneShape::Rich, 1));
    for (i, block) in late.chunks(BLOCK).enumerate() {
        take(
            &mut analysis,
            &mut watch,
            1.4 + (i * BLOCK) as f64 / rate,
            block,
        );
    }

    let onsets: Vec<f64> = rx
        .try_iter()
        .filter_map(|e| match e {
            Event::Note(n) => Some(n.onset_s),
            _ => None,
        })
        .collect();
    assert!(
        onsets.iter().any(|o| (o - 1.02).abs() < 0.03),
        "the note at 1.02 s: {onsets:?}"
    );
}

/// Onsets told for a stall that loses nothing, the device stamps after it off by `jitter_s`
/// (alternating): silence up to 1.0 s, nothing for 0.4 s, then the backlog with a note
/// struck at 1.02 s, all through the input ring.
fn jittered_stall_onsets(jitter_s: f64) -> Vec<f64> {
    use kontra_dsp::synth::{bass_tone, ToneShape};

    let rate = 48_000.0;
    let mut analysis = Analysis::new(rate, &Instrument::default_bass(), InputSource::Line);
    let mut watch = Watch::new(0.0);
    let (mut capture, mut drain) = input_ring(rate as usize * 2);
    let mut out = [0.0f32; BLOCK];
    let (tx, rx) = std::sync::mpsc::channel();
    let mut take = |analysis: &mut Analysis, watch: &mut Watch, now_s, at_s, block: &[f32]| {
        capture.push(block, at_s);
        let n = drain.read(&mut out);
        let arrival = Arrival {
            now_s,
            start_s: Some(0.0),
            skipped_s: drain.skipped_s(rate),
        };
        deliver(analysis, watch, arrival, &out[..n], &tx, |a, b| {
            a.process(b, false, &tx)
        });
    };

    let silence = vec![0.0f32; rate as usize];
    for (i, block) in silence.chunks(BLOCK).enumerate() {
        let at_s = (i * BLOCK) as f64 / rate;
        take(
            &mut analysis,
            &mut watch,
            at_s + BLOCK as f64 / rate,
            at_s,
            block,
        );
    }
    assert!(watch.idle(1.4, analysis.heard_s()).is_some(), "stalled");

    let mut late = vec![0.0f32; (0.02 * rate) as usize];
    late.extend_from_slice(&bass_tone(55.0, rate, 1.0, ToneShape::Rich, 1));
    for (i, block) in late.chunks(BLOCK).enumerate() {
        let sign = if i % 2 == 0 { 1.0 } else { -1.0 };
        let at_s = 1.0 + (i * BLOCK) as f64 / rate + sign * jitter_s;
        take(
            &mut analysis,
            &mut watch,
            1.4 + (i * BLOCK) as f64 / rate,
            at_s,
            block,
        );
    }

    rx.try_iter()
        .filter_map(|e| match e {
            Event::Note(n) => Some(n.onset_s),
            _ => None,
        })
        .collect()
}

/// Review #186, round 7: device stamps jitter (cpal: "avail_delay() varies between cycles"),
/// so a stall that lost nothing shows a skip of a few µs to 1 ms. That is no loss: the note
/// struck at 1.02 s stays.
#[test]
fn jittered_stall_keeps_the_note() {
    for jitter_s in [0.000_05, 0.000_2, 0.001] {
        let onsets = jittered_stall_onsets(jitter_s);
        assert!(
            onsets.iter().any(|o| (o - 1.02).abs() < 0.03),
            "jitter {jitter_s} s, the note at 1.02 s: {onsets:?}"
        );
    }
}

/// Onsets told around a stall that loses 0.8 s: silence (or, `ringing`, a note struck at
/// 1.0 s) up to 1.06 s, then back at 1.86 s; a new note struck `lead_s` after that.
fn resume_onsets(ringing: Ringing, lead_s: f64) -> Vec<f64> {
    use kontra_dsp::synth::{bass_tone, ToneShape};

    let rate = 48_000.0;
    let mut analysis = Analysis::new(rate, &Instrument::default_bass(), InputSource::Line);
    let mut watch = Watch::new(0.0);
    let (tx, rx) = std::sync::mpsc::channel();
    let tone = bass_tone(55.0, rate, 2.0, ToneShape::Rich, 1);
    let take = |analysis: &mut Analysis, watch: &mut Watch, at_s: f64, skipped_s, block| {
        let arrival = Arrival {
            now_s: at_s + <[f32]>::len(block) as f64 / rate,
            start_s: Some(0.0),
            skipped_s,
        };
        deliver(analysis, watch, arrival, block, &tx, |a, b| {
            a.process(b, false, &tx)
        });
    };
    let sound = |from_s: f64, to_s: f64| -> Vec<f32> {
        let range = (from_s * rate) as usize..(to_s * rate) as usize;
        match ringing {
            Ringing::Yes => tone[range].to_vec(),
            Ringing::No => vec![0.0; range.len()],
        }
    };

    let mut signal = vec![0.0f32; rate as usize];
    signal.extend_from_slice(&sound(0.0, 0.06));
    for (i, block) in signal.chunks(BLOCK).enumerate() {
        let at_s = (i * BLOCK) as f64 / rate;
        take(&mut analysis, &mut watch, at_s, 0.0, block);
    }
    assert!(watch.idle(1.4, analysis.heard_s()).is_some(), "stalled");

    // The new note struck harder than the one ringing (+6 dB), so it stands out of it.
    let mut after = sound(0.06, 0.06 + lead_s);
    let struck = bass_tone(73.4, rate, 1.0, ToneShape::Rich, 2);
    after.extend(struck.iter().map(|s| s * 2.0));
    for (i, block) in after.chunks(BLOCK).enumerate() {
        let at_s = 1.86 + (i * BLOCK) as f64 / rate;
        take(&mut analysis, &mut watch, at_s, 0.8, block);
    }

    rx.try_iter()
        .filter_map(|e| match e {
            Event::Note(n) => Some(n.onset_s),
            _ => None,
        })
        .collect()
}

/// Whether a note rings into the stall (`resume_onsets`).
#[derive(Debug, Clone, Copy)]
enum Ringing {
    Yes,
    No,
}

/// Review #186, round 7: a note struck 30 ms after the input resumes from a real loss is
/// played, not the jump back to the signal; it must be told.
#[test]
fn note_after_a_resume_is_kept() {
    for ringing in [Ringing::No, Ringing::Yes] {
        let onsets = resume_onsets(ringing, 0.03);
        assert!(
            onsets.iter().any(|o| (o - 1.89).abs() < 0.02),
            "{ringing:?}: the note at 1.89 s: {onsets:?}"
        );
        // The jump back to the ringing string at 1.86 s stays no note (round 4).
        assert!(
            !onsets.iter().any(|o| (1.1..1.875).contains(o)),
            "{ringing:?}: nothing inside the stall or at the resume: {onsets:?}"
        );
    }
}

/// LH-EIN-12: a MIDI file plays like a keyboard: note-ons at their times on the engine
/// clock, exact pitch, note-offs dropped.
#[test]
fn reference_lines_up_by_start_times() {
    // Microphone 0.1 s after the reference: drop 4800 reference frames at 48 kHz.
    assert_eq!(align_frames(0.1, 48_000.0), 4800);
    assert_eq!(align_frames(-0.02, 48_000.0), -960);
}

/// LH-EIN-11: a played signal in the reference is removed from the microphone once the
/// stage has learnt the way; what the reference does not hold stays.
#[test]
fn echo_stage_removes_what_played() {
    let (mut tx, rx) = rtrb::RingBuffer::<f32>::new(48_000 * 10);
    let reference = Reference {
        samples: rx,
        armed: Arc::new(AtomicBool::new(false)),
        start_s: Arc::new(AtomicU64::new(0.0f64.to_bits())),
    };
    let mut stage = EchoStage::new(reference, 48_000.0);
    let mut noise = kontra_dsp::synth::Noise::new(9);
    let band: Vec<f32> = (0..48_000 * 10).map(|_| noise.sample() * 0.3).collect();
    for &s in &band {
        tx.push(s).unwrap();
    }
    // The microphone hears the band 2 ms late at 0.5.
    let mic: Vec<f32> = (0..band.len())
        .map(|i| i.checked_sub(96).map_or(0.0, |j| band[j] * 0.5))
        .collect();
    let mut cleaned = Vec::new();
    stage.push(&mic, 0.0, |b| cleaned.extend_from_slice(b));
    let energy = |s: &[f32]| s.iter().map(|x| x * x).sum::<f32>();
    let late = 48_000 * 7..cleaned.len();
    let drop = 10.0 * (energy(&cleaned[late.clone()]) / energy(&mic[late])).log10();
    assert!(drop < -30.0, "{drop} dB");
}

#[test]
fn midi_file_plays_its_notes_in_real_time() {
    // 480 ticks per quarter at 120 BPM: E2 at 0 s, G2 at 0.25 s (240 ticks = varlen 0x81 0x70).
    let track: Vec<u8> = vec![
        0x00, 0x90, 40, 100, 0x81, 0x70, 0x80, 40, 0, 0x00, 0x90, 43, 64, 0x81, 0x70, 0x80, 43, 0,
        0x00, 0xff, 0x2f, 0x00,
    ];
    let mut file = b"MThd".to_vec();
    file.extend(6u32.to_be_bytes());
    file.extend([0, 0, 0, 1, 0x01, 0xe0]);
    file.extend(b"MTrk");
    file.extend((track.len() as u32).to_be_bytes());
    file.extend(track);
    let path = std::env::temp_dir().join(format!("kontra-midi-in-{}.mid", std::process::id()));
    std::fs::write(&path, file).unwrap();

    let clock = Clock::new();
    let (tx, rx) = std::sync::mpsc::channel();
    let kind = InputKind::MidiFile { path: path.clone() };
    let input = Input::start(
        &kind,
        &Instrument::default_bass(),
        InputSource::Midi,
        &clock,
        tx,
        None,
    )
    .unwrap();
    let mut notes = Vec::new();
    for event in rx.iter() {
        match event {
            Event::Note(n) => notes.push(n),
            Event::Ended => break,
            _ => {}
        }
    }
    let started = input.started_s().unwrap();
    let _ = std::fs::remove_file(&path);
    assert_eq!(notes.len(), 2, "{notes:?}");
    assert_eq!((notes[0].midi, notes[1].midi), (40.0, 43.0));
    assert!((notes[0].onset_s - started).abs() < 1e-9);
    assert!((notes[1].onset_s - started - 0.25).abs() < 1e-9);
    assert!(notes[1].level_db < notes[0].level_db, "softer velocity");
    assert!(notes.iter().all(|n| n.source == InputSource::Midi));
}

/// Feeds a 0.1 s real-time file at `file_rate` into a tap at `tap_rate`; returns what
/// reached the monitor ring.
fn monitored(file_rate: u32, tap_rate: u32, name: &str) -> Vec<f32> {
    let path = std::env::temp_dir().join(format!("kontra-{name}-{}.wav", std::process::id()));
    let samples: Vec<f32> = (0..file_rate / 10).map(|i| (i % 7) as f32 / 10.0).collect();
    write_wav(
        &path,
        &AudioBuffer {
            sample_rate: file_rate,
            channels: vec![samples],
        },
    )
    .unwrap();
    let (ring, mut rx) = rtrb::RingBuffer::<f32>::new(10_000);
    let (tx, events) = std::sync::mpsc::channel();
    let input = start_file(
        &path,
        Pace::RealTime,
        false,
        &Instrument::default_bass(),
        InputSource::Line,
        &Clock::new(),
        tx,
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicBool::new(false)),
        Some(Tap {
            ring,
            rate: tap_rate,
        }),
    )
    .unwrap();
    while !matches!(events.recv(), Ok(Event::Ended) | Err(_)) {}
    drop(input);
    std::fs::remove_file(path).unwrap();
    std::iter::from_fn(|| rx.pop().ok()).collect()
}

#[test]
fn real_time_file_reaches_the_monitor() {
    let heard = monitored(48_000, 48_000, "monitor-same");
    assert_eq!(heard.len(), 4800);
    assert!((heard[3] - 0.3).abs() < 1e-6);
}

#[test]
fn other_rate_is_not_monitored() {
    assert!(monitored(44_100, 48_000, "monitor-other").is_empty());
}

/// A take keeps the raw input from where recording began, with its place in the input.
#[test]
fn recording_keeps_the_raw_input() {
    let samples: Vec<f32> = (0..24_000).map(|i| (i % 100) as f32 / 100.0).collect();
    let path = std::env::temp_dir().join(format!("kontra-rec-{}.wav", std::process::id()));
    let buffer = AudioBuffer {
        sample_rate: 48_000,
        channels: vec![samples.clone()],
    };
    write_wav(&path, &buffer).unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let kind = InputKind::File {
        path: path.clone(),
        pace: Pace::RealTime,
        laptop_jack: false,
    };
    let clock = Clock::new();
    let input = Input::start(
        &kind,
        &Instrument::default_bass(),
        InputSource::Line,
        &clock,
        tx,
        None,
    )
    .unwrap();
    input.record();
    while !matches!(rx.recv(), Ok(Event::Ended) | Err(_)) {}
    let take = input.stop_recording().unwrap();
    let _ = std::fs::remove_file(&path);
    let first = take.first_sample as usize;
    assert!(first < 4 * BLOCK, "began at once: {first}");
    assert_eq!(take.samples, samples[first..]);
    assert_eq!(input.stop_recording(), None, "handed over once");
}
