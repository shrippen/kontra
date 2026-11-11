//! The audio devices of all modes (Runde 20: switching modes must not be noticeable). A `Rig`
//! holds the clock, the output, the input and their events; it stays open while the player
//! goes from the standby to a run, the tuner or ear training, and each mode attaches to it
//! instead of opening its own devices (closing and reopening alone left 31–42 ms of silence).
//!
//! ```text
//!  appcore ─ Rig ─┬─ clock, events
//!                 ├─ output (player, or silence)
//!                 └─ input (device, file)
//!  mode A ─attach─► rig ─into_rig─► mode B ─attach─► …
//!          clears A's sounds, settles in and out again, waits for Rebased{epoch}
//! ```

use crate::live::{input_kind, monitor_config, InputChoice, LiveOutput};
use kontra_engine::{Clock, EngineError, Event, Gains, Input, NullOutput, Output, Player};
use kontra_model::{InputSource, Instrument};
#[cfg(test)]
use std::sync::mpsc::Sender;
use std::sync::mpsc::{channel, Receiver};

/// The rate of an output that plays nothing.
const SILENT_RATE: u32 = 48_000;

/// Where a rig plays.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RigOutput {
    /// The output device (`None`: the system's default).
    Device(Option<String>),
    /// No output at all, e.g. a tuner without monitoring: nothing opens.
    Silent,
}

/// The devices a rig was opened with: a mode that wants others opens its own.
#[derive(Debug, Clone, PartialEq)]
struct Devices {
    input: InputChoice,
    output: RigOutput,
    source: InputSource,
}

/// Open audio devices, handed from mode to mode.
#[derive(Debug)]
pub struct Rig {
    devices: Devices,
    clock: Clock,
    output: LiveOutput,
    input: Input,
    events: Receiver<Event>,
    /// Lets a test report as a device would.
    #[cfg(test)]
    sender: Sender<Event>,
    /// The epoch the mode waits for; its events start after `Event::Rebased` with it.
    waiting: Option<u64>,
}

impl Rig {
    /// Opens the devices for a first mode with `instrument`, playing at `gains`. The output
    /// opens first: when it fails, no input is touched (a mode that can do without retries
    /// with `RigOutput::Silent`).
    pub fn open(
        input: &InputChoice,
        output: RigOutput,
        source: InputSource,
        instrument: &Instrument,
        gains: Gains,
    ) -> Result<Rig, EngineError> {
        let clock = Clock::new();
        let (sender, events) = channel();
        let mut player = match &output {
            RigOutput::Device(device) => {
                let config = monitor_config(input, device.as_deref());
                Some(Player::start(&config, &clock, sender.clone())?)
            }
            RigOutput::Silent => None,
        };
        if let Some(player) = player.as_mut() {
            player.set_gains(gains);
        }
        let input_device = Input::start(
            &input_kind(input),
            instrument,
            source,
            &clock,
            sender.clone(),
            player.as_mut(),
        )?;
        let output_now = match player {
            Some(player) => LiveOutput::Device(Box::new(player)),
            None => LiveOutput::Silent(NullOutput {
                sample_rate: SILENT_RATE,
            }),
        };
        Ok(Rig {
            devices: Devices {
                input: input.clone(),
                output,
                source,
            },
            clock,
            output: output_now,
            input: input_device,
            events,
            #[cfg(test)]
            sender,
            waiting: None,
        })
    }

    /// Whether this rig has the devices a mode asks for.
    pub fn fits(&self, input: &InputChoice, output: &RigOutput, source: InputSource) -> bool {
        let devices = &self.devices;
        devices.input == *input && devices.output == *output && devices.source == source
    }

    /// A new mode takes the rig over: what the last one planned stops (sounds, backing,
    /// transposing), the volumes move to `gains` (the mixer ramps them, no click), and output
    /// and input settle their starts again with the analysis for `instrument`. Events until
    /// the input confirms are the last mode's and are dropped (`poll`).
    pub fn attach(&mut self, instrument: &Instrument, gains: Gains) {
        self.output.clear_from(f64::NEG_INFINITY);
        self.output.stop_backing(f64::NEG_INFINITY);
        self.output.set_transpose(0.0);
        self.output.set_gains(gains);
        if let LiveOutput::Device(player) = &self.output {
            player.rebase();
        }
        self.waiting = self.input.rebase(instrument, self.devices.source);
    }

    /// The events since the last call that belong to the attached mode. Device errors always
    /// count: a device going away is news to whichever mode runs.
    pub fn poll(&mut self) -> Vec<Event> {
        let mut out = Vec::new();
        while let Ok(event) = self.events.try_recv() {
            match (&event, self.waiting) {
                (Event::Rebased { epoch }, Some(waiting)) if *epoch == waiting => {
                    self.waiting = None;
                }
                (Event::Error(..), _) | (_, None) => out.push(event),
                _ => {}
            }
        }
        out
    }

    /// The output stops; the rig goes on silent (a monitor whose device went away). A mode
    /// that asks for the device again gets a new rig (`fits`).
    pub fn silence_output(&mut self) {
        let rate = self.output.sample_rate();
        self.output = LiveOutput::Silent(NullOutput { sample_rate: rate });
        self.devices.output = RigOutput::Silent;
    }

    /// The player, while the output is a device.
    pub fn player(&self) -> Option<&Player> {
        match &self.output {
            LiveOutput::Device(player) => Some(player),
            LiveOutput::Silent(_) => None,
        }
    }

