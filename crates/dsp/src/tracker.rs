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
//! `MERGE_S`) is one note: the first onset with the pitch measured after the second. A faint
//! burst ≥ 12 dB below the attack after it is a precursor (the fretting finger tapping the
//! string): the note starts at the attack.
//!
//! While a note rings, its pitch is followed (LH-BEW-06): a new pitch without a new attack is a
//! hammer-on (up) or pull-off (down), one reached through pitches in between a slide. A slide
//! over one fret is a step like a hammer-on (IDMT: 0–22 ms, a synthetic one 2–24 ms) and is
//! told as one; over several frets the pitch rests on each fret on the way.
//!
//! ```text
//! emitted note ─► every 10 ms: level still ≥ −26 dB of its start? ─► YIN ─► ≥ 80 ct away,
//!                                                                          stable twice
//!              in-between pitches seen or a fret rested on? ─ yes: Slide ─┐
//!                                                             no: up/down ┴► emit
//! ```
//!
//! A note goes out as soon as its pitch is known, but some tones start up to a semitone flat or
//! with a component an octave below, and settle within about 100 ms (decision round 11). The
//! estimates followed 90–160 ms after the onset tell the settled pitch; one a semitone or an
//! octave higher than emitted becomes a `PitchRevision`:
//!
//! ```text
//! onset ─ emit (12–66 ms) ─ follow 90–160 ms ─► median ≥ 50 ct higher, on the semitone or
//!                                               octave above, half the estimates near it? ─► revise
//! ```
//!
//! A note goes out before the full window when a short one shows its period unambiguously
//! (R1, `tracker/early.rs`) or holds a pitch the tab expects (R2, `tracker/expect.rs`); the
//! full window still has the last word, and a note that proves a precursor is taken back.
//! An expected double stop gets a verdict on which of its tones sound (`tracker/double.rs`,
//! R8).

use crate::dehum::Dehum;
use crate::hum::HumFilter;
use crate::intonation::refine_hz;
use crate::level::amplitude_to_db;
use crate::onset::{OnsetConfig, OnsetDetector};
use crate::strike::{verdict_after_s, Strike};
use crate::yin::{PitchEstimate, Yin};
use kontra_model::{
    cents_between, hz_to_midi, DetectedNote, DoubleStopVerdict, ExpectedStrike, InputSource,
    Instrument, NoteRetraction, PitchRevision, StringVerdict, Technique, TechniqueRevision,
};
use std::ops::Range;

mod background;
mod double;
mod early;
mod expect;
mod settle;
mod strings;

use settle::{Settle, Steps, SETTLE_FROM_S, SETTLE_TO_S};

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
/// … unless it is this much louder (+12 dB): then the first was a precursor, e.g. the fretting
/// finger tapping the string 50–70 ms before the pluck (hardware check 13.10.: 22–28 dB below
/// it), and the note starts at the second. Two bursts of one attack differ by less.
const PRECURSOR_FACTOR: f32 = 4.0;
/// A precursor lasts at least this long before the pluck; a plain attack reaches 12 dB below its
/// peak sooner.
const PRECURSOR_MIN_S: f64 = 0.01;
/// How far after an onset a pluck without an onset of its own is looked for: a later one has its
/// own onset (`min_interval_s` 35 ms). Looked at before the first pitch estimate, which needs more
/// audio anyway (≈ 87 ms): no delay.
const PRECURSOR_LOOK_S: f64 = 0.06;
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
/// Agreeing estimates (≥ 80 ct away) that make a resting point on the way a fret passed. IDMT
/// slides over two frets rest 20–40 ms on the fret between (2–4 estimates); a hammer-on over two
/// frets passes it in one.
const FRET_STEP_AGREE: usize = 2;
/// How far the pitch must move on from a resting point to have passed a fret (half a fret).
const FRET_STEP_CENTS: f64 = 50.0;
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
    /// The strings for telling which one a note was played on (R7); `None`: not told.
    pub strings: Option<crate::StringModel>,
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
            InputSource::LaptopJack => OnsetConfig {
                margin: LAPTOP_MARGIN,
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
            strings: crate::StringModel::for_instrument(instrument),
        }
    }
}

/// Hum notches for inputs that pick up mains hum: the laptop jack (charger ground loop).
fn hum_filter(source: InputSource, sample_rate: f64) -> Option<HumFilter> {
    (source == InputSource::LaptopJack).then(|| HumFilter::new(sample_rate, HUM_WIDTH_HZ))
}

/// Notes emitted before their attack could be judged (`Strike`), waiting for the audio; more at
/// once would mean notes closer than 15 ms.
const STRIKES_WAITING: usize = 8;

/// Attacks at a microphone must reach this level (dBFS) in the high band.
const MICROPHONE_GATE_DB: f32 = -48.0;

/// Onset margin at the laptop jack. The jack's impedance cuts the treble and its noise floor
/// is high, so a soft attack rises in few bins of the onset band: on the IDMT laptop
/// simulation 8 found 56 % of the notes, 2 finds 87 % at higher precision (testdata/korpus/
/// QUELLEN.md). At the line input 2 would cost 5 points of precision.
const LAPTOP_MARGIN: f32 = 2.0;

