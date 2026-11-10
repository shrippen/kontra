//! Lowest layer: talks to sound and MIDI devices and the web, reads/writes audio and MIDI files, lists the
//! download folder and runs the external stem separator. No music logic here.

mod audio;
mod file;
mod folder;
mod http;
mod midi;
#[cfg(target_os = "linux")]
mod pipewire;
pub mod plugins;
mod separator;
mod urls;

pub use audio::{
    audio_priority, list_devices, monitor_rate, AudioError, Connection, DeviceInfo, Direction,
    InputStream, OutputStream, Priority, StreamConfig, NULL_DEVICE,
};
pub use file::{read_audio, write_wav, AudioBuffer, FileError};
pub use folder::{cache_dir_of, data_dir_of, download_dir, recent_files, RecentFile};
pub use http::{FileHttp, Http, HttpError, Web};
pub use midi::{
    list_midi_inputs, open_midi, read_midi_notes, MidiConnection, MidiError, MidiNote, MidiPort,
};
pub use separator::{find_separator, in_sandbox, SeparationJob, Separator, SeparatorError};
pub use urls::{file_url, local_path};
