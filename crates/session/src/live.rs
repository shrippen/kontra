//! Live practice and tuner with real devices or a recording as input.

use crate::calibration::{Calibration, CalibrationResult};
use crate::practice::{Practice, PracticeEvent, PracticeOptions, Told};
use crate::rig::{InputTap, OnOutputError, Rig, RigOutput};
use crate::session::Mode;
use crate::settings::ScoreSettings;
use kontra_engine::plugins::PluginError;
use kontra_engine::{
    monitor_rate, BackingTrack, Clock, EngineError, Event, Gains, Input, InputKind, NullOutput,
    Output, Pace, Player, Sound, StreamConfig,
};
use kontra_model::{Arrangement, InputSource, Instrument, Pitch};
use std::path::PathBuf;
use std::sync::mpsc::channel;

/// Where the player's sound comes from.
#[derive(Debug, Clone, PartialEq)]
pub enum InputChoice {
    Device {
        id: Option<String>,
        channel: Option<u16>,
    },
    /// A recording fed in real time (QA journeys). `laptop_jack` simulates the combo jack; a
    /// MIDI file (`.mid`) plays like a keyboard.
    File { path: PathBuf, laptop_jack: bool },
    /// A MIDI keyboard (LH-EIN-12); `None`: the first port.
    Midi { port: Option<String> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct LiveSetup {
    pub input: InputChoice,
    /// `None` = default device; no device found falls back to silence.
    pub output_device: Option<String>,
    pub source: InputSource,
    pub backing: Option<PathBuf>,
    /// The song with its bass when `backing` is it without (LH-BIB-09): the mixer can bring the
    /// bass back (`Gains::bass`).
    pub original: Option<PathBuf>,
    /// Folder of prepared backing tracks (`BackingCache`); `None` decodes every time.
    pub backing_cache: Option<PathBuf>,
    /// The onset gate measured at the calibration's count-in (LH-EIN-16); `None`: the input's
    /// default.
    pub onset_gate_db: Option<f32>,
}

impl LiveSetup {
    /// Sets the measured onset gate on `input`, if there is one.
    fn gate(&self, input: &mut Input) {
        if let Some(gate_db) = self.onset_gate_db {
            input.set_onset_gate(gate_db);
        }
    }
}

/// Either a real player or silence; lets `Practice` run without a sound card.
#[derive(Debug)]
pub enum LiveOutput {
    /// Boxed: a player is far larger than silence (on Windows by hundreds of bytes more).
    Device(Box<Player>),
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
    fn set_transpose(&mut self, semitones: f32) {
        if let LiveOutput::Device(p) = self {
            p.set_transpose(semitones);
        }
    }
    fn take_param_changes(&mut self) -> Vec<(u32, u32, f64)> {
        match self {
            LiveOutput::Device(p) => p.take_param_changes(),
            LiveOutput::Silent(_) => Vec::new(),
        }
    }
    fn take_param_texts(&mut self) -> Vec<(u32, u32, f64, String)> {
        match self {
            LiveOutput::Device(p) => p.take_param_texts(),
            LiveOutput::Silent(_) => Vec::new(),
        }
    }
    fn open_editor(&mut self, plugin: u32, title: &str) -> Result<(), PluginError> {
        match self {
            LiveOutput::Device(p) => p.open_editor(plugin, title),
            LiveOutput::Silent(_) => Err(PluginError::NoEditor),
        }
    }
}

impl LiveOutput {
    /// What the effect chain delays the bass by, in ms; `None` for silence.
    fn chain_latency_ms(&self) -> Option<f64> {
        match self {
            LiveOutput::Device(p) => Some(p.chain_latency_ms()),
            LiveOutput::Silent(_) => None,
        }
    }

    /// Output dropouts since the start (LH-NF-04); silence has none.
    pub fn xruns(&self) -> u64 {
        match self {
            LiveOutput::Device(p) => p.xruns(),
            LiveOutput::Silent(_) => 0,
        }
    }
}

/// Why a live practice did not start: its input (device, recording, MIDI) or the rest
/// (backing track, threads). Only the first is the input's fault (LH-EIN-15).
#[derive(Debug, thiserror::Error)]
pub enum LiveError {
    #[error(transparent)]
    Input(EngineError),
    #[error(transparent)]
    Setup(EngineError),
}

/// How a run came to its devices: it decides where the run starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Opened {
    /// Opened for this run: it starts with the input's first sample (a recording begins with
    /// the count-in).
    Fresh,
    /// Taken over from the mode before: the input runs since then; the run starts now.
    Attached,
}

/// A running live practice.
#[derive(Debug)]
pub struct Live {
    practice: Practice<LiveOutput>,
    /// The devices; the practice holds the output meanwhile (`into_rig` returns it).
    rig: Rig,
    opened: Opened,
    /// Dropouts of input and output replaced by `reopen` (LH-NF-04 counts since the start).
    xruns_before: u64,
    pub output_is_silent: bool,
    /// The bass input reaches the output (same rate on both devices); the gain decides if heard.
    can_monitor: bool,
}

impl Live {
    pub fn start(
        arrangement: Arrangement,
        mode: Mode,
        settings: ScoreSettings,
        options: PracticeOptions,
        setup: &LiveSetup,
    ) -> Result<Live, LiveError> {
        let instrument = arrangement.instrument.clone();
        // An output that does not open leaves the run silent.
        let rig = Rig::open(
            &setup.input,
            RigOutput::Device(setup.output_device.clone()),
            OnOutputError::PlaySilent,
            InputTap::Player,
            setup.source,
            &instrument,
            Gains::default(),
        )
        .map_err(LiveError::Input)?;
        Live::run_on(
            rig,
            Opened::Fresh,
            arrangement,
            mode,
            settings,
            options,
            setup,
        )
    }

