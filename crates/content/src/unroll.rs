//! Playing order of the written bars: repeats, alternate endings and directions
//! (D.C., D.S., segno, coda, fine) unrolled into a linear list of bar indices.
//!
//! ```text
//! |: A | B(1.) :| C(2.) | D  D.C. al Fine     A B A C D  A C(Fine)
//!                 Fine
//! ```
//! Rules: a jump is taken once; after a D.C./D.S. jump repeats are not taken again and
//! alternate endings play their last ending only (common practice).

use crate::error::ImportError;
use crate::score::{BarHead, Jump, Target, Until};

/// Upper bound for the unrolled timeline; protects against absurd repeat counts.
pub(crate) const MAX_BARS: usize = 20_000;

/// More plays than this are clamped (Guitar Pro allows up to 99, nobody writes that).
const MAX_PLAYS: u8 = 32;

/// What happens after a bar is played.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Next {
    Continue,
    Goto(usize),
    Stop,
}

/// Source bar indices in playing order.
pub(crate) fn unroll(bars: &[BarHead]) -> Result<Vec<usize>, ImportError> {
    Unroller::new(bars).run()
}

#[derive(Debug)]
struct Unroller<'a> {
    bars: &'a [BarHead],
    order: Vec<usize>,
    /// Repeat jumps already taken per closing bar.
    repeats: Vec<u8>,
    /// Direction jumps already taken per bar.
    taken: Vec<bool>,
    /// Last alternate ending of each bar's ending group (bit mask).
    finals: Vec<u32>,
    start: usize,
    pass: u32,
    /// Set once a D.C./D.S. jump was taken.
    after: Option<Until>,
}

impl<'a> Unroller<'a> {
    fn new(bars: &'a [BarHead]) -> Unroller<'a> {
        Unroller {
            bars,
            order: Vec::new(),
            repeats: vec![0; bars.len()],
            taken: vec![false; bars.len()],
            finals: finals(bars),
            start: 0,
            pass: 1,
            after: None,
        }
    }

    fn run(mut self) -> Result<Vec<usize>, ImportError> {
        let mut i = 0;
        while i < self.bars.len() {
            let bar = &self.bars[i];

            // A repeat opened on a fresh arrival (not after jumping back) starts a new section.
            if bar.repeat_open && i != self.start && self.after.is_none() {
                self.start = i;
                self.pass = 1;
            }
            if !self.plays(i) {
                i += 1;
                continue;
            }

            self.order.push(i);
            if self.order.len() > MAX_BARS {
                return Err(ImportError::TooLong(MAX_BARS));
            }
            if let Some(back) = self.repeat(i) {
                i = back;
                continue;
            }
            match self.direction(i) {
                Next::Continue => i += 1,
                Next::Goto(target) => i = target,
                Next::Stop => break,
            }
        }
        Ok(self.order)
    }

    /// Whether bar `i` is played on the current pass.
    fn plays(&self, i: usize) -> bool {
        let endings = self.bars[i].endings;
        if self.after.is_some() {
            return endings == 0 || endings & self.finals[i] != 0;
        }
        ending_plays(endings, self.pass)
    }

    /// Handles a repeat close; returns the bar to jump back to.
    fn repeat(&mut self, i: usize) -> Option<usize> {
        let plays = self.bars[i].repeat_plays.min(MAX_PLAYS);
        if plays == 0 || self.after.is_some() {
            return None;
        }
        if self.repeats[i] + 1 < plays {
            self.repeats[i] += 1;
            self.pass += 1;
            return Some(self.start);
        }
        self.repeats[i] = 0;
        self.pass = 1;
        self.start = i + 1;
        None
    }

    /// Fine, "to coda", D.C. and D.S. at the end of bar `i`.
    fn direction(&mut self, i: usize) -> Next {
        let bar = &self.bars[i];
        if let Some(until) = self.after {
            if bar.targets.contains(&Target::Fine) && matches!(until, Until::Fine | Until::Any) {
                return Next::Stop;
            }
            let codas = [
                (Jump::ToCoda, Until::Coda, Target::Coda),
                (Jump::ToDoubleCoda, Until::DoubleCoda, Target::DoubleCoda),
            ];
            for (jump, wanted, target) in codas {
                if !bar.jumps.contains(&jump) || !(until == wanted || until == Until::Any) {
                    continue;
                }
                if let Some(to) = self.find(target) {
                    self.after = Some(Until::End);
                    return Next::Goto(to);
                }
            }
        }

        if self.taken[i] {
            return Next::Continue;
        }
        for jump in &bar.jumps {
            let (to, until) = match *jump {
                Jump::DaCapo(until) => (Some(0), until),
                Jump::DalSegno(until) => (self.find(Target::Segno), until),
                Jump::DalSegnoSegno(until) => (self.find(Target::SegnoSegno), until),
                Jump::ToCoda | Jump::ToDoubleCoda => continue,
            };
            let Some(to) = to else {
                continue;
            };
            self.taken[i] = true;
            self.after = Some(until);
            return Next::Goto(to);
        }
        Next::Continue
    }

    fn find(&self, target: Target) -> Option<usize> {
        self.bars.iter().position(|b| b.targets.contains(&target))
    }
}

/// True if a bar with the `endings` mask is played on pass `pass` (1-based).
fn ending_plays(endings: u32, pass: u32) -> bool {
    if endings == 0 {
        return true;
    }
    let Some(bit) = 1u32.checked_shl(pass.saturating_sub(1)) else {
        return false;
    };
    endings & bit != 0
}

/// For each bar with endings: the highest ending of its group of adjacent ending bars.
fn finals(bars: &[BarHead]) -> Vec<u32> {
    let mut finals = vec![0; bars.len()];
    let mut i = 0;
    while i < bars.len() {
        if bars[i].endings == 0 {
            i += 1;
            continue;
        }
        let end = (i..bars.len())
            .find(|j| bars[*j].endings == 0)
            .unwrap_or(bars.len());
        let all = bars[i..end].iter().fold(0, |mask, b| mask | b.endings);
        let last = 1u32 << (31 - all.leading_zeros());
        finals[i..end].fill(last);
        i = end;
    }
    finals
}

#[cfg(test)]
mod tests {
    use super::*;
    use kontra_model::TimeSignature;

