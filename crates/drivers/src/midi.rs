//! MIDI input (LH-EIN-12): note-on and note-off from a keyboard (ALSA sequencer through midir),
//! and MIDI files read as a list of timed notes (recorded input for QA journeys).
//!
//! ```text
//! keyboard ─► ALSA seq ─► midir thread ─► on_note(timestamp µs, MidiNote)
//! take.mid ─► midly ─► [(seconds, MidiNote)]
//! ```

use std::path::{Path, PathBuf};
use thiserror::Error;

const NOTE_ON: u8 = 0x90;
const NOTE_OFF: u8 = 0x80;
const STATUS_MASK: u8 = 0xf0;
/// Microseconds per quarter note when a file sets no tempo (120 BPM).
const DEFAULT_TEMPO_US: f64 = 500_000.0;
/// Name midir gives this client in the ALSA sequencer.
const CLIENT_NAME: &str = "Kontra";
/// ALSA's loopback port: always there, never a keyboard.
const THROUGH: &str = "Midi Through";

#[derive(Debug, Error)]
pub enum MidiError {
    #[error("MIDI: {0}")]
    System(String),
    #[error("no MIDI input found")]
    NoPort,
    #[error("cannot read {path}: {message}")]
    File { path: PathBuf, message: String },
}

/// A MIDI input port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MidiPort {
    /// Stable within a session (midir's port id).
    pub id: String,
    pub name: String,
}

/// One key going down (velocity 1..=127) or up (velocity 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MidiNote {
    pub key: u8,
    pub velocity: u8,
}

/// An open input; dropping it closes the port.
pub struct MidiConnection {
    _connection: midir::MidiInputConnection<()>,
}

impl std::fmt::Debug for MidiConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MidiConnection")
    }
}

/// MIDI input ports of instruments (ALSA's loopback left out); none without a sequencer.
/// Names are the device's: "Keystation 49:Keystation 49 MIDI 1 20:0" → "Keystation 49".
pub fn list_midi_inputs() -> Vec<MidiPort> {
    let Ok(input) = midir::MidiInput::new(CLIENT_NAME) else {
        return Vec::new();
    };
    input
        .ports()
        .iter()
        .filter_map(|p| {
            let full = input.port_name(p).ok()?;
            Some(MidiPort {
                id: p.id(),
                name: device_name(&full)?,
            })
        })
        .collect()
}

/// The device part of an ALSA port name; `None` for the loopback port.
fn device_name(full: &str) -> Option<String> {
    let device = full
        .split_once(':')
        .map_or(full, |(device, _)| device)
        .trim();
    (!device.starts_with(THROUGH)).then(|| device.to_string())
}

/// Opens port `id` (`None`: the first one) and calls `on_note` for every note-on and note-off
/// with midir's timestamp in microseconds.
pub fn open_midi(
    id: Option<&str>,
    mut on_note: impl FnMut(u64, MidiNote) + Send + 'static,
) -> Result<MidiConnection, MidiError> {
    let input = midir::MidiInput::new(CLIENT_NAME).map_err(|e| MidiError::System(e.to_string()))?;
    let ports = input.ports();
    let is_instrument = |p: &&midir::MidiInputPort| {
        input
            .port_name(p)
            .ok()
            .and_then(|n| device_name(&n))
            .is_some()
    };
    let port = match id {
        Some(id) => ports.iter().find(|p| p.id() == id),
        None => ports.iter().find(is_instrument),
    }
    .ok_or(MidiError::NoPort)?;
    let connection = input
        .connect(
            port,
            "kontra-in",
            move |stamp, bytes, _| {
                if let Some(note) = parse(bytes) {
                    on_note(stamp, note);
                }
            },
            (),
        )
        .map_err(|e| MidiError::System(e.to_string()))?;
    Ok(MidiConnection {
        _connection: connection,
    })
}

/// Note-on or note-off from one complete message; a note-on with velocity 0 is a note-off.
fn parse(bytes: &[u8]) -> Option<MidiNote> {
    let [status, key, velocity, ..] = *bytes else {
        return None;
    };
    match status & STATUS_MASK {
        NOTE_ON => Some(MidiNote { key, velocity }),
        NOTE_OFF => Some(MidiNote { key, velocity: 0 }),
        _ => None,
    }
}

