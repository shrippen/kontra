//! Unplugged interface (LH-ALL-04): after a device error Kontra offers the next input and
//! watches the device list every two seconds; when the lost device is back it is used again.
//!
//! ```text
//! playing ──device error──► lost ──user takes offer──► fallback in use
//!                            │                          │
//!                            └──── list shows it again ─┴──► back: own device again
//! ```

use crate::{Core, UiEvent};
use kontra_session::{list_devices, DeviceInfo, Direction, InputChoice};
use std::time::{Duration, Instant};

const POLL: Duration = Duration::from_secs(2);

/// The input that went away; `id: None` was the system default.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Lost {
    pub(crate) id: Option<String>,
    polled: Instant,
}

/// The input to offer instead of `lost`: the system default if it is another device, else
/// the first other one.
pub(crate) fn next_input<'a>(
    devices: &'a [DeviceInfo],
    lost: Option<&str>,
) -> Option<&'a DeviceInfo> {
    let other = |d: &&DeviceInfo| Some(d.id.as_str()) != lost;
    devices
        .iter()
        .filter(other)
        .find(|d| d.is_default)
        .or_else(|| devices.iter().find(other))
}

/// Whether the lost input is listed again (the default: any default input).
pub(crate) fn is_back(devices: &[DeviceInfo], lost: Option<&str>) -> bool {
    match lost {
        Some(id) => devices.iter().any(|d| d.id == id),
        None => devices.iter().any(|d| d.is_default),
    }
}

impl Core {
    /// Called when a device failed: remembers the input and offers another of `devices`.
    pub(crate) fn input_lost_in(
        &mut self,
        opened: Option<&InputChoice>,
        devices: Vec<DeviceInfo>,
    ) -> Option<UiEvent> {
        if self.qa.input_file.is_some() {
            return None;
        }
        // The device as opened: with none saved, the one picked then (an interface), not "the
        // default input" the laptop's microphone would answer for (review #213).
        let opened = match opened {
            Some(InputChoice::Device { id: Some(id), .. }) => Some(id.clone()),
            _ => None,
        };
        let id = opened
            .or_else(|| self.fallback_input.clone())
            .or_else(|| self.settings.input_device.clone());
        let original = self
            .lost
            .as_ref()
            .map_or_else(|| id.clone(), |l| l.id.clone());
        self.lost = Some(Lost {
            id: original.clone(),
            polled: Instant::now(),
        });
        let offer = next_input(&devices, id.as_deref().or(original.as_deref()))?;
        Some(UiEvent::InputOffer {
            id: offer.id.clone(),
            text: self.tr("unplug.offer", &[("name", offer.name.clone())]),
        })
    }

    /// Uses `id` until the lost input comes back (the saved choice stays).
    pub fn use_offered_input(&mut self, id: &str) {
        self.stop_live();
        self.fallback_input = Some(id.to_string());
    }

    /// Every two seconds while an input is lost: is it back?
    pub(crate) fn watch_lost(&mut self) -> Option<UiEvent> {
        let lost = self.lost.as_mut()?;
        if lost.polled.elapsed() < POLL {
            return None;
        }
        lost.polled = Instant::now();
        let id = lost.id.clone();
        if !is_back(&list_devices(Direction::Input), id.as_deref()) {
            return None;
        }
        self.lost = None;
        if self.fallback_input.take().is_some() {
            // The run on the stand-in device ends; the next play uses the own one.
            self.stop_live();
        }
        Some(UiEvent::InputBack(self.tr("unplug.back", &[])))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kontra_session::Connection;

    fn device(id: &str, is_default: bool) -> DeviceInfo {
        DeviceInfo {
            id: id.into(),
            name: format!("Device {id}"),
            direction: Direction::Input,
            is_default,
            connection: Connection::Usb,
            sample_rate: 48_000,
            channels: 2,
            past_pipewire: false,
        }
    }

    #[test]
    fn offers_the_default_then_any_other_input() {
        let list = [device("laptop", false), device("pipewire", true)];
        assert_eq!(next_input(&list, Some("scarlett")).unwrap().id, "pipewire");
        // The default is the lost one itself: take another.
        let list = [device("scarlett", true), device("laptop", false)];
        assert_eq!(next_input(&list, Some("scarlett")).unwrap().id, "laptop");
        assert!(next_input(&[device("scarlett", true)], Some("scarlett")).is_none());
        assert!(next_input(&[], None).is_none());
    }

    #[test]
    fn notices_the_device_coming_back() {
        let without = [device("laptop", true)];
        let with = [device("laptop", true), device("scarlett", false)];
        assert!(!is_back(&without, Some("scarlett")));
        assert!(is_back(&with, Some("scarlett")));
        assert!(!is_back(&[], None));
        assert!(is_back(&without, None));
    }
}