    /// Runs on the devices of `rig`, at `gains`: nothing opens, nothing goes silent.
    pub fn on(
        mut rig: Rig,
        gains: Gains,
        arrangement: Arrangement,
        mode: Mode,
        settings: ScoreSettings,
        options: PracticeOptions,
        setup: &LiveSetup,
    ) -> Result<Live, LiveError> {
        rig.attach(&arrangement.instrument, gains);
        Live::run_on(
            rig,
            Opened::Attached,
            arrangement,
            mode,
            settings,
            options,
            setup,
        )
    }

    fn run_on(
        mut rig: Rig,
        opened: Opened,
        arrangement: Arrangement,
        mode: Mode,
        settings: ScoreSettings,
        options: PracticeOptions,
        setup: &LiveSetup,
    ) -> Result<Live, LiveError> {
        let output_is_silent = rig.player().is_none();
        let can_monitor = rig.is_monitoring();
        let sample_rate = rig.output().sample_rate();
        let load = |path: &std::path::Path| match &setup.backing_cache {
            Some(dir) => crate::BackingCache::new(dir).load(path, sample_rate),
            None => BackingTrack::load(path, sample_rate),
        };
        let backing = setup
            .backing
            .as_deref()
            .map(load)
            .transpose()
            .map_err(LiveError::Setup)?;
        // An original that does not load leaves the song without its bass, nothing more.
        let original = setup.original.as_deref().and_then(|p| load(p).ok());
        let backing = match (backing, original) {
            (Some(without), Some(original)) => Some(without.with_bass_of(&original)),
            (backing, _) => backing,
        };
        if let Some(input) = rig.input_mut() {
            setup.gate(input);
        }
        let output = rig.take_output();
        let clock = rig.clock().clone();
        let mut practice = Practice::new(arrangement, mode, settings, options, output, clock);
        practice.set_backing(backing);
        Ok(Live {
            practice,
            rig,
            opened,
            xruns_before: 0,
            output_is_silent,
            can_monitor,
        })
    }

    /// Hands the devices on to the next mode; the run's sounds stop with its `attach`.
    pub fn into_rig(self) -> Rig {
        let mut rig = self.rig;
        rig.put_output(self.practice.into_output());
        rig
    }