/// The notes of a MIDI file in time order, as (seconds from the start, note); all tracks and
/// channels, tempo changes applied.
pub fn read_midi_notes(path: &Path) -> Result<Vec<(f64, MidiNote)>, MidiError> {
    let error = |message: String| MidiError::File {
        path: path.to_path_buf(),
        message,
    };
    let bytes = std::fs::read(path).map_err(|e| error(e.to_string()))?;
    let smf = midly::Smf::parse(&bytes).map_err(|e| error(e.to_string()))?;
    let midly::Timing::Metrical(division) = smf.header.timing else {
        return Err(error("timecode timing is not supported".into()));
    };
    let division = f64::from(division.as_int().max(1));

    // (tick, event) of all tracks; tempo changes first at equal ticks.
    let mut events: Vec<(u64, Option<f64>, Option<MidiNote>)> = Vec::new();
    for track in &smf.tracks {
        let mut tick = 0u64;
        for event in track {
            tick += u64::from(event.delta.as_int());
            match event.kind {
                midly::TrackEventKind::Meta(midly::MetaMessage::Tempo(us)) => {
                    events.push((tick, Some(f64::from(us.as_int())), None));
                }
                midly::TrackEventKind::Midi { message, .. } => {
                    let note = match message {
                        midly::MidiMessage::NoteOn { key, vel } => MidiNote {
                            key: key.as_int(),
                            velocity: vel.as_int(),
                        },
                        midly::MidiMessage::NoteOff { key, .. } => MidiNote {
                            key: key.as_int(),
                            velocity: 0,
                        },
                        _ => continue,
                    };
                    events.push((tick, None, Some(note)));
                }
                _ => {}
            }
        }
    }
    events.sort_by_key(|(tick, tempo, _)| (*tick, tempo.is_none()));

    let mut out = Vec::new();
    let (mut last_tick, mut seconds, mut tempo_us) = (0u64, 0.0f64, DEFAULT_TEMPO_US);
    for (tick, tempo, note) in events {
        seconds += (tick - last_tick) as f64 / division * tempo_us / 1e6;
        last_tick = tick;
        if let Some(us) = tempo {
            tempo_us = us;
        }
        if let Some(note) = note {
            out.push((seconds, note));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_names_are_device_names() {
        assert_eq!(
            device_name("Keystation 49 MK3:Keystation 49 MK3 MIDI 1 20:0").as_deref(),
            Some("Keystation 49 MK3")
        );
        assert_eq!(device_name("Midi Through:Midi Through Port-0 14:0"), None);
        assert_eq!(device_name("USB Keys").as_deref(), Some("USB Keys"));
    }

    #[test]
    fn messages_become_notes() {
        assert_eq!(
            parse(&[0x90, 40, 100]),
            Some(MidiNote {
                key: 40,
                velocity: 100
            })
        );
        assert_eq!(
            parse(&[0x93, 40, 0]),
            Some(MidiNote {
                key: 40,
                velocity: 0
            })
        );
        assert_eq!(
            parse(&[0x80, 40, 64]),
            Some(MidiNote {
                key: 40,
                velocity: 0
            })
        );
        assert_eq!(parse(&[0xb0, 64, 127]), None, "sustain pedal is no note");
        assert_eq!(parse(&[0x90, 40]), None);
    }

    #[test]
    fn file_notes_follow_the_tempo() {
        // 480 ticks per quarter; quarter notes at 120 BPM, then a change to 60 BPM.
        let mut track = Vec::new();
        let tempo = |bpm: u32| (60_000_000 / bpm).to_be_bytes()[1..].to_vec();
        track.extend([0x00, 0xff, 0x51, 0x03]);
        track.extend(tempo(120));
        for (delta, status, key, vel) in [
            (0u8, 0x90u8, 40u8, 100u8),
            (0x00, 0x80, 40, 0),
            // 480 ticks later (varlen 0x83 0x60)
        ] {
            track.extend([delta, status, key, vel]);
        }
        track.extend([0x83, 0x60, 0xff, 0x51, 0x03]);
        track.extend(tempo(60));
        track.extend([0x00, 0x90, 43, 90]);
        track.extend([0x83, 0x60, 0x90, 45, 80]);
        track.extend([0x00, 0xff, 0x2f, 0x00]);
        let mut file = b"MThd".to_vec();
        file.extend(6u32.to_be_bytes());
        file.extend([0, 0, 0, 1, 0x01, 0xe0]);
        file.extend(b"MTrk");
        file.extend((track.len() as u32).to_be_bytes());
        file.extend(track);
        let path = std::env::temp_dir().join(format!("kontra-midi-{}.mid", std::process::id()));
        std::fs::write(&path, file).unwrap();

        let notes = read_midi_notes(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        let ons: Vec<(f64, u8)> = notes
            .iter()
            .filter(|(_, n)| n.velocity > 0)
            .map(|(t, n)| (*t, n.key))
            .collect();
        assert_eq!(ons.len(), 3);
        assert_eq!(ons[0], (0.0, 40));
        assert!((ons[1].0 - 0.5).abs() < 1e-9, "{ons:?}");
        assert!(
            (ons[2].0 - 1.5).abs() < 1e-9,
            "one quarter at 60 BPM: {ons:?}"
        );
    }
}
