//! Turns a sample stream into `DetectedNote`s: an onset opens a note, YIN fills in the pitch
//! once enough post-attack audio exists and two estimates in a row agree (concept 6.3).
//!
//! ```text
//! onset ─┬─ skip attack (12 ms) ─┬─ YIN window (≈ 2 periods of the lowest note) ─┬─ agree? ─► emit
//!        │                       │                                                └─ no: retry every 5 ms
//!        └──────────────── timeout (150 ms) ──────────────────────────────────────────────► emit best
//! ```

use crate::level::amplitude_to_db;
use crate::onset::{OnsetConfig, OnsetDetector};
use crate::yin::{PitchEstimate, Yin};
use kontra_model::{cents_between, hz_to_midi, DetectedNote, InputSource, Instrument};

const RETRY_S: f64 = 0.005;
const LEVEL_SPAN_S: f64 = 0.02;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackerConfig {
    pub min_hz: f64,
    pub max_hz: f64,
    pub a4_hz: f64,
    /// Skipped after the onset: the attack noise disturbs pitch estimation.
    pub attack_skip_s: f64,
    /// Two estimates closer than this are accepted.
    pub agree_cents: f64,
    /// Give up waiting for agreement after this time and emit the best estimate.
    pub timeout_s: f64,
    /// Estimates below this confidence are ignored.
    pub min_confidence: f32,
    pub source: InputSource,
    pub onset: OnsetConfig,
}

