//! Lowest layer: talks to sound and MIDI devices, reads/writes audio and MIDI files and runs the
//! external stem separator. No music logic here.

mod audio;
mod file;
mod midi;
mod separator;

pub use audio::{
    list_devices, AudioError, Connection, DeviceInfo, Direction, InputStream, OutputStream,
    StreamConfig, NULL_DEVICE,
};
pub use file::{read_audio, write_wav, AudioBuffer, FileError};
pub use midi::{
    list_midi_inputs, open_midi, read_midi_notes, MidiConnection, MidiError, MidiNote, MidiPort,
};
pub use separator::{find_separator, in_sandbox, SeparationJob, Separator, SeparatorError};
