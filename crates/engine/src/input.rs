//! Input: a sound device or a file, analyzed on a worker thread.

use crate::clock::Clock;
use crate::{EngineError, Event};
use kontra_drivers::{read_audio, InputStream, StreamConfig};
use kontra_dsp::synth::laptop_jack;
use kontra_dsp::{peak_db, rms_db, NoteTracker, TrackerConfig, Tuner};
use kontra_model::{InputSource, Instrument};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

const RING_S: f64 = 2.0;
const LEVEL_S: f64 = 0.05;
const BLOCK: usize = 480;
const IDLE: Duration = Duration::from_millis(2);
const CLIP_DB: f32 = -0.5;

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
}

/// Running input; dropping it stops the threads.
#[derive(Debug)]
pub struct Input {
    stop: Arc<AtomicBool>,
    tuner_on: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    _stream: Option<InputStream>,
    pub sample_rate: u32,
}

impl Input {
    pub fn start(
        kind: &InputKind,
        instrument: &Instrument,
        source: InputSource,
        clock: &Clock,
        events: Sender<Event>,
    ) -> Result<Input, EngineError> {
        let stop = Arc::new(AtomicBool::new(false));
        let tuner_on = Arc::new(AtomicBool::new(false));
        match kind {
            InputKind::Device(config) => {
                start_device(config, instrument, source, clock, events, stop, tuner_on)
            }
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
            ),
        }
    }

    /// Switches tuner readings on or off.
    pub fn set_tuner(&self, on: bool) {
        self.tuner_on.store(on, Ordering::Relaxed);
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

/// Analysis shared by device and file input.
struct Analysis {
    tracker: NoteTracker,
    tuner: Tuner,
    level: Vec<f32>,
    level_len: usize,
    start_s: f64,
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
        }
    }

    fn process(&mut self, block: &[f32], tuner_on: bool, events: &Sender<Event>) {
        for note in self.tracker.push(block) {
            let mut note = *note;
            note.onset_s += self.start_s;
            let _ = events.send(Event::Note(note));
        }
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
    }

    fn finish(&mut self, events: &Sender<Event>) {
        for note in self.tracker.flush() {
            let mut note = *note;
            note.onset_s += self.start_s;
            let _ = events.send(Event::Note(note));
        }
    }
}

fn start_device(
    config: &StreamConfig,
    instrument: &Instrument,
    source: InputSource,
    clock: &Clock,
    events: Sender<Event>,
    stop: Arc<AtomicBool>,
    tuner_on: Arc<AtomicBool>,
) -> Result<Input, EngineError> {
    // The ring size depends on the sample rate, which we only know after opening; 192 kHz is the cap.
    let (mut producer, mut consumer) = rtrb::RingBuffer::<f32>::new((RING_S * 192_000.0) as usize);
    let started = Arc::new(AtomicU64::new(f64::NAN.to_bits()));
    let started_cb = Arc::clone(&started);
    let clock_cb = clock.clone();
    let error_events = events.clone();
    let stream = InputStream::open(
        config,
        move |samples: &[f32]| {
            if f64::from_bits(started_cb.load(Ordering::Relaxed)).is_nan() {
                // First block: the stream started about one block earlier; calibration absorbs the rest.
                started_cb.store(clock_cb.now_s().to_bits(), Ordering::Relaxed);
            }
            for &s in samples {
                // A full ring means the analysis fell behind; dropping is better than blocking.
                let _ = producer.push(s);
            }
        },
        move |message| {
            let _ = error_events.send(Event::Error(message));
        },
    )?;
    let sample_rate = stream.sample_rate;
    let mut analysis = Analysis::new(f64::from(sample_rate), instrument, source);
    let stop_worker = Arc::clone(&stop);
    let tuner_worker = Arc::clone(&tuner_on);
    let worker = std::thread::Builder::new()
        .name("kontra-analysis".into())
        .spawn(move || {
            let mut block = vec![0.0f32; BLOCK];
            while !stop_worker.load(Ordering::Relaxed) {
                let available = consumer.slots().min(BLOCK);
                if available == 0 {
                    std::thread::sleep(IDLE);
                    continue;
                }
                let start = f64::from_bits(started.load(Ordering::Relaxed));
                if !start.is_nan() {
                    analysis.start_s = start;
                }
                for slot in block.iter_mut().take(available) {
                    *slot = consumer.pop().unwrap_or(0.0);
                }
                analysis.process(
                    &block[..available],
                    tuner_worker.load(Ordering::Relaxed),
                    &events,
                );
            }
        })
        .map_err(EngineError::Thread)?;
    Ok(Input {
        stop,
        tuner_on,
        worker: Some(worker),
        _stream: Some(stream),
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
) -> Result<Input, EngineError> {
    let buffer = read_audio(path)?;
    let sample_rate = buffer.sample_rate;
    let sr = f64::from(sample_rate);
    let mut samples = buffer.mono();
    if simulate_laptop {
        samples = laptop_jack(&samples, sr, 1);
    }
    let mut analysis = Analysis::new(sr, instrument, source);
    let clock = clock.clone();
    let stop_worker = Arc::clone(&stop);
    let tuner_worker = Arc::clone(&tuner_on);
    let worker = std::thread::Builder::new()
        .name("kontra-file-input".into())
        .spawn(move || {
            // Real-time files run on the engine clock; fast ones report file-relative times.
            let started = clock.now_s();
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
                analysis.process(block, tuner_worker.load(Ordering::Relaxed), &events);
            }
            analysis.finish(&events);
            let _ = events.send(Event::Ended);
        })
        .map_err(EngineError::Thread)?;
    Ok(Input {
        stop,
        tuner_on,
        worker: Some(worker),
        _stream: None,
        sample_rate,
    })
}
