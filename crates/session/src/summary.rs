//! Evaluation after a run (LH-BEW-03): hit rate, timing histogram and tendency, hardest bars,
//! bars the player hesitated at (DIDAKTIK I6), the one observation worth telling (LH-BEW-09),
//! intonation as direction and spread (LH-BEW-12).

use crate::matcher::{NoteResult, Outcome};
use crate::pocket::{Pocket, PocketVerdict};
use crate::technique::TechniqueTally;
use kontra_model::{Arrangement, Technique};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const BIN_MS: f64 = 10.0;
const HISTOGRAM_LIMIT_MS: f64 = 100.0;
const HARDEST_BARS: usize = 3;
/// A bar's first note this much later than the player's own mean is a hesitation.
const HESITATE_MS: f64 = 80.0;
/// A played note this close (real ms) to a missed one counts as that note outside the window.
const OUTSIDE_LIMIT_MS: f64 = 150.0;
/// Offsets this close count as the same timing.
const OUTSIDE_TOLERANCE_MS: f64 = 15.0;
/// A mean deviation within this many cents has no direction, only a spread.
const CENT_DIRECTION: f64 = 5.0;
/// Notes a string in one position needs for its own intonation.
const INTONATION_MIN_NOTES: usize = 8;
/// Frets per position: 0–4, 5–9, 10–14, …
const POSITION_FRETS: u8 = 5;

/// Judged notes a run needs before anything about it is told (LH-BEW-09).
const OBSERVE_MIN_NOTES: u32 = 8;
/// Notes a technique needs before its share counts as evidence.
const OBSERVE_MIN_TECHNIQUE: u32 = 4;
/// Hits needed before their timing reads as a lean.
const OBSERVE_MIN_HITS: u32 = 8;
/// A technique below this share is worth practising ...
const WEAK_TECHNIQUE: f64 = 0.6;
/// ... when it also trails the hit rate by this many percentage points, counted whole (all at
/// 50 % is not about the technique; 55 % against 75 % is 20 points, not 19.99…).
const TECHNIQUE_GAP_POINTS: f64 = 20.0;
/// A run that missed this share of its notes is about the notes.
const MANY_MISSED: f64 = 0.35;
/// Share of hits on one side of the beat that reads as a lean rather than scatter.
const LOPSIDED: f64 = 0.6;
/// Hits this close to the beat count as on time for the lean.
const ON_TIME_MS: f64 = 10.0;
/// A mean this far from the beat, with the hits lopsided the same way, may be a wrong latency
/// (LH-EIN-14).
const RECALIBRATE_MS: f64 = 5.0;

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
    /// Pitch deviation of hits and off-pitch notes (intonation).
    #[serde(default)]
    pub intonation: Option<Intonation>,
    /// The same per string and position, for those with enough notes, lowest string first.
    #[serde(default)]
    pub intonation_parts: Vec<IntonationPart>,
    pub histogram: TimingHistogram,
    /// Bars with the most missed notes, worst first; the hit rate breaks ties (index into
    /// `Arrangement::bars`).
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
    /// Double stops in the score this input does not judge by both tones (microphone, R8).
    #[serde(default)]
    pub untold_double_stops: u32,
    /// Median offset in real ms of the notes played next to missed ones, when most missed notes
    /// have one: the timing outside the window (`outside_offset_ms`).
    #[serde(default)]
    pub outside_offset_ms: Option<f64>,
    /// The missed notes behind `outside_offset_ms`, each with its own offset (LH-BEW-01).
    #[serde(default)]
    pub outside_notes: Vec<OutsideNote>,
    /// Hits more than `ON_TIME_MS` before the beat.
    #[serde(default)]
    pub early_hits: u32,
    /// Hits more than `ON_TIME_MS` after the beat.
    #[serde(default)]
    pub late_hits: u32,
    /// Hits before the beat, however close (LH-EIN-14).
    #[serde(default)]
    pub before_beat_hits: u32,
    /// Hits after the beat, however close.
    #[serde(default)]
    pub after_beat_hits: u32,
}

