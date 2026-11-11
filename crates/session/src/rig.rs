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
use std::sync::mpsc::{channel, Receiver, Sender};

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

/// What a rig does when its output device does not open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnOutputError {
    /// Fails; nothing opens (the standby retries later).
    Fail,
    /// Opens the input with a silent output: tuning or playing matters more.
    PlaySilent,
}

/// What the input last said about itself; a new mode hears it again (the device says it once).
#[derive(Debug, Clone, Copy, PartialEq)]
enum InputState {
    Running,
    /// Nothing delivered since `at_s` (`Event::Stalled`).
    Stalled(f64),
    /// A file reached its end (`Event::Ended`): no rebase follows.
    Ended,
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
    /// `None` after a reopen whose input did not open.
    input: Option<Input>,
    events: Receiver<Event>,
    /// For reopened devices: they report into the same channel.
    sender: Sender<Event>,
    /// The epoch the mode waits for; its events start after `Event::Rebased` with it.
    waiting: Option<u64>,
    state: InputState,
    /// A stall or end the last mode took, told again to the attached one.
    retell: Option<InputState>,
}

impl Rig {
    /// Opens the devices for a first mode with `instrument`, playing at `gains`. The output
    /// opens first: when it fails, `on_error` decides; with `Fail` no input is touched.
    pub fn open(
        input: &InputChoice,
        output: RigOutput,
        on_error: OnOutputError,
        source: InputSource,
        instrument: &Instrument,
        gains: Gains,
    ) -> Result<Rig, EngineError> {
        let clock = Clock::new();
        let (sender, events) = channel();
        let (mut player, output) = match start_player(input, &output, &clock, &sender) {
            Ok(player) => (player, output),
            Err(e) if on_error == OnOutputError::Fail => return Err(e),
            Err(_) => (None, RigOutput::Silent),
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
        Ok(Rig {
            devices: Devices {
                input: input.clone(),
                output,
                source,
            },
            clock,
            output: live_output(player),
            input: Some(input_device),
            events,
            sender,
            waiting: None,
            state: InputState::Running,
            retell: None,
        })
    }

    /// Opens input and output again on the same clock and channel (LH-EIN-15, "Erneut
    /// versuchen"): the old input closes first (an exclusive device would be busy), then the
    /// output (silent if it does not open: the input matters more), then the input for
    /// `instrument`. An input that fails leaves the rig without one (`fits` says no).
    pub fn reopen(
        &mut self,
        input: &InputChoice,
        output: RigOutput,
        source: InputSource,
        instrument: &Instrument,
    ) -> Result<(), EngineError> {
        self.input = None;
        let (mut player, output) = match start_player(input, &output, &self.clock, &self.sender) {
            Ok(player) => (player, output),
            Err(_) => (None, RigOutput::Silent),
        };
        let started = Input::start(
            &input_kind(input),
            instrument,
            source,
            &self.clock,
            self.sender.clone(),
            player.as_mut(),
        );
        self.output = live_output(player);
        self.devices = Devices {
            input: input.clone(),
            output,
            source,
        };
        self.waiting = None;
        self.state = InputState::Running;
        self.retell = None;
        self.input = Some(started?);
        Ok(())
    }

    /// Whether this rig has the devices a mode asks for.
    /// Not once its input is gone or ended: the next mode opens anew.
    pub fn fits(&self, input: &InputChoice, output: &RigOutput, source: InputSource) -> bool {
        let devices = &self.devices;
        let alive = self.input.is_some() && self.state != InputState::Ended;
        alive && devices.input == *input && devices.output == *output && devices.source == source
    }

    /// A new mode takes the rig over: what the last one planned stops (sounds, backing,
    /// transposing), the volumes move to `gains` (the mixer ramps them, no click), and output
    /// and input settle their starts again with the analysis for `instrument`. Events until
    /// the input confirms are the last mode's and are dropped (`poll`); a stall still going on
    /// is told again.
    pub fn attach(&mut self, instrument: &Instrument, gains: Gains) {
        self.output.clear_from(f64::NEG_INFINITY);
        self.output.stop_backing(f64::NEG_INFINITY);
        self.output.set_transpose(0.0);
        self.output.set_gains(gains);
        if let LiveOutput::Device(player) = &self.output {
            player.rebase();
        }
        let source = self.devices.source;
        self.waiting = self
            .input
            .as_mut()
            .and_then(|input| input.rebase(instrument, source));
        self.retell = Some(self.state).filter(|state| *state != InputState::Running);
    }

    /// The events since the last call that belong to the attached mode. What the device says
    /// (gone, stalled, back, ended) always counts: it is news to whichever mode runs. An end
    /// also ends the wait: no rebase follows it.
    pub fn poll(&mut self) -> Vec<Event> {
        let mut out = Vec::new();
        match self.retell.take() {
            Some(InputState::Stalled(at_s)) => out.push(Event::Stalled { at_s }),
            Some(InputState::Ended) => out.push(Event::Ended),
            Some(InputState::Running) | None => {}
        }
        while let Ok(event) = self.events.try_recv() {
            match &event {
                Event::Stalled { at_s } => self.state = InputState::Stalled(*at_s),
                Event::Resumed { .. } => self.state = InputState::Running,
                Event::Ended => {
                    self.state = InputState::Ended;
                    self.waiting = None;
                }
                _ => {}
            }
            match (&event, self.waiting) {
                (Event::Rebased { epoch }, Some(waiting)) if *epoch == waiting => {
                    self.waiting = None;
                }
                (
                    Event::Error(..) | Event::Stalled { .. } | Event::Resumed { .. } | Event::Ended,
                    _,
                )
                | (_, None) => out.push(event),
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

    /// A mode that plays on its own (a run) takes the output; the rig holds silence meanwhile.
    pub fn take_output(&mut self) -> LiveOutput {
        let rate = self.output.sample_rate();
        std::mem::replace(
            &mut self.output,
            LiveOutput::Silent(NullOutput { sample_rate: rate }),
        )
    }

    /// The output a mode took comes back.
    pub fn put_output(&mut self, output: LiveOutput) {
        self.output = output;
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

    pub fn input(&self) -> Option<&Input> {
        self.input.as_ref()
    }

    pub fn input_mut(&mut self) -> Option<&mut Input> {
        self.input.as_mut()
    }

    /// True while the input's samples reach the output (a device, at the input's rate).
    pub fn is_monitoring(&self) -> bool {
        let Some(input) = &self.input else {
            return false;
        };
        matches!(&self.output, LiveOutput::Device(p) if p.sample_rate() == input.sample_rate)
    }
}

/// The player for `output`; `None` for silence.
fn start_player(
    input: &InputChoice,
    output: &RigOutput,
    clock: &Clock,
    sender: &Sender<Event>,
) -> Result<Option<Player>, EngineError> {
    let RigOutput::Device(device) = output else {
        return Ok(None);
    };
    let config = monitor_config(input, device.as_deref());
    Player::start(&config, clock, sender.clone()).map(Some)
}

fn live_output(player: Option<Player>) -> LiveOutput {
    match player {
        Some(player) => LiveOutput::Device(Box::new(player)),
        None => LiveOutput::Silent(NullOutput {
            sample_rate: SILENT_RATE,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live::{Live, LiveTuner, TunerMonitor};
    use crate::monitor::LiveMonitor;
    use crate::practice::PracticeEvent;
    use crate::{Mode, PracticeOptions, ScoreSettings};
    use kontra_engine::Direction;
    use kontra_model::{Arrangement, Note, TempoMap, TICKS_PER_QUARTER};
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
    /// How long the mode before ran on the rig.
    const STANDBY: Duration = Duration::from_millis(1500);
    /// One bar of 4/4 at 120 bpm.
    const COUNT_IN_S: f64 = 2.0;
    /// Time between starting and reading the position, with room for a slow machine.
    const TOLERANCE_S: f64 = 0.2;

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

    /// A time no real event of these short files carries.
    const OLD_S: f64 = -7.0;

    /// Polls until the input confirms the rebase (or patience ends).
    fn poll_until_rebased(rig: &mut Rig) -> Vec<Event> {
        let mut seen = rig.poll();
        let start = Instant::now();
        while rig.waiting.is_some() && start.elapsed() < PATIENCE {
            std::thread::sleep(POLL_EVERY);
            seen.extend(rig.poll());
        }
        seen
    }

    fn null_output() -> RigOutput {
        RigOutput::Device(Some(NULL_OUTPUT.into()))
    }

    /// A mode's own readings from before never reach the next one.
    #[test]
    fn the_last_modes_readings_are_dropped() {
        let (input, path) = file_input("drop", 0.0);
        let bass = Instrument::default_bass();
        let mut rig = Rig::open(
            &input,
            null_output(),
            OnOutputError::Fail,
            InputSource::Line,
            &bass,
            Gains::default(),
        )
        .unwrap();
        rig.tell_for_test(Event::Heard { at_s: OLD_S });
        rig.attach(&bass, Gains::default());
        assert!(rig.waiting.is_some(), "a running file takes the rebase");

        let seen = poll_until_rebased(&mut rig);
        let _ = std::fs::remove_file(path);

        assert!(rig.waiting.is_none(), "the worker confirms the rebase");
        assert!(
            !seen
                .iter()
                .any(|e| matches!(e, Event::Heard { at_s } if *at_s == OLD_S)),
            "{seen:?}"
        );
    }

    /// Review #287: what the device says (gone, stalled, back, ended) is news to whichever
    /// mode runs; an end also ends the wait, as no rebase follows it.
    #[test]
    fn device_states_reach_the_next_mode() {
        let (input, path) = file_input("states", 0.0);
        let bass = Instrument::default_bass();
        let mut rig = Rig::open(
            &input,
            null_output(),
            OnOutputError::Fail,
            InputSource::Line,
            &bass,
            Gains::default(),
        )
        .unwrap();
        rig.tell_for_test(Event::Error(Direction::Input, "gone".into()));
        rig.tell_for_test(Event::Resumed { at_s: OLD_S });
        rig.tell_for_test(Event::Ended);
        rig.attach(&bass, Gains::default());

        let seen = rig.poll();
        let _ = std::fs::remove_file(path);

        assert!(
            seen.iter().any(|e| matches!(e, Event::Error(..))),
            "{seen:?}"
        );
        assert!(
            seen.iter().any(|e| matches!(e, Event::Resumed { .. })),
            "{seen:?}"
        );
        assert!(seen.iter().any(|e| matches!(e, Event::Ended)), "{seen:?}");
        assert!(rig.waiting.is_none(), "no rebase after the end");
    }

    /// Review #287: the device tells a stall once. The last mode took it; the next one hears
    /// it too, and only once.
    #[test]
    fn a_stall_is_told_to_the_next_mode() {
        let (input, path) = file_input("stall", 0.0);
        let bass = Instrument::default_bass();
        let mut rig = Rig::open(
            &input,
            null_output(),
            OnOutputError::Fail,
            InputSource::Line,
            &bass,
            Gains::default(),
        )
        .unwrap();
        rig.tell_for_test(Event::Stalled { at_s: OLD_S });
        let old_mode = rig.poll();
        assert!(old_mode.iter().any(|e| matches!(e, Event::Stalled { .. })));
        rig.attach(&bass, Gains::default());

        let seen = poll_until_rebased(&mut rig);
        let _ = std::fs::remove_file(path);

        let stalls = seen
            .iter()
            .filter(|e| matches!(e, Event::Stalled { at_s } if *at_s == OLD_S))
            .count();
        assert_eq!(stalls, 1, "{seen:?}");
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
            OnOutputError::Fail,
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

    fn setup_of(input: &InputChoice) -> crate::LiveSetup {
        crate::LiveSetup {
            input: input.clone(),
            output_device: Some(NULL_OUTPUT.into()),
            source: InputSource::Line,
            backing: None,
            original: None,
            backing_cache: None,
            onset_gate_db: None,
        }
    }

    fn clicks() -> Arrangement {
        let mut a = Arrangement::new("Clicks", Instrument::default_bass());
        a.tempo = TempoMap::constant(120.0);
        a.notes
            .push(Note::new(0, TICKS_PER_QUARTER, Pitch::parse("E1").unwrap()));
        a.normalize();
        a
    }

    /// A run on devices taken over starts now with its count-in, not at the input's first
    /// sample, which lies as far back as the mode before ran.
    #[test]
    fn a_run_on_a_taken_rig_starts_now() {
        let (input, path) = file_input("run", 0.0);
        let bass = Instrument::default_bass();
        let rig = Rig::open(
            &input,
            null_output(),
            OnOutputError::Fail,
            InputSource::Line,
            &bass,
            Gains::default(),
        )
        .unwrap();
        let start = Instant::now();
        while rig.input().and_then(Input::started_s).is_none() && start.elapsed() < PATIENCE {
            std::thread::sleep(POLL_EVERY);
        }
        std::thread::sleep(STANDBY);
        let clock = rig.clock().clone();

        let mut live = Live::on(
            rig,
            Gains::default(),
            clicks(),
            Mode::Play,
            ScoreSettings::default(),
            PracticeOptions::default(),
            &setup_of(&input),
        )
        .unwrap();
        assert!(live.play_aligned());
        let (start_s, ..) = live.practice().session().transport().anchor();
        let starts_in_s = start_s - clock.now_s();
        let back = live.into_rig();
        let _ = std::fs::remove_file(path);

        // One bar of count-in at 120 bpm.
        assert!(
            (starts_in_s - COUNT_IN_S).abs() < TOLERANCE_S,
            "starts in {starts_in_s} s"
        );
        assert!(back.player().is_some(), "the run hands its output back");
        assert!(back.fits(&input, &null_output(), InputSource::Line));
    }

    /// "Erneut versuchen" on the rig: same clock, a new input; one that does not open leaves
    /// the rig without an input, and no mode takes it over.
    #[test]
    fn a_reopen_without_input_leaves_no_fit() {
        let (input, path) = file_input("reopen", 0.0);
        let bass = Instrument::default_bass();
        let mut rig = Rig::open(
            &input,
            null_output(),
            OnOutputError::Fail,
            InputSource::Line,
            &bass,
            Gains::default(),
        )
        .unwrap();
        rig.reopen(&input, null_output(), InputSource::Line, &bass)
            .unwrap();
        assert!(rig.fits(&input, &null_output(), InputSource::Line));

        let gone = InputChoice::File {
            path: path.with_extension("missing.wav"),
            laptop_jack: false,
        };
        assert!(rig
            .reopen(&gone, null_output(), InputSource::Line, &bass)
            .is_err());
        let _ = std::fs::remove_file(path);
        assert!(rig.input().is_none());
        assert!(!rig.fits(&gone, &null_output(), InputSource::Line));
    }

    /// Review #287: an end the last mode took is told to the next one, and a rig whose input
    /// ended fits no mode.
    #[test]
    fn an_ended_input_is_told_and_fits_no_more() {
        let (input, path) = file_input("ended", 0.0);
        let bass = Instrument::default_bass();
        let mut rig = Rig::open(
            &input,
            null_output(),
            OnOutputError::Fail,
            InputSource::Line,
            &bass,
            Gains::default(),
        )
        .unwrap();
        rig.tell_for_test(Event::Ended);
        assert!(rig.poll().iter().any(|e| matches!(e, Event::Ended)));
        rig.attach(&bass, Gains::default());
        let seen = rig.poll();
        let _ = std::fs::remove_file(path);

        assert!(seen.iter().any(|e| matches!(e, Event::Ended)), "{seen:?}");
        assert!(!rig.fits(&input, &null_output(), InputSource::Line));
    }

    /// Review #288: "Erneut versuchen" whose input does not open keeps the run's output, with
    /// the volumes and effects the player set; the run gets no output of default volumes.
    #[test]
    fn a_failed_reopen_keeps_the_runs_output() {
        let (input, path) = file_input("keep", 0.0);
        let mut setup = setup_of(&input);
        setup.output_device = Some("kontra-test:no-such-output".into());
        let mut live = Live::start(
            clicks(),
            Mode::Play,
            ScoreSettings::default(),
            PracticeOptions::default(),
            &setup,
        )
        .unwrap();
        assert!(live.output_is_silent);

        setup.output_device = Some(NULL_OUTPUT.into());
        setup.input = InputChoice::File {
            path: path.with_extension("missing.wav"),
            laptop_jack: false,
        };
        assert!(live.reopen(&setup).is_err());
        let _ = std::fs::remove_file(path);

        assert!(
            matches!(live.practice().output(), LiveOutput::Silent(_)),
            "the output was replaced"
        );
        assert!(live.output_is_silent);
    }
}
