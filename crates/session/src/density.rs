//! Adaptive difficulty (LH-MOD-06): a share of the notes to play while a part is new, more as
//! the hit rate rises. Only between runs and only as a suggestion (round 5): the player decides.
//! Method (Harmonicon analysis, 29.10.): each section has its played share; a clean run raises it
//! one step.
//!
//! Each bar keeps its strongest attacks: the first, then the middle of the bar, the beats, the
//! eighths and the rest, earlier before later.
//!
//! ```text
//! 4/4  | e e e e e e e e |  100 %  every note
//!      | e e e e e   e   |   75 %
//!      | q   q   q   q   |   50 %  notes on a beat
//!      | h       h       |   25 %  beat 1 and the middle of the bar
//! ```
//! A bar keeps at least its first attack, so no bar goes silent.

use kontra_model::{Arrangement, Note, TICKS_PER_QUARTER};
use serde::{Deserialize, Serialize};

/// Hit rate of a clean run: the next run is offered more notes.
const MORE_FROM: f64 = 0.9;
/// Hit rate below which the next run is offered fewer notes.
const FEWER_BELOW: f64 = 0.6;
/// One step of the share, in percent.
const STEP_PERCENT: u8 = 25;
const ALL_PERCENT: u8 = 100;

/// The share of the written notes the player is asked to play, in percent: 25, 50, 75 or 100.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Density(u8);

impl Default for Density {
    fn default() -> Self {
        Density::FULL
    }
}

impl Density {
    /// Every note.
    pub const FULL: Density = Density(ALL_PERCENT);
    /// The fewest notes: a quarter of each bar's attacks.
    pub const FEWEST: Density = Density(STEP_PERCENT);

    /// The share in percent, as the UI shows it.
    pub fn percent(self) -> u8 {
        self.0
    }

    /// The density nearest to `percent`, on a step; out of range clamps.
    pub fn from_percent(percent: u8) -> Density {
        let steps = (u32::from(percent) + u32::from(STEP_PERCENT) / 2) / u32::from(STEP_PERCENT);
        let percent = (steps * u32::from(STEP_PERCENT))
            .clamp(u32::from(STEP_PERCENT), u32::from(ALL_PERCENT));
        Density(u8::try_from(percent).unwrap_or(ALL_PERCENT))
    }

    /// The suggestion after a run with `hit_rate` (0..=1): one step more notes after a clean
    /// run (from 90 %), one step fewer below 60 %, else none.
    pub fn suggest(self, hit_rate: f64) -> Option<Density> {
        let next = if hit_rate >= MORE_FROM {
            Density::from_percent(self.0.saturating_add(STEP_PERCENT))
        } else if hit_rate < FEWER_BELOW {
            Density::from_percent(self.0.saturating_sub(STEP_PERCENT))
        } else {
            return None;
        };
        (next != self).then_some(next)
    }

    /// `arrangement` with only the notes of this share. Tied notes and hammer-on/pull-off
    /// targets follow the attack they belong to.
    pub fn thin(self, arrangement: &Arrangement) -> Arrangement {
        let mut out = arrangement.clone();
        if self == Density::FULL {
            return out;
        }
        let kept = self.kept_attacks(arrangement);
        let mut keep_attack = true;
        out.notes = arrangement
            .notes
            .iter()
            .enumerate()
            .filter(|(i, note)| {
                if !note.needs_attack() {
                    return keep_attack;
                }
                keep_attack = kept.contains(i);
                keep_attack
            })
            .map(|(_, note)| note.clone())
            .collect();
        out
    }

    /// Indices of the attacks kept: per bar the strongest `share` of them, at least one.
    fn kept_attacks(self, arrangement: &Arrangement) -> Vec<usize> {
        let mut kept = Vec::new();
        let mut bar_attacks: Vec<(u8, usize)> = Vec::new();
        let mut current_bar = None;
        for (i, note) in arrangement.notes.iter().enumerate() {
            if !note.needs_attack() {
                continue;
            }
            let bar = arrangement.bar_at(note.start_ticks);
            if bar != current_bar {
                self.keep_strongest(&mut bar_attacks, &mut kept);
                current_bar = bar;
            }
            bar_attacks.push((weight(arrangement, note), i));
        }
        self.keep_strongest(&mut bar_attacks, &mut kept);
        kept
    }

    /// Moves the strongest share of one bar's attacks (weight, index) into `kept`.
    fn keep_strongest(self, attacks: &mut Vec<(u8, usize)>, kept: &mut Vec<usize>) {
        if attacks.is_empty() {
            return;
        }
        attacks.sort();
        let count = (attacks.len() * usize::from(self.0)).div_ceil(usize::from(ALL_PERCENT));
        kept.extend(attacks.iter().take(count.max(1)).map(|(_, i)| *i));
        attacks.clear();
    }
}

/// How weak a note's place in its bar is: 0 the bar's first attack, 1 the middle of a bar with
/// an even number of beats (4/4: beat 3), 2 a beat, 3 an eighth, 4 anything else.
fn weight(arrangement: &Arrangement, note: &Note) -> u8 {
    let Some(bar) = arrangement
        .bar_at(note.start_ticks)
        .and_then(|b| arrangement.bars.get(b))
    else {
        return 0;
    };
    if first_attack(arrangement, note) {
        return 0;
    }
    let offset = note.start_ticks - bar.start_ticks;
    let signature = bar.signature;
    let beat = TICKS_PER_QUARTER * 4 / u64::from(signature.denominator.max(1));
    let half = signature.bar_ticks() / 2;
    if offset == 0 || (signature.numerator.is_multiple_of(2) && offset == half) {
        1
    } else if offset.is_multiple_of(beat) {
        2
    } else if offset.is_multiple_of(TICKS_PER_QUARTER / 2) {
        3
    } else {
        4
    }
}

/// True if no earlier attack of its bar exists: the bar's first note to strike.
fn first_attack(arrangement: &Arrangement, note: &Note) -> bool {
    let Some(bar) = arrangement
        .bar_at(note.start_ticks)
        .and_then(|b| arrangement.bars.get(b))
    else {
        return false;
    };
    !arrangement
        .notes_between(bar.start_ticks, note.start_ticks)
        .iter()
        .any(Note::needs_attack)
}
