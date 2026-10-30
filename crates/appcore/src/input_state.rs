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
use kontra_session::{list_devices, Direction, InputChoice, PracticeEvent};

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

/// The state, what it belongs to and the input's name.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct InputStatus {
    state: InputState,
    using: InputUse,
    name: String,
}

impl InputStatus {
    pub(crate) fn new() -> InputStatus {
        InputStatus {
            state: InputState::Off,
            using: InputUse::Practice,
            name: String::new(),
        }
    }

    pub(crate) fn opened(&mut self, using: InputUse, name: String) {
        self.state = InputState::Connected;
        self.using = using;
        self.name = name;
    }

    /// `state`: `Failed` or `Denied`.
    pub(crate) fn failed(&mut self, using: InputUse, state: InputState) {
        self.state = state;
        self.using = using;
    }

    /// The input of `using` was closed; another use's state stays.
    pub(crate) fn stopped(&mut self, using: InputUse) {
        if self.using == using {
            self.state = InputState::Off;
        }
    }

    /// The device failed while open (unplugged); nothing was open: stays as it was.
    pub(crate) fn lost(mut self) -> InputStatus {
        if self.state != InputState::Off {
            self.state = InputState::Failed;
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

    /// Name of the open input ("Scarlett Solo USB", a recording's file name).
    pub fn input_name(&self) -> String {
        self.input.name.clone()
    }

    /// Opens the input again for what it fed: the tuner, or the run at the place it stood
    /// (paused; play goes on from there).
    pub fn retry_input(&mut self) -> Vec<UiEvent> {
        if self.input.using == InputUse::Tuner {
            return self
                .start_tuner()
                .err()
                .map(UiEvent::Error)
                .into_iter()
                .collect();
        }
        let at_s = self.live.as_ref().map(|_| self.position_s());
        self.stop_live();
        if let Err(e) = self.start_live() {
            return vec![UiEvent::Error(e)];
        }
        if let Some(at_s) = at_s {
            self.seek(at_s);
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
        assert_eq!(status.name, "Scarlett Solo");
    }

    // A late resume does not hide a failure.
    #[test]
    fn a_fault_stays_until_the_input_opens() {
        let mut status = connected();
        status.failed(InputUse::Practice, InputState::Failed);
        status.observe(&[PracticeEvent::InputResumed]);
        assert_eq!(status.state, InputState::Failed);
        status.opened(InputUse::Practice, "Scarlett Solo".into());
        assert_eq!(status.state, InputState::Connected);
    }

    // Unplugged while open: failed; a device error with nothing open changes nothing.
    #[test]
    fn a_lost_device_fails_only_an_open_input() {
        assert_eq!(connected().lost().state, InputState::Failed);
        assert_eq!(InputStatus::new().lost().state, InputState::Off);
    }

    // Closing the tuner ends its state, not the run's (and the other way round).
    #[test]
    fn only_its_own_use_stops_it() {
        let mut status = InputStatus::new();
        status.failed(InputUse::Tuner, InputState::Denied);
        status.stopped(InputUse::Practice);
        assert_eq!(status.state, InputState::Denied);
        status.stopped(InputUse::Tuner);
        assert_eq!(status.state, InputState::Off);
    }
}
