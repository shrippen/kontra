//! Building blocks (DIDAKTIK I6, LH-MOD-08): a part is cut into blocks of a few bars, practised
//! one at a time and joined, so the player always plays into something known.
//!
//! ```text
//! blocks      [1][2][3]                    forward:   1 · 2 · 1–2 · 3 · 1–3
//! bars        1-2 3-4 5-6                  from back: 3 · 2 · 2–3 · 1 · 1–3
//! ```
//!
//! A step sits after `passes_needed` passes in a row at or above the workshop band's floor;
//! then the next step's bars become the loop.
//!
//! Blocks are a fixed number of bars, or phrases: a phrase ends at a bar line with at least a
//! quarter of silence around it, as a bass line breathes between two phrases.
//!
//! ```text
//! notes   ♩♩♩♩|♩♩♩𝄽|♩♩♩♩|♩♩𝄽𝄽|     rests at bar lines 2|3 and 4|5
//! phrases [ 1   2 ][ 3   4 ]
//! ```

use crate::session::Workshop;
use kontra_model::{Arrangement, TICKS_PER_QUARTER};

/// Silence at a bar line that ends a phrase: a quarter.
const PHRASE_GAP_TICKS: u64 = TICKS_PER_QUARTER;
/// A phrase shorter than this joins the next one; one longer is cut into blocks of the most.
const PHRASE_MIN_BARS: usize = 2;
const PHRASE_MAX_BARS: usize = 4;

/// How the bars are cut into blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blocks {
    /// A fixed number of bars per block.
    Bars(usize),
    /// Phrases found in the rests of the line.
    Phrases,
}

/// In which order the blocks are learnt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainOrder {
    /// First block first; each new block is joined to the ones before.
    Forward,
    /// Last block first: every new block leads into what is already known.
    Backward,
}

/// Steps through the blocks of a part; bars are indices into `Arrangement::bars`.
#[derive(Debug, Clone, PartialEq)]
pub struct Chain {
    steps: Vec<(usize, usize)>,
    at: usize,
    streak: u32,
    floor: f64,
    passes_needed: u32,
}

impl Chain {
    /// Cuts bars `first..=last` of `a` into blocks, e.g. bars 0–5 by 2, forward:
    /// (0,1), (2,3), (0,3), (4,5), (0,5).
    pub fn new(
        a: &Arrangement,
        (first, last): (usize, usize),
        blocks: Blocks,
        order: ChainOrder,
        w: &Workshop,
    ) -> Chain {
        let (first, last) = (first.min(last), first.max(last));
        let blocks = match blocks {
            Blocks::Bars(size) => by_size(first, last, size),
            Blocks::Phrases => phrases(a, first, last),
        };

        let mut steps = Vec::new();
        match order {
            ChainOrder::Forward => {
                for (i, block) in blocks.iter().enumerate() {
                    steps.push(*block);
                    if i > 0 {
                        steps.push((first, block.1));
                    }
                }
            }
            ChainOrder::Backward => {
                for (i, block) in blocks.iter().enumerate().rev() {
                    steps.push(*block);
                    if i + 1 < blocks.len() {
                        steps.push((block.0, last));
                    }
                }
            }
        }

        Chain {
            steps,
            at: 0,
            streak: 0,
            floor: w.band.0,
            passes_needed: w.passes_needed.max(1),
        }
    }

    /// Bars of the current step.
    pub fn step(&self) -> (usize, usize) {
        self.steps[self.at]
    }

    /// Index of the current step, from 0.
    pub fn index(&self) -> usize {
        self.at
    }

    /// Number of steps, the whole part included.
    pub fn count(&self) -> usize {
        self.steps.len()
    }

    /// The last step (the whole part) is reached.
    pub fn is_last(&self) -> bool {
        self.at + 1 == self.steps.len()
    }

    /// Counts a finished pass; returns the next step's bars when the current one sits.
    pub fn on_pass(&mut self, hit_rate: f64) -> Option<(usize, usize)> {
        if hit_rate < self.floor {
            self.streak = 0;
            return None;
        }
        self.streak += 1;
        if self.streak < self.passes_needed || self.is_last() {
            return None;
        }

        self.streak = 0;
        self.at += 1;
        Some(self.step())
    }
}

/// Bars `first..=last` in blocks of `size`, the last one shorter if need be.
fn by_size(first: usize, last: usize, size: usize) -> Vec<(usize, usize)> {
    let size = size.max(1);
    (first..=last)
        .step_by(size)
        .map(|start| (start, (start + size - 1).min(last)))
        .collect()
}

/// Bars `first..=last` cut at the bar lines with a quarter of silence (see the module docs);
/// short phrases join the next, long ones are cut in blocks of `PHRASE_MAX_BARS`.
fn phrases(a: &Arrangement, first: usize, last: usize) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = first;
    for bar in first..=last {
        let length = bar + 1 - start;
        let end = bar == last
            || length >= PHRASE_MAX_BARS
            || (length >= PHRASE_MIN_BARS && breathes(a, bar + 1));
        if end {
            out.push((start, bar));
            start = bar + 1;
        }
    }

    // A short tail joins the phrase before it, when that stays within the most.
    if let [.., before, tail] = out.as_slice() {
        if tail.1 - tail.0 + 1 < PHRASE_MIN_BARS && tail.1 - before.0 < PHRASE_MAX_BARS {
            let joined = (before.0, tail.1);
            out.truncate(out.len() - 2);
            out.push(joined);
        }
    }
    out
}

