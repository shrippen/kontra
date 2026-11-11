use super::*;

fn mixer() -> Mixer {
    Mixer {
        voices: [IDLE_VOICE; VOICES],
        gains: Gains {
            click: 1.0,
            tone: 1.0,
            backing: 1.0,
            drums: 1.0,
            bass: 0.0,
            monitor: 1.0,
            shaping: Shaping::FLAT,
            plugins: None,
        },
        backing: None,
        next: None,
        monitor: None,
        shaper: Shaper::new(48_000.0),
        chains: None,
        shifter: Box::new(PitchShifter::new()),
        shift_next: None,
        reference: None,
        noise: Noise::new(1),
        sum_limit: OutputLimiter::new(48_000.0),
    }
}

#[test]
fn click_starts_on_time_and_ends() {
    let mut m = mixer();
    m.apply(Command::Schedule {
        at_s: 0.01,
        sound: Sound::Click(Accent::Downbeat),
    });
    let sr = 48_000.0;
    let before: f32 = (0..400).map(|i| m.render(i as f64 / sr, sr).0.abs()).sum();
    assert_eq!(before, 0.0);
    let during: f32 = (480..960)
        .map(|i| m.render(i as f64 / sr, sr).0.abs())
        .sum();
    assert!(during > 1.0);
    m.render(1.0, sr);
    assert!(m.voices.iter().all(|v| !v.active));
}

/// LH-WIE-06: many sounds at once went over full scale and were cut off hard (a crackle).
/// The sum stays under the limiter's ceiling instead.
#[test]
fn the_sum_stays_under_the_ceiling() {
    let mut m = mixer();
    for _ in 0..6 {
        m.apply(Command::Schedule {
            at_s: 0.0,
            sound: Sound::Click(Accent::Downbeat),
        });
    }
    let sr = 48_000.0;
    let peak = (0..2400)
        .map(|i| m.render(i as f64 / sr, sr).0.abs())
        .fold(0.0, f32::max);
    assert!(peak > 0.5, "the clicks sound: {peak}");
    assert!(peak <= kontra_dsp::LIMIT_CEILING + 1e-6, "peak {peak}");
}

/// Review #244: the echo reference is what the speakers play, so it follows the limiter.
#[test]
fn the_echo_reference_follows_the_limiter() {
    let (tx, mut rx) = rtrb::RingBuffer::<f32>::new(4800);
    let mut m = mixer();
    m.reference = Some(ReferenceTap {
        samples: tx,
        armed: Arc::new(AtomicBool::new(true)),
        start_s: Arc::new(AtomicU64::new(f64::NAN.to_bits())),
    });
    for _ in 0..6 {
        m.apply(Command::Schedule {
            at_s: 0.0,
            sound: Sound::Click(Accent::Downbeat),
        });
    }
    let sr = 48_000.0;
    for i in 0..2400 {
        let (left, _) = m.render(i as f64 / sr, sr);
        // No monitored bass here: the reference is the output (up to rounding).
        let reference = rx.pop().unwrap();
        assert!(
            (reference - left).abs() < 1e-6,
            "{reference} against {left}"
        );
    }
}

/// Review #244: an infinite sample (a broken float file) became NaN behind the limiter,
/// where the hard clip used to give full scale. It is silence now, and the next frame
/// plays again.
#[test]
fn a_sample_that_is_no_number_is_silence() {
    let mut limit = OutputLimiter::new(48_000.0);
    assert_eq!(limit.process((f32::INFINITY, 0.25), 48_000.0), (0.0, 0.25));
    assert_eq!(
        limit.process((f32::NAN, f32::NEG_INFINITY), 48_000.0),
        (0.0, 0.0)
    );
    assert_eq!(limit.gain(), 1.0);
    assert_eq!(limit.process((0.5, 0.5), 48_000.0), (0.5, 0.5));
}

