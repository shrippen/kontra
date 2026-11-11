//! Standby monitoring (LH-EIN-18): while Kontra is open and nothing else uses the devices, the
//! bass input plays through the output, and both stay open, so an interface that sleeps when
//! unused (Scarlett Solo) stays awake.
//!
//! ```text
//! bass ─► input ─► monitor ring ─► output (gains: monitor volume and effects, or 0)
//!           └─► analysis ─► levels and errors only (notes are dropped)
//! ```

use crate::live::{InputChoice, LiveSetup};
use crate::practice::PracticeEvent;
use crate::rig::{Rig, RigOutput};
use kontra_engine::plugins::PluginError;
use kontra_engine::{EngineError, Event, Gains, Output};
use kontra_model::Instrument;

/// Input and output held open for monitoring.
#[derive(Debug)]
pub struct LiveMonitor {
    rig: Rig,
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
        // No output is an error here, not silence: the caller tries again later (review #231).
        let output = RigOutput::Device(setup.output_device.clone());
        let rig = Rig::open(&setup.input, output, setup.source, instrument, gains);
        Some(rig.map(|rig| LiveMonitor { rig }))
    }

    /// Monitors on the devices of `rig`, at `gains`.
    pub fn on(mut rig: Rig, instrument: &Instrument, gains: Gains) -> LiveMonitor {
        rig.attach(instrument, gains);
        LiveMonitor { rig }
    }

    /// Hands the devices on to the next mode.
    pub fn into_rig(self) -> Rig {
        self.rig
    }

    /// New volumes and effects; heard at once.
    pub fn set_gains(&mut self, gains: Gains) {
        self.rig.output().set_gains(gains);
    }

    /// What the effect chain delays the bass by, in ms; 0 while silent.
    pub fn chain_latency_ms(&self) -> f64 {
        self.rig.player().map_or(0.0, |p| p.chain_latency_ms())
    }

    /// Changes the plugins of the effect chain made themselves since the last call, as
    /// (plugin id, parameter id, value).
    pub fn take_param_changes(&mut self) -> Vec<(u32, u32, f64)> {
        self.rig.output().take_param_changes()
    }

    /// The chain's plugins' texts for values since the last call.
    pub fn take_param_texts(&mut self) -> Vec<(u32, u32, f64, String)> {
        self.rig.output().take_param_texts()
    }

    /// Opens the own editor of the chain's plugin `plugin`.
    pub fn open_editor(&mut self, plugin: u32, title: &str) -> Result<(), PluginError> {
        self.rig.output().open_editor(plugin, title)
    }

    /// True once the output asked for its first block (before that, nothing plays).
    pub fn is_running(&self) -> bool {
        self.rig.player().is_some_and(|p| p.is_running())
    }

    /// The input reaches the output (both at the same rate).
    pub fn can_monitor(&self) -> bool {
        self.rig.is_monitoring()
    }

    /// Errors since the last call (a device gone); everything else is dropped.
    pub fn poll(&mut self) -> Vec<PracticeEvent> {
        let mut out = Vec::new();
        for event in self.rig.poll() {
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
