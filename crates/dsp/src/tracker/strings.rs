//! Which string a note was played on (R7): checked after the note, where the tab names one.
//!
//! ```text
//! emit ─ tab strike near, one tone, its pitch, a string, ≥ 2 strings can play it? ─► plan
//! peak ─ 30 ms ─┬──────── 250 ms of the tone ────────┤ ─► B ─► nearest string ─► verdict
//!               └─ the next attack cuts it short: < MIN_WINDOW_S left ─► no statement
//! ```
//!
//! A dead note told late (`mark_muted`), a tone fading fast (palm mute) and too few partials
//! give no statement either; a profile without string values (fretless, a low B) says so at
//! once.

use super::{NoteTracker, Waiting};
use crate::inharmonicity::{InharmonicityMeter, Pick, StringModel, FROM_PEAK_S, WINDOW_S};
use kontra_model::{midi_to_hz, StringHeard, StringVerdict, Unclear};

/// Notes waiting for their string at once (one every 35 ms over 0.3 s).
const SLOTS: usize = 8;
/// The attack's peak is looked for this far before and after the onset.
const PEAK_BEFORE_S: f64 = 0.01;
const PEAK_AFTER_S: f64 = 0.03;
/// A window cut short by the next attack must still hold this much of the tone. IDMT-ST lines
/// (`kontra-cli korpus strings`): shorter windows told the string barely above chance.
const MIN_WINDOW_S: f64 = 0.1;
/// The window ends this long before the next attack (its onset may come a few ms late).
const CUT_GUARD_S: f64 = 0.01;
/// Fewer partials say nothing: B rests on too few points. IDMT-ST lines: from 14 on 93 % right
/// (63 % of the notes), with 6 84 % (all); single notes (IDMT-SMT-Bass) lose 1.5 %.
const MIN_PARTIALS: u8 = 14;
/// A tone fading faster than this over the first `DECAY_FROM_S` of the window is muted (palm
/// mute), whose string was told at chance. Only the start: a note stopped before the next one
/// fades fast at its end, and the whole window cost 60 % of the IDMT-ST notes.
const MUTED_DECAY_DB_PER_S: f64 = 30.0;
const DECAY_FROM_S: f64 = 0.1;
/// The level at the stretch's start and end is measured over this span.
const DECAY_SPAN_S: f64 = 0.04;

/// The string checks of a tracker; allocation-free after `new`.
#[derive(Debug)]
pub(super) struct Strings {
    model: Option<StringModel>,
    /// Only where the profile has a model (it holds a long FFT).
    meter: Option<InharmonicityMeter>,
    samples: Vec<f32>,
    checks: [Option<Check>; SLOTS],
    told: Vec<StringVerdict>,
}

/// A note waiting for its string.
#[derive(Debug, Clone, Copy)]
struct Check {
    /// The note's reported onset (sample).
    report: u64,
    midi: u8,
    hz: f64,
    expected: u8,
    /// The next attack, which ends the tone.
    cut: Option<u64>,
    muted: bool,
}

impl Strings {
    pub(super) fn new(model: Option<StringModel>, sample_rate: f64) -> Strings {
        let meter = model.map(|_| InharmonicityMeter::new(sample_rate));
        let window = meter.as_ref().map_or(0, InharmonicityMeter::window);
        Strings {
            model,
            meter,
            samples: vec![0.0; window],
            checks: [None; SLOTS],
            told: Vec::with_capacity(2 * SLOTS),
        }
    }

    pub(super) fn told(&self) -> &[StringVerdict] {
        &self.told
    }

    pub(super) fn clear_told(&mut self) {
        self.told.clear();
    }
}

impl NoteTracker {
    /// Plans the string check of a note emitted at `report` with `midi`, if the tab expects a
    /// single tone there on a string, at the pitch heard.
    pub(super) fn plan_string(&mut self, report: u64, midi: f64) {
        let Some(strike) = self.expected.near(report) else {
            return;
        };
        let [tone] = strike.tones() else {
            return;
        };
        let Some(expected) = tone.string else {
            return;
        };
        if f64::from(tone.midi) != midi.round() {
            return;
        }
        let onset_s = report as f64 / self.sample_rate;
        let Some(model) = self.strings.model else {
            self.tell_string(
                onset_s,
                StringHeard::Unclear(Unclear::NotSupported),
                None,
                0,
            );
            return;
        };
        match model.pick(tone.midi, 0.0) {
            Pick::OnlyOne => return,
            Pick::Unknown => {
                let heard = StringHeard::Unclear(Unclear::NotSupported);
                self.tell_string(onset_s, heard, None, 0);
                return;
            }
            Pick::String(_) => {}
        }
        let check = Check {
            report,
            midi: tone.midi,
            hz: midi_to_hz(midi, self.config.a4_hz),
            expected,
            cut: None,
            muted: false,
        };
        let checks = &mut self.strings.checks;
        let slot = checks.iter().position(Option::is_none).unwrap_or_else(|| {
            let oldest = checks.iter().flatten().map(|c| c.report).min();
            checks
                .iter()
                .position(|c| c.map(|c| c.report) == oldest)
                .unwrap_or(0)
        });
        checks[slot] = Some(check);
    }

    /// A new attack at `onset` ends the tones still waiting.
    pub(super) fn cut_strings(&mut self, onset: u64) {
        for check in self.strings.checks.iter_mut().flatten() {
            if check.report < onset && check.cut.is_none() {
                check.cut = Some(onset);
            }
        }
    }

