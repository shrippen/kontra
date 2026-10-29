//! Input: a sound device or a file, analyzed on a worker thread and optionally monitored
//! (copied to a player's output); or MIDI (LH-EIN-12), whose note-ons are the notes as played.

mod ring;

use crate::clock::Clock;
use crate::player::{Player, Reference};
use crate::{EngineError, Event};
use kontra_drivers::{
    open_midi, read_audio, read_midi_notes, InputStream, MidiConnection, MidiNote, StreamConfig,
};
use kontra_dsp::synth::laptop_jack;
use kontra_dsp::{peak_db, rms_db, EchoCanceller, NoteTracker, TrackerConfig, Tuner, ECHO_BLOCK};
use kontra_model::{DetectedNote, InputSource, Instrument};
use ring::input_ring;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

const RING_S: f64 = 2.0;
const LEVEL_S: f64 = 0.05;
const BLOCK: usize = 480;
const IDLE: Duration = Duration::from_millis(2);
const CLIP_DB: f32 = -0.5;
/// No data from the device this long: it has stopped delivering (LH-EIN-15).
const STALL_S: f64 = 0.25;
/// Silence fed for a stretch the device lost, at most: a bass note is told within about 100 ms
/// (LH-NF-15), so one struck before the stretch ends in it; the rest only moves the clock.
const BRIDGE_S: f64 = 0.5;
/// Onsets this close to a resume are the jump from the silence bridge back to the signal (a
/// string ringing on), not a note: the tracker places them within a few ms of it. A note
/// really struck there lost its attack in the stall anyway.
const RESUME_GUARD_S: f64 = 0.05;
/// MIDI velocity of a full-scale note.
const VELOCITY_MAX: f32 = 127.0;
const US_PER_S: f64 = 1e6;

/// How fast a file is fed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pace {
    /// Like a live instrument (QA journeys, `--input-file`).
    RealTime,
    /// As fast as possible (tests, batch analysis).
    Fast,
}

/// Where the audio comes from.
#[derive(Debug, Clone, PartialEq)]
pub enum InputKind {
    Device(StreamConfig),
    File {
        path: PathBuf,
        pace: Pace,
        /// Simulate a laptop combo jack (QA `--input-profile laptop-jack`).
        laptop_jack: bool,
    },
    /// A MIDI input port (`None`: the first).
    Midi {
        port: Option<String>,
    },
    /// A MIDI file played as if from a keyboard, in real time (QA journeys).
    MidiFile {
        path: PathBuf,
    },
}

/// Write end of a player's monitor ring and the player's rate. Input at another rate is not
/// monitored: resampling would add latency for a feature that only helps when it is fast.
struct Tap {
    ring: rtrb::Producer<f32>,
    rate: u32,
}

impl Tap {
    fn from_player(player: Option<&mut Player>) -> Option<Tap> {
        let player = player?;
        let rate = player.sample_rate;
        player.take_monitor().map(|ring| Tap { ring, rate })
    }

    /// Copies `samples`; a full ring (output stalled) drops them instead of blocking.
    fn push(&mut self, samples: &[f32]) {
        for &s in samples {
            let _ = self.ring.push(s);
        }
    }
}

/// Watches a device for stalls (LH-EIN-15): no data for `STALL_S` → `Event::Stalled`; data
/// again → `Event::Resumed`. The device dropped what it did not deliver: `deliver` puts
/// silence where the stretch began, so notes struck before it end at their own time, and moves
/// the clock on by the rest.
///
/// ```text
/// device   ██████·········· nothing for 0.8 s ··········██████
/// events         Stalled{at_s: 2.0}                     Resumed{at_s: 2.8}
/// analysis ██████░░░░░ 0.5 s silence ░░░░░ +0.3 s shift  ██████
/// ```
struct Watch {
    last_data_s: f64,
    stalled: bool,
    /// Time lost in stalls and not filled with silence, added to the input's start.
    shift_s: f64,
    /// Device time skipped as of the last data (`Drain::skipped_s`).
    skipped_s: f64,
}

/// The end of a stall on the input's own count, and the seconds the device did not deliver.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Lost {
    from_s: f64,
    lost_s: f64,
}

impl Watch {
    fn new(now_s: f64) -> Watch {
        Watch {
            last_data_s: now_s,
            stalled: false,
            shift_s: 0.0,
            skipped_s: 0.0,
        }
    }

    /// No data at `now_s`; heard up to `heard_s`. Armed with the first data: a device slow to
    /// start is no stall.
    fn idle(&mut self, now_s: f64, heard_s: Option<f64>) -> Option<Event> {
        let at_s = heard_s?;
        if self.stalled || now_s - self.last_data_s < STALL_S {
            return None;
        }
        self.stalled = true;
        Some(Event::Stalled { at_s })
    }

    /// Data at `now_s`, the device having skipped `skipped_s` so far; without a gap it would
    /// continue at `heard_s`. After a stall, what was lost: what the device skipped meanwhile.
    /// Data captured during the stall and delivered late, in one piece or many, was not lost.
    fn data(&mut self, now_s: f64, heard_s: Option<f64>, skipped_s: f64) -> Option<Lost> {
        self.last_data_s = now_s;
        let before_s = std::mem::replace(&mut self.skipped_s, skipped_s);
        if !std::mem::take(&mut self.stalled) {
            return None;
        }
        let from_s = heard_s?;
        Some(Lost {
            from_s,
            lost_s: (skipped_s - before_s).max(0.0),
        })
    }
}

