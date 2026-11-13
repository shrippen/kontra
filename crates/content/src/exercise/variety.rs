//! Variety of a rendered piece: a seeded random source, small changes in a repeated bar, the
//! approach note before a chord change and the run of a bass fill. A seed of 0 never reaches
//! here: the plain piece has no variation.

use super::chord::Chord;
use super::*;

const GOLDEN: u64 = 0x9E37_79B9_7F4A_7C15;
const MIX_1: u64 = 0xBF58_476D_1CE4_E5B9;
const MIX_2: u64 = 0x94D0_49BB_1331_11EB;

/// SplitMix64: small, reproducible, good enough to vary a bass line.
pub(super) struct Rng(u64);

impl Rng {
    /// A source for `seed`, set apart by `salts` (part, repeat, bar …): the same place of the
    /// same piece always draws the same numbers.
    pub(super) fn new(seed: u64, salts: &[u64]) -> Rng {
        let mut rng = Rng(seed);
        for salt in salts {
            rng.0 ^= salt.wrapping_add(1).wrapping_mul(GOLDEN);
            rng.draw();
        }
        rng
    }

    pub(super) fn draw(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(GOLDEN);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(MIX_1);
        z = (z ^ (z >> 27)).wrapping_mul(MIX_2);
        z ^ (z >> 31)
    }

    /// A number below `n` (`n` > 0).
    pub(super) fn below(&mut self, n: usize) -> usize {
        (self.draw() % n.max(1) as u64) as usize
    }

    /// True once in `n` draws.
    pub(super) fn one_in(&mut self, n: usize) -> bool {
        self.below(n) == 0
    }
}

/// How far an approach note lies from the root it leads to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Reach {
    Half,
    Whole,
}

/// Which side a run or an approach comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Side {
    Below,
    Above,
}

/// The side `next` is approached from: below when it lies higher than `root`.
pub(super) fn side(root: i32, next: i32) -> Side {
    if next >= root {
        Side::Below
    } else {
        Side::Above
    }
}

/// A note leading from `chord` on `root` into `next`: one step from it, not a tone of the
/// chord where one is free. `None` when the root stays.
pub(super) fn lead_in(chord: &Chord, root: i32, next: i32, reach: Reach) -> Option<i32> {
    if (next - root).rem_euclid(OCTAVE) == 0 {
        return None;
    }
    let sign = if side(root, next) == Side::Below {
        -1
    } else {
        1
    };
    let steps = match reach {
        Reach::Half => [1, 2],
        Reach::Whole => [2, 1],
    };
    let candidates: Vec<i32> = steps
        .iter()
        .flat_map(|d| [next + sign * d, next - sign * d])
        .filter(|k| (LOWEST..=HIGHEST).contains(k))
        .collect();
    let tones = chord.pitch_classes();
    candidates
        .iter()
        .copied()
        .find(|k| !tones.contains(&(k - root).rem_euclid(OCTAVE)))
        .or_else(|| candidates.first().copied())
}

/// A change of one beat in a repeated bar.
#[derive(Debug, Clone, Copy)]
enum Twist {
    /// The last note leads to the next root.
    Approach,
    /// A note on a beat jumps an octave.
    Octave,
    /// A note is muted.
    Ghost,
    /// An eighth note on the "and" of four leads to the next root.
    Pickup,
}

const EASY_TWISTS: [Twist; 1] = [Twist::Approach];
const MEDIUM_TWISTS: [Twist; 2] = [Twist::Approach, Twist::Octave];
const HARD_TWISTS: [Twist; 4] = [Twist::Approach, Twist::Octave, Twist::Ghost, Twist::Pickup];

/// The bar `tones` fill: where it starts, its chord and root, the root of the next bar.
pub(super) struct BarAt<'a> {
    pub(super) start: u32,
    pub(super) chord: &'a Chord,
    pub(super) root: i32,
    pub(super) next: i32,
}

