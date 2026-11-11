//! A USB input without output (a guitar cable, a webcam) that newly appears: Kontra asks once
//! whether an instrument or a microphone hangs on it (LH-EIN-02, round 19) and remembers the
//! answer by device id.
//!
//! ```text
//! list, every POLL ──► new USB-only name, no answer yet ──► question (one at a time)
//!                                                              │
//!   "instrument": counts as an interface ◄── answer ──────────┴─► "microphone": as before
//! ```
//!
//! Only the app watches (`keep_devices_open`); the first look at the start only notes what is
//! plugged in, so nothing already there is asked about. Devices are listed, never opened.

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
    /// Ids seen at the last look; `None` before the first.
    seen: Option<BTreeSet<String>>,
    /// The id of the device the open question is about.
    pending: Option<String>,
}

/// The USB inputs without an output in `devices`, id to name. The id is the stable node name;
/// the name is translated by the audio server.
fn usb_only(devices: &[DeviceInfo]) -> BTreeMap<String, String> {
    devices
        .iter()
        .filter(|d| d.connection == Connection::UsbMicrophone)
        .map(|d| (d.id.clone(), d.name.clone()))
        .collect()
}

/// The inputs as the player answered: an instrument on a USB-only input makes it an
/// interface for the choice.
pub(super) fn as_answered(
    mut devices: Vec<DeviceInfo>,
    answers: &BTreeMap<String, UsbRole>,
) -> Vec<DeviceInfo> {
    for d in &mut devices {
        let instrument = answers.get(&d.id) == Some(&UsbRole::Instrument);
        if instrument && d.connection == Connection::UsbMicrophone {
            d.connection = Connection::Usb;
        }
    }
    devices
}

impl UsbAsk {
    /// Takes this look: the id to ask about, if a new one without answer came and no question
    /// is open. What still waits for its turn stays unseen, so the next look finds it again.
    fn look(
        &mut self,
        now: &BTreeSet<String>,
        answers: &BTreeMap<String, UsbRole>,
    ) -> Option<String> {
        let first = self.seen.is_none();
        let before = self.seen.take().unwrap_or_default();
        let mut fresh: Vec<String> = now
            .iter()
            .filter(|id| !before.contains(*id) && !answers.contains_key(*id))
            .cloned()
            .collect();
        let ask = match (first, &self.pending) {
            (false, None) if !fresh.is_empty() => Some(fresh.remove(0)),
            _ => None,
        };
        let waiting = if first { Vec::new() } else { fresh };
        self.seen = Some(
            now.iter()
                .filter(|id| !waiting.contains(id))
                .cloned()
                .collect(),
        );
        ask
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

    /// Every few seconds: is there a new USB-only input to ask about? Only for the app, and only
    /// while idle: listing asks the audio server and would stall a run, the tuner or a calibration.
    pub(crate) fn watch_usb(&mut self) -> Option<UiEvent> {
        if !self.standby.wanted || self.qa.input_file.is_some() || !self.idle() {
            return None;
        }
        let usb = &mut self.standby.usb;
        if usb.polled.is_some_and(|at| at.elapsed() < POLL) {
            return None;
        }
        usb.polled = Some(Instant::now());
        self.look_at_usb(&list_devices(Direction::Input))
    }

    /// One look at `devices`: the question for a new USB-only input, or its end when the
    /// device it was about is gone (the ones waiting come next look).
    fn look_at_usb(&mut self, devices: &[DeviceInfo]) -> Option<UiEvent> {
        let names = usb_only(devices);
        let now: BTreeSet<String> = names.keys().cloned().collect();
        // The open question holds the next one back during this look, also when it ends.
        let id = self.standby.usb.look(&now, &self.settings.usb_inputs);
        let gone = self.standby.usb.pending.take_if(|id| !now.contains(id));
        if gone.is_some() {
            return Some(UiEvent::UsbAsk(String::new()));
        }
        let id = id?;
        let name = names.get(&id).cloned().unwrap_or_default();
        let text = self.tr("usbask.question", &[("name", name.clone())]);
        self.standby.usb.pending = Some(id);
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
        let Some(id) = self.standby.usb.pending.take() else {
            return;
        };
        self.update_settings(|s| {
            s.usb_inputs.insert(id, role);
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
            again.settings.usb_inputs.get("id-Guitar Link"),
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

    #[test]
    fn a_waiting_device_is_asked_after_the_open_ones_goes() {
        let (mut core, dir) = core("waiting");
        let a = device("A cable", Connection::UsbMicrophone);
        let b = device("B cable", Connection::UsbMicrophone);
        core.look_at_usb(&[]);
        assert_eq!(
            asked(core.look_at_usb(&[a, b.clone()])).as_deref(),
            Some("A cable")
        );
        // A is unplugged unanswered: B still waits for its turn.
        assert_eq!(
            core.look_at_usb(std::slice::from_ref(&b)),
            Some(UiEvent::UsbAsk(String::new()))
        );
        assert_eq!(asked(core.look_at_usb(&[b])).as_deref(), Some("B cable"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_answer_follows_the_id_not_the_translated_name() {
        let (mut core, dir) = core("id");
        let cable = device("Guitar Link", Connection::UsbMicrophone);
        core.look_at_usb(&[]);
        core.look_at_usb(std::slice::from_ref(&cable));
        core.answer_usb("instrument");

        let mut renamed = cable;
        renamed.name = "Gitarrenkabel".into();
        core.look_at_usb(&[]);
        assert_eq!(
            asked(core.look_at_usb(std::slice::from_ref(&renamed))),
            None
        );
        let list = as_answered(vec![renamed], &core.settings.usb_inputs);
        assert_eq!(list[0].connection, Connection::Usb);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_watch_waits_for_an_idle_core() {
        let (mut core, dir) = core("idle");
        assert!(core.idle());
        let take = dir.join("take.wav");
        kontra_session::write_mono_wav(&take, &vec![0.0; 48_000], 48_000).unwrap();
        core.qa.input_file = Some(take);
        core.update_settings(|s| s.output_device = Some("kontra:null".into()));
        core.start_tuner().unwrap();
        assert!(!core.idle(), "a tuner listens: no device listing");
        let _ = std::fs::remove_dir_all(dir);
    }
}
