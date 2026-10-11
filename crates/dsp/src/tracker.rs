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
//!
//! While a note rings, its pitch is followed (LH-BEW-06): a new pitch without a new attack is a
//! hammer-on (up) or pull-off (down), one reached through pitches in between a slide.
//!
//! ```text
//! emitted note ─► every 10 ms: level still ≥ −26 dB of its start? ─► YIN ─► ≥ 80 ct away,
//!                                                                          stable twice
//!                                          in-between pitches seen? ─ yes: Slide ─┐
//!                                                                     no: up/down ┴► emit
//! ```

use crate::intonation::refine_hz;
use crate::level::amplitude_to_db;
use crate::onset::{OnsetConfig, OnsetDetector};
use crate::yin::{PitchEstimate, Yin};
use kontra_model::{cents_between, hz_to_midi, DetectedNote, InputSource, Instrument, Technique};

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
/// A ghost note has nearly gone (below this share of its attack) this soon after it.
const GHOST_AFTER_S: f64 = 0.06;
const GHOST_LEFT: f32 = 0.1;
/// How often a ringing note's pitch is checked.
const FOLLOW_STEP_S: f64 = 0.01;
/// A ringing note is followed while its level stays above this share of its start (−26 dB).
const FOLLOW_FLOOR: f32 = 0.05;
/// A new pitch at least this far (cents) from the ringing one is a new note …
const LEGATO_CENTS: f64 = 80.0;
/// … pitches between this and `LEGATO_CENTS` on the way there make it a slide.
const GLIDE_CENTS: f64 = 30.0;
/// An octave jump of the estimate is a pitch error, not a played note (±this many cents).
const OCTAVE_SLACK_CENTS: f64 = 100.0;
/// In-between estimates that make a pitch change a slide. The YIN window (≈ 50 ms) spans both
/// pitches of a hammer-on for a few estimates too: technique corpus 1–4 for hammer-ons and
/// pull-offs, 7–9 for 80 ms slides.
const SLIDE_BETWEEN: usize = 6;
/// At a microphone reverb and the band let the pitch wander for a moment: the new pitch must
/// hold longer there.
/// At a microphone reverb and the band let the pitch wander for a moment: the new pitch must
/// hold longer there.
/// A legato note waits this long: an attack within it means the pitch changed for a pluck
/// whose onset was still being checked (`RISE_AFTER_S`).
const LEGATO_HOLD_S: f64 = 0.06;
/// Estimates of the new pitch that must agree (80 ms) before it counts. IDMT recordings: 2 gave
/// 62 % precision (vibrato and decays moved the pitch), 8 keeps 78 % and finds real legato notes
/// (recall 82.5 → 87.9 %); reverb at a microphone needs ≥ 6.
const LEGATO_AGREE: usize = 8;

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
        // Noise gate at a microphone (LH-EIN-11): an attack must stand out of the room's
        // noise (fan, street at about −50 dBFS), not only out of silence.
        let onset = match source {
            InputSource::Microphone => OnsetConfig {
                gate_db: MICROPHONE_GATE_DB,
                ..OnsetConfig::default()
            },
            _ => OnsetConfig::default(),
        };
        TrackerConfig {
            min_hz,
            max_hz,
            a4_hz: instrument.a4_hz,
            attack_skip_s: 0.012,
            agree_cents: 20.0,
            timeout_s: 0.15,
            min_confidence: 0.6,
            source,
            onset,
        }
    }
}

/// Attack brightness (energy of the first difference over the energy, first 8 ms) above which
/// the attack was slapped or popped (LH-BEW-06). Technique corpus: plucks ≤ 0.077 (also on line
/// sets), pops ≥ 0.14, slaps ≥ 0.42.
const STRIKE_BRIGHTNESS: f32 = 0.12;
const BRIGHTNESS_S: f64 = 0.008;
/// From here up a bright attack is a pop (index finger on the high strings), below a slap.
const POP_FROM_MIDI: f64 = 38.0;

/// Attacks at a microphone must reach this level (dBFS) in the high band.
const MICROPHONE_GATE_DB: f32 = -48.0;

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

/// A note that rings on after it was emitted, whose pitch is followed.
#[derive(Debug, Clone, Copy)]
struct Sounding {
    midi: f64,
    peak: f32,
    next_try: u64,
    change: Option<Change>,
}

