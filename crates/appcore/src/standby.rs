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
use crate::{Core, Settings};
use kontra_model::InputSource;
use kontra_session::{LiveMonitor, LiveSetup};
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

impl Core {
    /// Holds input and output open while nothing else uses them (the app, at start).
    pub fn keep_devices_open(&mut self) {
        self.standby.wanted = true;
    }

    /// True while the standby monitor holds the devices.
    pub fn is_standing_by(&self) -> bool {
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

    /// Monitoring is on, but the standby devices run at different rates.
    pub(crate) fn standby_hint(&self) -> Option<String> {
        let monitor = self.standby.monitor.as_ref()?;
        if !self.settings.monitor || monitor.can_monitor() {
            return None;
        }
        Some(self.tr("hint.monitor.rate", &[]))
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

        let setup = self.monitor_setup();
        let opened = LiveMonitor::start(&setup, &self.instrument(), gains(&self.settings));
        match opened {
            Some(Ok(monitor)) => {
                self.standby.monitor = Some(monitor);
                self.standby.retry_at = None;
            }
            Some(Err(_)) | None => self.standby.retry_at = Some(Instant::now() + RETRY),
        }
    }

    /// The run's devices, without backing track.
    fn monitor_setup(&self) -> LiveSetup {
        LiveSetup {
            input: self.input_choice(),
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
