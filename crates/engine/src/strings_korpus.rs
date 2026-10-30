//! The string check (R7) on annotated recordings: each note expected on its annotated string,
//! the tracker's verdict matched back to it (`kontra-cli korpus strings`).

use crate::korpus::Annotation;
use crate::offline::analyze_reported;
use kontra_model::{ExpectedStrike, ExpectedTone, InputSource, Instrument, StringVerdict};

/// How far an expected strike reaches around its annotated onset (as in `korpus measure`).
const EXPECT_WINDOW_S: f64 = 0.08;

/// The verdict told for one annotated note.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StringTold {
    /// Index into the annotation.
    pub note: usize,
    pub verdict: StringVerdict,
}

/// Runs the tracker on `samples` with every note of `notes` expected on its string from
/// `strings` (index, 0 = lowest; `None`: not expected) and returns the verdicts with the note
/// each belongs to (the nearest annotated onset within `window_s`).
pub fn tell_strings(
    samples: &[f32],
    sample_rate: u32,
    notes: &[Annotation],
    strings: &[Option<u8>],
    instrument: &Instrument,
    window_s: f64,
) -> Vec<StringTold> {
    let mut expected: Vec<ExpectedStrike> = notes
        .iter()
        .zip(strings)
        .filter_map(|(n, s)| {
            let midi = u8::try_from(n.midi.round() as i64).ok()?;
            let tone = ExpectedTone {
                midi,
                string: Some((*s)?),
            };
            Some(ExpectedStrike::new(n.onset_s, EXPECT_WINDOW_S).with(tone))
        })
        .collect();
    expected.sort_by(|a, b| a.onset_s.total_cmp(&b.onset_s));
    let rate = f64::from(sample_rate);
    let reported = analyze_reported(samples, rate, instrument, InputSource::Line, &expected);
    reported
        .strings
        .iter()
        .filter_map(|v| {
            let (note, _) = notes
                .iter()
                .enumerate()
                .filter(|(i, _)| strings.get(*i).copied().flatten().is_some())
                .map(|(i, n)| (i, (n.onset_s - v.onset_s).abs()))
                .filter(|(_, d)| *d <= window_s)
                .min_by(|a, b| a.1.total_cmp(&b.1))?;
            Some(StringTold { note, verdict: *v })
        })
        .collect()
}
