//! Standby monitoring (LH-EIN-18): input and output stay open while Kontra is open, so the
//! bass is heard ("Bass hören" in the sidebar) and an interface that sleeps when unused stays
//! awake.
//!
//! ```text
//! idle ──tick──► LiveMonitor (cable input only) ──practice, tuner, ear, calibration──► closed
//!   ▲                                                                                  │
//!   └──────────────────────── they end, next tick ◄────────────────────────────────────┘
//! ```
//!
//! Only the app asks for it (`keep_devices_open`): tests and recordings never open a device.

use crate::mixer::gains;
use crate::{chosen_input, Core, Settings};
use kontra_model::InputSource;
use kontra_session::{list_devices, DeviceInfo, Direction, InputChoice, LiveMonitor, LiveSetup};
use std::time::{Duration, Instant};

/// A monitor that did not open (interface unplugged) is tried again this much later.
const RETRY: Duration = Duration::from_secs(5);

/// Standby state held by `Core`.
#[derive(Debug, Default)]
pub(crate) struct Standby {
    wanted: bool,
    monitor: Option<LiveMonitor>,
    retry_at: Option<Instant>,
}

/// The input the standby may hold open, by id: the stand-in the player took for a lost input,
/// else the saved input if it is listed, else an interface (USB). Never the system's default
/// (on a laptop the built-in microphone, review #231), and none while a lost input waits for
/// the player's answer: `None` waits and looks again.
fn standby_input(
    stand_in: Option<&str>,
    lost: bool,
    saved: Option<&str>,
    devices: &[DeviceInfo],
) -> Option<String> {
    if let Some(id) = stand_in {
        return Some(id.to_string());
    }
    if lost {
        return None;
    }
    chosen_input(saved, devices)
}

/// Whether the wait for the lost input `lost` goes on after a run opened the device `opened`
/// (`None`: a recording or MIDI). It ends when Kontra itself picked another device: the player
/// plugged in another interface. With a stand-in taken it goes on: the own input may return.
fn still_lost(lost: Option<&str>, stand_in: Option<&str>, opened: Option<&str>) -> bool {
    let Some(opened) = opened else {
        return true;
    };
    stand_in.is_some() || lost == Some(opened)
}

impl Core {
    /// Holds input and output open while nothing else uses them (the app, at start).
    pub fn keep_devices_open(&mut self) {
        self.standby.wanted = true;
    }

    /// True while the standby monitor holds the devices.
    #[cfg(test)]
    pub(crate) fn is_standing_by(&self) -> bool {
        self.standby.monitor.is_some()
    }

    /// Frees the devices for practice, tuner, ear training or calibration; the next idle tick
    /// opens them again.
    pub(crate) fn release_standby(&mut self) {
        self.standby.monitor = None;
        self.standby.retry_at = None;
    }

    /// Another device or connection chosen: the standby monitor lets go, the next tick opens
    /// the new devices (owner 10.11.: the output changed only after a restart), and a stand-in
    /// for a lost input ends (the choice is the player's now).
    pub(crate) fn switch_devices(&mut self, before: &Settings) {
        let s = &self.settings;
        let input = before.input_device != s.input_device || before.input_kind != s.input_kind;
        let other = before.output_device != s.output_device || before.midi_port != s.midi_port;
        if input {
            self.fallback_input = None;
            self.lost = None;
        }
        if input || other {
            self.release_standby();
        }
    }

    /// New volumes and effects for the standby monitor.
    pub(crate) fn remix_standby(&mut self) {
        let gains = gains(&self.settings);
        if let Some(monitor) = self.standby.monitor.as_mut() {
            monitor.set_gains(gains);
        }
    }

    /// Monitoring is on, but nothing is heard between runs: no input the standby may open
    /// (none chosen and no interface, or it did not open), or devices at different rates.
    pub(crate) fn standby_hint(&self) -> Option<String> {
        if !self.settings.monitor {
            return None;
        }
        let Some(monitor) = self.standby.monitor.as_ref() else {
            return self
                .may_stand_by()
                .then(|| self.tr("hint.monitor.input", &[]));
        };
        if monitor.can_monitor() {
            return None;
        }
        Some(self.tr("hint.monitor.rate", &[]))
    }

    /// A run opened `opened`: an input other than the lost one, chosen by Kontra (no stand-in
    /// taken), ends the wait for the lost one.
    pub(crate) fn forget_lost_for(&mut self, opened: Option<&str>) {
        let lost = self.lost.as_ref().and_then(|l| l.id.as_deref());
        if self.lost.is_some() && !still_lost(lost, self.fallback_input.as_deref(), opened) {
            self.lost = None;
        }
    }

