//! What the input does (LH-EIN-15), shown as long as it lasts, with "Erneut versuchen".
//!
//! ```text
//!        opened            Stalled             Resumed
//!  Off ─────────► Connected ───────► NoSignal ───────► Connected
//!   │ start fails: Failed or Denied (refused by the system)
//!   │ device error while running: Failed
//!   └◄── input of the same use stopped          retry: open it again (a run keeps its place)
//! ```

use crate::{Core, UiEvent};
use kontra_session::{
    list_devices, DeviceInfo, Direction, EngineError, InputChoice, PracticeEvent,
};

/// The input's state for the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputState {
    /// No input open: nothing to show.
    Off,
    Connected,
    /// The device stopped delivering; judging pauses until it does again.
    NoSignal,
    /// The input did not open or failed while running.
    Failed,
    /// The system refused the device (permission).
    Denied,
}

impl InputState {
    /// The key the UI words it with (`input.state.<key>`).
    pub fn key(self) -> &'static str {
        match self {
            InputState::Off => "off",
            InputState::Connected => "connected",
            InputState::NoSignal => "nosignal",
            InputState::Failed => "failed",
            InputState::Denied => "denied",
        }
    }
}

/// Which side really failed (owner, review #213: "Eingang nachprüfen"). An interface that is
/// input and output may report its output first; if the input device is no longer listed,
/// the input is lost too. Recordings and MIDI cannot vanish this way: only for a device are
/// the `inputs` listed.
pub(crate) fn lost_side(
    side: Direction,
    input: Option<&InputChoice>,
    inputs: impl FnOnce() -> Vec<DeviceInfo>,
) -> Direction {
    let Some(InputChoice::Device { id, .. }) = input else {
        return side;
    };
    if side == Direction::Output && !crate::unplug::is_back(&inputs(), id.as_deref()) {
        return Direction::Input;
    }
    side
}

/// What the input feeds; a retry opens it for the same.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InputUse {
    Practice,
    Tuner,
    /// Ear training: its input is remembered for a loss, but it shows no state.
    Ear,
}

/// Why a run did not start: its input (the state shows it) or anything else (a message).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Unstarted {
    Input,
    Other(String),
}

/// The state, what it belongs to and its detail: the input's name while connected, the
/// system's reason after a fault.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct InputStatus {
    state: InputState,
    using: InputUse,
    detail: String,
    /// The input as it was opened (the device resolved then, not the choice now) and by whom;
    /// gone when that use stops, so a later loss cannot blame an old device.
    opened: Option<(InputUse, InputChoice)>,
}

impl InputStatus {
    pub(crate) fn new() -> InputStatus {
        InputStatus {
            state: InputState::Off,
            using: InputUse::Practice,
            detail: String::new(),
            opened: None,
        }
    }

    pub(crate) fn opened(&mut self, using: InputUse, name: String, choice: InputChoice) {
        self.state = InputState::Connected;
        self.using = using;
        self.detail = name;
        self.opened = Some((using, choice));
    }

    /// Ear training opened `choice`: kept for a loss; no state to show.
    pub(crate) fn remember(&mut self, using: InputUse, choice: InputChoice) {
        self.opened = Some((using, choice));
    }

    /// The input as opened, if one is.
    pub(crate) fn opened_input(&self) -> Option<&InputChoice> {
        self.opened.as_ref().map(|(_, choice)| choice)
    }

    /// The input of `using` did not open: refused by the system, or any other fault.
    pub(crate) fn failed(&mut self, using: InputUse, error: &EngineError) {
        self.state = if error.is_denied() {
            InputState::Denied
        } else {
            InputState::Failed
        };
        self.using = using;
        self.detail = error.to_string();
    }

    pub(crate) fn state(&self) -> InputState {
        self.state
    }

    /// The input of `using` was closed; another use's state stays.
    pub(crate) fn stopped(&mut self, using: InputUse) {
        if self.using == using {
            self.state = InputState::Off;
        }
        if self.opened.as_ref().is_some_and(|(by, _)| *by == using) {
            self.opened = None;
        }
    }

