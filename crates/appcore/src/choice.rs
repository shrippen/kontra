//! Which device Kontra takes when the player chose none, or chose one that is gone (LH-EIN-02).
//!
//! ```text
//! saved and listed ─► that one
//! else by connection: interface  ─► a USB device with an output
//!                     microphone ─► a USB device                  (the system's default first)
//!                     laptop jack           ─► the built-in card
//! else ─► None: the system decides
//! ```

use crate::InputKind;
use kontra_session::{Connection, DeviceInfo};

/// The saved input if it is listed, else the one that fits the connection `kind` (LH-EIN-02):
/// an unplugged device or an ALSA id saved before PipeWire means "Automatic", not the system
/// default (review #73).
pub(crate) fn chosen_input(
    saved: Option<&str>,
    devices: &[DeviceInfo],
    kind: InputKind,
) -> Option<String> {
    if let Some(id) = saved.filter(|id| devices.iter().any(|d| d.id == *id)) {
        return Some(id.to_string());
    }
    preferred_input(devices, kind).map(|d| d.id.clone())
}

/// The saved output if it is listed (or the engine's timer), else the system default.
pub(crate) fn chosen_output(saved: Option<&str>, devices: &[DeviceInfo]) -> Option<String> {
    let id = saved?;
    if id == kontra_session::NULL_DEVICE || devices.iter().any(|d| d.id == id) {
        return Some(id.to_string());
    }
    None
}

/// The input Kontra takes when none is chosen: where the bass is plugged in by `kind`. An
/// interface is a USB device that also plays back, a USB microphone one that only listens (a webcam, never taken for the cable, review #231); the laptop's jack is its built-in card (QA
/// 27.10.: "Klinke direkt" still listened to the Scarlett). The system's default first if it
/// is one of them.
pub(crate) fn preferred_input(devices: &[DeviceInfo], kind: InputKind) -> Option<&DeviceInfo> {
    let fits = |d: &&DeviceInfo| match kind {
        InputKind::LaptopJack => d.connection == Connection::BuiltIn,
        InputKind::Line | InputKind::Midi => d.connection == Connection::Usb,
        InputKind::Microphone => {
            matches!(d.connection, Connection::Usb | Connection::UsbMicrophone)
        }
    };
    devices
        .iter()
        .filter(fits)
        .find(|d| d.is_default)
        .or_else(|| devices.iter().find(fits))
}