    /// The pitch of the note at `report` was revised from `was` to `midi`: its string check
    /// rests on the pitch, so it is planned again (review #207). A check already told stays
    /// told: only one still waiting, or one not planned for the other semitone, is planned.
    pub(super) fn replan_string(&mut self, report: u64, was: f64, midi: f64) {
        let waiting = self
            .strings
            .checks
            .iter()
            .flatten()
            .any(|c| c.report == report);
        if !waiting && was.round() == midi.round() {
            return;
        }
        self.forget_string(report);
        self.plan_string(report, midi);
    }

    /// The note at `report` was taken back (R1): no string is told for it.
    pub(super) fn forget_string(&mut self, report: u64) {
        for check in &mut self.strings.checks {
            if check.is_some_and(|c| c.report == report) {
                *check = None;
            }
        }
    }

    /// The note at `onset` was a dead note: its string is not told.
    pub(super) fn mute_string(&mut self, onset: u64) {
        for check in self.strings.checks.iter_mut().flatten() {
            if check.report == onset {
                check.muted = true;
            }
        }
    }

    /// Tells the strings whose tone is there (`ForAudio`), or all on what exists (`No`).
    pub(super) fn judge_strings(&mut self, waiting: Waiting) {
        let full = ((PEAK_AFTER_S + FROM_PEAK_S + WINDOW_S) * self.sample_rate) as u64;
        for i in 0..SLOTS {
            let Some(check) = self.strings.checks[i] else {
                continue;
            };
            let due = check
                .cut
                .map_or(check.report + full, |cut| cut.min(check.report + full));
            if waiting == Waiting::ForAudio && self.pushed < due {
                continue;
            }
            self.strings.checks[i] = None;
            let (heard, log_b, partials) = self.hear_string(&check);
            let onset_s = check.report as f64 / self.sample_rate;
            self.tell_string(onset_s, heard, log_b, partials);
        }
    }

    /// The string of `check` from its tone in the ring.
    fn hear_string(&mut self, check: &Check) -> (StringHeard, Option<f64>, u8) {
        let unclear = |why| (StringHeard::Unclear(why), None, 0);
        if check.muted {
            return unclear(Unclear::Muted);
        }
        let sr = self.sample_rate;
        let len = self.ring.len() as u64;
        let oldest = self.pushed.saturating_sub(len);
        let peak_from = check.report.saturating_sub((PEAK_BEFORE_S * sr) as u64);
        let peak_to = check.report + (PEAK_AFTER_S * sr) as u64;
        let peak = (peak_from.max(oldest)..peak_to.min(self.pushed))
            .max_by(|a, b| {
                let at = |s: &u64| self.ring[(s % len) as usize].abs();
                at(a).total_cmp(&at(b))
            })
            .unwrap_or(check.report);

        // From 30 ms after the peak to the window's end, the next attack or what exists.
        let from = (peak + (FROM_PEAK_S * sr) as u64).max(oldest);
        let cut = check
            .cut
            .map_or(u64::MAX, |c| c.saturating_sub((CUT_GUARD_S * sr) as u64));
        let to = (from + self.strings.samples.len() as u64)
            .min(cut)
            .min(self.pushed);
        if to < from + (MIN_WINDOW_S * sr) as u64 {
            return unclear(Unclear::ShortNote);
        }
        let count = (to - from) as usize;
        for (i, slot) in self.strings.samples.iter_mut().take(count).enumerate() {
            *slot = self.ring[((from + i as u64) % len) as usize];
        }
        let (Some(meter), Some(model)) = (self.strings.meter.as_mut(), self.strings.model) else {
            return unclear(Unclear::NotSupported);
        };
        let start = count.min((DECAY_FROM_S * sr) as usize);
        if decay_db_per_s(&self.strings.samples[..start], sr) > MUTED_DECAY_DB_PER_S {
            return unclear(Unclear::Muted);
        }
        let stiffness = meter.measure(&self.strings.samples[..count], check.hz);
        let measured = Some(stiffness.log_b);
        if stiffness.partials < MIN_PARTIALS {
            let heard = StringHeard::Unclear(Unclear::FewPartials);
            return (heard, measured, stiffness.partials);
        }
        let heard = match model.pick(check.midi, stiffness.log_b) {
            Pick::String(s) if s == check.expected => StringHeard::Expected,
            Pick::String(s) => StringHeard::Other(s),
            Pick::OnlyOne | Pick::Unknown => StringHeard::Unclear(Unclear::NotSupported),
        };
        (heard, measured, stiffness.partials)
    }

    fn tell_string(&mut self, onset_s: f64, heard: StringHeard, log_b: Option<f64>, partials: u8) {
        self.strings.told.push(StringVerdict {
            onset_s,
            heard,
            log_b,
            partials,
        });
    }
}

/// How fast `samples` fade (dB per second), from the level of the first and the last
/// `DECAY_SPAN_S`. Example: 0.25 s falling from −20 to −26 dBFS: 6 dB / 0.21 s ≈ 29 dB/s.
fn decay_db_per_s(samples: &[f32], sample_rate: f64) -> f64 {
    let span = (DECAY_SPAN_S * sample_rate) as usize;
    if samples.len() < 2 * span || span == 0 {
        return 0.0;
    }
    let rms = |x: &[f32]| {
        let power = x.iter().map(|v| f64::from(*v) * f64::from(*v)).sum::<f64>();
        (power / x.len() as f64).sqrt().max(f64::MIN_POSITIVE)
    };
    let start = rms(&samples[..span]);
    let end = rms(&samples[samples.len() - span..]);
    let seconds = (samples.len() - span) as f64 / sample_rate;
    20.0 * (start / end).log10() / seconds
}