    /// Once per tick: drops a monitor whose device failed, opens one when the devices are idle.
    pub(crate) fn tick_standby(&mut self) {
        let failed = self
            .standby
            .monitor
            .as_mut()
            .is_some_and(|m| !m.poll().is_empty());
        if failed {
            self.standby.monitor = None;
            self.standby.retry_at = Some(Instant::now() + RETRY);
        }
        if self.standby.monitor.is_some() || !self.may_stand_by() {
            return;
        }
        if self.standby.retry_at.is_some_and(|at| Instant::now() < at) {
            return;
        }

        let devices = list_devices(Direction::Input);
        let saved = self.settings.input_device.as_deref();
        let stand_in = self.fallback_input.as_deref();
        let Some(input) = standby_input(stand_in, self.lost.is_some(), saved, &devices) else {
            self.standby.retry_at = Some(Instant::now() + RETRY);
            return;
        };
        let setup = self.monitor_setup(input);
        let opened = LiveMonitor::start(&setup, &self.instrument(), gains(&self.settings));
        match opened {
            Some(Ok(monitor)) => {
                self.standby.monitor = Some(monitor);
                self.standby.retry_at = None;
            }
            Some(Err(_)) | None => self.standby.retry_at = Some(Instant::now() + RETRY),
        }
    }

    /// The run's devices with `input`, without backing track.
    fn monitor_setup(&self, input: String) -> LiveSetup {
        LiveSetup {
            input: InputChoice::Device {
                id: Some(input),
                channel: None,
            },
            output_device: self.output_device(),
            source: self.source(),
            backing: None,
            original: None,
            backing_cache: None,
            onset_gate_db: self.settings.onset_gate_db,
        }
    }

    /// Asked for, set up, a cable input (a microphone is not held open: privacy, feedback) and
    /// nothing else using the devices.
    fn may_stand_by(&self) -> bool {
        let cable = matches!(self.source(), InputSource::Line | InputSource::LaptopJack);
        self.standby.wanted
            && self.settings.setup_done
            && self.qa.input_file.is_none()
            && cable
            && self.live.is_none()
            && self.tuner.is_none()
            && self.calibration.is_none()
            && self.loopback.is_none()
            && self.live_ear.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InputKind, QaOptions};
    use std::path::PathBuf;

    /// A data folder after the setup, with a cable input and the null output.
    fn data_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("kontra-standby-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        let settings = serde_json::json!({ "output_device": "kontra:null", "setup_done": true });
        let _ = std::fs::write(dir.join("settings.json"), settings.to_string());
        dir
    }

    // These tests never tick a core that may stand by: that would open the real input.