    /// Opens input and output again (LH-EIN-15, "Erneut versuchen"); the run goes on where it
    /// stands. The time since the device fell silent counts as lost, not as missed notes.
    ///
    /// ```text
    /// old input ✕ ─► new output (monitor ring, echo reference) ─► new input ─► Resumed{now}
    /// ```
    pub fn reopen(&mut self, setup: &LiveSetup) -> Result<(), EngineError> {
        // The dropouts of the old input stay counted; the old output's once it is replaced.
        self.xruns_before += self.rig.input().map_or(0, Input::xruns);
        let instrument = self.practice.session().arrangement().instrument.clone();
        let device = RigOutput::Device(setup.output_device.clone());
        let reopened = self
            .rig
            .reopen(&setup.input, device, setup.source, &instrument);
        // Without an input the run keeps its output, with the player's volumes and effects
        // (review #288); the new one closes.
        if let Err(e) = reopened {
            drop(self.rig.take_output());
            return Err(e);
        }
        self.output_is_silent = self.rig.player().is_none();
        self.can_monitor = self.rig.is_monitoring();
        self.xruns_before += self.practice.output().xruns();
        self.practice.replace_output(self.rig.take_output());
        if let Some(input) = self.rig.input_mut() {
            setup.gate(input);
        }
        self.practice.handle(Event::Resumed {
            at_s: self.rig.clock().now_s(),
        });
        Ok(())
    }

    /// The input's block size in frames and its sample rate, once the device delivered audio.
    pub fn input_block(&self) -> Option<(u32, u32)> {
        let input = self.rig.input()?;
        input.block_frames().map(|b| (b, input.sample_rate))
    }

    /// What the effect chain delays the bass by, in ms; `None` without a device output.
    pub fn chain_latency_ms(&self) -> Option<f64> {
        self.practice.output().chain_latency_ms()
    }

    /// Changes the plugins of the effect chain made themselves since the last call, as
    /// (plugin id, parameter id, value).
    pub fn take_param_changes(&mut self) -> Vec<(u32, u32, f64)> {
        self.practice.take_param_changes()
    }

    /// Opens the own editor of the chain's plugin `plugin`.
    pub fn open_editor(&mut self, plugin: u32, title: &str) -> Result<(), PluginError> {
        self.practice.open_editor(plugin, title)
    }

    /// The chain's plugins' texts for values since the last call.
    pub fn take_param_texts(&mut self) -> Vec<(u32, u32, f64, String)> {
        self.practice.take_param_texts()
    }

    /// Input and output dropouts since the start (LH-NF-04).
    pub fn xruns(&self) -> u64 {
        self.xruns_before
            + self.rig.input().map_or(0, Input::xruns)
            + self.practice.output().xruns()
    }

    /// False when monitoring cannot work: no output, or input and output run at different rates.
    pub fn can_monitor(&self) -> bool {
        self.can_monitor
    }

    pub fn practice(&self) -> &Practice<LiveOutput> {
        &self.practice
    }

    pub fn practice_mut(&mut self) -> &mut Practice<LiveOutput> {
        &mut self.practice
    }

