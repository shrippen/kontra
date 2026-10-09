//! Tab generator: picks string and fret for notes that have none, so the fretting hand moves
//! as little as possible.
//!
//! Notes starting together form one step (a chord); its options are shapes with one string
//! per note. Dynamic programming runs over the shapes of every step. The state is the shape
//! plus where the hand is (lowest fretted fret), so open strings do not move the hand:
//!
//! ```text
//! step:      A1          C2          E1+B1 (chord)
//! shapes:   (E,5)(A,0)  (E,8)(A,3)  [(E,0)(A,2)]  not [(E,0)(E,7)]: same string
//! hand:      5    -      8    3      2
//! ```

use kontra_model::{Arrangement, Note, TICKS_PER_QUARTER};

/// Frets the hand covers without shifting (index on f, pinky on f+3).
const SPAN: u8 = 3;
/// Extra cost for a position shift beyond the span, and per fret a chord stretches beyond it.
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
/// Hand positions tracked (fret 0 = hand not placed yet).
const HANDS: usize = 37;
/// Shapes considered per chord; more is only possible for huge chords.
const MAX_SHAPES: usize = 512;

/// Cheapest shapes kept per chord for the search.
const KEPT_SHAPES: usize = 32;

/// (string, fret)
type Pos = (u8, u8);

/// Notes starting together and the shapes they may take; `shapes[s][k]` is for `notes[k]`.
#[derive(Debug)]
struct Step {
    notes: Vec<usize>,
    shapes: Vec<Vec<Pos>>,
}

/// A note of a chord with its candidate positions.
#[derive(Debug)]
struct Member {
    note: usize,
    options: Vec<Pos>,
    placement: Placement,
}

/// Whether the file already placed a note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Placement {
    Fixed,
    Free,
}

/// Assigns string and fret to notes without them; existing assignments are kept and
/// steer the choice for their neighbours. Notes of a chord get different strings; notes
/// outside the instrument's range (or that do not fit a chord) stay empty.
pub fn assign_tab(arrangement: &mut Arrangement) {
    let steps = steps(arrangement);
    if steps.is_empty() {
        return;
    }
    let choice = if steps.iter().all(|s| s.shapes.len() <= 1) {
        vec![0; steps.len()]
    } else {
        solve(&arrangement.notes, &steps)
    };
    for (step, shape) in steps.iter().zip(choice) {
        for (&note, &(string, fret)) in step.notes.iter().zip(&step.shapes[shape]) {
            let note = &mut arrangement.notes[note];
            note.string = Some(string);
            note.fret = Some(fret);
        }
    }
}

/// Groups notes by start tick (the arrangement is sorted) and lists their shapes.
fn steps(arrangement: &Arrangement) -> Vec<Step> {
    let notes = &arrangement.notes;
    let mut steps = Vec::new();
    let mut first = 0;
    while first < notes.len() {
        let start = notes[first].start_ticks;
        let end = (first..notes.len())
            .find(|i| notes[*i].start_ticks != start)
            .unwrap_or(notes.len());
        if let Some(step) = chord(arrangement, first..end) {
            steps.push(step);
        }
        first = end;
    }
    steps
}

/// Shapes for the notes in `range`; drops the highest free notes until a shape exists.
fn chord(arrangement: &Arrangement, range: std::ops::Range<usize>) -> Option<Step> {
    let mut members: Vec<Member> = range
        .filter_map(|note| {
            let (options, placement) = options(arrangement, &arrangement.notes[note]);
            let member = Member {
                note,
                options,
                placement,
            };
            (!member.options.is_empty()).then_some(member)
        })
        .collect();
    // Fixed notes first so free notes avoid their strings; then low to high.
    members.sort_by_key(|m| (m.placement, arrangement.notes[m.note].pitch));

    loop {
        if members.is_empty() {
            return None;
        }
        let mut shapes = Vec::new();
        shapes_of(&members, &mut Vec::new(), &mut shapes);
        if !shapes.is_empty() {
            // Big chords (guitar parts) have many shapes; the compact ones are enough.
            shapes.sort_by(|a, b| shape_cost(a).total_cmp(&shape_cost(b)));
            shapes.truncate(KEPT_SHAPES);
            let notes = members.iter().map(|m| m.note).collect();
            return Some(Step { notes, shapes });
        }
        // No string left for every note: give up the highest free note.
        let free = members
            .iter()
            .rposition(|m| m.placement == Placement::Free)?;
        members.remove(free);
    }
}