/// Hum notch width in the tracker. Unlike the tuner it reads right after the attack, where a
/// wide notch still rings at 50 Hz and pulls G2 and C2 an octave down (3 Hz and 1 Hz did);
/// 0.5 Hz rings 0.6 s but faintly, and still takes a grid off by 0.05 Hz down by 14 dB.
const HUM_WIDTH_HZ: f64 = 0.5;

#[derive(Debug, Clone, Copy)]
struct Pending {
    /// Where the pitch search starts (the latest attack burst).
    onset: u64,
    /// The onset reported (the first burst of a merged attack).
    report: u64,
    last: Option<PitchEstimate>,
    best: Option<PitchEstimate>,
    next_try: u64,
    /// Whether the onset was checked for a precursor (`attack_start`).
    checked: bool,
    /// The last early estimate (`try_early`).
    early: Option<PitchEstimate>,
}

/// A note that rings on after it was emitted, whose pitch is followed.
#[derive(Debug, Clone, Copy)]
struct Sounding {
    midi: f64,
    peak: f32,
    next_try: u64,
    change: Option<Change>,
    /// The struck note's settled pitch being heard, until it is told.
    settle: Option<Settle>,
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
    /// Where the pitch last rested on the way (`FRET_STEP_AGREE` estimates) …
    rest: Option<f64>,
    /// … and moved on from by half a fret or more: a fret passed, a slide over several frets.
    stepped: bool,
}

impl Change {
    fn new(first: u64, midi: f64) -> Change {
        Change {
            first,
            midi,
            agreeing: 0,
            between: 0,
            rest: None,
            stepped: false,
        }
    }

    /// Takes the next estimate (≥ `GLIDE_CENTS` away from the ringing pitch `from`).
    fn hear(&mut self, midi: f64, from: f64, agree_cents: f64) {
        if (midi - from).abs() * 100.0 < LEGATO_CENTS {
            self.between += 1;
            self.midi = midi;
            self.agreeing = 0;
            return;
        }
        if (midi - self.midi).abs() * 100.0 < agree_cents {
            self.agreeing += 1;
            return;
        }

        // Still moving: a slide on its way. Moving on by half a fret or more after resting is a
        // fret passed (fretted slides step fret by fret, they do not glide).
        if self.agreeing >= FRET_STEP_AGREE {
            self.rest = Some(self.midi);
        }
        let cents_from = |m: f64| (m - from).abs() * 100.0;
        self.stepped |= self
            .rest
            .is_some_and(|rest| cents_from(midi) >= cents_from(rest) + FRET_STEP_CENTS);
        self.between += 1;
        self.midi = midi;
        self.agreeing = 1;
    }

    /// A slide when pitches or a fret lay on the way, else a hammer-on (up) or pull-off (down).
    fn technique(&self, from: f64) -> Technique {
        if self.between >= SLIDE_BETWEEN || self.stepped {
            return Technique::Slide;
        }
        if self.midi > from {
            return Technique::HammerOn;
        }
        Technique::PullOff
    }
}

/// Whether waiting attacks wait for their audio or are judged on what exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Waiting {
    ForAudio,
    No,
}

/// Streaming note tracker; allocation-free after `new`.
#[derive(Debug)]
pub struct NoteTracker {
    config: TrackerConfig,
    sample_rate: f64,
    onsets: OnsetDetector,
    /// Mains hum removed before anything else, where the input picks it up (laptop jack) …
    hum: Option<HumFilter>,
    /// … and on every input followed and subtracted (E8).
    dehum: Dehum,
    /// The current block after `hum` and `dehum`.
    filtered: Vec<f32>,
    /// The input before `hum` (empty without), for `refine_hz`: the notches ring after an
    /// attack and moved the fine pitch by up to 25 ct (synthetic laptop set: p95 4.6 → 18.7 ct).
    raw_ring: Vec<f32>,
    raw_scratch: Vec<f32>,
    /// The input before `dehum` (after `hum`), for the background (`stands_out`): a change of
    /// the hum's level leaves a step after the subtraction, but none in the hum heard.
    heard: Vec<f32>,
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
    /// Onsets (samples) of emitted notes whose attack is judged once its audio is there.
    strikes: [Option<u64>; STRIKES_WAITING],
    revised: Vec<TechniqueRevision>,
    pitch_revised: Vec<PitchRevision>,
    retracted: Vec<NoteRetraction>,
    /// Strikes the tab expects, in samples (R2).
    expected: expect::Expectations,
    /// A note told early, checked once its full window is there.
    confirm: Option<expect::Confirm>,
    /// Notes waiting for the string they were played on (R7).
    strings: strings::Strings,
    /// Expected double stops waiting for their verdict (R8).
    double_stops: double::DoubleStops,
}