    /// Starts in step with the input's first sample (a recording begins with the count-in).
    /// Waits up to two seconds for the input to start; returns false if it did not. On devices
    /// taken over, the input started long ago: the run starts now.
    pub fn play_aligned(&mut self) -> bool {
        if self.opened == Opened::Attached {
            let now = self.rig.clock().now_s();
            self.practice.play_from(now);
            return true;
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while std::time::Instant::now() < deadline {
            if let Some(start) = self.rig.input().and_then(Input::started_s) {
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
        for event in self.rig.poll() {
            if let Some(e) = self.practice.handle(event) {
                out.push(e);
            }
        }
        out.extend(self.practice.tick());
        let told = self.practice.take_told();
        let Some(input) = self.rig.input_mut() else {
            return out;
        };
        for told in told {
            match told {
                Told::Strike(strike) => input.expect(&strike),
                Told::Forget => input.forget_expected(),
            }
        }
        out
    }
}

/// A running tuner, on a rig of its own or one it took over.
#[derive(Debug)]
pub struct LiveTuner {
    /// Its output is the monitor the bass passes through while tuning (LH-EIN-18), or silence.
    rig: Rig,
}

/// The bass heard while tuning: the output and the mixer's volume and effects.
#[derive(Debug, Clone)]
pub struct TunerMonitor {
    pub output_device: Option<String>,
    pub gains: Gains,
}

impl LiveTuner {
    pub fn start(
        input: &InputChoice,
        instrument: &Instrument,
        source: InputSource,
        monitor: Option<TunerMonitor>,
    ) -> Result<LiveTuner, EngineError> {
        let gains = monitor
            .as_ref()
            .map(|m| m.gains.clone())
            .unwrap_or_default();
        let output = match monitor {
            Some(m) => RigOutput::Device(m.output_device),
            None => RigOutput::Silent,
        };
        // An output that does not open leaves the tuner silent: tuning matters more.
        let silent = OnOutputError::PlaySilent;
        let tap = InputTap::Player;
        let rig = Rig::open(input, output, silent, tap, source, instrument, gains)?;
        if let Some(input) = rig.input() {
            input.set_tuner(true);
        }
        Ok(LiveTuner { rig })
    }

    /// Tunes on the devices of `rig`, with the bass at `gains`.
    pub fn on(mut rig: Rig, instrument: &Instrument, gains: Gains) -> LiveTuner {
        rig.attach(instrument, gains);
        if let Some(input) = rig.input() {
            input.set_tuner(true);
        }
        LiveTuner { rig }
    }

    /// Hands the devices on to the next mode; the readings stop.
    pub fn into_rig(self) -> Rig {
        if let Some(input) = self.rig.input() {
            input.set_tuner(false);
        }
        self.rig
    }

    /// Test: the output reports an error, as a device that went away would.
    #[cfg(test)]
    fn fail_output_for_test(&self) {
        self.rig.tell_for_test(Event::Error(
            kontra_engine::Direction::Output,
            "gone".into(),
        ));
    }

    /// True while the bass passes through the output.
    pub fn is_monitoring(&self) -> bool {
        self.rig.is_monitoring()
    }

    /// As `Live::take_param_changes`; nothing while silent.
    pub fn take_param_changes(&mut self) -> Vec<(u32, u32, f64)> {
        self.rig.output().take_param_changes()
    }

    /// As `Live::take_param_texts`; nothing while silent.
    pub fn take_param_texts(&mut self) -> Vec<(u32, u32, f64, String)> {
        self.rig.output().take_param_texts()
    }

    /// As `Live::open_editor`; none while silent.
    pub fn open_editor(&mut self, plugin: u32, title: &str) -> Result<(), PluginError> {
        self.rig.output().open_editor(plugin, title)
    }

    /// New volumes and effects; heard at once.
    pub fn set_gains(&mut self, gains: Gains) {
        self.rig.output().set_gains(gains);
    }

    /// The input's block size in frames and its sample rate, once the device delivered audio.
    pub fn input_block(&self) -> Option<(u32, u32)> {
        let input = self.rig.input()?;
        input.block_frames().map(|b| (b, input.sample_rate))
    }

    /// Readings and levels since the last call.
    pub fn poll(&mut self) -> Vec<PracticeEvent> {
        let mut out = Vec::new();
        for event in self.rig.poll() {
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
                // The monitor's output went away: tuning goes on without it.
                Event::Error(kontra_engine::Direction::Output, _) => self.rig.silence_output(),
                Event::Error(side, m) => out.push(PracticeEvent::Error(side, m)),
                Event::Ended => out.push(PracticeEvent::InputEnded),
                Event::Stalled { .. } => out.push(PracticeEvent::InputStalled),
                Event::Resumed { .. } => out.push(PracticeEvent::InputResumed),
                Event::Note(_)
                | Event::Technique(_)
                | Event::Pitch(_)
                | Event::String(_)
                | Event::Retracted(_)
                | Event::DoubleStop(_)
                | Event::End(_)
                | Event::Heard { .. }
                | Event::NoiseFloor { .. }
                | Event::Rebased { .. } => {}
            }
        }
        out
    }
}

/// One tone of an ear-training prompt: pitch, start after the prompt begins, length.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PromptTone {
    pub pitch: Pitch,
    pub at_s: f64,
    pub duration_s: f64,
}

/// Time between asking to play a prompt and its first tone (the output needs a block or two).
const PROMPT_LEAD_S: f64 = 0.15;
/// After the prompt the bass may still ring through the room or the pickup: notes starting
/// earlier than this after its end are not answers.
const PROMPT_TAIL_S: f64 = 0.1;

/// Ear training (LH-LRN-03): plays prompts on the output and hears the answers as notes.
///
/// ```text
/// play(prompt) ─► output ──► ear of the player ──► bass ──► input ─► Heard { midi }
///                  └─ notes before the prompt's end + tail are dropped
/// ```
#[derive(Debug)]
pub struct LiveEar {
    /// Its output plays the prompts; the bass is not heard through it (`quiet`).
    rig: Rig,
    a4_hz: f64,
    /// Engine time from which notes count as answers.
    listen_from_s: f64,
}

impl LiveEar {
    /// What the effect chain delays the bass by, in ms; `None` without a device output.
    pub fn chain_latency_ms(&self) -> Option<f64> {
        self.rig.player().map(Player::chain_latency_ms)
    }

