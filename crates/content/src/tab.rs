//! Tab generator: picks string and fret for notes that have none, so the fretting hand moves
//! as little as possible.
//!
//! Dynamic programming over all candidate positions of every note. The state is the
//! position plus where the hand is (fret of the last fretted note), so an open string
//! does not move the hand:
//!
//! ```text
//! note:      A1          C2          E1
//! options:  (E,5)(A,0)  (E,8)(A,3)  (E,0)
//! hand:      5    -      8    3      keeps last
//! ```

use kontra_model::{Arrangement, Note, TICKS_PER_QUARTER};

/// Frets the hand covers without shifting (index on f, pinky on f+3).
const SPAN: u8 = 3;
/// Extra cost for a position shift beyond the span.
const SHIFT_COST: f64 = 2.0;
/// Cost per string crossed.
const STRING_COST: f64 = 0.2;
/// Preference for low frets, per fret.
const FRET_COST: f64 = 0.05;
/// Open string while the hand is near the nut: cheap.
const OPEN_NEAR_COST: f64 = 0.1;
/// Open string while the hand is up the neck: allowed, but staying in position is better.
const OPEN_FAR_COST: f64 = 1.0;
/// Hand positions up to this fret count as "near the nut".
const NEAR_NUT: u8 = 5;
/// Moves after a rest of at least a bar cost half: the hand has time.
const REST_TICKS: u64 = TICKS_PER_QUARTER * 4;
/// Two notes of a chord on one string are impossible.
const BLOCKED: f64 = 1.0e6;
/// Hand positions tracked (fret 0 = hand not placed yet).
const HANDS: usize = 37;

/// One note to place: its index and the positions it may take.
#[derive(Debug)]
struct Step {
    note: usize,
    options: Vec<(u8, u8)>,
}

/// Assigns string and fret to notes without them; existing assignments are kept and
/// steer the choice for their neighbours. Notes outside the instrument's range stay empty.
pub fn assign_tab(arrangement: &mut Arrangement) {
    let steps = steps(arrangement);
    if steps.iter().all(|s| s.options.len() <= 1) {
        apply_single(arrangement, &steps);
        return;
    }
    let choice = solve(&arrangement.notes, &steps);
    for (step, option) in steps.iter().zip(choice) {
        let note = &mut arrangement.notes[step.note];
        let (string, fret) = step.options[option];
        note.string = Some(string);
        note.fret = Some(fret);
    }
}

/// Candidate positions per note, in timeline order.
fn steps(arrangement: &Arrangement) -> Vec<Step> {
    let instrument = &arrangement.instrument;
    let mut steps = Vec::new();
    for (index, note) in arrangement.notes.iter().enumerate() {
        if let (Some(string), Some(fret)) = (note.string, note.fret) {
            steps.push(Step {
                note: index,
                options: vec![(string, fret)],
            });
            continue;
        }
        let mut options = instrument.positions(note.pitch);
        // A given string without fret narrows the choice.
        if let Some(string) = note.string {
            let on_string: Vec<_> = options.iter().copied().filter(|o| o.0 == string).collect();
            if !on_string.is_empty() {
                options = on_string;
            }
        }
        if options.is_empty() {
            continue;
        }
        steps.push(Step {
            note: index,
            options,
        });
    }
    steps
}

fn apply_single(arrangement: &mut Arrangement, steps: &[Step]) {
    for step in steps {
        let Some(&(string, fret)) = step.options.first() else {
            continue;
        };
        let note = &mut arrangement.notes[step.note];
        note.string = Some(string);
        note.fret = Some(fret);
    }
}

/// Viterbi over (option, hand) states; returns the chosen option per step.
fn solve(notes: &[Note], steps: &[Step]) -> Vec<usize> {
    // cost[o * HANDS + h]; back pointers per step.
    let mut cost = vec![f64::INFINITY; steps[0].options.len() * HANDS];
    for (o, &(_, fret)) in steps[0].options.iter().enumerate() {
        let hand = usize::from(fret).min(HANDS - 1);
        cost[o * HANDS + hand] = FRET_COST * f64::from(fret);
    }
    let mut back: Vec<Vec<(usize, usize)>> = vec![vec![(0, 0); cost.len()]];

    for k in 1..steps.len() {
        let (prev, step) = (&steps[k - 1], &steps[k]);
        let chord = notes[prev.note].start_ticks == notes[step.note].start_ticks;
        let gap = notes[step.note]
            .start_ticks
            .saturating_sub(notes[prev.note].end_ticks());
        let mut next = vec![f64::INFINITY; step.options.len() * HANDS];
        let mut links = vec![(0, 0); next.len()];

        for (p, &from) in prev.options.iter().enumerate() {
            for hand in 0..HANDS {
                let base = cost[p * HANDS + hand];
                if !base.is_finite() {
                    continue;
                }
                for (o, &to) in step.options.iter().enumerate() {
                    let (moved, new_hand) = transition(from, to, hand, Rest::from_gap(gap));
                    let blocked = if chord && from.0 == to.0 {
                        BLOCKED
                    } else {
                        0.0
                    };
                    let total = base + moved + blocked;
                    let slot = o * HANDS + new_hand;
                    if total < next[slot] {
                        next[slot] = total;
                        links[slot] = (p, hand);
                    }
                }
            }
        }
        cost = next;
        back.push(links);
    }

    backtrack(&cost, &back, steps.len())
}