    /// The device failed while open (unplugged); nothing was open: stays as it was.
    pub(crate) fn lost(mut self, reason: &str) -> InputStatus {
        if self.state != InputState::Off {
            self.state = InputState::Failed;
            self.detail = reason.to_string();
        }
        self
    }

    /// Follows the engine's watch: no data for 250 ms, then data again.
    pub(crate) fn observe(&mut self, events: &[PracticeEvent]) {
        for event in events {
            self.state = match (event, self.state) {
                (PracticeEvent::InputStalled, InputState::Connected) => InputState::NoSignal,
                (PracticeEvent::InputResumed, InputState::NoSignal) => InputState::Connected,
                (_, state) => state,
            };
        }
    }
}

impl Core {
    pub fn input_state(&self) -> InputState {
        self.input.state
    }

    /// While connected the input's name ("Scarlett Solo USB", a recording's file name); after
    /// a fault the system's reason.
    pub fn input_detail(&self) -> String {
        self.input.detail.clone()
    }

    /// Opens the input again for what it fed: the tuner, or the run, which goes on where it
    /// stands (one run, a probe stays a probe). A fault shows as the state, not as a message.
    pub fn retry_input(&mut self) -> Vec<UiEvent> {
        if self.input.using == InputUse::Tuner {
            // Only the input can fail here; the state says so.
            let _ = self.start_tuner();
            return Vec::new();
        }
        if self.live.is_none() {
            return match self.start_live() {
                Err(Unstarted::Other(text)) => vec![UiEvent::Error(text)],
                Ok(_) | Err(Unstarted::Input) => Vec::new(),
            };
        }
        let setup = self.live_setup();
        let gains = crate::gains(&self.settings);
        let Some(live) = self.live.as_mut() else {
            return Vec::new();
        };
        match live.reopen(&setup) {
            Ok(()) => {
                live.practice_mut().set_gains(gains);
                self.input_opened(InputUse::Practice);
            }
            Err(e) => {
                // Without an input the run waits instead of counting the notes as missed.
                live.practice_mut().pause();
                self.input.failed(InputUse::Practice, &e);
            }
        }
        Vec::new()
    }

    /// A device failed while running: everything stops. A lost input shows as the input's
    /// state; a lost output, or an input without a state (ear training), as a message.
    pub(crate) fn device_lost(&mut self, side: Direction, reason: String) -> Vec<UiEvent> {
        self.device_lost_in(side, reason, || list_devices(Direction::Input))
    }

    /// `device_lost` with the input devices listed by `inputs` (tests list their own).
    fn device_lost_in(
        &mut self,
        side: Direction,
        reason: String,
        inputs: impl Fn() -> Vec<DeviceInfo>,
    ) -> Vec<UiEvent> {
        // The input as opened: the choice resolved now would pick another device (review #213).
        let side = lost_side(side, self.input.opened_input(), &inputs);
        let input = self.input.clone();
        let opened = input.opened_input().cloned();
        self.stop_live();
        self.tuner = None;
        self.live_ear = None;
        if side == Direction::Input {
            self.input = input.lost(&reason);
        }
        let mut events = Vec::new();
        if self.input.state() == InputState::Off {
            events.push(UiEvent::Error(
                self.tr("error.device.lost", &[("reason", reason)]),
            ));
        }
        // Another input stands in only for a lost input (LH-ALL-04).
        if side == Direction::Input {
            events.extend(self.input_lost_in(opened.as_ref(), inputs()));
        }
        events
    }

    /// The input of `using` opened.
    pub(crate) fn input_opened(&mut self, using: InputUse) {
        let choice = self.input_choice();
        let name = self.input_label(&choice);
        self.input.opened(using, name, choice);
    }

