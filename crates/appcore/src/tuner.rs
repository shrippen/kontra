//! The tuner: one input (and an output with "Bass hören" on a cable), nothing else open.

use crate::mixer::gains;
use crate::{Core, InputUse};
use kontra_session::{LiveSetup, LiveTuner};

impl Core {
    /// Starts the tuner; a failed input shows as the input's state (the text is for callers
    /// without one).
    pub fn start_tuner(&mut self) -> Result<(), String> {
        self.stop_live();
        self.end_ear();
        self.level.reset();
        self.release_standby();
        // The old tuner's devices go on to this one (or close first: a direct device opens
        // only once).
        self.end_tuner();
        let setup = self.tuner_setup();
        let started = match self.take_rig(&setup) {
            // The bass passes through only with "Bass hören" (the gains say so).
            Some(rig) => Ok(LiveTuner::on(
                rig,
                &self.instrument(),
                gains(&self.settings, &self.plugins),
            )),
            None => LiveTuner::start(
                &setup.input,
                &self.instrument(),
                setup.source,
                self.tuner_monitor(),
            ),
        };
        let tuner = match started {
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

    /// The devices the tuner asks for: a parked rig with them goes on to it.
    fn tuner_setup(&self) -> LiveSetup {
        LiveSetup {
            input: self.input_choice(),
            output_device: self.output_device(),
            source: self.source(),
            backing: None,
            original: None,
            backing_cache: None,
            onset_gate_db: None,
        }
    }

    /// True while the tuner listens.
    pub fn is_tuning(&self) -> bool {
        self.tuner.is_some()
    }

    /// Lays the loaded piece out on the settings' strings again and restarts a tuner that ran.
    pub(crate) fn relayout(&mut self) -> Result<(), String> {
        let tuner_running = self.tuner.is_some();
        // The tuner takes the new strings even when the piece cannot be laid out again.
        let piece = match self.loaded.as_ref().map(|l| l.track) {
            Some(track) => self.reload(track),
            None => Ok(()),
        };
        if !tuner_running {
            return piece;
        }

        // Both failures are told, the piece's first, in the one text the note bar has.
        match (piece, self.start_tuner()) {
            (Err(piece), Err(tuner)) => Err(format!("{piece}\n{tuner}")),
            (piece, tuner) => piece.and(tuner),
        }
    }

    pub fn stop_tuner(&mut self) {
        self.end_tuner();
        self.input.stopped(InputUse::Tuner);
    }
}
