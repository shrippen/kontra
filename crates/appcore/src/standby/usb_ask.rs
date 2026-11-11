//! A USB input without output (a guitar cable, a webcam) that newly appears: Kontra asks once
//! whether an instrument or a microphone hangs on it (LH-EIN-02, round 19) and remembers the
//! answer by device name.
//!
//! ```text
//! list, every POLL ──► new USB-only name, no answer yet ──► question (one at a time)
//!                                                              │
//!   "instrument": counts as an interface ◄── answer ──────────┴─► "microphone": as before
//! ```
//!
//! Only the app watches (`keep_devices_open`); the first look at the start only notes what is
//! plugged in, so nothing already there is asked about. Devices are listed, never opened.

use super::Standby;
use crate::store::UsbRole;
use crate::{Core, UiEvent};
use kontra_session::{list_devices, Connection, DeviceInfo, Direction};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

const POLL: Duration = Duration::from_secs(5);

/// What the watch knows: the USB-only inputs of the last look and the open question.
#[derive(Debug, Default)]
pub(super) struct UsbAsk {
    polled: Option<Instant>,
    /// Names seen at the last look; `None` before the first.
    seen: Option<BTreeSet<String>>,
    /// The device the open question is about.
    pending: Option<String>,
}

/// Names of the USB inputs without an output in `devices`.
fn usb_only(devices: &[DeviceInfo]) -> BTreeSet<String> {
    devices
        .iter()
        .filter(|d| d.connection == Connection::UsbMicrophone)
        .map(|d| d.name.clone())
        .collect()
}

/// The inputs as the player answered: an instrument on a USB-only input makes it an
/// interface for the choice.
pub(super) fn as_answered(
    mut devices: Vec<DeviceInfo>,
    answers: &BTreeMap<String, UsbRole>,
) -> Vec<DeviceInfo> {
    for d in &mut devices {
        let instrument = answers.get(&d.name) == Some(&UsbRole::Instrument);
        if instrument && d.connection == Connection::UsbMicrophone {
            d.connection = Connection::Usb;
        }
    }
    devices
}

impl UsbAsk {
    /// The device to ask about after this look: a name that was not there at the last one and
    /// has no answer. While a question is open the others wait, unseen, for their turn.
    fn next(
        &mut self,
        now: &BTreeSet<String>,
        answers: &BTreeMap<String, UsbRole>,
    ) -> Option<String> {
        let first = self.seen.is_none();
        let before = self.seen.take().unwrap_or_default();
        let mut fresh: Vec<String> = now
            .iter()
            .filter(|n| !before.contains(*n) && !answers.contains_key(*n))
            .cloned()
            .collect();
        let ask = match (first, &self.pending) {
            (false, None) if !fresh.is_empty() => Some(fresh.remove(0)),
            _ => None,
        };
        // What is still to ask stays unseen, so the next look finds it again.
        let waiting = if first { Vec::new() } else { fresh };
        self.seen = Some(
            now.iter()
                .filter(|n| !waiting.contains(n))
                .cloned()
                .collect(),
        );
        ask
    }
}

impl Standby {
    /// The open question, if any.
    fn asking(&self) -> Option<&str> {
        self.usb.pending.as_deref()
    }
}

impl Core {
    /// The input devices, with the player's answers applied (LH-EIN-02).
    pub(crate) fn inputs(&self) -> Vec<DeviceInfo> {
        as_answered(list_devices(Direction::Input), &self.settings.usb_inputs)
    }

    /// The input the player chose, else the one that fits the connection (LH-EIN-02): an
    /// interface, or the built-in card for the laptop jack.
    pub(crate) fn input_device(&self) -> Option<String> {
        crate::chosen_input(
            self.settings.input_device.as_deref(),
            &self.inputs(),
            self.settings.input_kind,
        )
    }

    /// Every few seconds: is there a new USB-only input to ask about? Only for the app.
    pub(crate) fn watch_usb(&mut self) -> Option<UiEvent> {
        let usb = &mut self.standby.usb;
        if !self.standby.wanted || self.qa.input_file.is_some() {
            return None;
        }
        if usb.polled.is_some_and(|at| at.elapsed() < POLL) {
            return None;
        }
        usb.polled = Some(Instant::now());
        self.look_at_usb(&list_devices(Direction::Input))
    }

    /// One look at `devices`: the question for a new USB-only input, or its end when the
    /// device it was about is gone.
    fn look_at_usb(&mut self, devices: &[DeviceInfo]) -> Option<UiEvent> {
        let now = usb_only(devices);
        let gone = self
            .standby
            .asking()
            .filter(|p| !now.contains(*p))
            .is_some();
        if gone {
            self.standby.usb.pending = None;
            self.standby.usb.seen = Some(now);
            return Some(UiEvent::UsbAsk(String::new()));
        }
        let name = self.standby.usb.next(&now, &self.settings.usb_inputs)?;
        let text = self.tr("usbask.question", &[("name", name.clone())]);
        self.standby.usb.pending = Some(name.clone());
        Some(UiEvent::UsbAsk(
            json!({ "name": name, "text": text }).to_string(),
        ))
    }