/// Walks the back pointers from the cheapest final state.
fn backtrack(cost: &[f64], back: &[Vec<(usize, usize)>], len: usize) -> Vec<usize> {
    let mut best = 0;
    for (slot, value) in cost.iter().enumerate() {
        if *value < cost[best] {
            best = slot;
        }
    }
    let mut choice = vec![0; len];
    let (mut option, mut hand) = (best / HANDS, best % HANDS);
    for k in (0..len).rev() {
        choice[k] = option;
        let (p, h) = back[k][option * HANDS + hand];
        option = p;
        hand = h;
    }
    choice
}

/// Whether the hand had time to move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rest {
    Short,
    Long,
}

impl Rest {
    fn from_gap(gap: u64) -> Rest {
        if gap >= REST_TICKS {
            Rest::Long
        } else {
            Rest::Short
        }
    }
}

/// Cost to go from position `from` to `to` with the hand at `hand`; returns the new hand.
fn transition(from: (u8, u8), to: (u8, u8), hand: usize, rest: Rest) -> (f64, usize) {
    let strings = f64::from(from.0.abs_diff(to.0)) * STRING_COST;
    let (_, fret) = to;

    // Open string: the hand stays where it is.
    if fret == 0 {
        let open = if hand <= usize::from(NEAR_NUT) {
            OPEN_NEAR_COST
        } else {
            OPEN_FAR_COST
        };
        return (strings + open, hand);
    }

    let target = usize::from(fret).min(HANDS - 1);
    // Hand not placed yet: it starts near the nut.
    let anchor = if hand == 0 { 1 } else { hand };
    let distance = anchor.abs_diff(target) as f64;
    let mut moved = distance;
    if distance > f64::from(SPAN) {
        moved += SHIFT_COST;
    }
    if rest == Rest::Long {
        moved *= 0.5;
    }
    (strings + moved + FRET_COST * f64::from(fret), target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kontra_model::{Instrument, Pitch};

    fn line(pitches: &[u8]) -> Arrangement {
        let mut a = Arrangement::new("t", Instrument::default_bass());
        for (i, p) in pitches.iter().enumerate() {
            let start = i as u64 * TICKS_PER_QUARTER;
            a.notes.push(Note::new(start, TICKS_PER_QUARTER, Pitch(*p)));
        }
        a
    }

    fn frets(a: &Arrangement) -> Vec<(u8, u8)> {
        a.notes
            .iter()
            .map(|n| (n.string.unwrap(), n.fret.unwrap()))
            .collect()
    }

    #[test]
    fn low_notes_stay_near_nut() {
        // E1 G1 A1 C2: open E, E3, open A, A3.
        let mut a = line(&[28, 31, 33, 36]);
        assign_tab(&mut a);
        assert_eq!(frets(&a), vec![(0, 0), (0, 3), (1, 0), (1, 3)]);
    }

    #[test]
    fn high_line_stays_in_position() {
        // C3 D3 E3 C3 played around fret 5-9 on D and G strings, no big jumps.
        let mut a = line(&[48, 50, 52, 48, 47, 45]);
        assign_tab(&mut a);
        let f = frets(&a);
        for pair in f.windows(2) {
            assert!(pair[0].1.abs_diff(pair[1].1) <= 5, "{f:?}");
        }
    }

    #[test]
    fn keeps_existing_assignment() {
        let mut a = line(&[33, 35]);
        a.notes[0].string = Some(0);
        a.notes[0].fret = Some(5);
        assign_tab(&mut a);
        // A1 stays on E string fret 5; B1 follows in position (E 7), not A 2 after a jump.
        assert_eq!(frets(&a)[0], (0, 5));
        assert_eq!(frets(&a)[1], (0, 7));
    }

    #[test]
    fn out_of_range_stays_empty() {
        let mut a = line(&[10, 28]);
        assign_tab(&mut a);
        assert_eq!(a.notes[0].string, None);
        assert_eq!(a.notes[1].fret, Some(0));
    }

    #[test]
    fn chord_uses_two_strings() {
        let mut a = line(&[33]);
        a.notes.push(Note::new(0, TICKS_PER_QUARTER, Pitch(40)));
        a.notes.sort_by_key(|n| (n.start_ticks, n.pitch));
        assign_tab(&mut a);
        let f = frets(&a);
        assert_ne!(f[0].0, f[1].0);
    }
}
