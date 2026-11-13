//! The device lists of the settings and setup pages. Listing asks the audio system for every
//! device: 690 ms for the inputs and 215 ms for the outputs under ALSA, so the settings page
//! stood 1.2 s before its first frame (FPS analysis, 12.11.). A thread lists them at start and
//! after each look; the pages show the last lists at once and the new ones when they come.
//!
//!   start, refresh, USB watch ──► thread: inputs, outputs ──► take_lists ──► pages, USB watch
//!   nothing listed yet ──► listed on the spot, as before

use crate::Core;
use kontra_session::{list_devices, DeviceInfo, Direction};
use std::sync::mpsc::{channel, Receiver, TryRecvError};

/// Lists the devices of one direction; tests put in a slow or fixed list.
pub(crate) type ListDevices = fn(Direction) -> Vec<DeviceInfo>;

#[derive(Debug)]
pub(crate) struct DeviceLists {
    list: ListDevices,
    known: Option<Lists>,
    job: Option<Receiver<Lists>>,
    /// New lists the pages, and the USB watch, have not seen yet.
    new_for_pages: bool,
    new_for_usb: bool,
}

impl Default for DeviceLists {
    fn default() -> Self {
        Self {
            list: list_devices,
            known: None,
            job: None,
            new_for_pages: false,
            new_for_usb: false,
        }
    }
}

#[derive(Debug, Clone)]
struct Lists {
    inputs: Vec<DeviceInfo>,
    outputs: Vec<DeviceInfo>,
}

impl Core {
    /// Lists the devices again in the background, unless a listing runs.
    pub fn relist_devices(&mut self) {
        if self.device_lists.job.is_some() {
            return;
        }
        let (tell, lists) = channel();
        let list = self.device_lister();
        std::thread::spawn(move || {
            let inputs = list(Direction::Input);
            let outputs = list(Direction::Output);
            // The core may be gone by now; nobody waits for the lists then.
            let _ = tell.send(Lists { inputs, outputs });
        });
        self.device_lists.job = Some(lists);
    }

    /// The first lists, at start; not in a QA run (input from a file), where listing would
    /// probe the real ALSA cards at every start. The settings page lists when it opens.
    pub fn list_devices_early(&mut self) {
        if self.qa.input_file.is_some() {
            return;
        }
        self.relist_devices();
    }

    /// True once new lists came; the pages then read `devices_json` again.
    pub fn devices_poll(&mut self) -> bool {
        self.take_lists();
        std::mem::take(&mut self.device_lists.new_for_pages)
    }

    /// The inputs of a listing the USB watch has not seen yet.
    pub(crate) fn new_inputs(&mut self) -> Option<Vec<DeviceInfo>> {
        self.take_lists();
        if !std::mem::take(&mut self.device_lists.new_for_usb) {
            return None;
        }
        self.device_lists.known.as_ref().map(|k| k.inputs.clone())
    }

    /// Keeps the lists of a finished listing.
    fn take_lists(&mut self) {
        let Some(job) = self.device_lists.job.as_ref() else {
            return;
        };
        match job.try_recv() {
            Ok(lists) => {
                self.device_lists.known = Some(lists);
                self.device_lists.new_for_pages = true;
                self.device_lists.new_for_usb = true;
                self.device_lists.job = None;
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => self.device_lists.job = None,
        }
    }

    /// The last list of one direction, or a fresh one if none came yet.
    pub(crate) fn listed(&self, direction: Direction) -> Vec<DeviceInfo> {
        let Some(known) = self.device_lists.known.as_ref() else {
            return (self.device_lister())(direction);
        };
        match direction {
            Direction::Input => known.inputs.clone(),
            Direction::Output => known.outputs.clone(),
        }
    }

    /// How this core lists devices: the system's list, or a test's.
    pub(crate) fn device_lister(&self) -> ListDevices {
        self.device_lists.list
    }

    #[cfg(test)]
    pub(crate) fn list_devices_with(&mut self, list: ListDevices) {
        self.device_lists.list = list;
    }
}