impl NoteTracker {
    pub fn new(sample_rate: f64, config: TrackerConfig) -> NoteTracker {
        // Window = longest period: the shortest window YIN can work with.
        let window_s = 1.0 / config.min_hz;
        let yin = Yin::new(sample_rate, config.min_hz, config.max_hz, window_s);
        let estimate_len = 2 * yin.needed();
        let ring_len = estimate_len
            + ((config.timeout_s + 0.2 + RISE_BEFORE_S + RISE_GAP_S) * sample_rate) as usize;
        let hum = hum_filter(config.source, sample_rate);
        let raw_len = |len: usize| if hum.is_some() { len } else { 0 };
        NoteTracker {
            onsets: OnsetDetector::new(sample_rate, config.onset),
            dehum: Dehum::new(sample_rate),
            hum,
            filtered: Vec::with_capacity((RETRY_S * sample_rate) as usize + 1),
            raw_ring: vec![0.0; raw_len(ring_len)],
            raw_scratch: vec![0.0; raw_len(estimate_len)],
            heard: vec![0.0; ring_len],
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
            strikes: [None; STRIKES_WAITING],
            revised: Vec::with_capacity(STRIKES_WAITING),
            pitch_revised: Vec::with_capacity(STRIKES_WAITING),
            retracted: Vec::with_capacity(STRIKES_WAITING),
            expected: expect::Expectations::default(),
            confirm: None,
            strings: strings::Strings::new(config.strings, sample_rate),
            double_stops: double::DoubleStops::new(sample_rate),
        }
    }

    /// A strike the tab expects; `onset_s` counts from the first sample pushed. Kept until it
    /// lies behind the input; when more are waiting than fit, the earliest gives way.
    pub fn expect(&mut self, strike: &ExpectedStrike) {
        self.expected.add(strike, self.sample_rate, self.pushed);
    }

    /// Drops the expected strikes (the transport jumped, the plan changed).
    pub fn forget_expected(&mut self) {
        self.expected.clear();
    }

    pub fn config(&self) -> &TrackerConfig {
        &self.config
    }

    /// Typical delay from attack to emitted note for the lowest note of the range.
    pub fn typical_delay_s(&self) -> f64 {
        self.config.attack_skip_s
            + (self.yin.needed() as f64 + RETRY_S * self.sample_rate) / self.sample_rate
    }

    /// Techniques told in the last `push` or `flush` for notes emitted earlier (their attack
    /// needed more audio, `Strike`); valid until the next call.
    pub fn revised(&self) -> &[TechniqueRevision] {
        &self.revised
    }

    /// Pitches told in the last `push` or `flush` for notes emitted earlier, once their tone
    /// settled (decision round 11); valid until the next call.
    pub fn pitch_revised(&self) -> &[PitchRevision] {
        &self.pitch_revised
    }

    /// Strings told in the last `push` or `flush` for notes emitted earlier (R7); valid until
    /// the next call.
    pub fn string_verdicts(&self) -> &[StringVerdict] {
        self.strings.told()
    }

    /// Takes no onset at `samples` (counted from the first sample pushed): see
    /// `OnsetDetector::mute`.
    pub fn mute_onsets(&mut self, samples: Range<u64>) {
        self.onsets.mute(samples);
    }

    /// Notes completed in the last `push` or `flush` (what they returned); valid until the next
    /// call.
    pub fn told(&self) -> &[DetectedNote] {
        &self.out
    }

    /// Notes taken back in the last `push` or `flush`: told early, then shown to be no note of
    /// their own (R1); valid until the next call.
    pub fn retracted(&self) -> &[NoteRetraction] {
        &self.retracted
    }

    /// Which tones of expected double stops sounded, told in the last `push` or `flush` for
    /// notes emitted earlier (R8); valid until the next call.
    pub fn double_stops(&self) -> &[DoubleStopVerdict] {
        self.double_stops.told()
    }

    /// Feeds samples; returns notes completed in this call (valid until the next call).
    pub fn push(&mut self, samples: &[f32]) -> &[DetectedNote] {
        self.out.clear();
        self.revised.clear();
        self.pitch_revised.clear();
        self.strings.clear_told();
        self.retracted.clear();
        self.double_stops.clear_told();
        let block = (RETRY_S * self.sample_rate) as usize;
        let mut filtered = std::mem::take(&mut self.filtered);
        for raw in samples.chunks(block.max(1)) {
            // Notches first: they ring when their input changes, as it would once hum is
            // subtracted (a G2 heard as the hum, 31.1, on the laptop jack).
            filtered.clear();
            filtered.extend_from_slice(raw);
            if let Some(hum) = self.hum.as_mut() {
                filtered.iter_mut().for_each(|x| *x = hum.process(*x));
            }
            self.store_heard(&filtered);
            filtered
                .iter_mut()
                .for_each(|x| *x = self.dehum.process(*x));
            self.store_raw(raw);
            let chunk = &filtered[..];
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
            self.check_confirm();
            self.follow();
            self.release_legato();
            self.judge_strikes(Waiting::ForAudio);
            self.judge_strings(Waiting::ForAudio);
            self.judge_doubles(Waiting::ForAudio);
        }
        self.filtered = filtered;
        &self.out
    }