    #[test]
    fn only_the_app_asks_for_it() {
        let dir = data_dir("asked");
        let mut core = Core::new(&dir, QaOptions::default());
        assert!(!core.may_stand_by(), "tests and the CLI hold no device");
        core.keep_devices_open();
        assert!(core.may_stand_by());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_microphone_is_not_held_open() {
        let dir = data_dir("mic");
        let mut core = Core::new(&dir, QaOptions::default());
        core.keep_devices_open();
        core.update_settings(|s| s.input_kind = InputKind::Microphone);
        assert!(!core.may_stand_by());
        core.update_settings(|s| s.input_kind = InputKind::Midi);
        assert!(!core.may_stand_by());
        core.update_settings(|s| s.input_kind = InputKind::LaptopJack);
        assert!(core.may_stand_by());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn not_before_the_setup() {
        let dir = data_dir("setup");
        let mut core = Core::new(&dir, QaOptions::default());
        core.keep_devices_open();
        core.update_settings(|s| s.setup_done = false);
        assert!(!core.may_stand_by());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_recording_opens_no_device() {
        let dir = data_dir("file");
        let qa = QaOptions {
            input_file: Some(dir.join("take.wav")),
            laptop_jack: false,
        };
        let mut core = Core::new(&dir, qa);
        core.keep_devices_open();
        assert!(!core.may_stand_by());
        core.tick();
        assert!(!core.is_standing_by());
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[cfg(test)]
mod device_change {
    use super::*;
    use crate::QaOptions;

    /// Owner 10.11.: a new output applied only after a restart; the standby monitor kept the
    /// old device open. A device change frees the devices, the next tick opens the new ones.
    #[test]
    fn a_new_output_reopens_at_once() {
        let dir = std::env::temp_dir().join(format!("kontra-standby-out-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        let mut core = Core::new(&dir, QaOptions::default());
        // As after a failed open: the next try waits.
        core.standby.retry_at = Some(Instant::now() + RETRY);
        core.reconfigure(|s| s.output_device = Some("kontra:null".into()));
        assert_eq!(
            core.standby.retry_at, None,
            "the new device is tried at once"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    fn core(name: &str) -> (Core, std::path::PathBuf) {
        let dir =
            std::env::temp_dir().join(format!("kontra-standby-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        (Core::new(&dir, QaOptions::default()), dir)
    }

    /// Audit 10.11.: an input chosen while a stand-in served for a lost one was ignored until
    /// the lost one came back.
    #[test]
    fn a_chosen_input_ends_the_stand_in() {
        let (mut core, dir) = core("stand-in");
        core.fallback_input = Some("pipewire:stand-in".into());
        core.reconfigure(|s| s.input_device = Some("pipewire:chosen".into()));
        assert_eq!(core.fallback_input, None);
        assert!(core.lost.is_none());
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Audit 10.11.: switching to a microphone kept the cable monitor open (and playing).
    #[test]
    fn a_microphone_closes_the_monitor() {
        let (mut core, dir) = core("to-mic");
        core.keep_devices_open();
        core.standby.retry_at = Some(Instant::now() + RETRY);
        core.retune(|s| s.input_kind = crate::InputKind::Microphone)
            .unwrap();
        assert_eq!(core.standby.retry_at, None, "the monitor let go");
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Audit 10.11.: taking the offered stand-in kept the monitor on the lost input.
    #[test]
    fn a_stand_in_reopens_the_monitor() {
        let (mut core, dir) = core("offer");
        core.standby.retry_at = Some(Instant::now() + RETRY);
        core.use_offered_input("pipewire:stand-in");
        assert_eq!(core.standby.retry_at, None);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Review #231: "Bass hören" on, but no input the standby may open (laptop jack on
    /// "Automatic", interface unplugged): the switch was on and nothing said why it is silent.
    #[test]
    fn says_when_no_input_is_heard() {
        let (mut core, dir) = core("no-input");
        core.keep_devices_open();
        core.update_settings(|s| s.setup_done = true);
        // Not ticked: no monitor is open, as when no input can be named.
        let hint = core.remix(|s| s.monitor = true);
        assert_eq!(hint, Some(core.tr("hint.monitor.input", &[])));
        assert_eq!(core.remix(|s| s.monitor = false), None);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Only a device change frees the devices: a count-in or a scoring change keeps the bass
    /// playing without a gap.
    #[test]
    fn other_changes_keep_the_monitor() {
        let (mut core, dir) = core("keep");
        let waiting = Instant::now() + RETRY;
        core.standby.retry_at = Some(waiting);
        core.reconfigure(|s| s.count_in_bars = 2);
        assert_eq!(core.standby.retry_at, Some(waiting));
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[cfg(test)]
mod which_input {
    use super::standby_input;
    use kontra_session::{Connection, DeviceInfo, Direction};

    fn device(id: &str, connection: Connection, is_default: bool) -> DeviceInfo {
        DeviceInfo {
            id: id.into(),
            name: id.into(),
            direction: Direction::Input,
            is_default,
            connection,
            sample_rate: 48_000,
            channels: 2,
            past_pipewire: false,
        }
    }

    /// Review #231: without an interface the standby opened the system's default input, on a
    /// laptop the built-in microphone, and held it open (with "Bass hören" on: feedback).
    #[test]
    fn never_the_system_default() {
        let laptop = [device("internal-mic", Connection::Other, true)];
        assert_eq!(standby_input(None, false, None, &laptop), None);
        // The saved interface is unplugged: wait for it, not the microphone.
        assert_eq!(standby_input(None, false, Some("solo"), &laptop), None);
    }

    #[test]
    fn the_interface_or_the_saved_input() {
        let devices = [
            device("internal-mic", Connection::Other, true),
            device("solo", Connection::Usb, false),
        ];
        assert_eq!(
            standby_input(None, false, None, &devices).as_deref(),
            Some("solo")
        );
        let saved = Some("internal-mic");
        assert_eq!(
            standby_input(None, false, saved, &devices).as_deref(),
            Some("internal-mic"),
            "chosen by the player (laptop jack)"
        );
    }

    /// Review #231: the lost interface stays away and the player plugs in another one. Runs
    /// used it, but the standby waited for the old one for good.
    #[test]
    fn another_input_in_use_ends_the_wait() {
        use super::still_lost;
        assert!(!still_lost(Some("solo"), None, Some("other-usb")));
        assert!(
            still_lost(Some("solo"), None, Some("solo")),
            "not back for good yet"
        );
        assert!(
            still_lost(Some("solo"), Some("other-usb"), Some("other-usb")),
            "a stand-in: Kontra goes back to the own input when it returns"
        );
        assert!(
            still_lost(Some("solo"), None, None),
            "a recording or MIDI says nothing"
        );
    }

    /// Review #231: an input lost in a run is offered a stand-in; the standby waits for the
    /// answer and does not pick one itself.
    #[test]
    fn a_lost_input_waits_for_the_answer() {
        let devices = [
            device("internal-mic", Connection::Other, true),
            device("other-usb", Connection::Usb, false),
        ];
        assert_eq!(standby_input(None, true, Some("solo"), &devices), None);
        assert_eq!(
            standby_input(Some("other-usb"), true, Some("solo"), &devices).as_deref(),
            Some("other-usb"),
            "the stand-in the player took"
        );
    }
}
