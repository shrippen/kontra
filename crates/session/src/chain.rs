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

use crate::session::Workshop;

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
    /// Cuts bars `first..=last` into blocks of `size` bars, e.g. bars 0–5 by 2, forward:
    /// (0,1), (2,3), (0,3), (4,5), (0,5).
    pub fn new(first: usize, last: usize, size: usize, order: ChainOrder, w: &Workshop) -> Chain {
        let size = size.max(1);
        let (first, last) = (first.min(last), first.max(last));
        let blocks: Vec<(usize, usize)> = (first..=last)
            .step_by(size)
            .map(|start| (start, (start + size - 1).min(last)))
            .collect();

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

#[cfg(test)]
mod tests {
    use super::*;

    fn chain(first: usize, last: usize, size: usize, order: ChainOrder) -> Vec<(usize, usize)> {
        Chain::new(first, last, size, order, &Workshop::default()).steps
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
        let mut c = Chain::new(0, 3, 2, ChainOrder::Forward, &Workshop::default());
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