    /// As `Live::take_param_changes`.
    pub fn take_param_changes(&mut self) -> Vec<(u32, u32, f64)> {
        self.rig.output().take_param_changes()
    }

    /// As `Live::open_editor`.
    pub fn open_editor(&mut self, plugin: u32, title: &str) -> Result<(), PluginError> {
        self.rig.output().open_editor(plugin, title)
    }

    /// As `Live::take_param_texts`.
    pub fn take_param_texts(&mut self) -> Vec<(u32, u32, f64, String)> {
        self.rig.output().take_param_texts()
    }

    pub fn start(setup: &LiveSetup, instrument: &Instrument) -> Result<LiveEar, EngineError> {
        let rig = open_rig(setup, instrument, quiet(Gains::default()))?;
        Ok(LiveEar::with(rig, setup, instrument))
    }

    /// Ear training on the devices of `rig`, its prompts at `gains`.
    pub fn on(mut rig: Rig, setup: &LiveSetup, instrument: &Instrument, gains: Gains) -> LiveEar {
        rig.attach(instrument, quiet(gains));
        LiveEar::with(rig, setup, instrument)
    }

    fn with(mut rig: Rig, setup: &LiveSetup, instrument: &Instrument) -> LiveEar {
        if let Some(input) = rig.input_mut() {
            setup.gate(input);
        }
        LiveEar {
            rig,
            a4_hz: instrument.a4_hz,
            listen_from_s: 0.0,
        }
    }

    /// Hands the devices on to the next mode.
    pub fn into_rig(self) -> Rig {
        self.rig
    }

    pub fn set_gains(&mut self, gains: kontra_engine::Gains) {
        self.rig.output().set_gains(quiet(gains));
    }

    /// Plays `tones` now (replacing a prompt still sounding); returns its length in seconds.
    pub fn play(&mut self, tones: &[PromptTone]) -> f64 {
        let start_s = self.rig.clock().now_s() + PROMPT_LEAD_S;
        let output = self.rig.output();
        output.clear_from(0.0);
        let mut end_s = start_s;
        for tone in tones {
            let at_s = start_s + tone.at_s;
            output.schedule(
                at_s,
                Sound::Tone {
                    hz: tone.pitch.hz(self.a4_hz) as f32,
                    duration_s: tone.duration_s as f32,
                    velocity: 1.0,
                },
            );
            end_s = end_s.max(at_s + tone.duration_s);
        }
        self.listen_from_s = end_s + PROMPT_TAIL_S;
        end_s - self.rig.clock().now_s()
    }

    /// Whether the prompt still sounds (answers are not heard yet).
    pub fn is_prompting(&self) -> bool {
        self.rig.clock().now_s() < self.listen_from_s
    }

    pub fn output_is_silent(&self) -> bool {
        self.rig.player().is_none()
    }

