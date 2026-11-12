//! The Bluetooth hint at a start (LH-EIN-06). Telling whether the output is Bluetooth lists
//! every output device, which held the window still for 580 ms per start under ALSA (FPS
//! analysis, 12.11.). A thread lists them; `tick` gives the hint when the answer is there.
//!
//!   play ──► thread: list outputs ──► bool ──► tick ──► "hint.bluetooth"

use crate::{Core, UiEvent};
use kontra_session::{list_devices, Connection, DeviceInfo, Direction};
use std::sync::mpsc::{channel, Receiver};

/// Lists the devices of one direction; tests put in a slow or fixed list.
pub(crate) type ListDevices = fn(Direction) -> Vec<DeviceInfo>;

#[derive(Debug)]
pub(crate) struct BluetoothCheck {
    list: ListDevices,
    answer: Option<Receiver<bool>>,
}

impl Default for BluetoothCheck {
    fn default() -> Self {
        Self {
            list: list_devices,
            answer: None,
        }
    }
}

impl Core {
    /// Asks in the background whether the chosen output is Bluetooth.
    pub(crate) fn ask_bluetooth(&mut self) {
        let (tell, answer) = channel();
        let list = self.bluetooth.list;
        let chosen = self.settings.output_device.clone();
        std::thread::spawn(move || {
            let bluetooth = list(Direction::Output).iter().any(|d| {
                let picked = match &chosen {
                    Some(id) => &d.id == id,
                    None => d.is_default,
                };
                picked && d.connection == Connection::Bluetooth
            });
            // The core may be gone by now; nobody waits for the answer then.
            let _ = tell.send(bluetooth);
        });
        self.bluetooth.answer = Some(answer);
    }

    /// The hint once the thread has answered yes; nothing while it lists. The bar shows the
    /// last message, so the monitor's hint from `play` follows again: it says what to do.
    pub(crate) fn bluetooth_hint(&mut self) -> Vec<UiEvent> {
        let Some(Ok(answer)) = self.bluetooth.answer.as_ref().map(Receiver::try_recv) else {
            return Vec::new();
        };
        self.bluetooth.answer = None;
        if !answer {
            return Vec::new();
        }
        let mut hints = vec![UiEvent::Message(self.tr("hint.bluetooth", &[]))];
        hints.extend(self.monitor_hint().map(UiEvent::Message));
        hints
    }

    /// Drops a question still open; its answer would come with nothing playing.
    pub(crate) fn forget_bluetooth(&mut self) {
        self.bluetooth.answer = None;
    }

    #[cfg(test)]
    pub(crate) fn list_devices_with(&mut self, list: ListDevices) {
        self.bluetooth.list = list;
    }
}
