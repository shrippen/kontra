//! Live practice and tuner with real devices or a recording as input.

use crate::calibration::{Calibration, CalibrationResult};
use crate::practice::{Practice, PracticeEvent, PracticeOptions, Told};
use crate::session::Mode;
use crate::settings::ScoreSettings;
use kontra_engine::{
    BackingTrack, Clock, EngineError, Event, Input, InputKind, NullOutput, Output, Pace, Player,
    Sound, StreamConfig,
};
use kontra_model::{Arrangement, InputSource, Instrument, Pitch};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};

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
    fn set_transpose(&mut self, semitones: f32) {
        if let LiveOutput::Device(p) = self {
            p.set_transpose(semitones);
        }
    }
}

impl LiveOutput {
    /// Output dropouts since the start (LH-NF-04); silence has none.
    pub fn xruns(&self) -> u64 {
        match self {
            LiveOutput::Device(p) => p.xruns(),
            LiveOutput::Silent(_) => 0,
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
    ) -> Result<Live, EngineError> {
        let clock = Clock::new();
        let (tx, rx) = channel();
        let mut output = open_output(setup.output_device.as_deref(), &clock, tx.clone());
        let output_is_silent = matches!(output, LiveOutput::Silent(_));
        let sample_rate = output.sample_rate();
        let load = |path: &std::path::Path| match &setup.backing_cache {
            Some(dir) => crate::BackingCache::new(dir).load(path, sample_rate),
            None => BackingTrack::load(path, sample_rate),
        };
        let backing = setup.backing.as_deref().map(load).transpose()?;
        // An original that does not load leaves the song without its bass, nothing more.
        let original = setup.original.as_deref().and_then(|p| load(p).ok());
        let backing = match (backing, original) {
            (Some(without), Some(original)) => Some(without.with_bass_of(&original)),
            (backing, _) => backing,
        };
        // The input starts before the practice owns the output: it takes the player's monitor ring.
        let player = match &mut output {
            LiveOutput::Device(p) => Some(p),
            LiveOutput::Silent(_) => None,
        };
        let input = Input::start(
            &input_kind(&setup.input),
            &arrangement.instrument,
            setup.source,
            &clock,
            tx,
            player,
        )?;
        let can_monitor = !output_is_silent && input.sample_rate == sample_rate;
        let mut practice =
            Practice::new(arrangement, mode, settings, options, output, clock.clone());
        practice.set_backing(backing);
        Ok(Live {
            practice,
            input,
            events: rx,
            output_is_silent,
            can_monitor,
        })
    }

    /// The input's block size in frames and its sample rate, once the device delivered audio.
    pub fn input_block(&self) -> Option<(u32, u32)> {
        self.input
            .block_frames()
            .map(|b| (b, self.input.sample_rate))
    }

    /// Input and output dropouts since the start (LH-NF-04).
    pub fn xruns(&self) -> u64 {
        self.input.xruns() + self.practice.output().xruns()
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
        for told in self.practice.take_told() {
            match told {
                Told::Strike(strike) => self.input.expect(&strike),
                Told::Forget => self.input.forget_expected(),
            }
        }
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
        let input = Input::start(&input_kind(input), instrument, source, &clock, tx, None)?;
        input.set_tuner(true);
        Ok(LiveTuner { input, events: rx })
    }

    /// The input's block size in frames and its sample rate, once the device delivered audio.
    pub fn input_block(&self) -> Option<(u32, u32)> {
        self.input
            .block_frames()
            .map(|b| (b, self.input.sample_rate))
    }

    /// Readings and levels since the last call.
    pub fn poll(&mut self) -> Vec<PracticeEvent> {
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
                Event::Note(_)
                | Event::Technique(_)
                | Event::Pitch(_)
                | Event::String(_)
                | Event::Heard { .. } => {}
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
    clock: Clock,
    output: LiveOutput,
    input: Input,
    events: Receiver<Event>,
    a4_hz: f64,
    /// Engine time from which notes count as answers.
    listen_from_s: f64,
}

impl LiveEar {
    pub fn start(setup: &LiveSetup, instrument: &Instrument) -> Result<LiveEar, EngineError> {
        let clock = Clock::new();
        let (tx, rx) = channel();
        let output = open_output(setup.output_device.as_deref(), &clock, tx.clone());
        let input = Input::start(
            &input_kind(&setup.input),
            instrument,
            setup.source,
            &clock,
            tx,
            None,
        )?;
        Ok(LiveEar {
            clock,
            output,
            input,
            events: rx,
            a4_hz: instrument.a4_hz,
            listen_from_s: 0.0,
        })
    }

    pub fn set_gains(&mut self, gains: kontra_engine::Gains) {
        self.output.set_gains(gains);
    }

    /// Plays `tones` now (replacing a prompt still sounding); returns its length in seconds.
    pub fn play(&mut self, tones: &[PromptTone]) -> f64 {
        let start_s = self.clock.now_s() + PROMPT_LEAD_S;
        self.output.clear_from(0.0);
        let mut end_s = start_s;
        for tone in tones {
            let at_s = start_s + tone.at_s;
            self.output.schedule(
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
        end_s - self.clock.now_s()
    }

    /// Whether the prompt still sounds (answers are not heard yet).
    pub fn is_prompting(&self) -> bool {
        self.clock.now_s() < self.listen_from_s
    }

    pub fn output_is_silent(&self) -> bool {
        matches!(self.output, LiveOutput::Silent(_))
    }

    /// Answers and levels since the last call.
    pub fn poll(&mut self) -> Vec<PracticeEvent> {
        let _ = &self.input;
        let mut out = Vec::new();
        while let Ok(event) = self.events.try_recv() {
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
                Event::Error(m) => out.push(PracticeEvent::Error(m)),
                Event::Ended => out.push(PracticeEvent::InputEnded),
                Event::Note(_)
                | Event::Tuner(_)
                | Event::Technique(_)
                | Event::Pitch(_)
                | Event::String(_)
                | Event::Heard { .. } => {}
            }
        }
        out
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

fn output_config(device: Option<&str>) -> StreamConfig {
    StreamConfig {
        device_id: device.map(str::to_string),
        buffer_frames: Some(LIVE_BUFFER_FRAMES),
        ..StreamConfig::default()
    }
}

pub(crate) fn open_output(
    device: Option<&str>,
    clock: &Clock,
    events: std::sync::mpsc::Sender<Event>,
) -> LiveOutput {
    let config = output_config(device);
    match Player::start(&config, clock, events) {
        Ok(player) => LiveOutput::Device(player),
        Err(_) => LiveOutput::Silent(NullOutput {
            sample_rate: 48_000,
        }),
    }
}

/// A running latency calibration: clicks on the output, plucks on the input.
#[derive(Debug)]
pub struct LiveCalibration {
    clock: Clock,
    output: LiveOutput,
    input: Input,
    events: Receiver<Event>,
    calibration: Option<Calibration>,
    /// Engine time of the first click after the input has started.
    lead_s: f64,
}

impl LiveCalibration {
    /// Starts input and output; the measured clicks begin `lead_s` after the input's first sample
    /// (a recording for QA starts its plucks at the same offset), the count-in one bar earlier.
    pub fn start(
        setup: &LiveSetup,
        instrument: &Instrument,
        lead_s: f64,
    ) -> Result<LiveCalibration, EngineError> {
        let clock = Clock::new();
        let (tx, rx) = channel();
        let output = open_output(setup.output_device.as_deref(), &clock, tx.clone());
        let input = Input::start(
            &input_kind(&setup.input),
            instrument,
            setup.source,
            &clock,
            tx,
            None,
        )?;
        Ok(LiveCalibration {
            clock,
            output,
            input,
            events: rx,
            calibration: None,
            lead_s,
        })
    }

    /// The click last passed (see `Calibration::beat_at`).
    pub fn beat(&self) -> Option<i32> {
        self.calibration.as_ref()?.beat_at(self.clock.now_s())
    }

    /// Polls; returns the result once all clicks are through.
    pub fn poll(&mut self) -> Option<CalibrationResult> {
        if self.calibration.is_none() {
            let start = self.input.started_s()?;
            let calibration = Calibration::new(start + self.lead_s);
            // The count-in and the measured clicks each start on an accent.
            let count_in = calibration.count_in();
            for clicks in [count_in.as_slice(), calibration.clicks()] {
                for (i, click) in clicks.iter().enumerate() {
                    let accent = if i == 0 {
                        kontra_engine::Accent::Downbeat
                    } else {
                        kontra_engine::Accent::Beat
                    };
                    self.output.schedule(*click, Sound::Click(accent));
                }
            }
            self.calibration = Some(calibration);
        }
        let calibration = self.calibration.as_mut()?;
        while let Ok(event) = self.events.try_recv() {
            if let Event::Note(note) = event {
                calibration.on_onset(note.onset_s);
            }
        }
        (self.clock.now_s() > calibration.ends_at()).then(|| calibration.result())
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
    let output = LiveOutput::Device(Player::start(&config, &clock, tx)?);
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
}
