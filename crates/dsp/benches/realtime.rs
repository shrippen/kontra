//! Real-time path (E4): what the analysis thread runs per input block, against the block's
//! budget. The engine hands the analysis 480 samples at 48 kHz (10 ms of audio) at a time; a
//! stage that needs more than that falls behind the bass.
//!
//! ```text
//! input ring ─► block (480) ─► NoteTracker::push ─► notes      (Dehum, onset, Yin inside)
//!                          └─► Tuner::push       ─► reading   (tuner page only)
//! microphone ─► EchoCanceller::process (512)      ─► cleaned   (backing on speakers)
//! monitor ─► Shaper::process, per sample           ─► (L, R)    (output callback, LH-EIN-08/-17)
//! ```
//!
//! Run with `cargo bench -p kontra-dsp`. Before Criterion's means it prints the share of the
//! budget per block (mean, p99, worst) from one pass over each signal: the worst block decides
//! whether the analysis keeps up. CI only builds the benches (timing on shared runners is noise).

use criterion::{criterion_group, Criterion, Throughput};
use kontra_dsp::synth::{add_noise, render, SynthNote, ToneShape};
use kontra_dsp::{
    EchoCanceller, Limit, NoteTracker, OnsetConfig, OnsetDetector, OutputLimiter, Shaper, Shaping,
    TrackerConfig, Tuner, Yin, ECHO_BLOCK,
};
use kontra_model::{InputSource, Instrument, Pitch, A4_HZ};
use std::f64::consts::TAU;
use std::hint::black_box;
use std::time::Instant;

const SR: f64 = 48_000.0;
/// The analysis block of the engine (`crates/engine/src/input.rs`).
const BLOCK: usize = 480;
const LINE_S: f64 = 8.0;
const SPACING_S: f64 = 0.25;
const NOTES: [&str; 8] = ["E1", "A1", "D2", "G2", "C2", "B1", "F1", "A2"];
/// Mains hum as in the corpus set `hum-line` (E8): 50 Hz, −40 dBFS, so `Dehum` subtracts.
const HUM_HZ: f64 = 50.0;
const HUM_DB: f32 = -40.0;
const NOISE_DB: f32 = -66.0;
/// Hum alone before the line, longer than the 2 s `Dehum` waits for steady hum.
const HUM_LEAD_S: f64 = 3.0;
/// Yin over the tracker's window for the lowest bass note.
const YIN_WINDOW_S: f64 = 0.06;
const PERCENT: f64 = 100.0;
const P99: f64 = 0.99;
const PASSES: usize = 5;
const BENCH_FLAG: &str = "--bench";

/// A bass line of eighth notes over the whole range of a four-string bass.
fn line() -> Vec<f32> {
    let notes: Vec<SynthNote> = (0..(LINE_S / SPACING_S) as usize)
        .filter_map(|i| {
            let pitch = Pitch::parse(NOTES[i % NOTES.len()])?;
            Some(SynthNote {
                onset_s: 0.2 + i as f64 * SPACING_S,
                hz: pitch.hz(A4_HZ),
                duration_s: SPACING_S * 0.9,
                velocity: 0.7,
            })
        })
        .collect();
    render(&notes, SR, LINE_S, ToneShape::Rich, 7)
}

/// `line()` on mains hum after `HUM_LEAD_S` of hum alone: the hum filter learns it there and
/// subtracts it under the notes (the costlier path).
fn line_on_hum() -> Vec<f32> {
    let mut signal = vec![0.0f32; (HUM_LEAD_S * SR) as usize];
    signal.extend(line());
    let amplitude = 10f64.powf(f64::from(HUM_DB) / 20.0);
    for (i, s) in signal.iter_mut().enumerate() {
        *s += (amplitude * (TAU * HUM_HZ * i as f64 / SR).sin()) as f32;
    }
    add_noise(&mut signal, NOISE_DB, 3);
    signal
}

fn tracker() -> NoteTracker {
    let config = TrackerConfig::for_instrument(&Instrument::default_bass(), InputSource::Line);
    NoteTracker::new(SR, config)
}

fn tuner() -> Tuner {
    let config = TrackerConfig::for_instrument(&Instrument::default_bass(), InputSource::Line);
    Tuner::new(SR, config.min_hz, config.max_hz)
}

/// Blocks of `signal`, round and round: the stage keeps its state as in a long session.
struct Blocks<'a> {
    signal: &'a [f32],
    at: usize,
}

impl<'a> Blocks<'a> {
    fn new(signal: &'a [f32]) -> Blocks<'a> {
        Blocks { signal, at: 0 }
    }

    fn next(&mut self, len: usize) -> &'a [f32] {
        if self.at + len > self.signal.len() {
            self.at = 0;
        }
        let block = &self.signal[self.at..self.at + len];
        self.at += len;
        block
    }
}

