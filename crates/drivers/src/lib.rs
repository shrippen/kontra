//! Lowest layer: talks to sound and MIDI devices, reads/writes audio and MIDI files, lists the
//! download folder and runs the external stem separator. No music logic here.

mod audio;
mod file;
mod folder;
mod midi;
#[cfg(target_os = "linux")]
mod pipewire;
mod separator;

pub use audio::{
    list_devices, AudioError, Connection, DeviceInfo, Direction, InputStream, OutputStream,
    StreamConfig, NULL_DEVICE,
};
pub use file::{read_audio, write_wav, AudioBuffer, FileError};
pub use folder::{download_dir, recent_files, RecentFile};
pub use midi::{
    list_midi_inputs, open_midi, read_midi_notes, MidiConnection, MidiError, MidiNote, MidiPort,
};
pub use separator::{find_separator, in_sandbox, SeparationJob, Separator, SeparatorError};
