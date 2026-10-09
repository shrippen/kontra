//! Input: a sound device or a file, analyzed on a worker thread and optionally monitored
//! (copied to a player's output).

use crate::clock::Clock;
use crate::player::Player;
use crate::{EngineError, Event};
use kontra_drivers::{read_audio, InputStream, StreamConfig};
use kontra_dsp::synth::laptop_jack;
use kontra_dsp::{peak_db, rms_db, NoteTracker, TrackerConfig, Tuner};
use kontra_model::{InputSource, Instrument};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
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
        let tap = Tap::from_player(monitor);
        match kind {
            InputKind::Device(config) => start_device(
                config, instrument, source, clock, events, stop, tuner_on, tap,
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
        }
    }

    /// Engine time of the input's first sample, once known (device: first callback; file: start
    /// of playback after decoding).
    pub fn started_s(&self) -> Option<f64> {
        let value = f64::from_bits(self.started.load(Ordering::Relaxed));
        (!value.is_nan()).then_some(value)
    }

    /// Largest device block in frames so far (the audio server's quantum), once a block came.
    pub fn block_frames(&self) -> Option<u32> {
        Some(self.block.load(Ordering::Relaxed)).filter(|b| *b > 0)
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
) -> Result<Input, EngineError> {
    // The ring size depends on the sample rate, which we only know after opening; 192 kHz is the cap.
    let (mut producer, mut consumer) = rtrb::RingBuffer::<f32>::new((RING_S * 192_000.0) as usize);
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
        move |samples: &[f32]| {
            if f64::from_bits(started_cb.load(Ordering::Relaxed)).is_nan() {
                // First block: the stream started about one block earlier; calibration absorbs the rest.
                started_cb.store(clock_cb.now_s().to_bits(), Ordering::Relaxed);
            }
            block_cb.fetch_max(
                u32::try_from(samples.len()).unwrap_or(u32::MAX),
                Ordering::Relaxed,
            );
            for &s in samples {
                // A full ring means the analysis fell behind; dropping is better than blocking.
                let _ = producer.push(s);
            }
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
        started: started_input,
        block,
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
        sample_rate,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kontra_drivers::{write_wav, AudioBuffer};

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
}