// LH-WIE-04: the shuffle's off-beat click is softer than the beat, so the pulse stays clear.
#[test]
fn offbeat_click_is_softer() {
    let sr = 48_000.0;
    let energy = |accent| {
        let mut m = mixer();
        m.apply(Command::Schedule {
            at_s: 0.0,
            sound: Sound::Click(accent),
        });
        (0..960)
            .map(|i| m.render(i as f64 / sr, sr).0.abs())
            .sum::<f32>()
    };
    let (beat, offbeat) = (energy(Accent::Beat), energy(Accent::Offbeat));
    assert!(offbeat > 0.1 * beat, "audible: {offbeat} vs {beat}");
    assert!(offbeat < 0.75 * beat, "softer: {offbeat} vs {beat}");
}

#[test]
fn drums_sound_and_end() {
    let sr = 48_000.0;
    for kind in [DrumKind::Kick, DrumKind::Snare, DrumKind::HiHat] {
        let mut m = mixer();
        m.apply(Command::Schedule {
            at_s: 0.0,
            sound: Sound::drum(kind),
        });
        let energy: f32 = (0..2400).map(|i| m.render(i as f64 / sr, sr).0.abs()).sum();
        assert!(energy > 1.0, "{kind:?}: {energy}");
        m.render(1.0, sr);
        assert!(m.voices.iter().all(|v| !v.active), "{kind:?}");
    }
    // The drum bus has its own volume.
    let mut m = mixer();
    m.gains.drums = 0.0;
    m.apply(Command::Schedule {
        at_s: 0.0,
        sound: Sound::drum(DrumKind::Kick),
    });
    assert_eq!(m.render(0.001, sr), (0.0, 0.0));
}

#[test]
fn clear_drops_future_sounds() {
    let mut m = mixer();
    m.apply(Command::Schedule {
        at_s: 1.0,
        sound: Sound::Click(Accent::Beat),
    });
    m.apply(Command::Schedule {
        at_s: 2.0,
        sound: Sound::Click(Accent::Beat),
    });
    m.apply(Command::ClearFrom { from_s: 1.5 });
    assert_eq!(m.voices.iter().filter(|v| v.active).count(), 1);
}

#[test]
fn backing_starts_at_frame_and_stops() {
    let mut m = mixer();
    let audio = BackingAudio::stereo(
        (0..2000)
            .flat_map(|i| [i as f32 / 4000.0, -(i as f32) / 4000.0])
            .collect(),
    );
    m.apply(Command::Backing {
        audio: audio.clone(),
        at_s: 1.0,
        from_frame: 100,
    });
    let sr = 1000.0;
    assert_eq!(m.render(0.5, sr), (0.0, 0.0));
    let (l, r) = m.render(1.010, sr);
    assert!((l - 110.0 / 4000.0).abs() < 1e-6 && (r + 110.0 / 4000.0).abs() < 1e-6);
    // A pending start (next loop pass) takes over at its time.
    m.apply(Command::Backing {
        audio: audio.clone(),
        at_s: 1.2,
        from_frame: 0,
    });
    assert!((m.render(1.1, sr).0 - 200.0 / 4000.0).abs() < 1e-6);
    assert!((m.render(1.201, sr).0 - 1.0 / 4000.0).abs() < 1e-6);
    m.apply(Command::StopBacking { at_s: 1.5 });
    assert_eq!(m.render(1.6, sr), (0.0, 0.0));
}

/// A mixer with a monitor ring; returns the ring's write end.
fn monitored(gain: f32) -> (Mixer, rtrb::Producer<f32>) {
    let (tx, rx) = rtrb::RingBuffer::<f32>::new(MONITOR_RING);
    let mut m = mixer();
    m.gains.monitor = gain;
    m.monitor = Some(rx);
    (m, tx)
}