    /// Answers and levels since the last call.
    pub fn poll(&mut self) -> Vec<PracticeEvent> {
        let mut out = Vec::new();
        for event in self.rig.poll() {
            match event {
                Event::Note(n) if n.onset_s >= self.listen_from_s => {
                    out.push(PracticeEvent::Heard { midi: n.midi })
                }
                Event::Level {
                    peak_db,
                    rms_db,
                    clipped,
                } => out.push(PracticeEvent::Level {
                    peak_db,
                    rms_db,
                    clipped,
                }),
                Event::Error(side, m) => out.push(PracticeEvent::Error(side, m)),
                Event::Ended => out.push(PracticeEvent::InputEnded),
                Event::Stalled { .. } => out.push(PracticeEvent::InputStalled),
                Event::Resumed { .. } => out.push(PracticeEvent::InputResumed),
                Event::Note(_)
                | Event::Tuner(_)
                | Event::Technique(_)
                | Event::Pitch(_)
                | Event::String(_)
                | Event::Retracted(_)
                | Event::DoubleStop(_)
                | Event::End(_)
                | Event::Heard { .. }
                | Event::NoiseFloor { .. }
                | Event::Rebased { .. } => {}
            }
        }
        out
    }
}

/// The devices of `setup` for ear training or calibration: silent when the output does not
/// open; a microphone does not take what plays (its prompts and clicks are what the player
/// answers; review #289).
fn open_rig(setup: &LiveSetup, instrument: &Instrument, gains: Gains) -> Result<Rig, EngineError> {
    let tap = match setup.source {
        InputSource::Microphone => InputTap::Off,
        _ => InputTap::Player,
    };
    Rig::open(
        &setup.input,
        RigOutput::Device(setup.output_device.clone()),
        OnOutputError::PlaySilent,
        tap,
        setup.source,
        instrument,
        gains,
    )
}

/// `gains` without the bass: ear training and calibration play only their own sounds (the
/// bass through the output would ring into the answers and plucks; LH-EIN-18).
fn quiet(gains: Gains) -> Gains {
    Gains {
        monitor: 0.0,
        ..gains
    }
}

pub(crate) fn input_kind(choice: &InputChoice) -> InputKind {
    match choice {
        InputChoice::Device { id, channel } => InputKind::Device(StreamConfig {
            device_id: id.clone(),
            input_channel: *channel,
            buffer_frames: Some(LIVE_BUFFER_FRAMES),
            ..StreamConfig::default()
        }),
        InputChoice::File { path, .. } if is_midi_file(path) => {
            InputKind::MidiFile { path: path.clone() }
        }
        InputChoice::File { path, laptop_jack } => InputKind::File {
            path: path.clone(),
            pace: Pace::RealTime,
            laptop_jack: *laptop_jack,
        },
        InputChoice::Midi { port } => InputKind::Midi { port: port.clone() },
    }
}

/// `.mid` / `.midi`: a MIDI recording, not audio.
pub fn is_midi_file(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("mid") || e.eq_ignore_ascii_case("midi"))
}

/// Frames per audio block for live input and output (LH-NF-01): PipeWire lowers its quantum to
/// the smallest block a client asks for; its default of 1024 alone is 41–45 ms round trip
/// (hardware check 13.10.), 128 is 20 ms.
const LIVE_BUFFER_FRAMES: u32 = 128;

pub(crate) fn output_config(device: Option<&str>) -> StreamConfig {
    StreamConfig {
        device_id: device.map(str::to_string),
        buffer_frames: Some(LIVE_BUFFER_FRAMES),
        ..StreamConfig::default()
    }
}

/// The output's stream for `setup`: at the input's rate where the system needs that to play
/// the bass through (Windows: input 48 kHz, output 44.1 kHz by default).
pub(crate) fn live_output_config(setup: &LiveSetup) -> StreamConfig {
    monitor_config(&setup.input, setup.output_device.as_deref())
}

pub(crate) fn monitor_config(input: &InputChoice, output: Option<&str>) -> StreamConfig {
    let mut config = output_config(output);
    if let InputChoice::Device { id, .. } = input {
        config.sample_rate = monitor_rate(id.as_deref(), output);
    }
    config
}

pub(crate) fn open_output(
    setup: &LiveSetup,
    clock: &Clock,
    events: std::sync::mpsc::Sender<Event>,
) -> LiveOutput {
    let config = live_output_config(setup);
    match Player::start(&config, clock, events) {
        Ok(player) => LiveOutput::Device(Box::new(player)),
        Err(_) => LiveOutput::Silent(NullOutput {
            sample_rate: 48_000,
        }),
    }
}

/// A running latency calibration: clicks on the output, plucks on the input.
#[derive(Debug)]
pub struct LiveCalibration {
    /// Its output plays the clicks; the bass is not heard through it (`quiet`).
    rig: Rig,
    /// When the devices were taken over (the input runs since long before); `None`: opened
    /// for the calibration, which counts from the input's first sample.
    taken_s: Option<f64>,
    calibration: Option<Calibration>,
    /// Engine time of the first click after the input has started.
    lead_s: f64,
    /// The onset gate from the count-in's noise floor, once measured (LH-EIN-16).
    onset_gate_db: Option<f32>,
}

impl LiveCalibration {
    /// Starts input and output; the measured clicks begin `lead_s` after the input's first sample
    /// (a recording for QA starts its plucks at the same offset), the count-in one bar earlier.
    pub fn start(
        setup: &LiveSetup,
        instrument: &Instrument,
        lead_s: f64,
    ) -> Result<LiveCalibration, EngineError> {
        let rig = open_rig(setup, instrument, quiet(Gains::default()))?;
        Ok(LiveCalibration::with(rig, None, setup, lead_s))
    }