/// When a block of device data was read.
struct Arrival {
    now_s: f64,
    /// The input's first sample on the engine clock; `None` until the callback stamped it.
    start_s: Option<f64>,
    /// Device time skipped so far (`Drain::skipped_s`).
    skipped_s: f64,
}

/// One block of device data past `watch` (LH-EIN-15) into `feed`, which runs the analysis
/// (with recording and echo removal around it).
fn deliver(
    analysis: &mut Analysis,
    watch: &mut Watch,
    arrival: Arrival,
    block: &[f32],
    events: &Sender<Event>,
    mut feed: impl FnMut(&mut Analysis, &[f32]),
) {
    let lost = watch.data(arrival.now_s, analysis.heard_s(), arrival.skipped_s);
    let restart = |analysis: &mut Analysis, shift_s: f64| {
        if let Some(start) = arrival.start_s {
            analysis.start_s = start + shift_s;
        }
    };
    restart(analysis, watch.shift_s);
    let Some(lost) = lost else {
        feed(analysis, block);
        return;
    };

    // Told first: the silence below is reported heard, and the session must already know it
    // lies in the stretch.
    let resumed_s = lost.from_s + lost.lost_s;
    let _ = events.send(Event::Resumed { at_s: resumed_s });
    analysis.resumed_s = Some(resumed_s);

    // Silence where the stretch began, then the clock moves on by the rest.
    let silent = (lost.lost_s.min(BRIDGE_S) * analysis.sample_rate) as usize;
    let zeros = [0.0f32; BLOCK];
    let mut left = silent;
    while left > 0 {
        let n = left.min(BLOCK);
        feed(analysis, &zeros[..n]);
        left -= n;
    }
    watch.shift_s += lost.lost_s - silent as f64 / analysis.sample_rate;
    restart(analysis, watch.shift_s);
    feed(analysis, block);
}

/// Raw input kept for a take (hardware checks): the analysis thread, not the audio callback,
/// appends while it is on.
#[derive(Debug, Default)]
struct Recorder {
    on: AtomicBool,
    /// Samples analysed since the start.
    position: AtomicU64,
    take: Mutex<Option<Recording>>,
}

/// A stretch of raw input; `first_sample` counts from the input's first sample (`started_s`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Recording {
    pub first_sample: u64,
    pub samples: Vec<f32>,
}

impl Recorder {
    fn feed(&self, block: &[f32]) {
        let position = self
            .position
            .fetch_add(block.len() as u64, Ordering::Relaxed);
        if !self.on.load(Ordering::Relaxed) {
            return;
        }
        let Ok(mut take) = self.take.lock() else {
            return;
        };
        take.get_or_insert_with(|| Recording {
            first_sample: position,
            samples: Vec::new(),
        })
        .samples
        .extend_from_slice(block);
    }
}

/// Running input; dropping it stops the threads.
#[derive(Debug)]
pub struct Input {
    stop: Arc<AtomicBool>,
    tuner_on: Arc<AtomicBool>,
    /// Engine time of the first sample (NaN until known).
    started: Arc<AtomicU64>,
    /// Largest block the device delivered at once: PipeWire's quantum (0 for files).
    block: Arc<AtomicU32>,
    worker: Option<JoinHandle<()>>,
    _stream: Option<InputStream>,
    _midi: Option<MidiConnection>,
    recorder: Arc<Recorder>,
    pub sample_rate: u32,
}

impl Input {
    /// Starts the input. With `monitor`, its samples also go to that player's output when both
    /// run at the same rate (real-time files and devices; the player's gain decides if it is heard).
    pub fn start(
        kind: &InputKind,
        instrument: &Instrument,
        source: InputSource,
        clock: &Clock,
        events: Sender<Event>,
        monitor: Option<&mut Player>,
    ) -> Result<Input, EngineError> {
        let stop = Arc::new(AtomicBool::new(false));
        let tuner_on = Arc::new(AtomicBool::new(false));
        // A microphone also hears the speakers: it takes what plays, to subtract it (LH-EIN-11).
        let (tap, reference) = match monitor {
            Some(player) => {
                let reference = (source == InputSource::Microphone)
                    .then(|| player.take_reference().map(|r| (r, player.sample_rate)))
                    .flatten();
                (Tap::from_player(Some(player)), reference)
            }
            None => (None, None),
        };
        match kind {
            InputKind::Device(config) => start_device(
                config, instrument, source, clock, events, stop, tuner_on, tap, reference,
            ),
            InputKind::File {
                path,
                pace,
                laptop_jack,
            } => start_file(
                path,
                *pace,
                *laptop_jack,
                instrument,
                source,
                clock,
                events,
                stop,
                tuner_on,
                tap.filter(|_| *pace == Pace::RealTime),
            ),
            InputKind::Midi { port } => start_midi(port.as_deref(), clock, events, stop, tuner_on),
            InputKind::MidiFile { path } => start_midi_file(path, clock, events, stop, tuner_on),
        }
    }