#[test]
fn monitor_plays_input_at_its_gain() {
    let (mut m, mut tx) = monitored(0.5);
    for _ in 0..10 {
        tx.push(0.4).unwrap();
    }
    let sr = 48_000.0;
    m.trim_monitor(sr);
    let (l, r) = m.render(0.0, sr);
    assert!((l - 0.2).abs() < 1e-6 && (r - 0.2).abs() < 1e-6);
    // Underrun: silence, not stale audio.
    for i in 1..10 {
        m.render(f64::from(i) / sr, sr);
    }
    assert_eq!(m.render(1.0, sr), (0.0, 0.0));
}

/// LH-EIN-17: with the limiter on, a monitored slap over full scale stays under the
/// ceiling instead of being cut off at full scale (which crackles).
#[test]
fn monitor_limits_peaks() {
    let (mut m, mut tx) = monitored(1.0);
    m.apply(Command::Gains(Gains {
        monitor: 1.0,
        shaping: Shaping {
            limit: kontra_dsp::Limit::On,
            ..Shaping::FLAT
        },
        ..m.gains.clone()
    }));
    let sr = 48_000.0;
    let slap = |i: usize| 1.6 * (std::f64::consts::TAU * 55.0 * i as f64 / sr).sin() as f32;
    let mut peak = 0.0f32;
    for block in 0..20 {
        for i in 0..480 {
            tx.push(slap(block * 480 + i)).unwrap();
        }
        m.trim_monitor(sr);
        for i in 0..480 {
            let (l, r) = m.render((block * 480 + i) as f64 / sr, sr);
            peak = peak.max(l.abs()).max(r.abs());
        }
    }
    assert!(peak <= kontra_dsp::LIMIT_CEILING + 1e-6, "peak {peak}");
}

/// LH-EIN-19: a new chain arrives through the ring between blocks, the monitored bass
/// fades over to it, and the old chain goes back out through the other ring.
#[test]
fn chains_come_in_and_go_back_out() {
    let (mut m, mut tx) = monitored(1.0);
    let (mut chains, incoming) = rtrb::RingBuffer::<Box<Chain>>::new(CHAIN_RING);
    let (retired, mut back) = rtrb::RingBuffer::<Box<Chain>>::new(CHAIN_RING);
    m.chains = Some(ChainLink { incoming, retired });
    let sr = 48_000.0;
    let fuzz: ChainSpec = vec![kontra_model::SlotSpec::new(kontra_model::EffectKind::Fuzz)].into();
    chains.push(Box::new(Chain::new(&fuzz, sr))).unwrap();
    m.apply(Command::Gains(Gains {
        shaping: Shaping {
            chain: fuzz,
            ..Shaping::FLAT
        },
        ..m.gains.clone()
    }));

    let tone = |i: usize| 0.3 * (std::f64::consts::TAU * 110.0 * i as f64 / sr).sin() as f32;
    let mut played = Vec::new();
    for block in 0..4 {
        for i in 0..480 {
            tx.push(tone(block * 480 + i)).unwrap();
        }
        m.trim_monitor(sr);
        m.swap_chains();
        for i in 0..480 {
            played.push(m.render((block * 480 + i) as f64 / sr, sr).0);
        }
    }
    // The fuzz is heard at the end, and the empty chain it replaced came back.
    let last = &played[played.len() - 480..];
    let apart = last
        .iter()
        .enumerate()
        .map(|(i, v)| (v - tone(1440 + i)).abs())
        .fold(0.0, f32::max);
    assert!(apart > 0.05, "{apart}");
    assert!(back.pop().is_ok());
}

