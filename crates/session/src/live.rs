//! Live practice and tuner with real devices or a recording as input.

use crate::practice::{Practice, PracticeEvent, PracticeOptions};
use crate::session::Mode;
use crate::settings::ScoreSettings;
use kontra_engine::{
    BackingTrack, Clock, EngineError, Event, Input, InputKind, NullOutput, Output, Pace, Player,
    Sound, StreamConfig,
};
use kontra_model::{Arrangement, InputSource, Instrument};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};

/// Where the player's sound comes from.
#[derive(Debug, Clone, PartialEq)]
pub enum InputChoice {
    Device {
        id: Option<String>,
        channel: Option<u16>,
    },
    /// A recording fed in real time (QA journeys). `laptop_jack` simulates the combo jack.
    File { path: PathBuf, laptop_jack: bool },
}

#[derive(Debug, Clone, PartialEq)]
pub struct LiveSetup {
    pub input: InputChoice,
    /// `None` = default device; no device found falls back to silence.
    pub output_device: Option<String>,
    pub source: InputSource,
    pub backing: Option<PathBuf>,
}

/// Either a real player or silence; lets `Practice` run without a sound card.
#[derive(Debug)]
pub enum LiveOutput {
    Device(Player),
    Silent(NullOutput),
}

impl Output for LiveOutput {
    fn sample_rate(&self) -> u32 {
        match self {
            LiveOutput::Device(p) => p.sample_rate(),
            LiveOutput::Silent(n) => n.sample_rate(),
        }
    }
    fn schedule(&mut self, at_s: f64, sound: Sound) -> bool {
        match self {
            LiveOutput::Device(p) => p.schedule(at_s, sound),
            LiveOutput::Silent(n) => n.schedule(at_s, sound),
        }
    }
    fn clear_from(&mut self, from_s: f64) {
        match self {
            LiveOutput::Device(p) => p.clear_from(from_s),
            LiveOutput::Silent(n) => n.clear_from(from_s),
        }
    }
    fn play_backing(&mut self, audio: kontra_engine::BackingAudio, at_s: f64, from_frame: usize) {
        match self {
            LiveOutput::Device(p) => p.play_backing(audio, at_s, from_frame),
            LiveOutput::Silent(n) => n.play_backing(audio, at_s, from_frame),
        }
    }
    fn stop_backing(&mut self, at_s: f64) {
        match self {
            LiveOutput::Device(p) => p.stop_backing(at_s),
            LiveOutput::Silent(n) => n.stop_backing(at_s),
        }
    }
    fn set_gains(&mut self, gains: kontra_engine::Gains) {
        match self {
            LiveOutput::Device(p) => p.set_gains(gains),
            LiveOutput::Silent(n) => n.set_gains(gains),
        }
    }
}

/// A running live practice.
#[derive(Debug)]
pub struct Live {
    practice: Practice<LiveOutput>,
    input: Input,
    events: Receiver<Event>,
    pub output_is_silent: bool,
}

impl Live {
    pub fn start(
        arrangement: Arrangement,
        mode: Mode,
        settings: ScoreSettings,
        options: PracticeOptions,
        setup: &LiveSetup,
    ) -> Result<Live, EngineError> {
        let clock = Clock::new();
        let (tx, rx) = channel();
        let output = open_output(setup.output_device.as_deref(), &clock, tx.clone());
        let output_is_silent = matches!(output, LiveOutput::Silent(_));
        let sample_rate = output.sample_rate();
        let backing = setup
            .backing
            .as_deref()
            .map(|p| BackingTrack::load(p, sample_rate))
            .transpose()?;
        let instrument = arrangement.instrument.clone();
        let mut practice =
            Practice::new(arrangement, mode, settings, options, output, clock.clone());
        practice.set_backing(backing);
        let input = Input::start(
            &input_kind(&setup.input),
            &instrument,
            setup.source,
            &clock,
            tx,
        )?;
        Ok(Live {
            practice,
            input,
            events: rx,
            output_is_silent,
        })
    }

    pub fn practice(&self) -> &Practice<LiveOutput> {
        &self.practice
    }

    pub fn practice_mut(&mut self) -> &mut Practice<LiveOutput> {
        &mut self.practice
    }

    /// Starts in step with the input's first sample (a recording begins with the count-in).
    /// Waits up to two seconds for the input to start; returns false if it did not.
    pub fn play_aligned(&mut self) -> bool {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while std::time::Instant::now() < deadline {
            if let Some(start) = self.input.started_s() {
                self.practice.play_from(start);
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        false
    }

    /// Drains engine events and advances the practice; call every 10–20 ms.
    pub fn poll(&mut self) -> Vec<PracticeEvent> {
        let mut out = Vec::new();
        while let Ok(event) = self.events.try_recv() {
            if let Some(e) = self.practice.handle(event) {
                out.push(e);
            }
        }
        out.extend(self.practice.tick());
        out
    }
}

/// A running tuner.
#[derive(Debug)]
pub struct LiveTuner {
    input: Input,
    events: Receiver<Event>,
}

impl LiveTuner {
    pub fn start(
        input: &InputChoice,
        instrument: &Instrument,
        source: InputSource,
    ) -> Result<LiveTuner, EngineError> {
        let clock = Clock::new();
        let (tx, rx) = channel();
        let input = Input::start(&input_kind(input), instrument, source, &clock, tx)?;
        input.set_tuner(true);
        Ok(LiveTuner { input, events: rx })
    }

    /// Readings and levels since the last call.
    pub fn poll(&mut self) -> Vec<PracticeEvent> {
        let _ = &self.input;
        let mut out = Vec::new();
        while let Ok(event) = self.events.try_recv() {
            match event {
                Event::Tuner(r) => out.push(PracticeEvent::Tuner {
                    note: r.pitch.to_string(),
                    cents: r.cents,
                }),
                Event::Level {
                    peak_db,
                    rms_db,
                    clipped,
                } => out.push(PracticeEvent::Level {
                    peak_db,
                    rms_db,
                    clipped,
                }),
                Event::Error(m) => out.push(PracticeEvent::Error(m)),
                Event::Ended => out.push(PracticeEvent::InputEnded),
                Event::Note(_) => {}
            }
        }
        out
    }
}

fn input_kind(choice: &InputChoice) -> InputKind {
    match choice {
        InputChoice::Device { id, channel } => InputKind::Device(StreamConfig {
            device_id: id.clone(),
            input_channel: *channel,
            ..StreamConfig::default()
        }),
        InputChoice::File { path, laptop_jack } => InputKind::File {
            path: path.clone(),
            pace: Pace::RealTime,
            laptop_jack: *laptop_jack,
        },
    }
}

fn open_output(
    device: Option<&str>,
    clock: &Clock,
    events: std::sync::mpsc::Sender<Event>,
) -> LiveOutput {
    let config = StreamConfig {
        device_id: device.map(str::to_string),
        ..StreamConfig::default()
    };
    match Player::start(&config, clock, events) {
        Ok(player) => LiveOutput::Device(player),
        Err(_) => LiveOutput::Silent(NullOutput {
            sample_rate: 48_000,
        }),
    }
}
