//! Lowest layer: talks to sound devices and reads/writes audio files. No music logic here.

mod audio;
mod file;

pub use audio::{
    list_devices, AudioError, Connection, DeviceInfo, Direction, InputStream, OutputStream,
    StreamConfig, NULL_DEVICE,
};
pub use file::{read_audio, write_wav, AudioBuffer, FileError};
