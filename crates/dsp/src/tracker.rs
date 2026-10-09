//! Turns a sample stream into `DetectedNote`s: an onset opens a note, YIN fills in the pitch
//! once enough post-attack audio exists and two estimates in a row agree (concept 6.3).
//!
//! ```text
//! onset ─┬─ skip attack (12 ms) ─┬─ YIN window (≈ 2 periods of the lowest note) ─┬─ agree? ─► emit
//!        │                       │                                                └─ no: retry every 5 ms
//!        └──────────────── timeout (150 ms) ──────────────────────────────────────────────► emit best
//! ```
//!
//! An onset only counts once the level after it has risen above the level before it
//! (`RISE_*`): a released or ringing note makes spectral changes too, but no new attack.
//! An attack that comes in two bursts (a second onset before the first has a pitch, within
//! `MERGE_S`) is one note: the first onset with the pitch measured after the second.

use crate::intonation::refine_hz;
use crate::level::amplitude_to_db;
use crate::onset::{OnsetConfig, OnsetDetector};
use crate::yin::{PitchEstimate, Yin};
use kontra_model::{cents_between, hz_to_midi, DetectedNote, InputSource, Instrument};

const RETRY_S: f64 = 0.005;
const LEVEL_SPAN_S: f64 = 0.02;
/// Level before an onset: peak over this span, ending `RISE_GAP_S` before it (−20..−8 ms). The
/// gap keeps an onset reported a few ms late from seeing its own attack as "before"; the span is
/// short so that a staccato note before does not count, and still sees ≥ 0.8 of a B0 wave's peak.
const RISE_BEFORE_S: f64 = 0.012;
const RISE_GAP_S: f64 = 0.008;
/// Level after an onset: peak over this span from the onset.
const RISE_AFTER_S: f64 = 0.04;
/// The level after must exceed the level before by this factor (+0.8 dB). Chosen on IDMT and
/// NSynth recordings (testdata/korpus/QUELLEN.md): keeps 90 % of real attacks, drops 62 % of
/// false ones; stricter factors drop few more false onsets but more soft notes.
const RISE_FACTOR: f32 = 1.1;
/// Below this peak the level before counts as silence (any onset is a rise).
const SILENCE: f32 = 1e-4;
/// A second onset this soon after one without pitch belongs to the same attack.
const MERGE_S: f64 = 0.08;

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
    /// Where the pitch search starts (the latest attack burst).
    onset: u64,
    /// The onset reported (the first burst of a merged attack).
    report: u64,
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
    /// An onset waiting for the level after it (`RISE_AFTER_S`), at most one at a time.
    candidate: Option<u64>,
    out: Vec<DetectedNote>,
}

