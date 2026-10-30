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
use kontra_session::{list_devices, Direction, EngineError, InputChoice, PracticeEvent};

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

/// What the input feeds; a retry opens it for the same.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InputUse {
    Practice,
    Tuner,
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
}

impl InputStatus {
    pub(crate) fn new() -> InputStatus {
        InputStatus {
            state: InputState::Off,
            using: InputUse::Practice,
            detail: String::new(),
        }
    }

    pub(crate) fn opened(&mut self, using: InputUse, name: String) {
        self.state = InputState::Connected;
        self.using = using;
        self.detail = name;
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

    /// The input of `using` opened.
    pub(crate) fn input_opened(&mut self, using: InputUse) {
        let name = self.input_label();
        self.input.opened(using, name);
    }

    /// What the player calls the input: the device's name, the recording's file name, the
    /// MIDI keyboard.
    fn input_label(&self) -> String {
        match self.input_choice() {
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
            .opened(InputUse::Practice, "Scarlett Solo".into());
        core.input.observe(&[PracticeEvent::InputStalled]);
        assert_eq!(core.input_state(), InputState::NoSignal);
        assert_eq!(core.translate(PracticeEvent::InputStalled), None);
        assert_eq!(core.translate(PracticeEvent::InputResumed), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn connected() -> InputStatus {
        let mut status = InputStatus::new();
        status.opened(InputUse::Practice, "Scarlett Solo".into());
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
        status.opened(InputUse::Practice, "Scarlett Solo".into());
        assert_eq!(status.state, InputState::Connected);
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
        status.opened(InputUse::Tuner, "Scarlett Solo".into());
        status.stopped(InputUse::Practice);
        assert_eq!(status.state, InputState::Connected);
        status.stopped(InputUse::Tuner);
        assert_eq!(status.state, InputState::Off);
    }
}
