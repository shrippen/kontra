//! Audio and MIDI devices as plain data: what the drivers list and the layers above show.

/// Output device id of a timer-driven output without sound (endurance tests, LH-NF-04).
pub const NULL_DEVICE: &str = "kontra:null";

/// What the last audio thread got (reported by the hardware check).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Priority {
    /// No audio stream ran yet.
    Unknown,
    Realtime,
    /// Refused (no RLIMIT_RTPRIO): blocks may get lost under load at small quanta.
    Normal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Input,
    Output,
}

/// How a device is connected; used for hints (Bluetooth adds latency, USB is likely an interface).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Connection {
    BuiltIn,
    Usb,
    /// A USB input without an output (webcam): not the bass interface. An interface also plays
    /// back (LH-EIN-02).
    UsbMicrophone,
    Bluetooth,
    Hdmi,
    Other,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub direction: Direction,
    pub is_default: bool,
    pub connection: Connection,
    pub sample_rate: u32,
    pub channels: u16,
    /// Opened through ALSA while PipeWire runs: the card has no PipeWire node (lost after
    /// "device busy"); replugging or restarting WirePlumber brings the node back.
    pub past_pipewire: bool,
}

/// A MIDI input port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MidiPort {
    /// Stable within a session (midir's port id).
    pub id: String,
    pub name: String,
}