    /// The player's answer, "instrument" or "microphone", to the open question: remembered by
    /// the device's name.
    pub fn answer_usb(&mut self, answer: &str) {
        let role = match answer {
            "instrument" => UsbRole::Instrument,
            "microphone" => UsbRole::Microphone,
            _ => return,
        };
        let Some(name) = self.standby.usb.pending.take() else {
            return;
        };
        self.update_settings(|s| {
            s.usb_inputs.insert(name, role);
        });
        // An instrument is an interface now: the standby looks again.
        self.release_standby();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::QaOptions;

    fn device(name: &str, connection: Connection) -> DeviceInfo {
        DeviceInfo {
            id: format!("id-{name}"),
            name: name.into(),
            direction: Direction::Input,
            is_default: false,
            connection,
            sample_rate: 48_000,
            channels: 1,
            past_pipewire: false,
        }
    }

    fn core(name: &str) -> (Core, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("kontra-usbask-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        (Core::new(&dir, QaOptions::default()), dir)
    }

    fn asked(event: Option<UiEvent>) -> Option<String> {
        match event {
            Some(UiEvent::UsbAsk(json)) if !json.is_empty() => {
                let v: serde_json::Value = serde_json::from_str(&json).unwrap();
                v["name"].as_str().map(str::to_string)
            }
            _ => None,
        }
    }

    #[test]
    fn asks_about_a_new_usb_input_once() {
        let (mut core, dir) = core("new");
        let solo = device("Scarlett", Connection::Usb);
        let cable = device("Guitar Link", Connection::UsbMicrophone);

        assert_eq!(
            asked(core.look_at_usb(std::slice::from_ref(&solo))),
            None,
            "first look"
        );
        assert_eq!(
            asked(core.look_at_usb(&[solo.clone(), cable.clone()])).as_deref(),
            Some("Guitar Link")
        );
        assert_eq!(asked(core.look_at_usb(&[solo, cable])), None, "not twice");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn asks_nothing_about_devices_already_there() {
        let (mut core, dir) = core("start");
        let cable = device("Guitar Link", Connection::UsbMicrophone);
        assert_eq!(asked(core.look_at_usb(std::slice::from_ref(&cable))), None);
        assert_eq!(asked(core.look_at_usb(&[cable])), None);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn interfaces_and_other_inputs_are_not_asked_about() {
        let (mut core, dir) = core("others");
        core.look_at_usb(&[]);
        let list = [
            device("Scarlett", Connection::Usb),
            device("HDA", Connection::BuiltIn),
        ];
        assert_eq!(asked(core.look_at_usb(&list)), None);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn an_instrument_counts_as_an_interface_and_is_kept() {
        let (mut core, dir) = core("instrument");
        let cable = device("Guitar Link", Connection::UsbMicrophone);
        core.look_at_usb(&[]);
        core.look_at_usb(std::slice::from_ref(&cable));
        core.answer_usb("instrument");

        let chosen = crate::chosen_input(
            None,
            &as_answered(vec![cable.clone()], &core.settings.usb_inputs),
            crate::InputKind::Line,
        );
        assert_eq!(chosen.as_deref(), Some("id-Guitar Link"));
        // Remembered across a restart, and plugging it in again asks nothing.
        let mut again = Core::new(&dir, QaOptions::default());
        assert_eq!(
            again.settings.usb_inputs.get("Guitar Link"),
            Some(&UsbRole::Instrument)
        );
        again.look_at_usb(&[]);
        assert_eq!(asked(again.look_at_usb(&[cable])), None);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_microphone_stays_one() {
        let (mut core, dir) = core("microphone");
        let webcam = device("Webcam", Connection::UsbMicrophone);
        core.look_at_usb(&[]);
        core.look_at_usb(std::slice::from_ref(&webcam));
        core.answer_usb("microphone");

        let list = as_answered(vec![webcam], &core.settings.usb_inputs);
        assert_eq!(list[0].connection, Connection::UsbMicrophone);
        assert_eq!(
            crate::chosen_input(None, &list, crate::InputKind::Line),
            None
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn one_question_at_a_time_and_it_ends_with_the_device() {
        let (mut core, dir) = core("queue");
        let a = device("A cable", Connection::UsbMicrophone);
        let b = device("B cable", Connection::UsbMicrophone);
        core.look_at_usb(&[]);
        assert_eq!(
            asked(core.look_at_usb(&[a.clone(), b.clone()])).as_deref(),
            Some("A cable")
        );
        assert_eq!(
            asked(core.look_at_usb(&[a.clone(), b.clone()])),
            None,
            "A is open"
        );

        core.answer_usb("microphone");
        assert_eq!(
            asked(core.look_at_usb(&[a.clone(), b.clone()])).as_deref(),
            Some("B cable")
        );
        // Unplugged unanswered: the question goes away.
        assert_eq!(core.look_at_usb(&[a]), Some(UiEvent::UsbAsk(String::new())));
        let _ = std::fs::remove_dir_all(dir);
    }
}