/// Changes one beat of the bar, never its first: the root on beat one stays.
pub(super) fn vary(tones: &mut Vec<Tone>, bar: &BarAt, level: Level, rng: &mut Rng) {
    let twists: &[Twist] = match level {
        Level::Easy => &EASY_TWISTS,
        Level::Medium => &MEDIUM_TWISTS,
        Level::Hard => &HARD_TWISTS,
    };
    let twist = twists[rng.below(twists.len())];
    let after_one: Vec<usize> = (0..tones.len())
        .filter(|i| tones[*i].start > bar.start && tones[*i].stroke == Stroke::Plain)
        .collect();
    let Some(last) = after_one.last().copied() else {
        return;
    };
    let reach = if rng.one_in(3) {
        Reach::Whole
    } else {
        Reach::Half
    };
    match twist {
        Twist::Approach => {
            if let Some(key) = lead_in(bar.chord, bar.root, bar.next, reach) {
                tones[last].key = key;
            }
        }
        Twist::Octave => {
            let on_beat: Vec<usize> = after_one
                .iter()
                .copied()
                .filter(|i| (tones[*i].start - bar.start).is_multiple_of(QUARTER))
                .collect();
            if on_beat.is_empty() {
                return;
            }
            let i = on_beat[rng.below(on_beat.len())];
            if tones[i].key + OCTAVE <= HIGHEST {
                tones[i].key += OCTAVE;
            }
        }
        Twist::Ghost => {
            let i = after_one[rng.below(after_one.len())];
            tones[i].stroke = Stroke::Ghost;
        }
        Twist::Pickup => {
            let Some(key) = lead_in(bar.chord, bar.root, bar.next, reach) else {
                return;
            };
            let at = bar.start + 3 * QUARTER + EIGHTH;
            clear_from(tones, at);
            tones.push(Tone {
                start: at,
                length: EIGHTH,
                key,
                stroke: Stroke::Plain,
                position: None,
            });
        }
    }
}

/// Removes what sounds from `at` on; a note that rings into it ends there.
fn clear_from(tones: &mut Vec<Tone>, at: u32) {
    tones.retain(|t| t.start < at);
    for tone in tones.iter_mut() {
        tone.length = tone.length.min(at - tone.start);
    }
}

/// What a fill plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Run {
    Chromatic,
    /// The notes of the key's scale.
    Scale,
}

/// Notes in a fill: four sixteenths on the last beat.
const FILL_NOTES: usize = 4;

/// Replaces the beat from `from` on by a run of four sixteenths into `target`, coming from
/// `side` (the other one where the range has no room). `scale` are the pitch classes of the key.
pub(super) fn fill(
    tones: &mut Vec<Tone>,
    from: u32,
    target: i32,
    run: Run,
    side: Side,
    scale: &[i32],
) {
    clear_from(tones, from);
    let in_run = |k: i32| run == Run::Chromatic || scale.contains(&k.rem_euclid(OCTAVE));
    let collect = |dir: i32| -> Vec<i32> {
        let mut found = Vec::new();
        let mut k = target + dir;
        while (LOWEST..=HIGHEST).contains(&k) && found.len() < FILL_NOTES {
            if in_run(k) {
                found.push(k);
            }
            k += dir;
        }
        found
    };
    let (first, second) = match side {
        Side::Below => (-1, 1),
        Side::Above => (1, -1),
    };
    let mut notes = collect(first);
    if notes.len() < FILL_NOTES {
        notes = collect(second);
    }
    // Collected nearest to the target first: play them towards it.
    notes.reverse();
    for (i, key) in notes.into_iter().enumerate() {
        tones.push(Tone {
            start: from + i as u32 * SIXTEENTH,
            length: SIXTEENTH,
            key,
            stroke: Stroke::Plain,
            position: None,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_place_same_numbers() {
        let mut x = Rng::new(7, &[1, 2, 3]);
        let mut y = Rng::new(7, &[1, 2, 3]);
        let mut z = Rng::new(7, &[1, 2, 4]);
        assert_eq!(x.draw(), y.draw());
        assert_ne!(x.draw(), z.draw());
    }

    #[test]
    fn lead_in_avoids_the_chords_own_tones() {
        let chord = Chord::parse("C").expect("C");
        // C (36) to F (41): from below, E is the chord's third, so F# from above.
        assert_eq!(lead_in(&chord, 36, 41, Reach::Half), Some(42));
        // Root stays: nothing to lead into.
        assert_eq!(lead_in(&chord, 36, 48, Reach::Half), None);
        // From above when the next root is lower.
        let g = Chord::parse("G").expect("G");
        assert_eq!(lead_in(&g, 43, 36, Reach::Half), Some(37));
    }

    #[test]
    fn a_fill_runs_into_the_target() {
        let mut tones = vec![Tone {
            start: 0,
            length: 2 * QUARTER,
            key: 40,
            stroke: Stroke::Plain,
            position: None,
        }];
        fill(&mut tones, QUARTER, 40, Run::Chromatic, Side::Below, &[]);
        let keys: Vec<i32> = tones.iter().map(|t| t.key).collect();
        assert_eq!(keys, [40, 36, 37, 38, 39]);
        assert_eq!(
            tones[0].length, QUARTER,
            "the held note ends where the run starts"
        );

        // C major below F (41): E, D, C, B, played upwards.
        let mut scale = Vec::new();
        let major: Vec<i32> = MAJOR.iter().map(|s| s % OCTAVE).collect();
        fill(&mut scale, 0, 41, Run::Scale, Side::Below, &major);
        let keys: Vec<i32> = scale.iter().map(|t| t.key).collect();
        assert_eq!(keys, [35, 36, 38, 40]);
    }
}