impl NoteTracker {
    pub fn new(sample_rate: f64, config: TrackerConfig) -> NoteTracker {
        // Window = longest period: the shortest window YIN can work with.
        let window_s = 1.0 / config.min_hz;
        let yin = Yin::new(sample_rate, config.min_hz, config.max_hz, window_s);
        let estimate_len = 2 * yin.needed();
        let ring_len = estimate_len
            + ((config.timeout_s + 0.2 + RISE_BEFORE_S + RISE_GAP_S) * sample_rate) as usize;
        NoteTracker {
            onsets: OnsetDetector::new(sample_rate, config.onset),
            config,
            sample_rate,
            yin,
            ring: vec![0.0; ring_len],
            scratch: vec![0.0; estimate_len],
            pushed: 0,
            pending: None,
            candidate: None,
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
                self.check_candidate();
                if let Some(earlier) = self.candidate.replace(onset) {
                    // Two onsets within the rise window: decide the first one now on what exists.
                    self.decide(earlier);
                }
            }
            self.check_candidate();
            self.advance();
        }
        &self.out
    }

    /// Emits a pending note now, e.g. at the end of a file.
    pub fn flush(&mut self) -> &[DetectedNote] {
        self.out.clear();
        if let Some(onset) = self.candidate.take() {
            self.decide(onset);
        }
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

    /// Decides the waiting onset once the audio after it is there.
    fn check_candidate(&mut self) {
        let Some(onset) = self.candidate else {
            return;
        };
        if self.pushed >= onset + (RISE_AFTER_S * self.sample_rate) as u64 {
            self.candidate = None;
            self.decide(onset);
        }
    }

    fn decide(&mut self, onset: u64) {
        if self.level_rises(onset) {
            self.start(onset);
        }
    }

    /// Peak after the onset against the peak just before it.
    fn level_rises(&self, onset: u64) -> bool {
        let sr = self.sample_rate;
        let gap = (RISE_GAP_S * sr) as u64;
        let before_end = onset.saturating_sub(gap);
        let before = self.peak(
            before_end.saturating_sub((RISE_BEFORE_S * sr) as u64),
            before_end,
        );
        let after = self.peak(onset, onset + (RISE_AFTER_S * sr) as u64);
        before < SILENCE || after >= before * RISE_FACTOR
    }

    /// Peak magnitude in `[from, to)`, limited to what the ring still holds.
    fn peak(&self, from: u64, to: u64) -> f32 {
        let len = self.ring.len() as u64;
        let from = from.max(self.pushed.saturating_sub(len));
        let to = to.min(self.pushed);
        (from..to)
            .map(|s| self.ring[(s % len) as usize].abs())
            .fold(0.0f32, f32::max)
    }

    fn start(&mut self, onset: u64) {
        let mut report = onset;
        if let Some(p) = self.pending.take() {
            let merge = (MERGE_S * self.sample_rate) as u64;
            if p.best.is_none() && onset < p.report + merge {
                report = p.report;
            } else {
                self.emit(p);
            }
        }
        let skip = (self.config.attack_skip_s * self.sample_rate) as u64;
        let first_try = onset + skip + self.yin.needed() as u64;
        self.pending = Some(Pending {
            onset,
            report,
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
    /// The plain estimate is used, not `estimate_fundamental` (its low-pass needs warm-up audio a
    /// fresh note does not have), then `refine_hz` removes the stiff-string bias for intonation.
    fn estimate(&mut self, p: &Pending, until: u64) -> Option<PitchEstimate> {
        let skip = (self.config.attack_skip_s * self.sample_rate) as u64;
        let from = (p.onset + skip).max(until.saturating_sub(self.yin.needed() as u64));
        let count = (until - from) as usize;
        self.copy_out(from, count);
        let mut estimate = self.yin.estimate(&self.scratch[..count])?;
        if estimate.confidence < self.config.min_confidence {
            return None;
        }
        if let Some(hz) = refine_hz(&self.scratch[..count], self.sample_rate, estimate.hz) {
            estimate.hz = hz;
        }
        Some(estimate)
    }

    fn copy_out(&mut self, from: u64, count: usize) {
        let len = self.ring.len() as u64;
        for (i, slot) in self.scratch.iter_mut().take(count).enumerate() {
            *slot = self.ring[((from + i as u64) % len) as usize];
        }
    }

    fn emit(&mut self, p: Pending) {
        let level_db = self.level_after(p.report);
        let (midi, confidence) = match p.best {
            Some(b) => (hz_to_midi(b.hz, self.config.a4_hz), b.confidence),
            None => (0.0, 0.0),
        };
        self.out.push(DetectedNote {
            onset_s: p.report as f64 / self.sample_rate,
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
    use crate::synth::{bass_tone, laptop_jack, render, SynthNote, ToneShape};
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
        assert_matches(&track(&signal, InputSource::Line), &notes, 5.0, 3.0);
    }

    #[test]
    fn sixteenths_at_120() {
        let notes = line(&["E1", "E1", "G1", "A1", "E1", "E1", "D2", "E2"], 0.125);
        let signal = render(&notes, SR, 1.5, ToneShape::Rich, 6);
        assert_matches(&track(&signal, InputSource::Line), &notes, 5.0, 3.0);
    }

    #[test]
    fn laptop_jack_signal() {
        let notes = line(&["E1", "A1", "D2", "G2", "C2", "F1"], 0.3);
        let signal = laptop_jack(&render(&notes, SR, 2.3, ToneShape::Dull, 7), SR, 8);
        assert_matches(&track(&signal, InputSource::LaptopJack), &notes, 8.0, 10.0);
    }

    #[test]
    fn hard_stop_is_no_new_note() {
        // A note cut off without fade (a muted string, a sample's end) changes the spectrum
        // like an onset, but the level falls: no second note.
        let hz = Pitch::parse("A1").unwrap().hz(A4_HZ);
        let mut signal = vec![0.0; (0.2 * SR) as usize];
        let tone = bass_tone(hz, SR, 2.0, ToneShape::Rich, 9);
        signal.extend_from_slice(&tone[..(0.6 * SR) as usize]);
        signal.extend(std::iter::repeat_n(0.0, (0.6 * SR) as usize));
        let found = track(&signal, InputSource::Line);
        assert_eq!(found.len(), 1, "found {found:?}");
        assert!((found[0].onset_s - 0.2).abs() < 0.005);
    }

    #[test]
    fn attack_in_two_bursts_is_one_note() {
        // A second burst 40 ms into the attack, before any pitch is known, belongs to the
        // first: one note at the first onset with the right pitch, not a pitchless one plus
        // a second note.
        let hz = Pitch::parse("D1").unwrap().hz(A4_HZ);
        let first = bass_tone(hz, SR, 0.04, ToneShape::Rich, 10);
        let second = bass_tone(hz, SR, 0.8, ToneShape::Rich, 11);
        let mut signal = vec![0.0; (0.2 * SR) as usize];
        signal.extend(first.iter().map(|x| x * 0.4));
        signal.extend(second.iter().map(|x| x * 0.9));
        signal.extend(std::iter::repeat_n(0.0, (0.3 * SR) as usize));
        let found = track(&signal, InputSource::Line);
        assert_eq!(found.len(), 1, "found {found:?}");
        assert!((found[0].onset_s - 0.2).abs() < 0.005, "{found:?}");
        assert!(
            (found[0].midi - hz_to_midi(hz, A4_HZ)).abs() < 0.1,
            "{found:?}"
        );
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
