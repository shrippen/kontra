//! J-E1 Synth-Bass über MIDI (P5 Aylin): a MIDI keyboard as input, a groove in song mode. The
//! take is a MIDI file played in real time like a keyboard; every second note comes 10 ms late,
//! and the judged offsets must show exactly that (timing from MIDI timestamps, not detection).
//! The keyboard view needs a Kante component and is not part of this test.

use crate::common::{run_until, write_midi, Profile, BPM, COUNT_IN_S};
use kontra_appcore::{Core, InputKind, QaOptions, UiEvent};
use std::path::Path;
use std::time::Duration;

const RIFF: [u8; 8] = [28, 33, 38, 43, 28, 33, 38, 43];
const DIVISION: u32 = 480;
/// 10 ticks at 120 BPM and 480 ticks per quarter: 10.4 ms.
const LATE_TICKS: u32 = 10;
const LATE_MS: f64 = LATE_TICKS as f64 * 60_000.0 / BPM / DIVISION as f64;
const TOLERANCE_MS: f64 = 2.0;

/// The riff as played on a keyboard: after the count-in bar, every second note late.
fn keyboard_take(path: &Path) {
    let count_in_ticks = (COUNT_IN_S * BPM / 60.0) as u32 * DIVISION;
    let mut events: Vec<(u32, [u8; 3])> = Vec::new();
    for (i, key) in RIFF.iter().enumerate() {
        let start = count_in_ticks + i as u32 * DIVISION + if i % 2 == 1 { LATE_TICKS } else { 0 };
        events.push((start, [0x90, *key, 100]));
        events.push((start + DIVISION / 2, [0x80, *key, 0]));
    }
    let mut track = vec![0x00, 0xff, 0x51, 0x03];
    track.extend(&((60_000_000.0 / BPM) as u32).to_be_bytes()[1..]);
    let mut last = 0;
    for (tick, message) in events {
        track.extend(varlen(tick - last));
        track.extend(message);
        last = tick;
    }
    track.extend([0x00, 0xff, 0x2f, 0x00]);
    let mut file = b"MThd".to_vec();
    file.extend(6u32.to_be_bytes());
    file.extend([0, 0, 0, 1]);
    file.extend((DIVISION as u16).to_be_bytes());
    file.extend(b"MTrk");
    file.extend((track.len() as u32).to_be_bytes());
    file.extend(track);
    std::fs::write(path, file).unwrap();
}

fn varlen(mut value: u32) -> Vec<u8> {
    let mut out = vec![(value & 0x7f) as u8];
    value >>= 7;
    while value > 0 {
        out.insert(0, (value & 0x7f) as u8 | 0x80);
        value >>= 7;
    }
    out
}

#[test]
fn midi_keyboard_is_judged_like_a_bass() {
    let profile = Profile::new("e1");
    let score = profile.dir.join("Groove.mid");
    write_midi(&score, "Groove", &RIFF);
    let take = profile.dir.join("keyboard.mid");
    keyboard_take(&take);

    // Step 1: MIDI as the connection (the take stands in for the keyboard).
    let qa = QaOptions {
        input_file: Some(take),
        laptop_jack: false,
    };
    let mut core = Core::new(&profile.dir, qa);
    core.update_settings(|s| s.language = Some("de".into()));
    core.reconfigure(|s| s.input_kind = InputKind::Midi);

    // Step 2: the groove in song mode.
    core.open(&score).unwrap();
    core.set_mode("song");
    core.play();
    let events = run_until(&mut core, Duration::from_secs(9), |e| {
        matches!(e, UiEvent::Finished { .. })
    });
    let judged: Vec<(&str, f64)> = events
        .iter()
        .filter_map(|e| match e {
            UiEvent::Judged {
                state, offset_ms, ..
            } => Some((*state, *offset_ms)),
            _ => None,
        })
        .collect();
    assert_eq!(judged.len(), RIFF.len(), "{judged:?}");
    for (i, (state, offset)) in judged.iter().enumerate() {
        assert_eq!(*state, "hit", "note {i}");
        let expected = if i % 2 == 1 { LATE_MS } else { 0.0 };
        assert!(
            (offset - expected).abs() <= TOLERANCE_MS,
            "note {i}: {offset} ms, played {expected} ms"
        );
    }
    assert!(
        events.iter().any(|e| matches!(e, UiEvent::Finished { .. })),
        "same summary as the bass"
    );
}
