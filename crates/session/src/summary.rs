//! Evaluation after a run (LH-BEW-03): hit rate, timing histogram and tendency, hardest bars,
//! bars the player hesitated at (DIDAKTIK I6).

use crate::matcher::{NoteResult, Outcome};
use crate::technique::TechniqueTally;
use kontra_model::{Arrangement, Technique};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const BIN_MS: f64 = 10.0;
const HISTOGRAM_LIMIT_MS: f64 = 100.0;
const HARDEST_BARS: usize = 3;
/// A bar's first note this much later than the player's own mean is a hesitation.
const HESITATE_MS: f64 = 80.0;

/// Counts of timing offsets in 10 ms bins from −100 to +100 ms; outliers go to the edge bins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimingHistogram {
    pub bin_ms: f64,
    pub first_bin_ms: f64,
    pub counts: Vec<u32>,
}

impl TimingHistogram {
    fn from_offsets(offsets: &[f64]) -> TimingHistogram {
        let bins = (2.0 * HISTOGRAM_LIMIT_MS / BIN_MS) as usize;
        let mut counts = vec![0u32; bins];
        for offset in offsets {
            let index = ((offset + HISTOGRAM_LIMIT_MS) / BIN_MS)
                .floor()
                .clamp(0.0, (bins - 1) as f64) as usize;
            counts[index] += 1;
        }
        TimingHistogram {
            bin_ms: BIN_MS,
            first_bin_ms: -HISTOGRAM_LIMIT_MS,
            counts,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Summary {
    /// Expected notes that were judged (hit, wrong or missed).
    pub judged: u32,
    pub hits: u32,
    pub wrong: u32,
    /// Right notes beyond the pitch limit ("unsauber").
    pub off_pitch: u32,
    pub missed: u32,
    pub extra: u32,
    /// hits / judged, 0..=1.
    pub hit_rate: f64,
    /// Mean timing offset of hits in ms (negative = early): the tendency.
    pub mean_offset_ms: Option<f64>,
    /// Standard deviation of the hit offsets in ms: the steadiness.
    pub spread_ms: Option<f64>,
    /// Mean absolute pitch deviation of hits and off-pitch notes in cents (intonation).
    pub mean_abs_cents: Option<f64>,
    pub histogram: TimingHistogram,
    /// Bars with the lowest hit rate, worst first (index into `Arrangement::bars`).
    pub hardest_bars: Vec<usize>,
    /// Bars the player hesitated at, in order (index into `Arrangement::bars`); see
    /// `hesitant_bars`.
    #[serde(default)]
    pub hesitant_bars: Vec<usize>,
    /// Techniques apart from the hit rate (LH-BEW-06): "Slides 8 of 10".
    #[serde(default)]
    pub techniques: Vec<TechniqueTally>,
    /// Techniques in the score this input cannot tell (slap and pop at a microphone).
    #[serde(default)]
    pub untold_techniques: Vec<Technique>,
}

impl Summary {
    pub fn from_results(results: &[NoteResult], arrangement: &Arrangement) -> Summary {
        let final_results = final_per_note(results);
        let count =
            |f: fn(&Outcome) -> bool| final_results.iter().filter(|r| f(&r.outcome)).count() as u32;
        let hits = count(|o| matches!(o, Outcome::Hit));
        let wrong = count(|o| matches!(o, Outcome::WrongPitch { .. }));
        let off_pitch = count(|o| matches!(o, Outcome::OffPitch));
        let missed = count(|o| matches!(o, Outcome::Missed));
        let extra = results
            .iter()
            .filter(|r| matches!(r.outcome, Outcome::Extra { .. }))
            .count() as u32;
        let judged = hits + wrong + off_pitch + missed;
        let offsets: Vec<f64> = final_results
            .iter()
            .filter(|r| r.is_hit())
            .filter_map(|r| r.offset_ms)
            .collect();
        let cents: Vec<f64> = final_results
            .iter()
            .filter(|r| matches!(r.outcome, Outcome::Hit | Outcome::OffPitch))
            .filter_map(|r| r.cents.map(f64::abs))
            .collect();
        let mean_offset = mean(&offsets);
        let spread = mean_offset.map(|m| {
            (offsets.iter().map(|o| (o - m).powi(2)).sum::<f64>() / offsets.len() as f64).sqrt()
        });
        Summary {
            judged,
            hits,
            wrong,
            off_pitch,
            missed,
            extra,
            hit_rate: if judged == 0 {
                0.0
            } else {
                f64::from(hits) / f64::from(judged)
            },
            mean_offset_ms: mean_offset,
            spread_ms: spread,
            mean_abs_cents: mean(&cents),
            histogram: TimingHistogram::from_offsets(&offsets),
            hardest_bars: hardest_bars(&final_results, arrangement),
            hesitant_bars: hesitant_bars(&final_results, mean_offset.unwrap_or(0.0)),
            techniques: Vec::new(),
            untold_techniques: Vec::new(),
        }
    }
}

/// In Wait mode a note can have several attempts; the last result per (pass, note) counts.
/// Extra notes are no notes of the score and are left out.
pub(crate) fn final_per_note(results: &[NoteResult]) -> Vec<NoteResult> {
    let mut last: BTreeMap<(u32, usize), NoteResult> = BTreeMap::new();
    for r in results {
        let Some(note) = r.note else {
            continue;
        };
        last.insert((r.pass, note), *r);
    }
    last.into_values().collect()
}

fn mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    Some(values.iter().sum::<f64>() / values.len() as f64)
}

fn hardest_bars(results: &[NoteResult], arrangement: &Arrangement) -> Vec<usize> {
    let mut per_bar: BTreeMap<usize, (u32, u32)> = BTreeMap::new();
    for r in results {
        let Some(bar) = r.bar else {
            continue;
        };
        let entry = per_bar.entry(bar).or_default();
        entry.1 += 1;
        if r.is_hit() {
            entry.0 += 1;
        }
    }
    let mut bars: Vec<(usize, f64)> = per_bar
        .into_iter()
        .filter(|(bar, (hits, total))| hits < total && *bar < arrangement.bars.len())
        .map(|(bar, (hits, total))| (bar, f64::from(hits) / f64::from(total)))
        .collect();
    bars.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    bars.into_iter()
        .take(HARDEST_BARS)
        .map(|(bar, _)| bar)
        .collect()
}

/// Bars the player hesitated at: the bar before was played to its end, then the bar's first
/// note came late (`HESITATE_MS` behind the run's mean offset) or not at all. `results` are the
/// final results per note, in order of pass and note.
///
/// ```text
/// bar      3          4
/// played   ♩ ♩ ♩ ♩ |     ♩ ♩ ♩      first note of bar 4 missed: hesitated at 4
/// ```
fn hesitant_bars(results: &[NoteResult], mean_offset_ms: f64) -> Vec<usize> {
    let mut bars = Vec::new();
    for pair in results.windows(2) {
        let [before, first] = pair else {
            continue;
        };
        let (Some(was), Some(bar)) = (before.bar, first.bar) else {
            continue;
        };
        if before.pass != first.pass || bar != was + 1 || matches!(before.outcome, Outcome::Missed)
        {
            continue;
        }

        let late = first
            .offset_ms
            .is_some_and(|ms| ms - mean_offset_ms >= HESITATE_MS);
        if (matches!(first.outcome, Outcome::Missed) || late) && !bars.contains(&bar) {
            bars.push(bar);
        }
    }
    bars.sort_unstable();
    bars
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One note per beat, four to a bar: `(outcome, offset)` in order.
    fn played(notes: &[(Outcome, Option<f64>)]) -> Vec<NoteResult> {
        notes
            .iter()
            .enumerate()
            .map(|(i, (outcome, offset_ms))| NoteResult {
                pass: 0,
                note: Some(i),
                bar: Some(i / 4),
                outcome: *outcome,
                offset_ms: *offset_ms,
                cents: None,
                position_s: i as f64 * 0.5,
            })
            .collect()
    }

    #[test]
    fn hesitating_at_a_bar_line_is_told_from_a_slip() {
        let on = (Outcome::Hit, Some(30.0));
        let late = (Outcome::Hit, Some(130.0));
        let missed = (Outcome::Missed, None);
        let results = played(&[
            on, on, on, on, // bar 0
            missed, on, on, on, // bar 1: stops before it
            on, on, missed, on, // bar 2: a slip inside the bar
            late, on, on, on, // bar 3: comes in late
            on, on, on, missed, // bar 4
            missed, on, on, on, // bar 5: the bar before was not played to its end
        ]);
        // The run drags by 30 ms on average; bar 3 comes 100 ms behind that.
        assert_eq!(hesitant_bars(&results, 30.0), [1, 3]);
        // Measured against a run that drags by 60 ms, 70 ms is no hesitation.
        assert_eq!(hesitant_bars(&results, 60.0), [1]);
    }
}