/// Candidate positions of one note.
fn options(arrangement: &Arrangement, note: &Note) -> (Vec<Pos>, Placement) {
    if let (Some(string), Some(fret)) = (note.string, note.fret) {
        return (vec![(string, fret)], Placement::Fixed);
    }
    let mut options = arrangement.instrument.positions(note.pitch);
    // A given string without fret narrows the choice.
    if let Some(string) = note.string {
        let on_string: Vec<_> = options.iter().copied().filter(|o| o.0 == string).collect();
        if !on_string.is_empty() {
            options = on_string;
        }
    }
    (options, Placement::Free)
}

/// Depth-first combinations with a string of its own for every free note.
fn shapes_of(members: &[Member], current: &mut Vec<Pos>, out: &mut Vec<Vec<Pos>>) {
    if out.len() >= MAX_SHAPES {
        return;
    }
    let Some(member) = members.get(current.len()) else {
        out.push(current.clone());
        return;
    };
    for &pos in &member.options {
        let taken = current.iter().any(|p| p.0 == pos.0);
        if taken && member.placement == Placement::Free {
            continue;
        }
        current.push(pos);
        shapes_of(members, current, out);
        current.pop();
    }
}

/// Lowest fretted fret (0 = all open), fret span and fret sum of a shape.
fn measure(shape: &[Pos]) -> (usize, u8, u32) {
    let fretted = shape.iter().map(|p| p.1).filter(|f| *f > 0);
    let low = fretted.clone().min().unwrap_or(0);
    let high = fretted.max().unwrap_or(0);
    let sum = shape.iter().map(|p| u32::from(p.1)).sum();
    (usize::from(low).min(HANDS - 1), high - low, sum)
}

/// Cost of the shape itself: high frets and stretches beyond the span.
fn shape_cost(shape: &[Pos]) -> f64 {
    let (_, span, sum) = measure(shape);
    let stretch = f64::from(span.saturating_sub(SPAN)) * SHIFT_COST;
    FRET_COST * f64::from(sum) + stretch
}

fn lowest_string(shape: &[Pos]) -> u8 {
    shape.iter().map(|p| p.0).min().unwrap_or(0)
}

