//! Standby monitoring (LH-EIN-18): while Kontra is open and nothing else uses the devices, the
//! bass input plays through the output, and both stay open, so an interface that sleeps when
//! unused (Scarlett Solo) stays awake.
//!
//! ```text
//! bass ─► input ─► monitor ring ─► output (gains: monitor volume and effects, or 0)
//!           └─► analysis ─► levels and errors only (notes are dropped)
//! ```

use crate::live::{input_kind, output_config, InputChoice, LiveSetup};
use crate::practice::PracticeEvent;
use kontra_engine::{Clock, EngineError, Event, Gains, Input, Output, Player};
use kontra_model::Instrument;
use std::sync::mpsc::{channel, Receiver};

/// Input and output held open for monitoring.
#[derive(Debug)]
pub struct LiveMonitor {
    output: Player,
    /// Held, not read: dropping it closes the input.
    _input: Input,
    events: Receiver<Event>,
    can_monitor: bool,
}

impl LiveMonitor {
    /// Opens the devices of `setup` and plays the input at `gains`. `None` for an input that
    /// is no device: a recording or MIDI has nothing to keep awake.
    pub fn start(
        setup: &LiveSetup,
        instrument: &Instrument,
        gains: Gains,
    ) -> Option<Result<LiveMonitor, EngineError>> {
        if !matches!(setup.input, InputChoice::Device { .. }) {
            return None;
        }
        Some(LiveMonitor::open(setup, instrument, gains))
    }

    fn open(
        setup: &LiveSetup,
        instrument: &Instrument,
        gains: Gains,
    ) -> Result<LiveMonitor, EngineError> {
        let clock = Clock::new();
        let (tx, rx) = channel();
        // No output is an error here, not silence: the caller tries again later (review #231).
        let config = output_config(setup.output_device.as_deref());
        let mut output = Player::start(&config, &clock, tx.clone())?;
        output.set_gains(gains);
        let sample_rate = output.sample_rate();
        let input = Input::start(
            &input_kind(&setup.input),
            instrument,
            setup.source,
            &clock,
            tx,
            Some(&mut output),
        )?;
        let can_monitor = input.sample_rate == sample_rate;
        Ok(LiveMonitor {
            output,
            _input: input,
            events: rx,
            can_monitor,
        })
    }

    /// New volumes and effects; heard at once.
    pub fn set_gains(&mut self, gains: Gains) {
        self.output.set_gains(gains);
    }

    /// The input reaches the output (both at the same rate).
    pub fn can_monitor(&self) -> bool {
        self.can_monitor
    }

    /// Errors since the last call (a device gone); everything else is dropped.
    pub fn poll(&mut self) -> Vec<PracticeEvent> {
        let mut out = Vec::new();
        while let Ok(event) = self.events.try_recv() {
            match event {
                Event::Error(side, m) => out.push(PracticeEvent::Error(side, m)),
                Event::Ended => out.push(PracticeEvent::InputEnded),
                _ => {}
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kontra_model::InputSource;

    /// Review #231: an output that does not open is an error the caller retries, not a silent
    /// monitor. The output opens first: no input is touched here.
    #[test]
    fn a_missing_output_is_an_error() {
        let setup = LiveSetup {
            input: InputChoice::Device {
                id: Some("kontra-test:no-such-input".into()),
                channel: None,
            },
            output_device: Some("kontra-test:no-such-output".into()),
            source: InputSource::Line,
            backing: None,
            original: None,
            backing_cache: None,
            onset_gate_db: None,
        };
        let started = LiveMonitor::start(&setup, &Instrument::default_bass(), Gains::default());
        assert!(matches!(started, Some(Err(_))), "{started:?}");
    }

    #[test]
    fn a_recording_is_not_monitored() {
        let setup = LiveSetup {
            input: InputChoice::File {
                path: "take.wav".into(),
                laptop_jack: false,
            },
            output_device: None,
            source: InputSource::Line,
            backing: None,
            original: None,
            backing_cache: None,
            onset_gate_db: None,
        };
        let started = LiveMonitor::start(&setup, &Instrument::default_bass(), Gains::default());
        assert!(started.is_none());
    }
}
