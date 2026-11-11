//! Standby monitoring (LH-EIN-18): while Kontra is open and nothing else uses the devices, the
//! bass input plays through the output, and both stay open, so an interface that sleeps when
//! unused (Scarlett Solo) stays awake.
//!
//! ```text
//! bass ─► input ─► monitor ring ─► output (gains: monitor volume and effects, or 0)
//!           └─► analysis ─► levels and errors only (notes are dropped)
//! ```

use crate::live::{input_kind, live_output_config, InputChoice, LiveSetup};
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
        let config = live_output_config(setup);
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

    /// What the effect chain delays the bass by, in ms.
    pub fn chain_latency_ms(&self) -> f64 {
        self.output.chain_latency_ms()
    }

    /// Changes the plugins of the effect chain made themselves since the last call, as
    /// (plugin id, parameter id, value).
    pub fn take_param_changes(&mut self) -> Vec<(u32, u32, f64)> {
        self.output.take_param_changes()
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

    /// Hardware, not for the suite's normal run (it opens the system's own devices): Win2,
    /// 10.11.2026, Scarlett Solo under Windows 11. Input 48 kHz and output 44.1 kHz gave a
    /// monitor that could not play the bass.
    #[test]
    #[ignore = "opens the real audio devices; by hand on the test machine: -- --ignored"]
    fn the_default_devices_can_monitor() {
        let setup = LiveSetup {
            input: InputChoice::Device {
                id: None,
                channel: None,
            },
            output_device: None,
            source: InputSource::Line,
            backing: None,
            original: None,
            backing_cache: None,
            onset_gate_db: None,
        };
        let started = LiveMonitor::start(&setup, &Instrument::default_bass(), Gains::default());
        let monitor = started.unwrap().unwrap();
        assert!(
            monitor.can_monitor(),
            "input and output run at different rates"
        );
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