/// Viterbi over (shape, hand) states; returns the chosen shape per step.
fn solve(notes: &[Note], steps: &[Step]) -> Vec<usize> {
    // cost[s * HANDS + h]; back pointers per step.
    let mut cost = vec![f64::INFINITY; steps[0].shapes.len() * HANDS];
    for (s, shape) in steps[0].shapes.iter().enumerate() {
        let (hand, _, _) = measure(shape);
        cost[s * HANDS + hand] = shape_cost(shape);
    }
    let mut back: Vec<Vec<(usize, usize)>> = vec![vec![(0, 0); cost.len()]];

    for k in 1..steps.len() {
        let (prev, step) = (&steps[k - 1], &steps[k]);
        let prev_end = prev.notes.iter().map(|n| notes[*n].end_ticks()).max();
        let gap = notes[step.notes[0]]
            .start_ticks
            .saturating_sub(prev_end.unwrap_or(0));
        let rest = Rest::from_gap(gap);
        let mut next = vec![f64::INFINITY; step.shapes.len() * HANDS];
        let mut links = vec![(0, 0); next.len()];

        for (p, from) in prev.shapes.iter().enumerate() {
            for hand in 0..HANDS {
                let base = cost[p * HANDS + hand];
                if !base.is_finite() {
                    continue;
                }
                for (s, to) in step.shapes.iter().enumerate() {
                    let (moved, new_hand) = transition(from, to, hand, rest);
                    let total = base + moved;
                    let slot = s * HANDS + new_hand;
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
    let (mut shape, mut hand) = (best / HANDS, best % HANDS);
    for k in (0..len).rev() {
        choice[k] = shape;
        let (p, h) = back[k][shape * HANDS + hand];
        shape = p;
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

/// Cost to go from shape `from` to `to` with the hand at `hand`; returns the new hand.
fn transition(from: &[Pos], to: &[Pos], hand: usize, rest: Rest) -> (f64, usize) {
    let strings = f64::from(lowest_string(from).abs_diff(lowest_string(to))) * STRING_COST;
    let (target, _, _) = measure(to);

    // Open strings only: the hand stays where it is.
    if target == 0 {
        let open = if hand <= usize::from(NEAR_NUT) {
            OPEN_NEAR_COST
        } else {
            OPEN_FAR_COST
        };
        return (strings + open, hand);
    }

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
    (strings + moved + shape_cost(to), target)
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

    /// Adds notes sounding together with the notes at `start`.
    fn with_chord(mut a: Arrangement, start: u64, pitches: &[u8]) -> Arrangement {
        for p in pitches {
            a.notes.push(Note::new(start, TICKS_PER_QUARTER, Pitch(*p)));
        }
        a.notes.sort_by_key(|n| (n.start_ticks, n.pitch));
        a
    }

    fn frets(a: &Arrangement) -> Vec<(u8, u8)> {
        a.notes
            .iter()
            .map(|n| (n.string.unwrap(), n.fret.unwrap()))
            .collect()
    }

    fn strings_at(a: &Arrangement, start: u64) -> Vec<u8> {
        let mut strings: Vec<u8> = a
            .notes
            .iter()
            .filter(|n| n.start_ticks == start)
            .filter_map(|n| n.string)
            .collect();
        strings.sort_unstable();
        strings
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
        let mut a = with_chord(line(&[33]), 0, &[40]);
        assign_tab(&mut a);
        let strings = strings_at(&a, 0);
        assert_eq!(strings.len(), 2);
        assert_ne!(strings[0], strings[1]);
    }

    #[test]
    fn three_note_chord_gets_three_strings_and_a_compact_shape() {
        // A1 E2 A2 (power chord with octave).
        let mut a = with_chord(line(&[33]), 0, &[40, 45]);
        assign_tab(&mut a);
        let f = frets(&a);
        let mut strings: Vec<u8> = f.iter().map(|p| p.0).collect();
        strings.sort_unstable();
        strings.dedup();
        assert_eq!(strings.len(), 3, "{f:?}");
        let fretted: Vec<u8> = f.iter().map(|p| p.1).filter(|x| *x > 0).collect();
        let span = fretted.iter().max().unwrap_or(&0) - fretted.iter().min().unwrap_or(&0);
        assert!(span <= SPAN, "{f:?}");
    }

    #[test]
    fn chord_avoids_string_of_fixed_note() {
        // D2 fixed on the A string (fret 5); A1 must not take the A string too.
        let mut a = with_chord(line(&[38]), 0, &[33]);
        let d2 = a.notes.iter().position(|n| n.pitch == Pitch(38)).unwrap();
        a.notes[d2].string = Some(1);
        a.notes[d2].fret = Some(5);
        assign_tab(&mut a);
        let a1 = a.notes.iter().find(|n| n.pitch == Pitch(33)).unwrap();
        assert_eq!((a1.string, a1.fret), (Some(0), Some(5)));
    }

    #[test]
    fn unison_on_two_strings() {
        // Two A1 at once: open A and E string fret 5.
        let mut a = with_chord(line(&[33]), 0, &[33]);
        assign_tab(&mut a);
        assert_eq!(strings_at(&a, 0), [0, 1]);
    }

    #[test]
    fn too_many_notes_leave_the_highest_empty() {
        // Five notes on a four-string bass: the highest stays without tab.
        let mut a = with_chord(line(&[28]), 0, &[33, 38, 43, 48]);
        assign_tab(&mut a);
        assert_eq!(strings_at(&a, 0), [0, 1, 2, 3]);
        let top = a.notes.iter().find(|n| n.pitch == Pitch(48)).unwrap();
        assert_eq!(top.string, None);
    }
}