    /// Engine time of the input's first sample, once known (device: first callback; file: start
    /// of playback after decoding).
    pub fn started_s(&self) -> Option<f64> {
        let value = f64::from_bits(self.started.load(Ordering::Relaxed));
        (!value.is_nan()).then_some(value)
    }

    /// Input dropouts since the start (LH-NF-04); files have none.
    pub fn xruns(&self) -> u64 {
        self._stream.as_ref().map_or(0, InputStream::xruns)
    }

    /// Largest device block in frames so far (the audio server's quantum), once a block came.
    pub fn block_frames(&self) -> Option<u32> {
        Some(self.block.load(Ordering::Relaxed)).filter(|b| *b > 0)
    }

    /// Switches tuner readings on or off.
    pub fn set_tuner(&self, on: bool) {
        self.tuner_on.store(on, Ordering::Relaxed);
    }

    /// Starts keeping the raw input from the next analysed block (audio devices and files).
    pub fn record(&self) {
        if let Ok(mut take) = self.recorder.take.lock() {
            *take = None;
        }
        self.recorder.on.store(true, Ordering::Relaxed);
    }

    /// Stops keeping the input and hands over what was kept; `None` if nothing came.
    pub fn stop_recording(&self) -> Option<Recording> {
        self.recorder.on.store(false, Ordering::Relaxed);
        self.recorder.take.lock().ok()?.take()
    }
}

impl Drop for Input {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// Whether `onset_s` is the jump at the resume `resumed_s` rather than a note.
fn at_resume(resumed_s: Option<f64>, onset_s: f64) -> bool {
    resumed_s.is_some_and(|at_s| (onset_s - at_s).abs() <= RESUME_GUARD_S)
}

/// Analysis shared by device and file input.
struct Analysis {
    tracker: NoteTracker,
    tuner: Tuner,
    level: Vec<f32>,
    level_len: usize,
    start_s: f64,
    sample_rate: f64,
    /// Samples analysed since the input started.
    analysed: u64,
    /// Where the input resumed after the last stall: no onset is told there.
    resumed_s: Option<f64>,
}

impl Analysis {
    fn new(sample_rate: f64, instrument: &Instrument, source: InputSource) -> Analysis {
        let config = TrackerConfig::for_instrument(instrument, source);
        let mut tuner = Tuner::new(sample_rate, config.min_hz, config.max_hz);
        tuner.set_reference(instrument.a4_hz);
        let level_len = (LEVEL_S * sample_rate) as usize;
        Analysis {
            tracker: NoteTracker::new(sample_rate, config),
            tuner,
            level: Vec::with_capacity(level_len),
            level_len,
            start_s: 0.0,
            sample_rate,
            analysed: 0,
            resumed_s: None,
        }
    }

    /// Heard up to here; `None` before the first sample.
    fn heard_s(&self) -> Option<f64> {
        if self.analysed == 0 {
            return None;
        }
        Some(self.start_s + self.analysed as f64 / self.sample_rate)
    }

    fn process(&mut self, block: &[f32], tuner_on: bool, events: &Sender<Event>) {
        let resumed_s = self.resumed_s;
        for note in self.tracker.push(block) {
            let mut note = *note;
            note.onset_s += self.start_s;
            if at_resume(resumed_s, note.onset_s) {
                continue;
            }
            let _ = events.send(Event::Note(note));
        }
        self.send_revisions(events);
        if tuner_on {
            if let Some(reading) = self.tuner.push(block) {
                let _ = events.send(Event::Tuner(reading));
            }
        }
        for &s in block {
            self.level.push(s);
            if self.level.len() < self.level_len {
                continue;
            }
            let peak = peak_db(&self.level);
            let _ = events.send(Event::Level {
                peak_db: peak,
                rms_db: rms_db(&self.level),
                clipped: peak > CLIP_DB,
            });
            self.level.clear();
        }

        // How far the analysis is, as often as the level: the session judges up to there.
        let before = self.analysed / self.level_len as u64;
        self.analysed += block.len() as u64;
        if self.analysed / self.level_len as u64 != before {
            let at_s = self.start_s + self.analysed as f64 / self.sample_rate;
            let _ = events.send(Event::Heard { at_s });
        }
    }

    fn finish(&mut self, events: &Sender<Event>) {
        let resumed_s = self.resumed_s;
        for note in self.tracker.flush() {
            let mut note = *note;
            note.onset_s += self.start_s;
            if at_resume(resumed_s, note.onset_s) {
                continue;
            }
            let _ = events.send(Event::Note(note));
        }
        self.send_revisions(events);
    }