/// The one thing a run most wants the player to hear (LH-BEW-09, after Harmonicon's coaching).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Observation {
    /// A technique trails the plain notes: "Slide: 2 of 6".
    Technique {
        technique: Technique,
        played: u32,
        judged: u32,
    },
    /// Most notes missed the window but were played beside it, by this many ms.
    Beside { offset_ms: f64 },
    /// Many notes did not sit (wrong or missed).
    Missed { notes: u32, judged: u32 },
    /// Hits lean to one side of the beat: `share` of them.
    Lean { side: Side, share: f64 },
    /// The mean missed the chosen aim (LH-BEW-08) on this side, by this many ms.
    Aim { side: Side, off_ms: f64 },
    /// Enough was played and nothing stands out.
    Solid,
}

/// Side of the beat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Early,
    Late,
}

/// Pitch deviation as direction and spread, no grade (LH-BEW-12): "9 cents flat on average,
/// spread ±6".
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Intonation {
    /// Negative = flat.
    pub mean_cent: f64,
    /// Standard deviation around the mean.
    pub spread_cent: f64,
}

/// Which way the intonation leans.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Leaning {
    Flat,
    Sharp,
}

impl Intonation {
    fn from_cents(cents: &[f64]) -> Option<Intonation> {
        let mean_cent = mean(cents)?;
        Some(Intonation {
            mean_cent,
            spread_cent: spread(cents, mean_cent),
        })
    }

    /// `None` when the mean lies within `CENT_DIRECTION`: only the spread tells.
    pub fn leaning(&self) -> Option<Leaning> {
        if self.mean_cent <= -CENT_DIRECTION {
            return Some(Leaning::Flat);
        }
        if self.mean_cent >= CENT_DIRECTION {
            return Some(Leaning::Sharp);
        }
        None
    }
}

/// Intonation on one string in one position.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct IntonationPart {
    /// 0 = lowest string.
    pub string: u8,
    pub first_fret: u8,
    pub last_fret: u8,
    pub intonation: Intonation,
}

/// Per string and position with at least `INTONATION_MIN_NOTES` notes. Example: eight notes on
/// the E string at frets 5 to 9 → one part {string 0, first fret 5}.
fn intonation_parts(results: &[NoteResult], arrangement: &Arrangement) -> Vec<IntonationPart> {
    let mut cents: BTreeMap<(u8, u8), Vec<f64>> = BTreeMap::new();
    for r in results {
        if !matches!(r.outcome, Outcome::Hit | Outcome::OffPitch) {
            continue;
        }
        let (Some(c), Some(note)) = (r.cents, r.note.and_then(|i| arrangement.notes.get(i))) else {
            continue;
        };
        let (Some(string), Some(fret)) = (note.string, note.fret) else {
            continue;
        };
        let first_fret = fret / POSITION_FRETS * POSITION_FRETS;
        cents.entry((string, first_fret)).or_default().push(c);
    }

    cents
        .into_iter()
        .filter(|(_, c)| c.len() >= INTONATION_MIN_NOTES)
        .filter_map(|((string, first_fret), c)| {
            Some(IntonationPart {
                string,
                first_fret,
                last_fret: first_fret + POSITION_FRETS - 1,
                intonation: Intonation::from_cents(&c)?,
            })
        })
        .collect()
}