    /// Calibrates on the devices of `rig`, its clicks at `gains`; they begin `lead_s` from now.
    pub fn on(
        mut rig: Rig,
        setup: &LiveSetup,
        instrument: &Instrument,
        gains: Gains,
        lead_s: f64,
    ) -> LiveCalibration {
        rig.attach(instrument, quiet(gains));
        let now = rig.clock().now_s();
        LiveCalibration::with(rig, Some(now), setup, lead_s)
    }

    fn with(mut rig: Rig, taken_s: Option<f64>, setup: &LiveSetup, lead_s: f64) -> Self {
        if let Some(input) = rig.input_mut() {
            setup.gate(input);
        }
        LiveCalibration {
            rig,
            taken_s,
            calibration: None,
            lead_s,
            onset_gate_db: None,
        }
    }

    /// Hands the devices on to the next mode.
    pub fn into_rig(self) -> Rig {
        self.rig
    }

    /// The onset gate from the count-in's noise floor, once its last click passed.
    pub fn onset_gate_db(&self) -> Option<f32> {
        self.onset_gate_db
    }

    /// The click last passed (see `Calibration::beat_at`).
    pub fn beat(&self) -> Option<i32> {
        self.calibration.as_ref()?.beat_at(self.rig.clock().now_s())
    }

    /// Polls; returns the result once all clicks are through.
    pub fn poll(&mut self) -> Option<CalibrationResult> {
        if self.calibration.is_none() {
            let start = match self.taken_s {
                Some(taken_s) => taken_s,
                None => self.rig.input().and_then(Input::started_s)?,
            };
            let calibration = Calibration::new(start + self.lead_s);
            // The count-in and the measured clicks each start on an accent.
            let count_in = calibration.count_in();
            // The count-in is the noise floor (LH-EIN-16): the player waits for the clicks.
            if let (Some(first), Some(input)) = (count_in.first(), self.rig.input_mut()) {
                input.measure_floor(*first, start + self.lead_s);
            }
            for clicks in [count_in.as_slice(), calibration.clicks()] {
                for (i, click) in clicks.iter().enumerate() {
                    let accent = if i == 0 {
                        kontra_engine::Accent::Downbeat
                    } else {
                        kontra_engine::Accent::Beat
                    };
                    self.rig.output().schedule(*click, Sound::Click(accent));
                }
            }
            self.calibration = Some(calibration);
        }
        let calibration = self.calibration.as_mut()?;
        for event in self.rig.poll() {
            match event {
                Event::Note(note) => calibration.on_onset(note.onset_s),
                // The plucks after the count-in already meet the new gate.
                Event::NoiseFloor { gate_db, .. } => {
                    self.onset_gate_db = Some(gate_db);
                    if let Some(input) = self.rig.input_mut() {
                        input.set_onset_gate(gate_db);
                    }
                }
                _ => {}
            }
        }
        (self.rig.clock().now_s() > calibration.ends_at()).then(|| calibration.result())
    }
}

/// Result of an endurance run (LH-NF-04).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoakReport {
    pub seconds: f64,
    pub xruns: u64,
    /// Loop passes played.
    pub passes: u32,
}