/// A pitch on the way away from the ringing one.
#[derive(Debug, Clone, Copy)]
struct Change {
    /// End of the first window that heard it.
    first: u64,
    midi: f64,
    agreeing: usize,
    /// Estimates between the old and the new pitch.
    between: usize,
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
    sounding: Option<Sounding>,
    /// A legato note waiting for `LEGATO_HOLD_S`, and the sample it is due at.
    legato: Option<(DetectedNote, u64)>,
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
            sounding: None,
            legato: None,
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
            self.follow();
            self.release_legato();
        }
        &self.out
    }

    /// Emits a pending note now, e.g. at the end of a file.
    pub fn flush(&mut self) -> &[DetectedNote] {
        self.out.clear();
        if let Some((note, _)) = self.legato.take() {
            self.out.push(note);
        }
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
        // A new attack ends the ringing note; a legato note just before it was this attack.
        self.sounding = None;
        self.legato = None;
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
    /// `refine_hz` gets all audio since the attack (up to the scratch length): it needs 2.25
    /// periods, more than `needed()` holds for notes near `min_hz` (B0 −20 ct on a 5-string),
    /// which then kept the bias (+8 ct).
    fn estimate(&mut self, p: &Pending, until: u64) -> Option<PitchEstimate> {
        let skip = (self.config.attack_skip_s * self.sample_rate) as u64;
        let from = (p.onset + skip).max(until.saturating_sub(self.scratch.len() as u64));
        let count = (until - from) as usize;
        self.copy_out(from, count);
        let yin_from = count.saturating_sub(self.yin.needed());
        let mut estimate = self.yin.estimate(&self.scratch[yin_from..count])?;
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
        if p.best.is_some() {
            let peak = 10f32.powf(level_db / 20.0);
            let step = (FOLLOW_STEP_S * self.sample_rate) as u64;
            self.sounding = Some(Sounding {
                midi,
                peak,
                next_try: self.pushed.max(p.next_try) + step,
                change: None,
            });
        }
        let technique = match p.best {
            // An attack without a pitch, or one that dies at once: a muted (ghost) note.
            None => Some(Technique::Ghost),
            Some(_) if self.dies_at_once(p.report) => Some(Technique::Ghost),
            // At a microphone room noise brightens soft plucks (corpus "room-double": up to
            // 0.4); slaps and pops are told only on a cable.
            Some(_)
                if self.config.source != InputSource::Microphone
                    && self.brightness(p.report) > STRIKE_BRIGHTNESS =>
            {
                Some(if midi >= POP_FROM_MIDI {
                    Technique::Pop
                } else {
                    Technique::Slap
                })
            }
            Some(_) => None,
        };
        self.out.push(DetectedNote {
            onset_s: p.report as f64 / self.sample_rate,
            midi,
            confidence,
            level_db,
            source: self.config.source,
            technique,
        });
    }

    /// True if the note at `onset` has nearly gone `GHOST_AFTER_S` later (a muted string).
    fn dies_at_once(&self, onset: u64) -> bool {
        let sr = self.sample_rate;
        let attack = self.peak(onset, onset + (LEVEL_SPAN_S * sr) as u64);
        let at = onset + (GHOST_AFTER_S * sr) as u64;
        if self.pushed < at + (LEVEL_SPAN_S * sr) as u64 {
            return false;
        }
        self.peak(at, at + (LEVEL_SPAN_S * sr) as u64) < attack * GHOST_LEFT
    }

    /// Follows the ringing note's pitch; emits a legato note when it moves to a new one.
    fn follow(&mut self) {
        if self.pending.is_some() {
            return;
        }
        let Some(mut s) = self.sounding else {
            return;
        };
        let step = (FOLLOW_STEP_S * self.sample_rate) as u64;
        let span = (LEVEL_SPAN_S * self.sample_rate) as u64;
        let window = self.yin.needed() as u64;
        while s.next_try <= self.pushed {
            let until = s.next_try;
            s.next_try += step;
            if self.peak(until.saturating_sub(span), until) < s.peak * FOLLOW_FLOOR {
                self.sounding = None;
                return;
            }
            let Some(estimate) = self.estimate_until(until) else {
                continue;
            };
            let midi = hz_to_midi(estimate.hz, self.config.a4_hz);
            let away = (midi - s.midi).abs() * 100.0;
            if (away - 1200.0).abs() < OCTAVE_SLACK_CENTS
                || (away - 2400.0).abs() < OCTAVE_SLACK_CENTS
            {
                continue;
            }
            if away < GLIDE_CENTS {
                s.change = None;
                continue;
            }
            let far = away >= LEGATO_CENTS;
            let change = s.change.get_or_insert(Change {
                first: until,
                midi,
                agreeing: 0,
                between: 0,
            });
            if !far {
                change.between += 1;
                change.midi = midi;
                change.agreeing = 0;
                continue;
            }
            if (midi - change.midi).abs() * 100.0 < self.config.agree_cents {
                change.agreeing += 1;
            } else {
                // Still moving: a slide on its way.
                change.between += 1;
                change.midi = midi;
                change.agreeing = 1;
            }
            if change.agreeing < LEGATO_AGREE {
                continue;
            }
            let technique = if change.between >= SLIDE_BETWEEN {
                Technique::Slide
            } else if midi > s.midi {
                Technique::HammerOn
            } else {
                Technique::PullOff
            };
            let onset = change.first.saturating_sub(window / 2);
            let level_db = self.level_after(until.saturating_sub(span));
            // Held back: an attack right after it means a plucked note, not a legato one.
            self.legato = Some((
                DetectedNote {
                    onset_s: onset as f64 / self.sample_rate,
                    midi,
                    confidence: estimate.confidence,
                    level_db,
                    source: self.config.source,
                    technique: Some(technique),
                },
                until + (LEGATO_HOLD_S * self.sample_rate) as u64,
            ));
            s.midi = midi;
            s.change = None;
        }
        self.sounding = Some(s);
    }

    /// Emits a held-back legato note once no attack came within `LEGATO_HOLD_S`.
    fn release_legato(&mut self) {
        if let Some((note, due)) = self.legato {
            if self.pushed >= due {
                self.legato = None;
                self.out.push(note);
            }
        }
    }

    /// YIN over the `needed()` samples before `until` (a ringing note: no attack to skip).
    fn estimate_until(&mut self, until: u64) -> Option<PitchEstimate> {
        let count = self.yin.needed();
        let from = until.checked_sub(count as u64)?;
        self.copy_out(from, count);
        let estimate = self.yin.estimate(&self.scratch[..count])?;
        (estimate.confidence >= self.config.min_confidence).then_some(estimate)
    }

    /// How bright the attack at `onset` is: energy of the first difference over the energy in
    /// the first `BRIGHTNESS_S`. A slap or pop has a hard transient, a finger pluck does not.
    fn brightness(&self, onset: u64) -> f32 {
        let len = self.ring.len() as u64;
        let span = (BRIGHTNESS_S * self.sample_rate) as u64;
        let start = onset.max(self.pushed.saturating_sub(len));
        let end = (onset + span).min(self.pushed);
        let (mut energy, mut change, mut previous) = (0.0f32, 0.0f32, None);
        for s in start..end {
            let x = self.ring[(s % len) as usize];
            energy += x * x;
            if let Some(p) = previous {
                let d: f32 = x - p;
                change += d * d;
            }
            previous = Some(x);
        }
        change / energy.max(f32::MIN_POSITIVE)
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
    use kontra_model::{InstrumentKind, Pitch, Tuning, A4_HZ};

    const SR: f64 = 48_000.0;

    fn bass5() -> Instrument {
        Instrument::new(InstrumentKind::Fretted, Tuning::bass5_standard(), 24).unwrap()
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
    fn low_b_cents_off_pitch() {
        // J-B4 (P3, fretless): B0 below its pitch read +8 ct off (−20 ct as −11.7).
        let base = Pitch::parse("B0").unwrap().hz(A4_HZ);
        let notes: Vec<SynthNote> = [-40.0, -20.0, 0.0, 20.0]
            .iter()
            .enumerate()
            .map(|(i, cents)| SynthNote {
                onset_s: 0.2 + i as f64 * 0.5,
                hz: base * 2f64.powf(cents / 1200.0),
                duration_s: 0.45,
                velocity: 0.7,
            })
            .collect();
        let signal = render(&notes, SR, 2.3, ToneShape::Rich, 12);
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
