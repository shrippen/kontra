//! LH-NF-15: a note is told with its pitch soon after its attack, p95 per note on the line
//! sets. 70 ms from E1 up; below E1 (the low B needs about 65 ms of signal alone) and where a
//! set misses 70 ms today, the measured value is the limit: not to get worse (30.10.2026).

use super::{Evaluation, KorpusSet};

/// E1, the lowest string of a four-string bass.
pub(super) const E1_MIDI: f64 = 28.0;
/// Round 17: the target for every note from E1 up.
const TARGET_MS: f64 = 70.0;
/// Measured 30.10.: `line-5` below E1 94.7 ms (the 80 ms noted before was p95 over all notes).
const BELOW_E1_MS: f64 = 95.0;
/// Measured 30.10., from E1 up: `line-5` 74.7 ms (a five-string's longer pitch window),
/// `hum-line` 72.0 ms (E8).
const LINE5_MS: f64 = 75.0;
const HUM_LINE_MS: f64 = 72.5;

/// The p95 limits from the attack to the report with a pitch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReportLimits {
    pub from_e1_ms: f64,
    pub below_e1_ms: f64,
}

impl ReportLimits {
    /// Whether `e` keeps both limits.
    pub fn met(self, e: &Evaluation) -> bool {
        e.p95_report_from_e1_ms <= self.from_e1_ms && e.p95_report_below_e1_ms <= self.below_e1_ms
    }
}

impl KorpusSet {
    /// LH-NF-15 limits for the line sets; `None` for the others.
    pub fn report_limits(self) -> Option<ReportLimits> {
        let from_e1_ms = match self {
            KorpusSet::Line4 => TARGET_MS,
            KorpusSet::Line5 => LINE5_MS,
            KorpusSet::HumLine => HUM_LINE_MS,
            _ => return None,
        };
        Some(ReportLimits {
            from_e1_ms,
            below_e1_ms: BELOW_E1_MS,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::korpus::{evaluate, SAMPLE_RATE};

    /// 70 ms from E1 up holds where it is met; a set over its limit fails, from E1 up and below.
    #[test]
    fn limits_hold_per_note_range() {
        let mut e = evaluate(KorpusSet::Line4, &[], SAMPLE_RATE, &[]);
        let line4 = KorpusSet::Line4.report_limits().unwrap();
        e.p95_report_from_e1_ms = 70.0;
        assert!(line4.met(&e));
        e.p95_report_from_e1_ms = 70.1;
        assert!(!line4.met(&e));
        e.p95_report_from_e1_ms = 60.0;
        e.p95_report_below_e1_ms = 95.1;
        assert!(!line4.met(&e));
        assert!(KorpusSet::Techniques.report_limits().is_none());
    }
}