    /// What the player calls the input: the device's name, the recording's file name, the
    /// MIDI keyboard.
    fn input_label(&self, choice: &InputChoice) -> String {
        match choice.clone() {
            InputChoice::File { path, .. } => path
                .file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into()),
            InputChoice::Midi { .. } => self.midi_port_name(),
            InputChoice::Device { id, .. } => list_devices(Direction::Input)
                .into_iter()
                .find(|d| id.as_ref().map_or(d.is_default, |id| *id == d.id))
                .map_or_else(String::new, |d| d.name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::QaOptions;

    // The state replaces the passing message: "no signal" stays while the device is silent
    // instead of being overwritten by the next hint.
    #[test]
    fn a_stall_shows_as_the_state() {
        let dir = std::env::temp_dir().join(format!("kontra-input-state-{}", std::process::id()));
        let mut core = Core::new(&dir, QaOptions::default());
        core.input
            .opened(InputUse::Practice, "Scarlett Solo".into(), scarlett());
        core.input.observe(&[PracticeEvent::InputStalled]);
        assert_eq!(core.input_state(), InputState::NoSignal);
        assert_eq!(core.translate(PracticeEvent::InputStalled), None);
        assert_eq!(core.translate(PracticeEvent::InputResumed), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn connected() -> InputStatus {
        let mut status = InputStatus::new();
        status.opened(InputUse::Practice, "Scarlett Solo".into(), scarlett());
        status
    }

    #[test]
    fn follows_a_stall_and_its_end() {
        let mut status = connected();
        status.observe(&[PracticeEvent::InputStalled]);
        assert_eq!(status.state, InputState::NoSignal);
        status.observe(&[PracticeEvent::InputResumed]);
        assert_eq!(status.state, InputState::Connected);
        assert_eq!(status.detail, "Scarlett Solo");
    }

    // A late resume does not hide a failure.
    #[test]
    fn a_fault_stays_until_the_input_opens() {
        let mut status = connected();
        status = status.lost("unplugged");
        status.observe(&[PracticeEvent::InputResumed]);
        assert_eq!(status.state, InputState::Failed);
        status.opened(InputUse::Practice, "Scarlett Solo".into(), scarlett());
        assert_eq!(status.state, InputState::Connected);
    }

    // Review #213: headphones (the output) dropping out are no input fault; the message says
    // the device is gone.
    #[test]
    fn a_lost_output_is_no_input_fault() {
        let dir = std::env::temp_dir().join(format!("kontra-output-lost-{}", std::process::id()));
        let mut core = core_on_scarlett(&dir);
        let both = || vec![device("laptop", true), device("scarlett", false)];
        let events = core.device_lost_in(Direction::Output, "headphones gone".into(), both);
        assert_ne!(core.input_state(), InputState::Failed);
        assert!(
            matches!(events.as_slice(), [UiEvent::Error(text)] if text.contains("headphones gone")),
            "{events:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn device(id: &str, is_default: bool) -> DeviceInfo {
        DeviceInfo {
            id: id.into(),
            name: id.into(),
            direction: Direction::Input,
            is_default,
            connection: kontra_session::Connection::Usb,
            sample_rate: 48_000,
            channels: 2,
            past_pipewire: false,
        }
    }

    fn scarlett() -> InputChoice {
        InputChoice::Device {
            id: Some("scarlett".into()),
            channel: None,
        }
    }

    /// A core fed by a recording (no real device anywhere) that opened the Scarlett.
    fn core_on_scarlett(dir: &std::path::Path) -> Core {
        let qa = QaOptions {
            input_file: Some(dir.join("take.wav")),
            laptop_jack: false,
        };
        let mut core = Core::new(dir, qa);
        core.input
            .opened(InputUse::Practice, "Scarlett Solo".into(), scarlett());
        core
    }

    // Scarlett as input and output, unplugged: its output reports first; the Scarlett that was
    // opened is gone from the list (the laptop's microphone, the default, is still there).
    // Review #213 round 4: the choice resolved anew fell back to the laptop.
    #[test]
    fn an_output_loss_checks_the_opened_input() {
        let dir = std::env::temp_dir().join(format!("kontra-output-check-{}", std::process::id()));
        let mut core = core_on_scarlett(&dir);
        core.device_lost_in(Direction::Output, "gone".into(), || {
            vec![device("laptop", true)]
        });
        assert_eq!(core.input_state(), InputState::Failed);

        let mut core = core_on_scarlett(&dir);
        let both = || vec![device("laptop", true), device("scarlett", false)];
        core.device_lost_in(Direction::Output, "headphones gone".into(), both);
        assert_ne!(core.input_state(), InputState::Failed);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// No recording: the lost input is watched (LH-ALL-04). Devices come from `inputs` only.
    fn core_on(dir: &std::path::Path, using: InputUse) -> Core {
        let mut core = Core::new(dir, QaOptions::default());
        core.input.remember(using, scarlett());
        core
    }

    // Review #213 round 5: ear training opens the input too. Its output reporting first must
    // still find the Scarlett gone.
    #[test]
    fn ear_training_remembers_its_input() {
        let dir = std::env::temp_dir().join(format!("kontra-ear-input-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let silent = serde_json::json!({ "output_device": "kontra:null" });
        std::fs::write(dir.join("settings.json"), silent.to_string()).unwrap();
        let take = dir.join("take.wav");
        kontra_session::write_mono_wav(&take, &[0.0; 4800], 48_000).unwrap();
        let qa = QaOptions {
            input_file: Some(take.clone()),
            laptop_jack: false,
        };
        let mut core = Core::new(&dir, qa);
        core.ear_start("echo").unwrap();
        assert!(
            matches!(core.input.opened_input(), Some(InputChoice::File { path, .. }) if *path == take),
            "{:?}",
            core.input.opened_input()
        );
        core.ear_stop();
        assert_eq!(core.input.opened_input(), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // Review #213 round 5 (Bestand): an auto-picked Scarlett (none saved) was watched as "the
    // default input", so the laptop's microphone counted as "back" after 2 s.
    #[test]
    fn a_lost_input_watches_the_opened_device() {
        let dir = std::env::temp_dir().join(format!("kontra-lost-watch-{}", std::process::id()));
        for using in [InputUse::Practice, InputUse::Ear] {
            let mut core = core_on(&dir, using);
            core.device_lost_in(Direction::Output, "gone".into(), || {
                vec![device("laptop", true)]
            });
            let lost = core.lost.as_ref().map(|l| l.id.clone());
            assert_eq!(lost, Some(Some("scarlett".into())), "{using:?}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    // The default input: lost only when no default input is listed. A recording cannot
    // vanish with the output, and nothing is listed for it.
    #[test]
    fn an_output_loss_lists_only_for_a_device() {
        let default = InputChoice::Device {
            id: None,
            channel: None,
        };
        let laptop = || vec![device("laptop", true)];
        assert_eq!(
            lost_side(Direction::Output, Some(&default), laptop),
            Direction::Output
        );
        assert_eq!(
            lost_side(Direction::Output, Some(&default), Vec::new),
            Direction::Input
        );
        let file = InputChoice::File {
            path: "take.wav".into(),
            laptop_jack: false,
        };
        let unlisted = || -> Vec<DeviceInfo> { panic!("listed devices for a recording") };
        assert_eq!(
            lost_side(Direction::Output, Some(&file), unlisted),
            Direction::Output
        );
        assert_eq!(
            lost_side(Direction::Output, None, unlisted),
            Direction::Output
        );
    }

    // Unplugged while open: failed; a device error with nothing open changes nothing.
    #[test]
    fn a_lost_device_fails_only_an_open_input() {
        let lost = connected().lost("unplugged");
        assert_eq!(
            (lost.state, lost.detail.as_str()),
            (InputState::Failed, "unplugged")
        );
        assert_eq!(InputStatus::new().lost("unplugged").state, InputState::Off);
    }

    // Closing the tuner ends its state, not the run's (and the other way round).
    #[test]
    fn only_its_own_use_stops_it() {
        let mut status = InputStatus::new();
        status.opened(InputUse::Tuner, "Scarlett Solo".into(), scarlett());
        status.stopped(InputUse::Practice);
        assert_eq!(status.state, InputState::Connected);
        status.stopped(InputUse::Tuner);
        assert_eq!(status.state, InputState::Off);
    }
}
