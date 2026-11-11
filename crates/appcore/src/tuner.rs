//! The tuner: one input (and an output with "Bass hören" on a cable), nothing else open.

use crate::{Core, InputUse};
use kontra_session::LiveTuner;

impl Core {
    /// Starts the tuner; a failed input shows as the input's state (the text is for callers
    /// without one).
    pub fn start_tuner(&mut self) -> Result<(), String> {
        self.stop_live();
        self.live_ear = None;
        self.level.reset();
        self.release_standby();
        // Release the old tuner's input first: a direct device opens only once.
        self.tuner = None;
        let tuner = match LiveTuner::start(
            &self.input_choice(),
            &self.instrument(),
            self.source(),
            self.tuner_monitor(),
        ) {
            Ok(tuner) => tuner,
            Err(e) => {
                self.input.failed(InputUse::Tuner, &e);
                return Err(self.tr("error.audio", &[("reason", e.to_string())]));
            }
        };
        self.tuner = Some(tuner);
        self.input_opened(InputUse::Tuner);
        self.input_since = Some(std::time::Instant::now());
        Ok(())
    }

    /// True while the tuner listens.
    pub fn is_tuning(&self) -> bool {
        self.tuner.is_some()
    }

    pub fn stop_tuner(&mut self) {
        self.tuner = None;
        self.input.stopped(InputUse::Tuner);
    }
}
