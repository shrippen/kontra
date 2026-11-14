//! The Lastenheft limits a corpus evaluation has to keep, in one place: the CI check
//! (`kontra-cli korpus check`) and the engine test both ask `KorpusSet::requirement_misses`.

use super::{Evaluation, KorpusSet};

/// LH-NF-16: at most one false note per minute where nothing is played.
const IDLE_MAX_PER_MINUTE: f64 = 1.0;
/// LH-NF-02: onset error below 10 ms, at a microphone in a room too.
const ONSET_MS: f64 = 10.0;
/// Legato notes have no attack: their time comes from the pitch window (±30 ms).
const LEGATO_ONSET_MS: f64 = 30.0;
/// LH-EIN-11: a microphone in a room needs 90 % found and pitched.
const ROOM_MIN: f64 = 0.9;
const FOUND_MIN: f64 = 0.99;
/// LH-NF-03: pitch on the line (98 %) and on the laptop jack (95 %).
const LINE_PITCH_MIN: f64 = 0.98;
const JACK_PITCH_MIN: f64 = 0.95;
/// LH-BEW-06: 85 % of the technique set recognised; elsewhere (almost) none invented.
const TECHNIQUE_MIN: f64 = 0.85;
const NO_TECHNIQUE_MIN: f64 = 0.95;
/// LH-BEW-05: fretless intonation within 3 cents, for every note (a mean hides one that
/// is always off, B0 +8 ct, J-B4); the mean stays below 6.
const FRETLESS_MAX_CENTS: f64 = 3.0;
const FRETLESS_MEAN_CENTS: f64 = 6.0;

/// The pass marks one set is held to.
struct Limits {
    found: f64,
    pitch: f64,
    onset_ms: f64,
    technique: f64,
}

impl KorpusSet {
    /// What `e` misses of the requirements for this set, empty if it meets them.
    /// `minutes` is the length of the recording (for the idle sets).
    pub fn requirement_misses(self, e: &Evaluation, minutes: f64) -> Vec<String> {
        if self.is_idle() {
            return idle_misses(e, minutes);
        }

        let mut misses = core_misses(e, &self.limits());
        misses.extend(self.report_misses(e));
        if self == KorpusSet::Fretless5 {
            misses.extend(fretless_misses(e));
        }
        misses
    }

    /// Room recordings get the microphone marks, the technique set the legato window.
    fn limits(self) -> Limits {
        let room = matches!(self, KorpusSet::RoomDouble | KorpusSet::RoomDoubleBacking);
        let techniques = self == KorpusSet::Techniques;
        let pitch = match self {
            KorpusSet::LaptopJack4 => JACK_PITCH_MIN,
            _ if room => ROOM_MIN,
            _ => LINE_PITCH_MIN,
        };
        Limits {
            found: if room { ROOM_MIN } else { FOUND_MIN },
            pitch,
            onset_ms: if techniques {
                LEGATO_ONSET_MS
            } else {
                ONSET_MS
            },
            technique: if techniques {
                TECHNIQUE_MIN
            } else {
                NO_TECHNIQUE_MIN
            },
        }
    }

    /// LH-NF-15: told with a pitch soon after the attack, per note (`report.rs`).
    fn report_misses(self, e: &Evaluation) -> Option<String> {
        let limits = self.report_limits()?;
        if e.p95_report_ms > 0.0 && limits.met(e) {
            return None;
        }
        Some(format!(
            "p95 report {:.1} ms from E1, {:.1} ms below",
            e.p95_report_from_e1_ms, e.p95_report_below_e1_ms
        ))
    }
}

fn idle_misses(e: &Evaluation, minutes: f64) -> Vec<String> {
    let per_minute = e.pitched as f64 / minutes;
    if per_minute <= IDLE_MAX_PER_MINUTE {
        return Vec::new();
    }
    vec![format!(
        "{} false notes, {per_minute:.1} per minute",
        e.pitched
    )]
}

fn core_misses(e: &Evaluation, l: &Limits) -> Vec<String> {
    let mut misses = Vec::new();
    if e.recall() < l.found {
        misses.push(format!("recall {:.3}", e.recall()));
    }
    if e.precision() < l.found {
        misses.push(format!("precision {:.3}", e.precision()));
    }
    if e.p95_abs_onset_ms >= l.onset_ms {
        misses.push(format!("p95 onset {:.1} ms", e.p95_abs_onset_ms));
    }
    if e.pitch_accuracy() < l.pitch {
        misses.push(format!("pitch {:.3}", e.pitch_accuracy()));
    }
    if e.technique_accuracy() < l.technique {
        misses.push(format!("technique {:.3}", e.technique_accuracy()));
    }
    misses
}

fn fretless_misses(e: &Evaluation) -> Vec<String> {
    let mut misses = Vec::new();
    if e.mean_abs_cents >= FRETLESS_MEAN_CENTS {
        misses.push(format!("fretless cents {:.1}", e.mean_abs_cents));
    }
    if e.p95_abs_cents > FRETLESS_MAX_CENTS || e.max_abs_cents > FRETLESS_MAX_CENTS {
        misses.push(format!(
            "fretless cents p95 {:.1}, max {:.1}",
            e.p95_abs_cents, e.max_abs_cents
        ));
    }
    misses
}