/// LH-WIE-02 (round 8): the mixer transposes the backing while it plays; a note in the
/// backing still sounds at its own time (the shifter is fed ahead by its latency).
#[test]
fn backing_is_transposed_in_time() {
    use kontra_dsp::Yin;
    let sr = 48_000.0;
    // Silence, then A2 from 0.5 s, as interleaved stereo.
    let mut audio = vec![0.0f32; 2 * 24_000];
    for i in 0..48_000 {
        let x = 0.5 * (std::f64::consts::TAU * 110.0 * f64::from(i) / sr).sin() as f32;
        audio.extend([x, x]);
    }
    let mut m = mixer();
    m.gains.click = 0.0;
    m.apply(Command::Backing {
        audio: BackingAudio::stereo(audio),
        at_s: 0.0,
        from_frame: 0,
    });
    m.apply(Command::Transpose(3.0));
    let out: Vec<f32> = (0..72_000)
        .map(|i| m.render(f64::from(i) / sr, sr).0)
        .collect();

    let peak = out.iter().fold(0.0f32, |p, s| p.max(s.abs()));
    let onset = out.iter().position(|s| s.abs() > peak * 0.3).unwrap_or(0);
    assert!(onset.abs_diff(24_000) < 240, "starts at {onset}, not 24000");
    let mut yin = Yin::new(sr, 40.0, 800.0, 0.04);
    let hz = yin.estimate(&out[40_000..64_000]).map_or(0.0, |e| e.hz);
    let cents = 1200.0 * (hz / (110.0 * 2f64.powf(3.0 / 12.0))).log2();
    assert!(cents.abs() < 5.0, "{hz} Hz, {cents} cents");
}

/// LH-BIB-09: the removed bass comes back at `Gains::bass`, also while it plays.
#[test]
fn removed_bass_follows_its_gain() {
    let mut m = mixer();
    // Every frame: song without bass 0.1, removed bass 0.4 (left), -0.4 (right).
    let audio = BackingAudio::with_bass([0.1, 0.1, 0.4, -0.4].repeat(1000));
    m.apply(Command::Backing {
        audio,
        at_s: 0.0,
        from_frame: 0,
    });
    let sr = 1000.0;
    assert_eq!(m.render(0.1, sr), (0.1, 0.1));

    m.apply(Command::Gains(Gains {
        bass: 0.5,
        ..m.gains.clone()
    }));
    let (l, r) = m.render(0.2, sr);
    assert!((l - 0.3).abs() < 1e-6 && (r + 0.1).abs() < 1e-6, "{l} {r}");
}

#[test]
fn monitor_is_shaped() {
    // A 6 dB low boost doubles a slow ramp (low frequency) once the change has glided in.
    let (mut m, mut tx) = monitored(1.0);
    m.apply(Command::Gains(Gains {
        shaping: Shaping {
            low_db: 6.0,
            ..Shaping::FLAT
        },
        ..m.gains.clone()
    }));
    let sr = 48_000.0;
    let mut last = 0.0;
    for i in 0..4000 {
        tx.push(0.1).unwrap();
        last = m.render(f64::from(i) / sr, sr).0;
    }
    assert!((last - 0.2).abs() < 0.01, "{last}");
}

#[test]
fn muted_monitor_drains_its_ring() {
    let (mut m, mut tx) = monitored(0.0);
    for _ in 0..1000 {
        tx.push(0.4).unwrap();
    }
    m.trim_monitor(48_000.0);
    assert_eq!(m.render(0.0, 48_000.0), (0.0, 0.0));
    // Switched on later: starts with fresh input, not with what queued while muted.
    m.apply(Command::Gains(Gains {
        monitor: 1.0,
        ..m.gains.clone()
    }));
    assert_eq!(m.render(0.0, 48_000.0), (0.0, 0.0));
}

#[test]
fn lagging_monitor_is_cut_to_the_newest_audio() {
    let (mut m, mut tx) = monitored(1.0);
    let sr = 1000.0;
    // 200 ms queued at 1 kHz: older samples 0.1, the newest 20 ms 0.3.
    for i in 0..200 {
        tx.push(if i < 180 { 0.1 } else { 0.3 }).unwrap();
    }
    m.trim_monitor(sr);
    assert!((m.render(0.0, sr).0 - 0.3).abs() < 1e-6);
    // Within the limit nothing is dropped.
    for _ in 0..50 {
        tx.push(0.2).unwrap();
    }
    m.trim_monitor(sr);
    assert!((m.render(0.0, sr).0 - 0.3).abs() < 1e-6);
}