    /// Test: reports `event` as a device would.
    #[cfg(test)]
    pub(crate) fn tell_for_test(&self, event: Event) {
        let _ = self.sender.send(event);
    }

    pub fn clock(&self) -> &Clock {
        &self.clock
    }

    pub fn output(&mut self) -> &mut LiveOutput {
        &mut self.output
    }

    pub fn input(&self) -> &Input {
        &self.input
    }

    pub fn input_mut(&mut self) -> &mut Input {
        &mut self.input
    }

    /// True while the input's samples reach the output (a device, at the input's rate).
    pub fn is_monitoring(&self) -> bool {
        matches!(&self.output, LiveOutput::Device(p) if p.sample_rate() == self.input.sample_rate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live::{LiveTuner, TunerMonitor};
    use crate::monitor::LiveMonitor;
    use crate::practice::PracticeEvent;
    use kontra_engine::Direction;
    use kontra_model::{Pitch, A4_HZ};
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    const RATE: u32 = 48_000;
    /// Long enough to outlast every test: the file plays in real time.
    const FILE_S: f64 = 4.0;
    const PATIENCE: Duration = Duration::from_secs(3);
    const POLL_EVERY: Duration = Duration::from_millis(5);
    /// Plays nothing and opens no sound card.
    const NULL_OUTPUT: &str = "kontra:null";

    /// A file input playing `hz` (0: silence) for `FILE_S`.
    fn file_input(name: &str, hz: f64) -> (InputChoice, PathBuf) {
        let path =
            std::env::temp_dir().join(format!("kontra-rig-{name}-{}.wav", std::process::id()));
        let samples: Vec<f32> = (0..(FILE_S * f64::from(RATE)) as usize)
            .map(|i| (0.5 * (std::f64::consts::TAU * hz * i as f64 / f64::from(RATE)).sin()) as f32)
            .collect();
        crate::write_mono_wav(&path, &samples, RATE).unwrap();
        let input = InputChoice::File {
            path: path.clone(),
            laptop_jack: false,
        };
        (input, path)
    }

    fn null_output() -> RigOutput {
        RigOutput::Device(Some(NULL_OUTPUT.into()))
    }

    /// Events the last mode left behind never reach the next one; a device error does.
    #[test]
    fn the_last_modes_events_are_dropped() {
        let (input, path) = file_input("drop", 0.0);
        let bass = Instrument::default_bass();
        let mut rig = Rig::open(
            &input,
            null_output(),
            InputSource::Line,
            &bass,
            Gains::default(),
        )
        .unwrap();
        rig.tell_for_test(Event::Ended);
        rig.tell_for_test(Event::Error(Direction::Input, "gone".into()));
        rig.attach(&bass, Gains::default());
        assert!(rig.waiting.is_some(), "a running file takes the rebase");

        let mut seen = rig.poll();
        let start = Instant::now();
        while rig.waiting.is_some() && start.elapsed() < PATIENCE {
            std::thread::sleep(POLL_EVERY);
            seen.extend(rig.poll());
        }
        let _ = std::fs::remove_file(path);

        assert!(rig.waiting.is_none(), "the worker confirms the rebase");
        assert!(!seen.iter().any(|e| matches!(e, Event::Ended)), "{seen:?}");
        assert!(
            seen.iter()
                .any(|e| matches!(e, Event::Error(Direction::Input, _))),
            "{seen:?}"
        );
    }

    /// The standby hands its devices to the tuner: the tuner reads the bass on them.
    #[test]
    fn a_tuner_tunes_on_the_standbys_rig() {
        let a1 = Pitch::parse("A1").unwrap();
        let (input, path) = file_input("tune", a1.hz(A4_HZ));
        let bass = Instrument::default_bass();
        let rig = Rig::open(
            &input,
            null_output(),
            InputSource::Line,
            &bass,
            Gains::default(),
        )
        .unwrap();
        let standby = LiveMonitor::on(rig, &bass, Gains::default());
        let mut tuner = LiveTuner::on(standby.into_rig(), &bass, Gains::default());

        let mut note = None;
        let start = Instant::now();
        while note.is_none() && start.elapsed() < PATIENCE {
            std::thread::sleep(POLL_EVERY);
            note = tuner.poll().into_iter().find_map(|e| match e {
                PracticeEvent::Tuner { note, .. } => Some(note),
                _ => None,
            });
        }
        let rig = tuner.into_rig();
        let _ = std::fs::remove_file(path);

        assert_eq!(note, Some(a1.to_string()));
        assert!(rig.fits(&input, &null_output(), InputSource::Line));
    }

    /// A tuner whose monitor output does not open tunes on a silent rig; a mode that wants
    /// the output opens it anew.
    #[test]
    fn a_missing_output_leaves_the_tuner_silent() {
        let (input, path) = file_input("silent", 0.0);
        let monitor = TunerMonitor {
            output_device: Some("kontra-test:no-such-output".into()),
            gains: Gains::default(),
        };
        let bass = Instrument::default_bass();
        let tuner = LiveTuner::start(&input, &bass, InputSource::Line, Some(monitor)).unwrap();
        let rig = tuner.into_rig();
        let _ = std::fs::remove_file(path);

        assert!(rig.player().is_none());
        assert!(rig.fits(&input, &RigOutput::Silent, InputSource::Line));
    }
}