/// A missed note played beside the timing window.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OutsideNote {
    pub pass: u32,
    /// Index into `Arrangement::notes`.
    pub note: usize,
    /// Real milliseconds, negative = early.
    pub offset_ms: f64,
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
            .filter_map(|r| r.cents)
            .collect();
        let mean_offset = mean(&offsets);
        let spread = mean_offset.map(|m| spread(&offsets, m));
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
            intonation: Intonation::from_cents(&cents),
            intonation_parts: intonation_parts(&final_results, arrangement),
            histogram: TimingHistogram::from_offsets(&offsets),
            hardest_bars: hardest_bars(&final_results, arrangement),
            hesitant_bars: hesitant_bars(&final_results, mean_offset.unwrap_or(0.0)),
            techniques: Vec::new(),
            untold_techniques: Vec::new(),
            untold_double_stops: 0,
            outside_offset_ms: None,
            outside_notes: Vec::new(),
            early_hits: offsets.iter().filter(|o| **o < -ON_TIME_MS).count() as u32,
            late_hits: offsets.iter().filter(|o| **o > ON_TIME_MS).count() as u32,
            before_beat_hits: offsets.iter().filter(|o| **o < 0.0).count() as u32,
            after_beat_hits: offsets.iter().filter(|o| **o > 0.0).count() as u32,
        }
    }

    /// The side the hits lean to so steadily that the latency may be wrong (LH-EIN-14): at
    /// least `OBSERVE_MIN_HITS` hits, `LOPSIDED` of them on one side, the mean `RECALIBRATE_MS`
    /// off the beat. Only a hint, never a correction: a lean the chosen aim covers (laid back)
    /// is played on purpose.
    pub fn recalibrate(&self, pocket: Pocket) -> Option<Side> {
        if self.hits < OBSERVE_MIN_HITS {
            return None;
        }
        // Whole ms, like the aim: 5.3 ms is held on the beat.
        let mean = self.mean_offset_ms?.round();
        if mean.abs() < RECALIBRATE_MS {
            return None;
        }

        let (side, same) = if mean < 0.0 {
            (Side::Early, self.before_beat_hits)
        } else {
            (Side::Late, self.after_beat_hits)
        };
        if f64::from(same) < LOPSIDED * f64::from(self.hits) {
            return None;
        }

        // With an aim, only a mean outside it on the same side.
        match (pocket.judge(mean), side) {
            (None, _)
            | (Some(PocketVerdict::Early(_)), Side::Early)
            | (Some(PocketVerdict::Late(_)), Side::Late) => Some(side),
            _ => None,
        }
    }

    /// The observation for this run, picked in a fixed order, most specific first: a weak
    /// technique, notes played beside the window, many notes missed, a missed timing aim
    /// (`pocket`), a lean (only without an aim), else solid. `None` below `OBSERVE_MIN_NOTES`
    /// (one missed slide is no diagnosis), and below `OBSERVE_MIN_HITS` hits when none of the
    /// first three applies.
    pub fn observation(&self, pocket: Pocket) -> Option<Observation> {
        if self.judged < OBSERVE_MIN_NOTES {
            return None;
        }

        // The weakest technique with enough notes that trails the hit rate.
        let weakest = self
            .techniques
            .iter()
            .filter(|t| t.judged >= OBSERVE_MIN_TECHNIQUE)
            .map(|t| (t, f64::from(t.played) / f64::from(t.judged)))
            .filter(|(_, share)| {
                *share < WEAK_TECHNIQUE
                    && ((self.hit_rate - share) * 100.0).round() >= TECHNIQUE_GAP_POINTS
            })
            .min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((t, _)) = weakest {
            return Some(Observation::Technique {
                technique: t.technique,
                played: t.played,
                judged: t.judged,
            });
        }

        // Missed, but beside the window: the timing is the point, not the notes.
        if let Some(offset_ms) = self.outside_offset_ms.filter(|_| self.missed > self.hits) {
            return Some(Observation::Beside { offset_ms });
        }

        let not_sitting = self.wrong + self.missed;
        if f64::from(not_sitting) >= MANY_MISSED * f64::from(self.judged) {
            return Some(Observation::Missed {
                notes: not_sitting,
                judged: self.judged,
            });
        }

        // Too few hits to tell the timing: nothing to say, not "solid".
        if self.hits < OBSERVE_MIN_HITS {
            return None;
        }
        // A missed aim outweighs a lean: the player chose where to sit. Judged in whole ms,
        // like the pocket line: 5.3 ms against ±5 ms is held, not "0 ms behind".
        let verdict = self.mean_offset_ms.and_then(|m| pocket.judge(m.round()));
        match verdict {
            Some(PocketVerdict::Early(off_ms)) => {
                return Some(Observation::Aim {
                    side: Side::Early,
                    off_ms,
                })
            }
            Some(PocketVerdict::Late(off_ms)) => {
                return Some(Observation::Aim {
                    side: Side::Late,
                    off_ms,
                })
            }
            _ => {}
        }

        if pocket == Pocket::Free {
            let hits = f64::from(self.hits);
            let early = f64::from(self.early_hits) / hits;
            let late = f64::from(self.late_hits) / hits;
            if late >= LOPSIDED {
                return Some(Observation::Lean {
                    side: Side::Late,
                    share: late,
                });
            }
            if early >= LOPSIDED {
                return Some(Observation::Lean {
                    side: Side::Early,
                    share: early,
                });
            }
        }
        Some(Observation::Solid)
    }
}