    /// Techniques and pitches told late, on the same clock as the notes.
    fn send_revisions(&self, events: &Sender<Event>) {
        for revision in self.tracker.revised() {
            let mut revision = *revision;
            revision.onset_s += self.start_s;
            if at_resume(self.resumed_s, revision.onset_s) {
                continue;
            }
            let _ = events.send(Event::Technique(revision));
        }
        for revision in self.tracker.pitch_revised() {
            let mut revision = *revision;
            revision.onset_s += self.start_s;
            if at_resume(self.resumed_s, revision.onset_s) {
                continue;
            }
            let _ = events.send(Event::Pitch(revision));
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn start_device(
    config: &StreamConfig,
    instrument: &Instrument,
    source: InputSource,
    clock: &Clock,
    events: Sender<Event>,
    stop: Arc<AtomicBool>,
    tuner_on: Arc<AtomicBool>,
    tap: Option<Tap>,
    reference: Option<(Reference, u32)>,
) -> Result<Input, EngineError> {
    // The ring size depends on the sample rate, which we only know after opening; 192 kHz is the cap.
    let (mut capture, mut drain) = input_ring((RING_S * 192_000.0) as usize);
    let started = Arc::new(AtomicU64::new(f64::NAN.to_bits()));
    let started_cb = Arc::clone(&started);
    let started_input = Arc::clone(&started);
    let block = Arc::new(AtomicU32::new(0));
    let block_cb = Arc::clone(&block);
    let clock_cb = clock.clone();
    let error_events = events.clone();
    // The device's rate is known only after opening; until it matches the player's, nothing is
    // monitored.
    let tap_rate = tap.as_ref().map_or(0, |t| t.rate);
    let mut tap = tap;
    let monitor_on = Arc::new(AtomicBool::new(false));
    let monitor_cb = Arc::clone(&monitor_on);
    let stream = InputStream::open(
        config,
        move |samples: &[f32], captured_s: f64| {
            if f64::from_bits(started_cb.load(Ordering::Relaxed)).is_nan() {
                // First block: the stream started about one block earlier; calibration absorbs the rest.
                started_cb.store(clock_cb.now_s().to_bits(), Ordering::Relaxed);
            }
            block_cb.fetch_max(
                u32::try_from(samples.len()).unwrap_or(u32::MAX),
                Ordering::Relaxed,
            );
            capture.push(samples, captured_s);
            if let Some(tap) = tap.as_mut().filter(|_| monitor_cb.load(Ordering::Relaxed)) {
                tap.push(samples);
            }
        },
        move |message| {
            let _ = error_events.send(Event::Error(message));
        },
    )?;
    let sample_rate = stream.sample_rate;
    monitor_on.store(tap_rate == sample_rate, Ordering::Relaxed);
    let mut analysis = Analysis::new(f64::from(sample_rate), instrument, source);
    // Echo removal only when input and output run at the same rate (as monitoring).
    let mut echo = reference
        .filter(|(_, rate)| *rate == sample_rate)
        .map(|(reference, _)| EchoStage::new(reference, f64::from(sample_rate)));
    let stop_worker = Arc::clone(&stop);
    let tuner_worker = Arc::clone(&tuner_on);
    let recorder = Arc::new(Recorder::default());
    let recorder_worker = Arc::clone(&recorder);
    let clock_worker = clock.clone();
    let worker = std::thread::Builder::new()
        .name("kontra-analysis".into())
        .spawn(move || {
            let mut block = vec![0.0f32; BLOCK];
            let mut watch = Watch::new(clock_worker.now_s());
            let rate = f64::from(sample_rate);
            while !stop_worker.load(Ordering::Relaxed) {
                let available = drain.read(&mut block);
                if available == 0 {
                    if let Some(e) = watch.idle(clock_worker.now_s(), analysis.heard_s()) {
                        let _ = events.send(e);
                    }
                    std::thread::sleep(IDLE);
                    continue;
                }
                let start = f64::from_bits(started.load(Ordering::Relaxed));
                let arrival = Arrival {
                    now_s: clock_worker.now_s(),
                    start_s: (!start.is_nan()).then_some(start),
                    skipped_s: drain.skipped_s(rate),
                };
                let tuner = tuner_worker.load(Ordering::Relaxed);
                let feed = |analysis: &mut Analysis, samples: &[f32]| {
                    recorder_worker.feed(samples);
                    match echo.as_mut() {
                        Some(stage) => stage.push(samples, analysis.start_s, |cleaned| {
                            analysis.process(cleaned, tuner, &events)
                        }),
                        None => analysis.process(samples, tuner, &events),
                    }
                };
                deliver(
                    &mut analysis,
                    &mut watch,
                    arrival,
                    &block[..available],
                    &events,
                    feed,
                );
            }
        })
        .map_err(EngineError::Thread)?;
    Ok(Input {
        stop,
        tuner_on,
        started: started_input,
        block,
        worker: Some(worker),
        _stream: Some(stream),
        _midi: None,
        recorder,
        sample_rate,
    })
}

#[allow(clippy::too_many_arguments)]
fn start_file(
    path: &std::path::Path,
    pace: Pace,
    simulate_laptop: bool,
    instrument: &Instrument,
    source: InputSource,
    clock: &Clock,
    events: Sender<Event>,
    stop: Arc<AtomicBool>,
    tuner_on: Arc<AtomicBool>,
    tap: Option<Tap>,
) -> Result<Input, EngineError> {
    let buffer = read_audio(path)?;
    let sample_rate = buffer.sample_rate;
    let sr = f64::from(sample_rate);
    let mut samples = buffer.mono();
    if simulate_laptop {
        samples = laptop_jack(&samples, sr, 1);
    }
    let mut analysis = Analysis::new(sr, instrument, source);
    let mut tap = tap.filter(|t| t.rate == sample_rate);
    let clock = clock.clone();
    let started_flag = Arc::new(AtomicU64::new(f64::NAN.to_bits()));
    let started_worker = Arc::clone(&started_flag);
    let stop_worker = Arc::clone(&stop);
    let tuner_worker = Arc::clone(&tuner_on);
    let recorder = Arc::new(Recorder::default());
    let recorder_worker = Arc::clone(&recorder);
    let worker = std::thread::Builder::new()
        .name("kontra-file-input".into())
        .spawn(move || {
            // Real-time files run on the engine clock; fast ones report file-relative times.
            let started = clock.now_s();
            started_worker.store(started.to_bits(), Ordering::Relaxed);
            analysis.start_s = if pace == Pace::RealTime { started } else { 0.0 };
            for (i, block) in samples.chunks(BLOCK).enumerate() {
                if stop_worker.load(Ordering::Relaxed) {
                    return;
                }
                if pace == Pace::RealTime {
                    let due = started + (i * BLOCK) as f64 / sr;
                    let wait = due - clock.now_s();
                    if wait > 0.0 {
                        std::thread::sleep(Duration::from_secs_f64(wait));
                    }
                }
                if let Some(tap) = tap.as_mut() {
                    tap.push(block);
                }
                recorder_worker.feed(block);
                analysis.process(block, tuner_worker.load(Ordering::Relaxed), &events);
            }
            analysis.finish(&events);
            let _ = events.send(Event::Ended);
        })
        .map_err(EngineError::Thread)?;
    Ok(Input {
        stop,
        tuner_on,
        started: started_flag,
        block: Arc::new(AtomicU32::new(0)),
        worker: Some(worker),
        _stream: None,
        _midi: None,
        recorder,
        sample_rate,
    })
}

/// Removes what played from a microphone signal before analysis (LH-EIN-11). The reference
/// starts at the mixer's `start_s`, the microphone at the input's start: the difference is
/// dropped from (or padded to) the reference once, the remaining delay (buffers, room) the
/// canceller learns.
///
/// ```text
/// mic ─► blocks of 512 ──────────────► EchoCanceller ─► analysis
/// reference ─► align once ─► 512 ─────────┘
/// ```
struct EchoStage {
    canceller: EchoCanceller,
    reference: Reference,
    sample_rate: f64,
    /// Zeros still owed to the reference (it started after the microphone).
    pad: usize,
    aligned: bool,
    mic: Vec<f32>,
    played: Vec<f32>,
    filled: usize,
    /// Microphone samples taken so far.
    taken: u64,
}

impl EchoStage {
    fn new(reference: Reference, sample_rate: f64) -> EchoStage {
        reference.armed.store(true, Ordering::Relaxed);
        EchoStage {
            canceller: EchoCanceller::new(),
            reference,
            sample_rate,
            pad: 0,
            aligned: false,
            mic: vec![0.0; ECHO_BLOCK],
            played: vec![0.0; ECHO_BLOCK],
            filled: 0,
            taken: 0,
        }
    }

    /// Feeds microphone samples (input started at `mic_start_s`); calls `out` with each
    /// cleaned block.
    fn push(&mut self, samples: &[f32], mic_start_s: f64, mut out: impl FnMut(&[f32])) {
        for &s in samples {
            self.mic[self.filled] = s;
            self.filled += 1;
            self.taken += 1;
            if self.filled < ECHO_BLOCK {
                continue;
            }
            self.filled = 0;
            let block_start_s =
                mic_start_s + (self.taken - ECHO_BLOCK as u64) as f64 / self.sample_rate;
            if self.align(block_start_s) {
                self.fill_played();
                self.canceller.process(&mut self.mic, &self.played);
            }
            out(&self.mic);
        }
    }

    /// Lines the reference up with the microphone once its start is known; false until then.
    fn align(&mut self, block_start_s: f64) -> bool {
        if self.aligned {
            return true;
        }
        let ref_start = f64::from_bits(self.reference.start_s.load(Ordering::Relaxed));
        if ref_start.is_nan() {
            return false;
        }
        let offset = align_frames(block_start_s - ref_start, self.sample_rate);
        if offset >= 0 {
            for _ in 0..offset {
                let _ = self.reference.samples.pop();
            }
        } else {
            self.pad = usize::try_from(offset.unsigned_abs()).unwrap_or(usize::MAX);
        }
        self.aligned = true;
        true
    }

    fn fill_played(&mut self) {
        for slot in self.played.iter_mut() {
            *slot = if self.pad > 0 {
                self.pad -= 1;
                0.0
            } else {
                self.reference.samples.pop().unwrap_or(0.0)
            };
        }
    }
}

impl Drop for EchoStage {
    fn drop(&mut self) {
        self.reference.armed.store(false, Ordering::Relaxed);
    }
}

/// Reference frames to drop (positive: it started earlier) or to pad (negative) for a
/// microphone that started `mic_minus_ref_s` after the reference.
fn align_frames(mic_minus_ref_s: f64, sample_rate: f64) -> i64 {
    (mic_minus_ref_s * sample_rate).round() as i64
}

/// A key going down as a detected note: exact pitch, sure, level from the velocity.
fn midi_note(onset_s: f64, note: MidiNote) -> Option<DetectedNote> {
    (note.velocity > 0).then(|| DetectedNote {
        onset_s,
        midi: f64::from(note.key),
        confidence: 1.0,
        level_db: kontra_dsp::amplitude_to_db(f32::from(note.velocity) / VELOCITY_MAX),
        source: InputSource::Midi,
        technique: None,
    })
}

/// A keyboard: midir's timestamps (µs, its own epoch) are put on the engine clock at the first
/// message, so the spacing of notes is the keyboard's, not that of thread wake-ups.
fn start_midi(
    port: Option<&str>,
    clock: &Clock,
    events: Sender<Event>,
    stop: Arc<AtomicBool>,
    tuner_on: Arc<AtomicBool>,
) -> Result<Input, EngineError> {
    let clock = clock.clone();
    let started = Arc::new(AtomicU64::new(clock.now_s().to_bits()));
    let mut offset_s: Option<f64> = None;
    let connection = open_midi(port, move |stamp_us, note| {
        let stamp_s = stamp_us as f64 / US_PER_S;
        let offset = *offset_s.get_or_insert_with(|| clock.now_s() - stamp_s);
        if let Some(detected) = midi_note(stamp_s + offset, note) {
            let _ = events.send(Event::Note(detected));
        }
    })?;
    Ok(Input {
        stop,
        tuner_on,
        started,
        block: Arc::new(AtomicU32::new(0)),
        worker: None,
        _stream: None,
        _midi: Some(connection),
        recorder: Arc::default(),
        sample_rate: 0,
    })
}

/// A MIDI file played in real time from now, as a keyboard would send it.
fn start_midi_file(
    path: &std::path::Path,
    clock: &Clock,
    events: Sender<Event>,
    stop: Arc<AtomicBool>,
    tuner_on: Arc<AtomicBool>,
) -> Result<Input, EngineError> {
    let notes = read_midi_notes(path)?;
    let clock = clock.clone();
    let started_flag = Arc::new(AtomicU64::new(f64::NAN.to_bits()));
    let started_worker = Arc::clone(&started_flag);
    let stop_worker = Arc::clone(&stop);
    let worker = std::thread::Builder::new()
        .name("kontra-midi-file".into())
        .spawn(move || {
            let started = clock.now_s();
            started_worker.store(started.to_bits(), Ordering::Relaxed);
            for (at_s, note) in notes {
                while clock.now_s() < started + at_s {
                    if stop_worker.load(Ordering::Relaxed) {
                        return;
                    }
                    std::thread::sleep(IDLE);
                }
                if let Some(detected) = midi_note(started + at_s, note) {
                    let _ = events.send(Event::Note(detected));
                }
            }
            let _ = events.send(Event::Ended);
        })
        .map_err(EngineError::Thread)?;
    Ok(Input {
        stop,
        tuner_on,
        started: started_flag,
        block: Arc::new(AtomicU32::new(0)),
        worker: Some(worker),
        _stream: None,
        _midi: None,
        recorder: Arc::default(),
        sample_rate: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kontra_drivers::{write_wav, AudioBuffer};

    /// LH-EIN-15: 0.8 s without data → one Stalled after 250 ms, then what was lost where
    /// the new samples really start.
    #[test]
    fn stall_is_told_with_what_was_lost() {
        let mut watch = Watch::new(0.0);
        assert!(watch.data(0.01, None, 0.0).is_none(), "no stall yet");
        assert!(watch.idle(0.2, Some(1.0)).is_none(), "under 250 ms");
        assert!(matches!(watch.idle(1.3, Some(1.0)), Some(Event::Stalled { at_s }) if at_s == 1.0));
        assert!(watch.idle(1.5, Some(1.0)).is_none(), "told once");

        // Data again at 1.8 s; the device skipped 0.79 s meanwhile.
        let lost = watch.data(1.8, Some(1.0), 0.79).expect("resumed");
        assert_eq!(lost.from_s, 1.0);
        assert!((lost.lost_s - 0.79).abs() < 1e-9, "{lost:?}");
        assert!(watch.data(1.81, Some(1.8), 0.79).is_none());
    }

    /// A device slow to start is no stall (PR-Agent #186); one that delivers late but all of
    /// it (a burst) lost nothing.
    #[test]
    fn late_start_or_burst_loses_nothing() {
        let mut watch = Watch::new(0.0);
        assert!(
            watch.idle(0.3, None).is_none(),
            "not armed before the first data"
        );

        watch.data(0.31, None, 0.0);
        watch.idle(1.4, Some(1.0));
        let lost = watch.data(1.5, Some(1.0), 0.0).expect("resumed");
        assert_eq!(lost.lost_s, 0.0);
    }

    /// Review #186, round 3: the device captured on, but delivers the 0.8 s late and in pieces
    /// of 0.1 s. Nothing was lost, though the first piece alone shows a small backlog.
    #[test]
    fn late_pieces_lose_nothing() {
        let rate = 1000.0;
        let (mut capture, mut drain) = input_ring(1000);
        let mut out = [0.0f32; 100];
        let mut watch = Watch::new(0.0);
        capture.push(&[0.0; 100], 0.0);
        drain.read(&mut out);
        watch.data(0.1, Some(0.1), drain.skipped_s(rate));
        assert!(watch.idle(0.5, Some(0.1)).is_some(), "stalled");

        // Captured on from 0.1 s, delivered from 0.9 s in pieces.
        let mut lost = None;
        for piece in 0..8 {
            let at_s = f64::from(piece) * 0.1;
            capture.push(&[0.0; 100], 0.1 + at_s);
            drain.read(&mut out);
            let skipped_s = drain.skipped_s(rate);
            lost = lost.or(watch.data(0.9 + at_s, Some(0.1 + at_s), skipped_s));
        }
        let lost = lost.expect("resumed");
        assert!(lost.lost_s.abs() < 1e-9, "{lost:?}");
    }

    /// PR-Agent #186: a note struck 60 ms before the device stalls for 0.8 s is told after the
    /// stall; it must keep its own time, not move by the lost 0.8 s.
    #[test]
    fn note_before_a_stall_keeps_its_time() {
        use kontra_dsp::synth::{bass_tone, ToneShape};

        let rate = 48_000.0;
        let mut analysis = Analysis::new(rate, &Instrument::default_bass(), InputSource::Line);
        let mut watch = Watch::new(0.0);
        let (tx, rx) = std::sync::mpsc::channel();
        let tone = bass_tone(55.0, rate, 2.0, ToneShape::Rich, 1);
        // A block captured at `at_s`, the device having skipped `skipped_s` so far.
        let take = |analysis: &mut Analysis, watch: &mut Watch, at_s: f64, skipped_s, block| {
            let arrival = Arrival {
                now_s: at_s + <[f32]>::len(block) as f64 / rate,
                start_s: Some(0.0),
                skipped_s,
            };
            deliver(analysis, watch, arrival, block, &tx, |a, b| {
                a.process(b, false, &tx)
            });
        };

        // Silence, then the string from 1.0 s; the device stops at 1.06 s.
        let mut signal = vec![0.0f32; rate as usize];
        signal.extend_from_slice(&tone[..(0.06 * rate) as usize]);
        for (i, block) in signal.chunks(BLOCK).enumerate() {
            take(
                &mut analysis,
                &mut watch,
                (i * BLOCK) as f64 / rate,
                0.0,
                block,
            );
        }
        assert!(watch.idle(1.4, analysis.heard_s()).is_some(), "stalled");

        // Back at 1.86 s, 0.8 s skipped; the string still rings for 0.5 s, then a new note (D)
        // at 2.36 s.
        let mut after = tone[(0.06 * rate) as usize..(0.56 * rate) as usize].to_vec();
        after.extend_from_slice(&bass_tone(73.4, rate, 1.0, ToneShape::Rich, 2));
        for (i, block) in after.chunks(BLOCK).enumerate() {
            take(
                &mut analysis,
                &mut watch,
                1.86 + (i * BLOCK) as f64 / rate,
                0.8,
                block,
            );
        }

        let onsets: Vec<f64> = rx
            .try_iter()
            .filter_map(|e| match e {
                Event::Note(n) => Some(n.onset_s),
                _ => None,
            })
            .collect();
        assert!(
            onsets.iter().any(|o| (o - 1.0).abs() < 0.03),
            "the note at 1.0 s: {onsets:?}"
        );
        // Review #186, round 4: the string ringing on after the silence is no new note either.
        assert!(
            !onsets.iter().any(|o| (1.1..2.3).contains(o)),
            "nothing inside the stall or at the resume (1.86 s): {onsets:?}"
        );
        assert!(
            onsets.iter().any(|o| (o - 2.36).abs() < 0.03),
            "the note after the stall at 2.36 s: {onsets:?}"
        );
    }

    /// LH-EIN-12: a MIDI file plays like a keyboard: note-ons at their times on the engine
    /// clock, exact pitch, note-offs dropped.
    #[test]
    fn reference_lines_up_by_start_times() {
        // Microphone 0.1 s after the reference: drop 4800 reference frames at 48 kHz.
        assert_eq!(align_frames(0.1, 48_000.0), 4800);
        assert_eq!(align_frames(-0.02, 48_000.0), -960);
    }

    /// LH-EIN-11: a played signal in the reference is removed from the microphone once the
    /// stage has learnt the way; what the reference does not hold stays.
    #[test]
    fn echo_stage_removes_what_played() {
        let (mut tx, rx) = rtrb::RingBuffer::<f32>::new(48_000 * 10);
        let reference = Reference {
            samples: rx,
            armed: Arc::new(AtomicBool::new(false)),
            start_s: Arc::new(AtomicU64::new(0.0f64.to_bits())),
        };
        let mut stage = EchoStage::new(reference, 48_000.0);
        let mut noise = kontra_dsp::synth::Noise::new(9);
        let band: Vec<f32> = (0..48_000 * 10).map(|_| noise.sample() * 0.3).collect();
        for &s in &band {
            tx.push(s).unwrap();
        }
        // The microphone hears the band 2 ms late at 0.5.
        let mic: Vec<f32> = (0..band.len())
            .map(|i| i.checked_sub(96).map_or(0.0, |j| band[j] * 0.5))
            .collect();
        let mut cleaned = Vec::new();
        stage.push(&mic, 0.0, |b| cleaned.extend_from_slice(b));
        let energy = |s: &[f32]| s.iter().map(|x| x * x).sum::<f32>();
        let late = 48_000 * 7..cleaned.len();
        let drop = 10.0 * (energy(&cleaned[late.clone()]) / energy(&mic[late])).log10();
        assert!(drop < -30.0, "{drop} dB");
    }

    #[test]
    fn midi_file_plays_its_notes_in_real_time() {
        // 480 ticks per quarter at 120 BPM: E2 at 0 s, G2 at 0.25 s (240 ticks = varlen 0x81 0x70).
        let track: Vec<u8> = vec![
            0x00, 0x90, 40, 100, 0x81, 0x70, 0x80, 40, 0, 0x00, 0x90, 43, 64, 0x81, 0x70, 0x80, 43,
            0, 0x00, 0xff, 0x2f, 0x00,
        ];
        let mut file = b"MThd".to_vec();
        file.extend(6u32.to_be_bytes());
        file.extend([0, 0, 0, 1, 0x01, 0xe0]);
        file.extend(b"MTrk");
        file.extend((track.len() as u32).to_be_bytes());
        file.extend(track);
        let path = std::env::temp_dir().join(format!("kontra-midi-in-{}.mid", std::process::id()));
        std::fs::write(&path, file).unwrap();

        let clock = Clock::new();
        let (tx, rx) = std::sync::mpsc::channel();
        let kind = InputKind::MidiFile { path: path.clone() };
        let input = Input::start(
            &kind,
            &Instrument::default_bass(),
            InputSource::Midi,
            &clock,
            tx,
            None,
        )
        .unwrap();
        let mut notes = Vec::new();
        for event in rx.iter() {
            match event {
                Event::Note(n) => notes.push(n),
                Event::Ended => break,
                _ => {}
            }
        }
        let started = input.started_s().unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!(notes.len(), 2, "{notes:?}");
        assert_eq!((notes[0].midi, notes[1].midi), (40.0, 43.0));
        assert!((notes[0].onset_s - started).abs() < 1e-9);
        assert!((notes[1].onset_s - started - 0.25).abs() < 1e-9);
        assert!(notes[1].level_db < notes[0].level_db, "softer velocity");
        assert!(notes.iter().all(|n| n.source == InputSource::Midi));
    }

    /// Feeds a 0.1 s real-time file at `file_rate` into a tap at `tap_rate`; returns what
    /// reached the monitor ring.
    fn monitored(file_rate: u32, tap_rate: u32, name: &str) -> Vec<f32> {
        let path = std::env::temp_dir().join(format!("kontra-{name}-{}.wav", std::process::id()));
        let samples: Vec<f32> = (0..file_rate / 10).map(|i| (i % 7) as f32 / 10.0).collect();
        write_wav(
            &path,
            &AudioBuffer {
                sample_rate: file_rate,
                channels: vec![samples],
            },
        )
        .unwrap();
        let (ring, mut rx) = rtrb::RingBuffer::<f32>::new(10_000);
        let (tx, events) = std::sync::mpsc::channel();
        let input = start_file(
            &path,
            Pace::RealTime,
            false,
            &Instrument::default_bass(),
            InputSource::Line,
            &Clock::new(),
            tx,
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicBool::new(false)),
            Some(Tap {
                ring,
                rate: tap_rate,
            }),
        )
        .unwrap();
        while !matches!(events.recv(), Ok(Event::Ended) | Err(_)) {}
        drop(input);
        std::fs::remove_file(path).unwrap();
        std::iter::from_fn(|| rx.pop().ok()).collect()
    }

    #[test]
    fn real_time_file_reaches_the_monitor() {
        let heard = monitored(48_000, 48_000, "monitor-same");
        assert_eq!(heard.len(), 4800);
        assert!((heard[3] - 0.3).abs() < 1e-6);
    }

    #[test]
    fn other_rate_is_not_monitored() {
        assert!(monitored(44_100, 48_000, "monitor-other").is_empty());
    }

    /// A take keeps the raw input from where recording began, with its place in the input.
    #[test]
    fn recording_keeps_the_raw_input() {
        let samples: Vec<f32> = (0..24_000).map(|i| (i % 100) as f32 / 100.0).collect();
        let path = std::env::temp_dir().join(format!("kontra-rec-{}.wav", std::process::id()));
        let buffer = AudioBuffer {
            sample_rate: 48_000,
            channels: vec![samples.clone()],
        };
        write_wav(&path, &buffer).unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let kind = InputKind::File {
            path: path.clone(),
            pace: Pace::RealTime,
            laptop_jack: false,
        };
        let clock = Clock::new();
        let input = Input::start(
            &kind,
            &Instrument::default_bass(),
            InputSource::Line,
            &clock,
            tx,
            None,
        )
        .unwrap();
        input.record();
        while !matches!(rx.recv(), Ok(Event::Ended) | Err(_)) {}
        let take = input.stop_recording().unwrap();
        let _ = std::fs::remove_file(&path);
        let first = take.first_sample as usize;
        assert!(first < 4 * BLOCK, "began at once: {first}");
        assert_eq!(take.samples, samples[first..]);
        assert_eq!(input.stop_recording(), None, "handed over once");
    }
}