/// Plays `arrangement` in a loop for `duration_s` at `speed`, with metronome, reference tones
/// and the backing time-stretched to that speed, on `output_device` (`NULL_DEVICE`: a timer
/// instead of a sound card), and counts dropouts. No input: this measures the output path.
pub fn soak(
    arrangement: Arrangement,
    backing: Option<&std::path::Path>,
    speed: f64,
    duration_s: f64,
    output_device: Option<&str>,
) -> Result<SoakReport, EngineError> {
    const TICK: std::time::Duration = std::time::Duration::from_millis(10);
    let clock = Clock::new();
    let (tx, rx) = channel();
    let config = output_config(output_device);
    let output = LiveOutput::Device(Box::new(Player::start(&config, &clock, tx)?));
    let backing = backing
        .map(|p| BackingTrack::load(p, output.sample_rate()))
        .transpose()?;
    let end_s = arrangement.duration_s();
    let options = PracticeOptions {
        reference_tones: true,
        ..PracticeOptions::default()
    };
    let mut practice = Practice::new(
        arrangement,
        Mode::Play,
        ScoreSettings::default(),
        options,
        output,
        clock.clone(),
    );
    practice.set_backing(backing);
    practice.set_speed(speed);
    practice.set_loop(Some(crate::LoopRange {
        start_s: 0.0,
        end_s,
    }));
    practice.play();
    let started = std::time::Instant::now();
    while started.elapsed().as_secs_f64() < duration_s {
        while rx.try_recv().is_ok() {}
        practice.tick();
        std::thread::sleep(TICK);
    }
    Ok(SoakReport {
        seconds: started.elapsed().as_secs_f64(),
        xruns: practice.output().xruns(),
        passes: practice.session().transport().pass(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Hardware check 13.10.: without a request PipeWire runs at 1024 frames, 41–45 ms round trip.
    #[test]
    fn live_streams_ask_for_a_small_buffer() {
        let device = InputChoice::Device {
            id: None,
            channel: None,
        };
        let InputKind::Device(input) = input_kind(&device) else {
            panic!("a device input");
        };
        assert_eq!(input.buffer_frames, Some(LIVE_BUFFER_FRAMES));
        assert_eq!(output_config(None).buffer_frames, Some(LIVE_BUFFER_FRAMES));
    }

    /// Ear training and calibration never pass the bass on: only the monitor volume goes.
    #[test]
    fn quiet_gains_keep_all_but_the_bass() {
        let gains = Gains {
            monitor: 0.9,
            click: 0.3,
            ..Gains::default()
        };
        let quiet = quiet(gains);
        assert_eq!(quiet.monitor, 0.0);
        assert_eq!(quiet.click, 0.3);
    }

    /// Review #289: ear training and calibration at a microphone ran without echo removal
    /// before the rig and stay so: their prompts and clicks are what the player answers or
    /// plucks on. The player keeps what it plays to itself.
    #[test]
    fn a_microphone_keeps_its_prompts_and_clicks() {
        let path = std::env::temp_dir().join(format!("kontra-mic-{}.wav", std::process::id()));
        crate::write_mono_wav(&path, &vec![0.0; 48_000], 48_000).unwrap();
        let setup = LiveSetup {
            input: InputChoice::File {
                path: path.clone(),
                laptop_jack: false,
            },
            output_device: Some("kontra:null".into()),
            source: InputSource::Microphone,
            backing: None,
            original: None,
            backing_cache: None,
            onset_gate_db: None,
        };
        let bass = Instrument::default_bass();
        let mut ear = LiveEar::start(&setup, &bass).unwrap();
        let mut calibration = LiveCalibration::start(&setup, &bass, 1.0).unwrap();
        let _ = std::fs::remove_file(path);

        for output in [ear.rig.output(), calibration.rig.output()] {
            let LiveOutput::Device(player) = output else {
                panic!("a device output");
            };
            assert!(
                player.take_reference().is_some(),
                "the echo removal took it"
            );
        }
    }

    /// Review #258: an output error while tuning ended the tuner. Tuning comes first: only the
    /// monitor goes.
    #[test]
    fn an_output_error_keeps_the_tuner() {
        let path =
            std::env::temp_dir().join(format!("kontra-tuner-err-{}.wav", std::process::id()));
        crate::write_mono_wav(&path, &vec![0.0; 48_000], 48_000).unwrap();
        let input = InputChoice::File {
            path,
            laptop_jack: false,
        };
        let monitor = TunerMonitor {
            output_device: Some("kontra:null".into()),
            gains: Gains::default(),
        };
        let started = LiveTuner::start(
            &input,
            &Instrument::default_bass(),
            InputSource::Line,
            Some(monitor),
        );
        let mut tuner = started.unwrap();
        tuner.fail_output_for_test();
        let events = tuner.poll();
        assert!(
            !events.iter().any(|e| matches!(e, PracticeEvent::Error(..))),
            "{events:?}"
        );
        assert!(!tuner.is_monitoring());
    }
}