    fn bars(n: usize) -> Vec<BarHead> {
        vec![BarHead::new(TimeSignature::COMMON); n]
    }

    #[test]
    fn plain_order() {
        assert_eq!(unroll(&bars(3)).unwrap(), vec![0, 1, 2]);
    }

    #[test]
    fn simple_repeat() {
        let mut b = bars(3);
        b[0].repeat_open = true;
        b[1].repeat_plays = 3;
        assert_eq!(unroll(&b).unwrap(), vec![0, 1, 0, 1, 0, 1, 2]);
    }

    #[test]
    fn repeat_without_open_starts_at_beginning() {
        let mut b = bars(4);
        b[1].repeat_plays = 2;
        b[3].repeat_plays = 2;
        assert_eq!(unroll(&b).unwrap(), vec![0, 1, 0, 1, 2, 3, 2, 3]);
    }

    #[test]
    fn alternate_endings() {
        // |: 0 | 1 (1.) :| 2 (2.) | 3
        let mut b = bars(4);
        b[0].repeat_open = true;
        b[1].endings = 0b01;
        b[1].repeat_plays = 2;
        b[2].endings = 0b10;
        assert_eq!(unroll(&b).unwrap(), vec![0, 1, 0, 2, 3]);
    }

    #[test]
    fn endings_before_close() {
        // GP7 style: |: 0 | 1 (1.,3.) | 2 (2.) | 3 x4 :| 4
        let mut b = bars(5);
        b[0].repeat_open = true;
        b[1].endings = 0b101;
        b[2].endings = 0b010;
        b[3].repeat_plays = 4;
        let order = unroll(&b).unwrap();
        assert_eq!(order, vec![0, 1, 3, 0, 2, 3, 0, 1, 3, 0, 3, 4]);
    }

    #[test]
    fn too_long_is_an_error() {
        let mut b = bars(MAX_BARS);
        b[MAX_BARS - 1].repeat_plays = 2;
        assert!(matches!(unroll(&b), Err(ImportError::TooLong(_))));
    }

    #[test]
    fn da_capo_al_fine_skips_repeats_and_first_ending() {
        // |: 0 | 1 (1.) :| 2 (2.) Fine | 3 D.C. al Fine
        let mut b = bars(4);
        b[0].repeat_open = true;
        b[1].endings = 0b01;
        b[1].repeat_plays = 2;
        b[2].endings = 0b10;
        b[2].targets.push(Target::Fine);
        b[3].jumps.push(Jump::DaCapo(Until::Fine));
        assert_eq!(unroll(&b).unwrap(), vec![0, 1, 0, 2, 3, 0, 2]);
    }

    #[test]
    fn dal_segno_al_coda() {
        // 0 | 1 Segno | 2 to Coda | 3 D.S. al Coda | 4 Coda | 5
        let mut b = bars(6);
        b[1].targets.push(Target::Segno);
        b[2].jumps.push(Jump::ToCoda);
        b[3].jumps.push(Jump::DalSegno(Until::Coda));
        b[4].targets.push(Target::Coda);
        assert_eq!(unroll(&b).unwrap(), vec![0, 1, 2, 3, 1, 2, 4, 5]);
    }

    #[test]
    fn plain_da_capo_plays_to_end() {
        // Fine is ignored without "al Fine"; the jump is taken once.
        let mut b = bars(3);
        b[0].targets.push(Target::Fine);
        b[1].jumps.push(Jump::DaCapo(Until::End));
        assert_eq!(unroll(&b).unwrap(), vec![0, 1, 0, 1, 2]);
    }

    #[test]
    fn segno_segno_and_double_coda() {
        let mut b = bars(5);
        b[1].targets.push(Target::SegnoSegno);
        b[1].jumps.push(Jump::ToDoubleCoda);
        b[2].jumps.push(Jump::DalSegnoSegno(Until::DoubleCoda));
        b[4].targets.push(Target::DoubleCoda);
        assert_eq!(unroll(&b).unwrap(), vec![0, 1, 2, 1, 4]);
    }

    #[test]
    fn jump_without_target_is_ignored() {
        let mut b = bars(2);
        b[0].jumps.push(Jump::DalSegno(Until::End));
        b[1].jumps.push(Jump::ToCoda);
        assert_eq!(unroll(&b).unwrap(), vec![0, 1]);
    }

    #[test]
    fn any_takes_fine_and_coda() {
        // MusicXML: 0 tocoda | 1 D.C. | 2 coda
        let mut b = bars(3);
        b[0].jumps.push(Jump::ToCoda);
        b[1].jumps.push(Jump::DaCapo(Until::Any));
        b[2].targets.push(Target::Coda);
        assert_eq!(unroll(&b).unwrap(), vec![0, 1, 0, 2]);
    }
}