    /// Emits a pending note now, e.g. at the end of a file.
    pub fn flush(&mut self) -> &[DetectedNote] {
        self.out.clear();
        self.revised.clear();
        self.pitch_revised.clear();
        self.strings.clear_told();
        self.retracted.clear();
        self.double_stops.clear_told();
        if let Some(mut s) = self.sounding {
            self.settle(&mut s, u64::MAX);
            self.sounding = Some(s);
        }
        if let Some((note, _)) = self.legato.take() {
            self.out.push(note);
        }
        if let Some(onset) = self.candidate.take() {
            self.decide(onset);
        }
        if let Some(p) = self.pending.take() {
            self.emit(p);
        }
        self.finish_confirm();
        self.judge_strikes(Waiting::No);
        self.judge_strings(Waiting::No);
        self.judge_doubles(Waiting::No);
        &self.out
    }

    /// Keeps the input before `hum` (only with it), from `pushed` on; call before `store`.
    fn store_raw(&mut self, raw: &[f32]) {
        let len = self.raw_ring.len() as u64;
        if len == 0 {
            return;
        }
        for (i, &x) in raw.iter().enumerate() {
            self.raw_ring[((self.pushed + i as u64) % len) as usize] = x;
        }
    }

    /// Keeps the input before `dehum`, from `pushed` on; call before `store`.
    fn store_heard(&mut self, chunk: &[f32]) {
        let len = self.heard.len() as u64;
        for (i, &x) in chunk.iter().enumerate() {
            self.heard[((self.pushed + i as u64) % len) as usize] = x;
        }
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

    /// Where the attack after `onset` really starts: a faint precursor too close before the pluck
    /// for an onset of its own (`min_interval_s`) moves the onset to the pluck. Needs the audio
    /// up to `PRECURSOR_LOOK_S` after the onset.
    ///
    /// ```text
    /// level   precursor (−30 dB)   pluck (−10 dB)
    ///         ▁▂▁                  ▆█▇▇▆
    ///         ↑ onset              ↑ first sample within 12 dB of the peak: the attack
    /// ```
    fn attack_start(&self, onset: u64) -> u64 {
        let end = (onset + (PRECURSOR_LOOK_S * self.sample_rate) as u64).min(self.pushed);
        let threshold = self.peak(onset, end) / PRECURSOR_FACTOR;
        let len = self.ring.len() as u64;
        let from = onset.max(self.pushed.saturating_sub(len));
        let Some(rise) = (from..end).find(|s| self.ring[(s % len) as usize].abs() >= threshold)
        else {
            return onset;
        };
        if rise < onset + (PRECURSOR_MIN_S * self.sample_rate) as u64 {
            return onset;
        }
        rise
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
        // A second burst of an attack told early belongs to it (`MERGE_S`).
        if self.joins_early(onset) {
            return;
        }
        self.retracts_early(onset);
        self.finish_confirm();
        self.cut_strings(onset);
        self.cut_doubles(onset);
        // A new attack ends the ringing note (told with what was heard before it); a legato note
        // just before it was this attack.
        if let Some(mut s) = self.sounding {
            self.settle(&mut s, onset);
        }
        self.sounding = None;
        self.legato = None;
        let report = match self.pending.take() {
            Some(p) => self.close_pending(p, onset),
            None => onset,
        };
        self.pending = Some(Pending {
            onset,
            report,
            last: None,
            best: None,
            next_try: self.first_try(onset),
            checked: false,
            early: None,
        });
    }

    /// Ends the pending note at a new attack at `onset`; returns the onset to report for it.
    fn close_pending(&mut self, p: Pending, onset: u64) -> u64 {
        let unpitched = p.best.is_none();

        // A faint burst before a much louder attack was a precursor: dropped.
        if unpitched && self.is_precursor(&p, onset) {
            return onset;
        }

        // A second burst of the same attack: one note from the first.
        let merge = (MERGE_S * self.sample_rate) as u64;
        if unpitched && onset < p.report + merge {
            return p.report;
        }
        self.emit(p);
        onset
    }

    /// Whether the attack at `onset` is so much louder than the pending note's burst that the
    /// burst was only a precursor of it.
    fn is_precursor(&self, p: &Pending, onset: u64) -> bool {
        let before = self.peak(p.report, onset);
        let after = self.peak(onset, onset + (RISE_AFTER_S * self.sample_rate) as u64);
        after >= before * PRECURSOR_FACTOR
    }

    fn advance(&mut self) {
        let Some(mut p) = self.pending else {
            return;
        };
        if self.try_early(&mut p) {
            self.pending = None;
            self.emit_early(p);
            return;
        }
        if self.search(&mut p) {
            self.pending = None;
            self.emit(p);
            return;
        }
        self.pending = Some(p);
    }

    /// The first full-window try after an attack at `onset`.
    fn full_try(&self, onset: u64) -> u64 {
        let skip = (self.config.attack_skip_s * self.sample_rate) as u64;
        onset + skip + self.yin.needed() as u64
    }

    /// Full-window estimates due for `p`; true once two agree or the time is up (`p.best` is
    /// the pitch then, if any).
    fn search(&mut self, p: &mut Pending) -> bool {
        if !p.checked && p.next_try <= self.pushed {
            p.checked = true;
            let start = self.attack_start(p.onset);
            if start != p.onset {
                p.onset = start;
                p.report = start;
                p.next_try = self.full_try(start);
            }
        }
        let timeout = p.onset + (self.config.timeout_s * self.sample_rate) as u64;
        while p.next_try <= self.pushed {
            if let Some(estimate) = self.estimate(p, p.next_try) {
                if p.best.is_none_or(|b| estimate.confidence > b.confidence) {
                    p.best = Some(estimate);
                }
                let agrees = p.last.is_some_and(|l| {
                    cents_between(l.hz, estimate.hz).abs() < self.config.agree_cents
                });
                if agrees {
                    p.best = Some(estimate);
                    return true;
                }
                p.last = Some(estimate);
            }
            p.next_try += (RETRY_S * self.sample_rate) as u64;
        }
        self.pushed >= timeout
    }

    /// YIN over the newest `needed()` samples before `until` (all after the attack).
    ///
    /// `estimate_attack` is used (a fading thump does not count), not `estimate_fundamental` (its
    /// low-pass needs warm-up audio a fresh note does not have), then `refine_hz` removes the
    /// stiff-string bias for intonation.
    /// `refine_hz` gets all audio since the attack (up to the scratch length): it needs 2.25
    /// periods, more than `needed()` holds for notes near `min_hz` (B0 −20 ct on a 5-string),
    /// which then kept the bias (+8 ct).
    fn estimate(&mut self, p: &Pending, until: u64) -> Option<PitchEstimate> {
        let skip = (self.config.attack_skip_s * self.sample_rate) as u64;
        let from = (p.onset + skip).max(until.saturating_sub(self.scratch.len() as u64));
        let count = (until - from) as usize;
        self.copy_out(from, count);
        let yin_from = count.saturating_sub(self.yin.needed());
        let mut estimate = self.yin.estimate_attack(&self.scratch[yin_from..count])?;
        if estimate.confidence < self.config.min_confidence {
            return None;
        }
        let fine = match self.raw_ring.len() as u64 {
            0 => &self.scratch[..count],
            len => {
                for (i, slot) in self.raw_scratch.iter_mut().take(count).enumerate() {
                    *slot = self.raw_ring[((from + i as u64) % len) as usize];
                }
                &self.raw_scratch[..count]
            }
        };
        if let Some(hz) = refine_hz(fine, self.sample_rate, estimate.hz) {
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
        let over_background = !self.stands_out(&p);
        let level_db = self.level_after(p.report);
        let (midi, confidence) = match p.best {
            Some(b) => (hz_to_midi(b.hz, self.config.a4_hz), b.confidence),
            None => (0.0, 0.0),
        };
        // An attack without a pitch is a dead (muted) note; one with a pitch is judged on its
        // attack, now or, when its audio is not there yet, by a revision later.
        let technique = match p.best {
            None => Some(Technique::Muted),
            Some(_) => match self.strike(p.report) {
                Some(strike) => strike.technique(self.config.source),
                None => {
                    self.wait_for_strike(p.report);
                    None
                }
            },
        };
        // A double stop often has no clear single pitch: judged whatever was heard.
        self.announce_double(&p);
        if p.best.is_some() {
            let peak = 10f32.powf(level_db / 20.0);
            let step = (FOLLOW_STEP_S * self.sample_rate) as u64;
            let muted = technique == Some(Technique::Muted);
            self.sounding = Some(Sounding {
                midi,
                peak,
                next_try: self.pushed.max(p.next_try) + step,
                change: None,
                settle: Some(Settle::new(
                    p.report,
                    midi,
                    muted,
                    Steps::for_source(self.config.source),
                )),
            });
        }
        if p.best.is_some() && technique != Some(Technique::Muted) {
            self.plan_string(p.report, midi);
        }
        self.out.push(DetectedNote {
            onset_s: p.report as f64 / self.sample_rate,
            midi,
            confidence,
            level_db,
            source: self.config.source,
            technique,
            over_background,
        });
    }

    /// The attack at `onset` measured, if the ring holds its audio.
    fn strike(&self, onset: u64) -> Option<Strike> {
        let len = self.ring.len() as u64;
        let available = self.pushed.saturating_sub(len)..self.pushed;
        Strike::measure(onset, self.sample_rate, available, |i| {
            self.ring[(i % len) as usize]
        })
    }

    /// Keeps `onset` until its attack can be judged; the oldest waiting one gives way.
    fn wait_for_strike(&mut self, onset: u64) {
        let slot = self
            .strikes
            .iter()
            .position(Option::is_none)
            .unwrap_or_else(|| {
                let oldest = self.strikes.iter().flatten().min().copied();
                self.strikes.iter().position(|s| *s == oldest).unwrap_or(0)
            });
        self.strikes[slot] = Some(onset);
    }

    /// Judges waiting attacks whose audio is there (`ForAudio`), or all on what exists (`No`,
    /// at the end of a file); a technique found becomes a revision.
    fn judge_strikes(&mut self, waiting: Waiting) {
        let due = (verdict_after_s() * self.sample_rate) as u64;
        for i in 0..STRIKES_WAITING {
            let Some(onset) = self.strikes[i] else {
                continue;
            };
            if waiting == Waiting::ForAudio && self.pushed < onset + due {
                continue;
            }
            self.strikes[i] = None;
            let technique = self
                .strike(onset)
                .and_then(|s| s.technique(self.config.source));
            if technique == Some(Technique::Muted) {
                self.mark_muted(onset);
            }
            if let Some(technique) = technique {
                self.revised.push(TechniqueRevision {
                    onset_s: onset as f64 / self.sample_rate,
                    technique,
                });
            }
        }
    }

    /// Follows the ringing note's pitch; emits a legato note when it moves to a new one.
    fn follow(&mut self) {
        if self.pending.is_some() || self.confirm.is_some() {
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
                self.settle(&mut s, until);
                self.sounding = None;
                return;
            }
            let settle_end = s
                .settle
                .map(|t| t.onset + (SETTLE_TO_S * self.sample_rate) as u64);
            if settle_end.is_some_and(|end| until > end) {
                self.settle(&mut s, until);
            }
            let Some(estimate) = self.estimate_until(until) else {
                continue;
            };
            let midi = hz_to_midi(estimate.hz, self.config.a4_hz);
            let settle_from = (SETTLE_FROM_S * self.sample_rate) as u64;
            if let Some(t) = s.settle.as_mut().filter(|t| until >= t.onset + settle_from) {
                t.hear(until, midi);
            }
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
            let change = s.change.get_or_insert(Change::new(until, midi));
            change.hear(midi, s.midi, self.config.agree_cents);
            if change.agreeing < LEGATO_AGREE {
                continue;
            }

            // Still settling: the struck note rising to its pitch is no new note.
            if s.settle.is_some() && self.settle(&mut s, until) {
                continue;
            }
            let Some(change) = s.change else {
                continue;
            };
            let technique = change.technique(s.midi);
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
                    over_background: false,
                },
                until + (LEGATO_HOLD_S * self.sample_rate) as u64,
            ));
            s.midi = midi;
            s.change = None;
        }
        self.sounding = Some(s);
    }

    /// Tells the struck note's settled pitch from estimates up to `before`, if it corrects the
    /// emitted one; the ringing pitch is then the settled one. Returns whether it was revised.
    fn settle(&mut self, s: &mut Sounding, before: u64) -> bool {
        let Some(settle) = s.settle.take() else {
            return false;
        };
        let Some(midi) = settle.settled(before) else {
            return false;
        };
        self.pitch_revised.push(PitchRevision {
            onset_s: settle.onset as f64 / self.sample_rate,
            midi,
        });
        self.replan_string(settle.onset, s.midi, midi);
        s.midi = midi;
        s.change = None;
        true
    }

    /// A dead note told late: its settling pitch does not count.
    fn mark_muted(&mut self, onset: u64) {
        self.mute_string(onset);
        let settle = self
            .sounding
            .as_mut()
            .and_then(|s| s.settle.as_mut())
            .filter(|t| t.onset == onset);
        if let Some(t) = settle {
            t.muted = true;
        }
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
    use crate::synth::{
        bass_tone, laptop_jack, render, render_articulated, Articulation, Played, SynthNote,
        ToneShape,
    };
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
            apply(&t, &mut out);
        }
        out.extend_from_slice(t.flush());
        apply(&t, &mut out);
        out
    }

    /// The last call's revisions and retractions applied, as the session does (R1).
    fn apply(t: &NoteTracker, notes: &mut Vec<DetectedNote>) {
        for r in t.pitch_revised() {
            let of = notes.iter_mut().filter(|n| n.onset_s == r.onset_s);
            of.for_each(|n| n.midi = r.midi);
        }
        notes.retain(|n| t.retracted().iter().all(|r| r.onset_s != n.onset_s));
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
    fn quiet_laptop_jack_signal() {
        // J-A1 (P1): a passive bass in the combo jack, input level turned low. Without hum
        // notches G2 came out an octave low (on 50 Hz) and the hum made notes of its own; with
        // the line input's onset margin the soft attacks of A1 and C2 went unheard.
        // The input runs a while before the first note, as in the app: the hum filter settles.
        const QUIET: f32 = 0.05;
        const LEAD_IN_S: f64 = 2.0;
        let notes: Vec<SynthNote> = line(&["E1", "G1", "G2", "D3", "A1", "G2", "C2", "D3"], 0.4)
            .into_iter()
            .map(|n| SynthNote {
                onset_s: n.onset_s + LEAD_IN_S,
                ..n
            })
            .collect();
        let quiet: Vec<f32> = render(&notes, SR, LEAD_IN_S + 3.6, ToneShape::Dull, 9)
            .iter()
            .map(|s| s * QUIET)
            .collect();
        let signal = laptop_jack(&quiet, SR, 10);
        assert_matches(&track(&signal, InputSource::LaptopJack), &notes, 10.0, 15.0);
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
    fn faint_burst_before_the_pluck_is_no_onset() {
        // Fretting the note taps the string faintly 50 ms before the pluck (hardware check
        // 13.10., H5): the note starts at the pluck with the pluck's level, not 50 ms early
        // and 26 dB too quiet.
        let hz = Pitch::parse("C2").unwrap().hz(A4_HZ);
        let tap = bass_tone(hz, SR, 0.05, ToneShape::Rich, 10);
        let pluck = bass_tone(hz, SR, 0.8, ToneShape::Rich, 11);
        let mut signal = vec![0.0; (0.2 * SR) as usize];
        signal.extend(tap.iter().map(|x| x * 0.03));
        signal.extend(pluck.iter().map(|x| x * 0.9));
        signal.extend(std::iter::repeat_n(0.0, (0.3 * SR) as usize));
        let found = track(&signal, InputSource::Line);
        assert_eq!(found.len(), 1, "found {found:?}");
        assert!((found[0].onset_s - 0.25).abs() < 0.005, "{found:?}");
        assert!(found[0].level_db > -6.0, "{found:?}");
    }

    #[test]
    fn faint_burst_right_before_the_pluck_is_no_onset() {
        // The same 30 ms before the pluck, too close for a second onset (`min_interval_s`).
        let hz = Pitch::parse("G2").unwrap().hz(A4_HZ);
        let tap = bass_tone(hz, SR, 0.03, ToneShape::Rich, 10);
        let pluck = bass_tone(hz, SR, 0.8, ToneShape::Rich, 11);
        let mut signal = vec![0.0; (0.2 * SR) as usize];
        signal.extend(tap.iter().map(|x| x * 0.03));
        signal.extend(pluck.iter().map(|x| x * 0.9));
        signal.extend(std::iter::repeat_n(0.0, (0.3 * SR) as usize));
        let found = track(&signal, InputSource::Line);
        assert_eq!(found.len(), 1, "found {found:?}");
        assert!((found[0].onset_s - 0.23).abs() < 0.005, "{found:?}");
        assert!(found[0].level_db > -6.0, "{found:?}");
    }

    /// A pluck and legato notes after it on the same string (one phrase).
    fn phrase(steps: &[(&str, f64, Articulation)]) -> Vec<f32> {
        let played: Vec<Played> = steps
            .iter()
            .enumerate()
            .map(|(i, (name, onset_s, how))| {
                let end = steps.get(i + 1).map_or(onset_s + 0.5, |s| s.1);
                Played {
                    note: SynthNote {
                        onset_s: *onset_s,
                        hz: Pitch::parse(name).unwrap().hz(A4_HZ),
                        duration_s: end - onset_s,
                        velocity: 0.7,
                    },
                    how: *how,
                }
            })
            .collect();
        render_articulated(&played, SR, 1.5, 13)
    }

    /// What `follow` makes of these estimates after the ringing pitch `from`.
    fn heard(from: f64, estimates: &[f64]) -> Option<Technique> {
        let mut change: Option<Change> = None;
        for &midi in estimates {
            if (midi - from).abs() * 100.0 < GLIDE_CENTS {
                change = None;
                continue;
            }
            let c = change.get_or_insert(Change::new(0, midi));
            c.hear(midi, from, 20.0);
            if c.agreeing >= LEGATO_AGREE {
                return Some(c.technique(from));
            }
        }
        None
    }

    #[test]
    fn slide_over_two_frets_is_a_slide() {
        // Kontra's estimates every 10 ms at two IDMT slides (005 at 9.2 s, 012 at 18.5 s): the
        // pitch rests on the fret between for 20–30 ms. Too few pitches in between for a glide
        // (5 of `SLIDE_BETWEEN`); the rest on the way makes them slides, not hammer-ons.
        let idmt_005 = [
            45.41, 45.80, 46.00, 46.04, 46.04, 46.12, 46.41, 46.86, 47.01, 47.00, 47.01, 47.02,
            47.04, 47.04, 47.04, 47.02, 47.02, 47.03,
        ];
        let idmt_012 = [
            48.12, 48.58, 48.88, 48.95, 49.12, 49.66, 49.91, 49.94, 49.93, 49.93, 49.93, 49.96,
            49.97, 49.98, 49.98, 49.99, 49.99,
        ];
        assert_eq!(heard(45.05, &idmt_005), Some(Technique::Slide));
        assert_eq!(heard(47.95, &idmt_012), Some(Technique::Slide));
    }

    #[test]
    fn settling_hammer_on_passes_no_fret() {
        // A synthetic hammer-on A2 to B2: the estimate passes A#2 once and settles on B2.
        let estimates = [
            45.26, 46.04, 46.81, 47.00, 47.04, 47.05, 47.04, 47.04, 47.04, 47.04, 47.04, 47.04,
        ];
        assert_eq!(heard(45.05, &estimates), Some(Technique::HammerOn));
    }

    #[test]
    fn hammer_on_over_two_frets_stays_one() {
        let signal = phrase(&[
            ("A2", 0.2, Articulation::Pluck),
            ("B2", 0.5, Articulation::Legato),
        ]);
        let found = track(&signal, InputSource::Line);
        assert_eq!(found.len(), 2, "found {found:?}");
        assert_eq!(found[1].technique, Some(Technique::HammerOn), "{found:?}");
    }

    /// A D2 that starts `flat_cents` low and rises to pitch within `rise_s` (FiloBass double
    /// bass, testdata/korpus/QUELLEN.md "Halbton zu tief"), after 0.2 s of silence.
    fn rising_tone(flat_cents: f64, rise_s: f64) -> Vec<f32> {
        let hz = Pitch::parse("D2").unwrap().hz(A4_HZ);
        let lead = (0.2 * SR) as usize;
        let mut signal = vec![0.0; lead];
        let mut phase = 0.0f64;
        for i in 0..(0.8 * SR) as usize {
            let t = i as f64 / SR;
            let cents = flat_cents * (1.0 - t / rise_s).max(0.0);
            phase += std::f64::consts::TAU * hz * 2f64.powf(cents / 1200.0) / SR;
            let tone: f64 = (1..=6).map(|h| (phase * h as f64).sin() / h as f64).sum();
            let envelope = (t / 0.002).min(1.0) * (-t / 0.8).exp();
            signal.push((0.4 * envelope * tone) as f32);
        }
        signal
    }

    /// Notes and pitch revisions, with the revisions applied.
    fn track_revised(signal: &[f32]) -> (Vec<DetectedNote>, Vec<PitchRevision>) {
        let config = TrackerConfig::for_instrument(&Instrument::default_bass(), InputSource::Line);
        let mut t = NoteTracker::new(SR, config);
        let mut notes = Vec::new();
        let mut revisions = Vec::new();
        for block in signal.chunks(480) {
            notes.extend_from_slice(t.push(block));
            revisions.extend_from_slice(t.pitch_revised());
        }
        notes.extend_from_slice(t.flush());
        revisions.extend_from_slice(t.pitch_revised());
        (notes, revisions)
    }

    #[test]
    fn flat_start_is_revised_once_settled() {
        // Emitted a semitone low while the tone still rises; told right once it has settled.
        let (notes, revisions) = track_revised(&rising_tone(-140.0, 0.12));
        assert_eq!(notes.len(), 1, "{notes:?}");
        assert_eq!(
            notes[0].midi.round(),
            37.0,
            "emitted during the rise: {notes:?}"
        );
        let last = revisions.last().map(|r| r.midi.round());
        assert_eq!(last, Some(38.0), "{revisions:?}");
        assert!(revisions.iter().all(|r| r.onset_s == notes[0].onset_s));
    }

    #[test]
    fn steady_tone_is_not_revised() {
        let (notes, revisions) = track_revised(&rising_tone(0.0, 0.12));
        assert_eq!(notes.len(), 1, "{notes:?}");
        assert!(revisions.is_empty(), "{revisions:?}");
    }

    #[test]
    fn falling_tone_is_not_revised() {
        // Only a rise is settling (a flat start, an octave below at the attack); a fall after
        // the note is a pull-off or a decay. Only the full window may correct an early report,
        // and it still hears the fall's start (above D2), not the tone it falls to.
        let (_, revisions) = track_revised(&rising_tone(140.0, 0.12));
        assert!(revisions.len() <= 1, "{revisions:?}");
        assert!(revisions.iter().all(|r| r.midi > 38.5), "{revisions:?}");
    }

    /// A settle window holding `estimates`, after a note emitted as `emitted`.
    fn settle_of(emitted: f64, estimates: &[f64], steps: Steps) -> Settle {
        let mut settle = Settle::new(0, emitted, false, steps);
        for (i, m) in estimates.iter().enumerate() {
            settle.hear(i as u64, *m);
        }
        settle
    }

    #[test]
    fn settled_pitch_rules() {
        let up = [38.0, 38.02, 37.98, 38.01];
        let any = Steps::SemitoneOrOctave;
        // The median of the latest three: 38.02, 37.98, 38.01.
        let settled = settle_of(37.3, &up, any).settled(u64::MAX);
        assert!(
            settled.is_some_and(|m| (m - 38.01).abs() < 1e-9),
            "{settled:?}"
        );
        // The latest reach the next semitone: the median of all decides.
        let rising = [38.0, 38.02, 38.6, 38.7];
        let settled = settle_of(37.3, &rising, any).settled(u64::MAX);
        assert!(settled.is_some_and(|m| m.round() == 38.0), "{settled:?}");
        // Too few estimates, too close, downward, unstable, after a new attack.
        assert_eq!(settle_of(37.3, &up[..2], any).settled(u64::MAX), None);
        assert_eq!(settle_of(37.55, &up, any).settled(u64::MAX), None);
        assert_eq!(settle_of(39.0, &up, any).settled(u64::MAX), None);
        let wandering = [37.4, 38.6, 39.2, 36.8];
        assert_eq!(settle_of(37.0, &wandering, any).settled(u64::MAX), None);
        assert_eq!(settle_of(37.3, &up, any).settled(1), None);
        // An octave up, but not at a microphone.
        let octave = [50.0, 50.01, 49.99];
        assert!(settle_of(38.0, &octave, any).settled(u64::MAX).is_some());
        assert_eq!(
            settle_of(38.0, &octave, Steps::Semitone).settled(u64::MAX),
            None
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