impl TrackerConfig {
    pub fn for_instrument(instrument: &Instrument, source: InputSource) -> TrackerConfig {
        let (min_hz, max_hz) = instrument.detection_range_hz().unwrap_or((27.0, 420.0));
        TrackerConfig {
            min_hz,
            max_hz,
            a4_hz: instrument.a4_hz,
            attack_skip_s: 0.012,
            agree_cents: 20.0,
            timeout_s: 0.15,
            min_confidence: 0.6,
            source,
            onset: OnsetConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Pending {
    onset: u64,
    last: Option<PitchEstimate>,
    best: Option<PitchEstimate>,
    next_try: u64,
}

/// Streaming note tracker; allocation-free after `new`.
#[derive(Debug)]
pub struct NoteTracker {
    config: TrackerConfig,
    sample_rate: f64,
    onsets: OnsetDetector,
    yin: Yin,
    ring: Vec<f32>,
    scratch: Vec<f32>,
    pushed: u64,
    pending: Option<Pending>,
    out: Vec<DetectedNote>,
}

impl NoteTracker {
    pub fn new(sample_rate: f64, config: TrackerConfig) -> NoteTracker {
        // Window = longest period: the shortest window YIN can work with.
        let window_s = 1.0 / config.min_hz;
        let yin = Yin::new(sample_rate, config.min_hz, config.max_hz, window_s);
        let estimate_len = 2 * yin.needed();
        let ring_len = estimate_len + ((config.timeout_s + 0.2) * sample_rate) as usize;
        NoteTracker {
            onsets: OnsetDetector::new(sample_rate, config.onset),
            config,
            sample_rate,
            yin,
            ring: vec![0.0; ring_len],
            scratch: vec![0.0; estimate_len],
            pushed: 0,
            pending: None,
            out: Vec::with_capacity(16),
        }
    }

    pub fn config(&self) -> &TrackerConfig {
        &self.config
    }

    /// Typical delay from attack to emitted note for the lowest note of the range.
    pub fn typical_delay_s(&self) -> f64 {
        self.config.attack_skip_s
            + (self.yin.needed() as f64 + RETRY_S * self.sample_rate) / self.sample_rate
    }

    /// Feeds samples; returns notes completed in this call (valid until the next call).
    pub fn push(&mut self, samples: &[f32]) -> &[DetectedNote] {
        self.out.clear();
        let block = (RETRY_S * self.sample_rate) as usize;
        for chunk in samples.chunks(block.max(1)) {
            self.store(chunk);
            let count = self.onsets.push(chunk).len();
            for i in 0..count {
                let onset = self.onsets.last_found()[i].sample;
                self.start(onset);
            }
            self.advance();
        }
        &self.out
    }

    /// Emits a pending note now, e.g. at the end of a file.
    pub fn flush(&mut self) -> &[DetectedNote] {
        self.out.clear();
        if let Some(p) = self.pending.take() {
            self.emit(p);
        }
        &self.out
    }

    fn store(&mut self, chunk: &[f32]) {
        let len = self.ring.len() as u64;
        for &x in chunk {
            self.ring[(self.pushed % len) as usize] = x;
            self.pushed += 1;
        }
    }

    fn start(&mut self, onset: u64) {
        if let Some(p) = self.pending.take() {
            self.emit(p);
        }
        let skip = (self.config.attack_skip_s * self.sample_rate) as u64;
        let first_try = onset + skip + self.yin.needed() as u64;
        self.pending = Some(Pending {
            onset,
            last: None,
            best: None,
            next_try: first_try,
        });
    }

    fn advance(&mut self) {
        let Some(mut p) = self.pending else {
            return;
        };
        let timeout = p.onset + (self.config.timeout_s * self.sample_rate) as u64;
        while p.next_try <= self.pushed {
            if let Some(estimate) = self.estimate(&p, p.next_try) {
                if p.best.is_none_or(|b| estimate.confidence > b.confidence) {
                    p.best = Some(estimate);
                }
                let agrees = p.last.is_some_and(|l| {
                    cents_between(l.hz, estimate.hz).abs() < self.config.agree_cents
                });
                if agrees {
                    p.best = Some(estimate);
                    self.pending = None;
                    self.emit(p);
                    return;
                }
                p.last = Some(estimate);
            }
            p.next_try += (RETRY_S * self.sample_rate) as u64;
        }
        if self.pushed >= timeout {
            self.pending = None;
            self.emit(p);
            return;
        }
        self.pending = Some(p);
    }

    /// YIN over the newest `needed()` samples before `until` (all after the attack).
    ///
    /// The plain estimate is used, not `estimate_fundamental`: its low-pass needs warm-up audio
    /// that a fresh note does not have yet. Stiff strings make the plain estimate read up to
    /// ~8 cents sharp on the lowest notes, well inside the scoring tolerance.
    fn estimate(&mut self, p: &Pending, until: u64) -> Option<PitchEstimate> {
        let skip = (self.config.attack_skip_s * self.sample_rate) as u64;
        let from = (p.onset + skip).max(until.saturating_sub(self.yin.needed() as u64));
        let count = (until - from) as usize;
        self.copy_out(from, count);
        let estimate = self.yin.estimate(&self.scratch[..count])?;
        (estimate.confidence >= self.config.min_confidence).then_some(estimate)
    }

    fn copy_out(&mut self, from: u64, count: usize) {
        let len = self.ring.len() as u64;
        for (i, slot) in self.scratch.iter_mut().take(count).enumerate() {
            *slot = self.ring[((from + i as u64) % len) as usize];
        }
    }

    fn emit(&mut self, p: Pending) {
        let level_db = self.level_after(p.onset);
        let (midi, confidence) = match p.best {
            Some(b) => (hz_to_midi(b.hz, self.config.a4_hz), b.confidence),
            None => (0.0, 0.0),
        };
        self.out.push(DetectedNote {
            onset_s: p.onset as f64 / self.sample_rate,
            midi,
            confidence,
            level_db,
            source: self.config.source,
        });
    }

    fn level_after(&self, onset: u64) -> f32 {
        let len = self.ring.len() as u64;
        let span = (LEVEL_SPAN_S * self.sample_rate) as u64;
        let end = (onset + span).min(self.pushed);
        let start = onset.max(self.pushed.saturating_sub(len));
        let peak = (start..end)
            .map(|s| self.ring[(s % len) as usize].abs())
            .fold(0.0f32, f32::max);
        amplitude_to_db(peak)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::{laptop_jack, render, SynthNote, ToneShape};
    use kontra_model::{BassKind, Pitch, Tuning, A4_HZ};

    const SR: f64 = 48_000.0;

    fn bass5() -> Instrument {
        Instrument::new(BassKind::Fretted, Tuning::bass5_standard(), 24).unwrap()
    }

    fn line(names: &[&str], spacing_s: f64) -> Vec<SynthNote> {
        names
            .iter()
            .enumerate()
            .map(|(i, n)| SynthNote {
                onset_s: 0.2 + i as f64 * spacing_s,
                hz: Pitch::parse(n).unwrap().hz(A4_HZ),
                duration_s: spacing_s * 0.95,
                velocity: 0.7,
            })
            .collect()
    }

    fn track(signal: &[f32], source: InputSource) -> Vec<DetectedNote> {
        let mut t = NoteTracker::new(SR, TrackerConfig::for_instrument(&bass5(), source));
        let mut out = Vec::new();
        for block in signal.chunks(480) {
            out.extend_from_slice(t.push(block));
        }
        out.extend_from_slice(t.flush());
        out
    }

    fn assert_matches(found: &[DetectedNote], notes: &[SynthNote], max_ms: f64, max_cents: f64) {
        assert_eq!(found.len(), notes.len(), "found {found:?}");
        for (f, n) in found.iter().zip(notes) {
            let expected = hz_to_midi(n.hz, A4_HZ);
            let ms = (f.onset_s - n.onset_s) * 1000.0;
            let cents = (f.midi - expected) * 100.0;
            assert!(ms.abs() < max_ms, "{}: {ms:.1} ms", n.hz);
            assert!(cents.abs() < max_cents, "{}: {cents:.1} cents", n.hz);
        }
    }

    #[test]
    fn walking_line_on_five_string() {
        let notes = line(
            &[
                "B0", "D1", "E1", "F#1", "G1", "A1", "B1", "D2", "E2", "G2", "C3", "B0",
            ],
            0.25,
        );
        let signal = render(&notes, SR, 3.6, ToneShape::Rich, 5);
        assert_matches(&track(&signal, InputSource::Line), &notes, 5.0, 10.0);
    }

    #[test]
    fn sixteenths_at_120() {
        let notes = line(&["E1", "E1", "G1", "A1", "E1", "E1", "D2", "E2"], 0.125);
        let signal = render(&notes, SR, 1.5, ToneShape::Rich, 6);
        assert_matches(&track(&signal, InputSource::Line), &notes, 5.0, 10.0);
    }

    #[test]
    fn laptop_jack_signal() {
        let notes = line(&["E1", "A1", "D2", "G2", "C2", "F1"], 0.3);
        let signal = laptop_jack(&render(&notes, SR, 2.3, ToneShape::Dull, 7), SR, 8);
        assert_matches(&track(&signal, InputSource::LaptopJack), &notes, 8.0, 10.0);
    }

    #[test]
    fn emits_within_eighty_ms_for_low_e() {
        let t = NoteTracker::new(
            SR,
            TrackerConfig::for_instrument(&Instrument::default_bass(), InputSource::Line),
        );
        assert!(t.typical_delay_s() < 0.08, "{}", t.typical_delay_s());
    }
}