/// Whether the line rests for at least a quarter around the start of `bar`.
fn breathes(a: &Arrangement, bar: usize) -> bool {
    let Some(line) = a.bars.get(bar).map(|b| b.start_ticks) else {
        return true;
    };
    let before = a
        .notes
        .iter()
        .filter(|n| n.start_ticks < line)
        .map(|n| n.end_ticks())
        .max()
        .unwrap_or(0);
    let after = a
        .notes
        .iter()
        .map(|n| n.start_ticks)
        .filter(|t| *t >= line)
        .min()
        .unwrap_or(u64::MAX);
    after.saturating_sub(before) >= PHRASE_GAP_TICKS
}

#[cfg(test)]
mod tests {
    use super::*;
    use kontra_model::{Bar, Instrument, Note, Pitch, TimeSignature};

    const BAR: u64 = 4 * TICKS_PER_QUARTER;

    /// `bars` bars of 4/4; each string is one bar of quarters, "x" a note, "." a rest.
    fn line(bars: &[&str]) -> Arrangement {
        let mut a = Arrangement::new("Linie", Instrument::default_bass());
        for (i, beats) in bars.iter().enumerate() {
            let start = i as u64 * BAR;
            a.bars.push(Bar {
                start_ticks: start,
                signature: TimeSignature {
                    numerator: 4,
                    denominator: 4,
                },
                source_index: i as u32,
                key_fifths: 0,
                repeat_open: false,
                repeat_close: false,
                endings: 0,
            });
            for (b, c) in beats.chars().enumerate() {
                if c == 'x' {
                    let at = start + b as u64 * TICKS_PER_QUARTER;
                    a.notes.push(Note::new(at, TICKS_PER_QUARTER, Pitch(40)));
                }
            }
        }
        a
    }

    fn chain(first: usize, last: usize, size: usize, order: ChainOrder) -> Vec<(usize, usize)> {
        let a = line(&["xxxx"; 12]);
        Chain::new(
            &a,
            (first, last),
            Blocks::Bars(size),
            order,
            &Workshop::default(),
        )
        .steps
    }

    #[test]
    fn phrases_end_where_the_line_rests() {
        let a = line(&[
            "xxxx", "xxx.", "xxxx", "xx..", "xxxx", "xxxx", "xxx.", "xxxx",
        ]);
        // The short tail (bar 7) joins bars 4–6.
        assert_eq!(phrases(&a, 0, 7), [(0, 1), (2, 3), (4, 7)]);
    }

    #[test]
    fn a_line_without_rests_falls_back_to_four_bars() {
        let a = line(&["xxxx"; 10]);
        assert_eq!(phrases(&a, 0, 9), [(0, 3), (4, 7), (8, 9)]);
    }

    #[test]
    fn a_rest_in_every_bar_still_gives_two_bar_phrases() {
        let a = line(&["xxx."; 6]);
        assert_eq!(phrases(&a, 0, 5), [(0, 1), (2, 3), (4, 5)]);
    }

    #[test]
    fn phrases_chain_like_blocks() {
        let a = line(&["xxxx", "xxx.", "xxxx", "xx.."]);
        let c = Chain::new(
            &a,
            (0, 3),
            Blocks::Phrases,
            ChainOrder::Forward,
            &Workshop::default(),
        );
        assert_eq!(c.steps, [(0, 1), (2, 3), (0, 3)]);
    }

    #[test]
    fn joins_blocks_forward_and_from_the_back() {
        let forward = chain(0, 5, 2, ChainOrder::Forward);
        assert_eq!(forward, [(0, 1), (2, 3), (0, 3), (4, 5), (0, 5)]);

        let backward = chain(0, 5, 2, ChainOrder::Backward);
        assert_eq!(backward, [(4, 5), (2, 3), (2, 5), (0, 1), (0, 5)]);
    }

    #[test]
    fn a_short_last_block_and_a_single_block() {
        assert_eq!(
            chain(3, 7, 4, ChainOrder::Forward),
            [(3, 6), (7, 7), (3, 7)]
        );
        assert_eq!(chain(0, 1, 2, ChainOrder::Forward), [(0, 1)]);
        assert_eq!(
            chain(3, 0, 2, ChainOrder::Forward),
            [(0, 1), (2, 3), (0, 3)]
        );
    }

    #[test]
    fn moves_on_after_two_passes_that_sit() {
        let a = line(&["xxxx"; 4]);
        let mut c = Chain::new(
            &a,
            (0, 3),
            Blocks::Bars(2),
            ChainOrder::Forward,
            &Workshop::default(),
        );
        assert_eq!(c.on_pass(0.95), None);
        assert_eq!(c.on_pass(0.6), None, "a weak pass starts the count again");
        assert_eq!(c.on_pass(0.85), None);
        assert_eq!(c.on_pass(0.85), Some((2, 3)));
        assert_eq!(c.on_pass(0.9), None);
        assert_eq!(c.on_pass(0.9), Some((0, 3)));
        assert!(c.is_last());
        assert_eq!(c.on_pass(1.0), None);
        assert_eq!(c.on_pass(1.0), None, "the whole part stays");
        assert_eq!((c.index(), c.count()), (2, 3));
    }
}
