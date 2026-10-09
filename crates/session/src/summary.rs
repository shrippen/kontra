//! Evaluation after a run (LH-BEW-03): hit rate, timing histogram and tendency, hardest bars.

use crate::matcher::{NoteResult, Outcome};
use kontra_model::Arrangement;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const BIN_MS: f64 = 10.0;
const HISTOGRAM_LIMIT_MS: f64 = 100.0;
const HARDEST_BARS: usize = 3;

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
        }
    }
}

/// In Wait mode a note can have several attempts; the last result per (pass, note) counts.
fn final_per_note(results: &[NoteResult]) -> Vec<NoteResult> {
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