/// Where the notes fell that missed the timing window, in real ms (score seconds / `speed`;
/// negative = early). Every extra note of a pass within `OUTSIDE_LIMIT_MS` of a missed note is
/// a candidate offset; the one most missed notes share (within `OUTSIDE_TOLERANCE_MS`) wins,
/// the nearer on a tie, and the median of the offsets near it is told. `None` unless at least
/// half the missed notes share it. Example: sixteenths 125 ms apart, every note 70 ms late:
/// +70 fits all eight notes, −55 (the note before, late) only seven.
pub(crate) fn outside_offset_ms(results: &[NoteResult], speed: f64) -> Option<f64> {
    let mut offsets: Vec<f64> = outside_notes(results, speed)
        .iter()
        .map(|n| n.offset_ms)
        .collect();
    offsets.sort_by(f64::total_cmp);
    offsets.get(offsets.len() / 2).copied()
}

/// The missed notes that share the timing `outside_offset_ms` tells, each with the offset of
/// its played note; empty when fewer than half the missed notes share one.
pub(crate) fn outside_notes(results: &[NoteResult], speed: f64) -> Vec<OutsideNote> {
    let speed = speed.max(f64::EPSILON);
    let missed: Vec<NoteResult> = final_per_note(results)
        .into_iter()
        .filter(|r| matches!(r.outcome, Outcome::Missed))
        .collect();
    let extras: Vec<&NoteResult> = results
        .iter()
        .filter(|r| matches!(r.outcome, Outcome::Extra { .. }))
        .collect();

    // Per missed note, the offsets of the extra notes around it.
    let around: Vec<Vec<f64>> = missed
        .iter()
        .map(|m| {
            extras
                .iter()
                .filter(|e| e.pass == m.pass)
                .map(|e| (e.position_s - m.position_s) * 1000.0 / speed)
                .filter(|ms| ms.abs() <= OUTSIDE_LIMIT_MS)
                .collect()
        })
        .collect();
    // For an offset, each missed note's nearest one to it, if close enough.
    let near = |at: f64| -> Vec<OutsideNote> {
        missed
            .iter()
            .zip(&around)
            .filter_map(|(m, offsets)| {
                let offset_ms = offsets
                    .iter()
                    .copied()
                    .min_by(|a, b| (a - at).abs().total_cmp(&(b - at).abs()))
                    .filter(|o| (o - at).abs() <= OUTSIDE_TOLERANCE_MS)?;
                Some(OutsideNote {
                    pass: m.pass,
                    note: m.note?,
                    offset_ms,
                })
            })
            .collect()
    };
    // Candidates to the whole ms: a long run has thousands of notes but few distinct offsets.
    let mut candidates: Vec<f64> = around.iter().flatten().map(|o| o.round()).collect();
    candidates.sort_by(f64::total_cmp);
    candidates.dedup();
    let best = candidates
        .into_iter()
        .map(|at| (near(at).len(), at))
        .max_by(|(n, a), (m, b)| n.cmp(m).then(b.abs().total_cmp(&a.abs())));
    let Some((count, at)) = best else {
        return Vec::new();
    };
    if count * 2 < missed.len() {
        return Vec::new();
    }
    near(at)
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

/// Standard deviation around `mean`.
fn spread(values: &[f64], mean: f64) -> f64 {
    (values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64).sqrt()
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
    // Most missed notes first: 6 of 8 missed is harder than 1 of 1. The rate breaks ties.
    let mut bars: Vec<(usize, u32, f64)> = per_bar
        .into_iter()
        .filter(|(bar, (hits, total))| hits < total && *bar < arrangement.bars.len())
        .map(|(bar, (hits, total))| (bar, total - hits, f64::from(hits) / f64::from(total)))
        .collect();
    bars.sort_by(|a, b| b.1.cmp(&a.1).then(a.2.total_cmp(&b.2)).then(a.0.cmp(&b.0)));
    bars.into_iter()
        .take(HARDEST_BARS)
        .map(|(bar, _, _)| bar)
        .collect()
}

/// Bars the player hesitated at: the bar before was played to its end, then the bar's first
/// note came late (`HESITATE_MS` behind the run's mean offset) or not at all, in at least half
/// of the passes that came there (a loop of twenty passes with one slip is no hesitation).
/// `results` are the final results per note, in order of pass and note.
///
/// ```text
/// bar      3          4
/// played   ♩ ♩ ♩ ♩ |     ♩ ♩ ♩      first note of bar 4 missed: hesitated at 4
/// ```
fn hesitant_bars(results: &[NoteResult], mean_offset_ms: f64) -> Vec<usize> {
    // Per bar: passes that came to its first note from a played bar, and those that hesitated.
    let mut seen: BTreeMap<usize, (u32, u32)> = BTreeMap::new();
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
        let entry = seen.entry(bar).or_default();
        entry.0 += 1;
        if matches!(first.outcome, Outcome::Missed) || late {
            entry.1 += 1;
        }
    }
    seen.into_iter()
        .filter(|(_, (came, hesitated))| *hesitated > 0 && 2 * hesitated >= *came)
        .map(|(bar, _)| bar)
        .collect()
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

    /// QA 27.10. (P3 Mira, M2): a strict window and every note 40 ms early read "0 % hit,
    /// 128 missed" with no word of the timing. The missed notes' nearest played notes tell it.
    #[test]
    fn notes_outside_the_window_tell_their_offset() {
        let missed = (Outcome::Missed, None);
        let mut results = played(&[missed, missed, missed, missed]);
        let extra = |at: f64| NoteResult {
            pass: 0,
            note: None,
            bar: None,
            outcome: Outcome::Extra { played_midi: 40.0 },
            offset_ms: None,
            cents: None,
            position_s: at,
        };
        // Played 40 ms early at full speed; at half speed the same gap in the score is 20 ms.
        results.extend([0.0, 0.5, 1.0, 1.5].map(|at| extra(at - 0.040)));
        let early = outside_offset_ms(&results, 1.0).expect("an offset");
        assert!((early + 40.0).abs() < 1e-6, "{early}");
        let slow = outside_offset_ms(&results, 0.5).expect("an offset");
        assert!((slow + 80.0).abs() < 1e-6, "{slow}");

        // Each missed note keeps its own offset for the note list (M6).
        let notes = outside_notes(&results, 1.0);
        assert_eq!(notes.len(), 4, "{notes:?}");
        assert!(notes.iter().all(|n| (n.offset_ms + 40.0).abs() < 1e-6));

        // Nothing played near the missed notes: nothing to tell.
        let far = played(&[missed, missed]);
        assert_eq!(outside_offset_ms(&far, 1.0), None);

        // Sixteenths at 120 BPM (125 ms apart): 70 ms late lies nearer the next note than its
        // own; it must still read late (PR-Agent #147: it read "55 ms early").
        let dense = |shift: f64| {
            let mut r: Vec<NoteResult> = (0..8)
                .map(|i| NoteResult {
                    position_s: f64::from(i) * 0.125,
                    ..played(&[missed])[0]
                })
                .enumerate()
                .map(|(i, r)| NoteResult { note: Some(i), ..r })
                .collect();
            r.extend((0..8).map(|i| extra(f64::from(i) * 0.125 + shift)));
            r
        };
        let early = outside_offset_ms(&dense(-0.040), 1.0).expect("an offset");
        assert!((early + 40.0).abs() < 1e-6, "{early}");
        let late = outside_offset_ms(&dense(0.070), 1.0).expect("an offset");
        assert!((late - 70.0).abs() < 1e-6, "{late}");
        let early = outside_offset_ms(&dense(-0.070), 1.0).expect("an offset");
        assert!((early + 70.0).abs() < 1e-6, "{early}");
    }

    /// LH-EIN-14: a steady lean off the beat asks for a new calibration; scatter, too few hits
    /// or a lean the aim covers do not.
    #[test]
    fn steady_lean_asks_to_recalibrate() {
        let arrangement = Arrangement::new("Run", kontra_model::Instrument::default_bass());
        let summary = |offsets: &[f64]| {
            let notes: Vec<(Outcome, Option<f64>)> =
                offsets.iter().map(|o| (Outcome::Hit, Some(*o))).collect();
            Summary::from_results(&played(&notes), &arrangement)
        };

        let late = summary(&[25.0; 10]);
        assert_eq!(late.recalibrate(Pocket::Free), Some(Side::Late));
        assert_eq!(late.recalibrate(Pocket::OnBeat), Some(Side::Late));
        assert_eq!(
            late.recalibrate(Pocket::LaidBack),
            None,
            "played behind on purpose"
        );
        assert_eq!(
            summary(&[-8.0; 10]).recalibrate(Pocket::Free),
            Some(Side::Early)
        );

        // Symmetric scatter with a mean of 8 ms: half on each side.
        let scatter = [
            -30.0, 46.0, -30.0, 46.0, -30.0, 46.0, -30.0, 46.0, -30.0, 46.0,
        ];
        assert_eq!(summary(&scatter).recalibrate(Pocket::Free), None);

        assert_eq!(
            summary(&[25.0; 7]).recalibrate(Pocket::Free),
            None,
            "too few hits"
        );
        assert_eq!(
            summary(&[4.0; 10]).recalibrate(Pocket::Free),
            None,
            "near the beat"
        );
        // Judged in whole ms like the aim: 5.3 ms is held on the beat, no hint.
        assert_eq!(summary(&[5.3; 10]).recalibrate(Pocket::OnBeat), None);
    }

    /// LH-BEW-09: one observation per run, each only with enough notes behind it.
    #[test]
    fn one_observation_with_enough_evidence() {
        let arrangement = Arrangement::new("Run", kontra_model::Instrument::default_bass());
        let hit = |ms: f64| (Outcome::Hit, Some(ms));
        let missed = (Outcome::Missed, None);
        let summary =
            |notes: &[(Outcome, Option<f64>)]| Summary::from_results(&played(notes), &arrangement);
        let slide = |played: u32, judged: u32| TechniqueTally {
            technique: Technique::Slide,
            played,
            judged,
        };

        // Too few notes: nothing to tell.
        assert_eq!(summary(&[hit(0.0); 7]).observation(Pocket::Free), None);

        // One missed slide among ten good notes is no diagnosis.
        let mut s = summary(&[hit(0.0); 10]);
        s.techniques = vec![slide(0, 1)];
        assert_eq!(s.observation(Pocket::Free), Some(Observation::Solid));

        // Two of six slides at a 90 % hit rate: the slide is the point.
        let mut notes = vec![hit(0.0); 9];
        notes.push(missed);
        let mut s = summary(&notes);
        s.techniques = vec![slide(2, 6)];
        let expected = Observation::Technique {
            technique: Technique::Slide,
            played: 2,
            judged: 6,
        };
        assert_eq!(s.observation(Pocket::Free), Some(expected));

        // Half the notes missed.
        let mut notes = vec![hit(0.0); 5];
        notes.extend([missed; 5]);
        let expected = Observation::Missed {
            notes: 5,
            judged: 10,
        };
        assert_eq!(summary(&notes).observation(Pocket::Free), Some(expected));

        // Missed, but played beside the window: the timing is told, not the misses.
        let mut s = summary(&notes);
        s.missed = 6;
        s.outside_offset_ms = Some(-40.0);
        let expected = Observation::Beside { offset_ms: -40.0 };
        assert_eq!(s.observation(Pocket::Free), Some(expected));

        // Eight of ten hits 25 ms late lean late; with a laid-back aim the pocket line tells it.
        let mut notes = vec![hit(25.0); 8];
        notes.extend([hit(0.0); 2]);
        let s = summary(&notes);
        let lean = Some(Observation::Lean {
            side: Side::Late,
            share: 0.8,
        });
        assert_eq!(s.observation(Pocket::Free), lean);
        assert_eq!(s.observation(Pocket::LaidBack), Some(Observation::Solid));

        // PR-Agent #184: an aim on the beat (±5 ms) missed by 20 ms is no "solid" run.
        let s = summary(&[hit(25.0); 10]);
        let aim = Some(Observation::Aim {
            side: Side::Late,
            off_ms: 20.0,
        });
        assert_eq!(s.observation(Pocket::OnBeat), aim);

        // PR-Agent #184: 0.3 ms past ±5 ms reads "0 ms behind": held, as the pocket line says.
        let s = summary(&[hit(5.3); 10]);
        assert_eq!(s.observation(Pocket::OnBeat), Some(Observation::Solid));

        // PR-Agent #184: seven hits 25 ms late and three missed: too few hits to name the aim,
        // and then not "solid" either.
        let mut notes = vec![hit(25.0); 7];
        notes.extend([missed; 3]);
        assert_eq!(summary(&notes).observation(Pocket::OnBeat), None);

        // PR-Agent #184: 11 of 20 slides at 15 of 20 hits is exactly 20 points under.
        let mut notes = vec![hit(0.0); 15];
        notes.extend([missed; 5]);
        let mut s = summary(&notes);
        s.techniques = vec![slide(11, 20)];
        let expected = Observation::Technique {
            technique: Technique::Slide,
            played: 11,
            judged: 20,
        };
        assert_eq!(s.observation(Pocket::Free), Some(expected));

        // Wide scatter both ways is no lean.
        let notes: Vec<_> = (0..10)
            .map(|i| hit(if i % 2 == 0 { 30.0 } else { -30.0 }))
            .collect();
        assert_eq!(
            summary(&notes).observation(Pocket::Free),
            Some(Observation::Solid)
        );
    }

    /// LH-BEW-12: constant −10 cents reads "flat"; ±10 around 0 has no direction, only spread.
    #[test]
    fn intonation_tells_direction_and_spread() {
        let flat = Intonation::from_cents(&[-10.0; 8]).expect("intonation");
        assert_eq!(flat.leaning(), Some(Leaning::Flat));
        assert!(flat.spread_cent.abs() < 1e-9, "{flat:?}");

        let scattered = Intonation::from_cents(&[-10.0, 10.0, -10.0, 10.0]).expect("intonation");
        assert_eq!(scattered.leaning(), None);
        assert!((scattered.spread_cent - 10.0).abs() < 1e-9, "{scattered:?}");

        assert_eq!(Intonation::from_cents(&[]), None);
    }

    /// LH-BEW-12: per string and position only from eight notes on. Eight on the E string at
    /// frets 5 to 9, three on the A string: one part.
    #[test]
    fn intonation_parts_need_eight_notes() {
        use kontra_model::{Instrument, Note, Pitch};

        let mut arrangement = Arrangement::new("Parts", Instrument::default_bass());
        let place = |string: u8, fret: u8| Note {
            string: Some(string),
            fret: Some(fret),
            ..Note::new(0, 1, Pitch(28 + fret))
        };
        arrangement
            .notes
            .extend((5..13).map(|i| place(0, 5 + i % 5)));
        arrangement.notes.extend((0..3).map(|_| place(1, 2)));
        let mut results = played(&[(Outcome::Hit, Some(0.0)); 11]);
        for (i, r) in results.iter_mut().enumerate() {
            r.cents = Some(if i < 8 { 12.0 } else { -20.0 });
        }

        let parts = intonation_parts(&results, &arrangement);
        assert_eq!(parts.len(), 1, "{parts:?}");
        assert_eq!((parts[0].string, parts[0].first_fret), (0, 5));
        assert_eq!(parts[0].intonation.leaning(), Some(Leaning::Sharp));
    }

    /// Harmonicon analysis (29.10.): the hardest bar is the one that costs the most notes. A bar
    /// with its one note missed (0 of 1) ranked above one with 6 of 8 missed (2 of 8).
    #[test]
    fn hardest_bars_rank_by_misses() {
        use kontra_model::{Instrument, Note, Pitch, TempoMap, TICKS_PER_QUARTER};

        let mut arrangement = Arrangement::new("Bars", Instrument::default_bass());
        arrangement.tempo = TempoMap::constant(120.0);
        for i in 0..16 {
            let start = i * TICKS_PER_QUARTER;
            arrangement
                .notes
                .push(Note::new(start, TICKS_PER_QUARTER, Pitch(28)));
        }
        arrangement.normalize();

        let hit = (Outcome::Hit, Some(0.0));
        let missed = (Outcome::Missed, None);
        let in_bar = |bar: usize, notes: &[(Outcome, Option<f64>)]| -> Vec<NoteResult> {
            played(notes)
                .into_iter()
                .map(|r| NoteResult {
                    bar: Some(bar),
                    ..r
                })
                .collect()
        };
        let mut results = in_bar(0, &[missed]);
        results.extend(in_bar(
            1,
            &[hit, hit, missed, missed, missed, missed, missed, missed],
        ));

        assert_eq!(hardest_bars(&results, &arrangement), vec![1, 0]);
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

    #[test]
    fn one_slip_in_a_loop_is_no_hesitation() {
        let on = (Outcome::Hit, Some(0.0));
        let missed = (Outcome::Missed, None);
        // Bars 0 and 1 looped four times; bar 1's first note missed once, then twice.
        let mut results = Vec::new();
        for pass in 0..4 {
            let start = if pass == 0 { missed } else { on };
            for mut r in played(&[on, on, on, on, start, on, on, on]) {
                r.pass = pass;
                results.push(r);
            }
        }
        assert!(hesitant_bars(&results, 0.0).is_empty(), "once in four");
        results[4 + 8].outcome = Outcome::Missed;
        assert_eq!(hesitant_bars(&results, 0.0), [1], "half of the passes");
    }
}
