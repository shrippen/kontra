//! Devices kept by id. An ASIO host loads one driver at a time: while a stream holds it,
//! listing the devices again finds nothing, so the other direction of the same driver
//! (loopback: output first, then input) reuses the device found the first time.

use std::sync::Mutex;

pub(super) struct Kept<T> {
    devices: Mutex<Vec<(String, T)>>,
}

impl<T: Clone> Kept<T> {
    pub(super) const fn new() -> Self {
        Self {
            devices: Mutex::new(Vec::new()),
        }
    }

    /// What `find` returns, remembered for `id`; when it finds nothing, the one remembered.
    pub(super) fn find(&self, id: &str, find: impl FnOnce() -> Option<T>) -> Option<T> {
        let mut devices = self.devices.lock().ok()?;
        let slot = devices.iter().position(|(kept, _)| kept == id);
        let Some(found) = find() else {
            return slot.map(|i| devices[i].1.clone());
        };

        match slot {
            Some(i) => devices[i].1 = found.clone(),
            None => devices.push((id.to_string(), found.clone())),
        }
        Some(found)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Win2 (11.11.2026, Focusrite ASIO): loopback with one ASIO driver for both directions
    /// failed with "no audio device found for Input", the driver being held by the output.
    #[test]
    fn a_busy_driver_is_found_again() {
        let kept = Kept::new();

        assert_eq!(kept.find("asio:X", || Some(1)), Some(1));
        assert_eq!(
            kept.find("asio:X", || None),
            Some(1),
            "listing finds nothing"
        );
        assert_eq!(kept.find("asio:Y", || None), None, "never seen");
        assert_eq!(
            kept.find("asio:X", || Some(2)),
            Some(2),
            "a fresh listing wins"
        );
        assert_eq!(kept.find("asio:X", || None), Some(2));
    }
}
