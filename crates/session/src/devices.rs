//! The sound and MIDI devices the system offers, for the layers above session.

use kontra_model::{DeviceInfo, Direction, MidiPort, Priority};

/// Audio input or output devices, the interfaces first.
pub fn list_devices(direction: Direction) -> Vec<DeviceInfo> {
    kontra_engine::list_devices(direction)
}

/// MIDI input ports of instruments; none without a sequencer.
pub fn list_midi_inputs() -> Vec<MidiPort> {
    kontra_engine::list_midi_inputs()
}

/// The scheduling the audio thread got; `Unknown` until a stream ran.
pub fn audio_priority() -> Priority {
    kontra_engine::audio_priority()
}
