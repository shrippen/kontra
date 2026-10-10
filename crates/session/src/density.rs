//! Adaptive difficulty (LH-MOD-06): fewer notes to play while a part is new, more as the hit
//! rate rises. Only between runs and only as a suggestion (round 5): the player decides.
//!
//! ```text
//! 4/4  | e e e e e e e e |     Full    every note
//!      | q   q   q   q   |     Beats   notes on a beat
//!      | h       h       |     Sparse  notes on beat 1 and the middle of the bar
//! ```
//! A bar keeps at least its first attack, so no bar goes silent.

use kontra_model::{Arrangement, Note};
use serde::{Deserialize, Serialize};

/// Hit rate from which the next run is offered more notes.
const MORE_FROM: f64 = 0.9;
/// Hit rate below which the next run is offered fewer notes.
const FEWER_BELOW: f64 = 0.6;

/// How many of the written notes the player is asked to play.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub enum Density {
    Sparse,
    Beats,
    #[default]
    Full,
}

impl Density {
    /// 1 (sparse) to 3 (all notes), as the UI shows it.
    pub fn level(self) -> u8 {
        match self {
            Density::Sparse => 1,
            Density::Beats => 2,
            Density::Full => 3,
        }
    }

    /// The density of `level`; out of range clamps.
    pub fn from_level(level: u8) -> Density {
        match level {
            0 | 1 => Density::Sparse,
            2 => Density::Beats,
            _ => Density::Full,
        }
    }

    /// The suggestion after a run with `hit_rate` (0..=1): one step more notes from 90 %, one
    /// step fewer below 60 %, else none.
    pub fn suggest(self, hit_rate: f64) -> Option<Density> {
        let next = if hit_rate >= MORE_FROM {
            Density::from_level(self.level() + 1)
        } else if hit_rate < FEWER_BELOW {
            Density::from_level(self.level().saturating_sub(1))
        } else {
            return None;
        };
        (next != self).then_some(next)
    }

    /// `arrangement` with only the notes of this density. Tied notes and hammer-on/pull-off
    /// targets follow the attack they belong to.
    pub fn thin(self, arrangement: &Arrangement) -> Arrangement {
        let mut out = arrangement.clone();
        if self == Density::Full {
            return out;
        }
        let mut keep_attack = true;
        out.notes = arrangement
            .notes
            .iter()
            .filter(|note| {
                if !note.needs_attack() {
                    return keep_attack;
                }
                keep_attack = first_attack(arrangement, note) || self.on_grid(arrangement, note);
                keep_attack
            })
            .cloned()
            .collect();
        out
    }

    /// True if `note` starts on this density's grid in its bar.
    fn on_grid(self, arrangement: &Arrangement, note: &Note) -> bool {
        let Some(bar) = arrangement
            .bar_at(note.start_ticks)
            .and_then(|b| arrangement.bars.get(b))
        else {
            return true;
        };
        let offset = note.start_ticks - bar.start_ticks;
        let signature = bar.signature;
        let beat = kontra_model::TICKS_PER_QUARTER * 4 / u64::from(signature.denominator.max(1));
        match self {
            Density::Full => true,
            Density::Beats => offset.is_multiple_of(beat),
            // Beat 1, and the middle of a bar with an even number of beats (4/4: beat 3).
            Density::Sparse => {
                let half = signature.bar_ticks() / 2;
                offset == 0 || (signature.numerator.is_multiple_of(2) && offset == half)
            }
        }
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