/// `PASSES` passes over `signal` in blocks, each with a fresh stage from `make`, every block
/// timed; per block the fastest pass counts (the others lost the CPU to something else). Prints
/// mean, p99 and worst as share of the block budget, e.g.
/// "tracker/line  mean 4.1 %  p99 9.8 %  worst 14.2 %  of 10.0 ms".
fn budget<S: FnMut(&[f32])>(name: &str, signal: &[f32], len: usize, make: impl Fn() -> S) {
    let blocks = signal.len() / len;
    let mut took = vec![f64::INFINITY; blocks];
    for _ in 0..PASSES {
        let mut stage = make();
        for (fastest, block) in took.iter_mut().zip(signal.chunks_exact(len)) {
            let start = Instant::now();
            stage(black_box(block));
            *fastest = fastest.min(start.elapsed().as_secs_f64());
        }
    }
    if took.is_empty() {
        return;
    }
    took.sort_by(f64::total_cmp);

    let budget_s = len as f64 / SR;
    let share = |s: f64| s / budget_s * PERCENT;
    let mean = took.iter().sum::<f64>() / took.len() as f64;
    let p99 = took[((took.len() - 1) as f64 * P99) as usize];
    let worst = took[took.len() - 1];
    println!(
        "{name:<20} mean {:5.1} %  p99 {:5.1} %  worst {:5.1} %  of {:.1} ms",
        share(mean),
        share(p99),
        share(worst),
        budget_s * 1e3
    );
}

fn report() {
    let line = line();
    let hum = line_on_hum();
    println!("Share of the block budget ({BLOCK} samples at {SR} Hz), fastest of {PASSES} passes:");

    budget("tracker/line", &line, BLOCK, || {
        let mut t = tracker();
        move |b: &[f32]| {
            black_box(t.push(b));
        }
    });
    budget("tracker/hum", &hum, BLOCK, || {
        let mut t = tracker();
        move |b: &[f32]| {
            black_box(t.push(b));
        }
    });
    budget("onset/line", &line, BLOCK, || {
        let mut onsets = OnsetDetector::new(SR, OnsetConfig::default());
        move |b: &[f32]| {
            black_box(onsets.push(b));
        }
    });
    budget("tuner/line", &line, BLOCK, || {
        let mut tune = tuner();
        move |b: &[f32]| {
            black_box(tune.push(b));
        }
    });
    budget("echo/line", &line, ECHO_BLOCK, || {
        let mut echo = EchoCanceller::new();
        let mut mic = vec![0.0f32; ECHO_BLOCK];
        let mut played = Blocks::new(&hum);
        move |b: &[f32]| {
            mic.copy_from_slice(b);
            echo.process(&mut mic, played.next(ECHO_BLOCK));
        }
    });
    budget("monitor/line", &line, BLOCK, || {
        let mut shape = shaper();
        move |b: &[f32]| {
            for &x in b {
                black_box(shape.process(x, SR));
            }
        }
    });
    // The limiter on the output sum (LH-WIE-06), working: the line at twice full scale.
    budget("output/limit", &line, BLOCK, || {
        let mut limit = OutputLimiter::new(SR);
        move |b: &[f32]| {
            for &x in b {
                black_box(limit.process((2.0 * x, 2.0 * x), SR));
            }
        }
    });
}

/// Monitor shaping with every stage working: bands, compressor, warmth, overdrive, width and
/// limiter (the output callback runs it per sample).
fn shaper() -> Shaper {
    let mut shaper = Shaper::new(SR);
    shaper.set(Shaping {
        low_db: 4.0,
        mid_db: -3.0,
        high_db: 2.0,
        compress: 0.5,
        warmth: 0.5,
        drive: 0.5,
        width: 0.5,
        limit: Limit::On,
        ..Shaping::FLAT
    });
    shaper
}

fn stages(c: &mut Criterion) {
    let line = line();
    let hum = line_on_hum();
    let mut group = c.benchmark_group("block");
    group.throughput(Throughput::Elements(BLOCK as u64));

    let mut t = tracker();
    let mut blocks = Blocks::new(&line);
    group.bench_function("tracker/line", |b| {
        b.iter(|| black_box(t.push(blocks.next(BLOCK)).len()))
    });

    let mut t = tracker();
    let mut blocks = Blocks::new(&hum);
    group.bench_function("tracker/hum", |b| {
        b.iter(|| black_box(t.push(blocks.next(BLOCK)).len()))
    });

    let mut onsets = OnsetDetector::new(SR, OnsetConfig::default());
    let mut blocks = Blocks::new(&line);
    group.bench_function("onset/line", |b| {
        b.iter(|| black_box(onsets.push(blocks.next(BLOCK)).len()))
    });

    let mut tune = tuner();
    let mut blocks = Blocks::new(&line);
    group.bench_function("tuner/line", |b| {
        b.iter(|| black_box(tune.push(blocks.next(BLOCK))))
    });

    // One pitch estimate over the window the tracker uses for E1 (two of its periods).
    let config = TrackerConfig::for_instrument(&Instrument::default_bass(), InputSource::Line);
    let mut yin = Yin::new(SR, config.min_hz, config.max_hz, YIN_WINDOW_S);
    let window = &line[..yin.needed()];
    group.bench_function("yin/estimate", |b| {
        b.iter(|| black_box(yin.estimate(window)))
    });
    group.finish();

    let mut group = c.benchmark_group("echo");
    group.throughput(Throughput::Elements(ECHO_BLOCK as u64));
    let mut echo = EchoCanceller::new();
    let mut mic = vec![0.0f32; ECHO_BLOCK];
    let mut heard = Blocks::new(&line);
    let mut played = Blocks::new(&hum);
    group.bench_function("process", |b| {
        b.iter(|| {
            mic.copy_from_slice(heard.next(ECHO_BLOCK));
            echo.process(&mut mic, played.next(ECHO_BLOCK));
        })
    });
    group.finish();
}

criterion_group!(benches, stages);

fn main() {
    // `cargo bench` passes `--bench`; `cargo test --benches` runs each bench once, no report.
    if std::env::args().any(|a| a == BENCH_FLAG) {
        report();
    }
    benches();
    Criterion::default().configure_from_args().final_summary();
}
