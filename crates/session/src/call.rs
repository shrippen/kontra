//! Call and response (LH-LRN-19, DIDAKTIK I8): Kontra plays two bars of bass over the drums
//! and chords, the player answers with the same motif in the next two. The motif is made of the
//! chords' tones within a fifth, in the rhythm of the piece's own line, so it grooves with the
//! drums. Kontra's call is played, never judged: through a microphone the player's input hears
//! it too.
//!
//! ```text
//! bars    | 1     2     | 3     4     | 5     6     | 7     8     |
//!         | call        | answer      | call        | answer      |
//!         | Kontra      | player      | Kontra      | player      |
//! drums   | softer      | full        | softer      | full        |
//! ```

use kontra_model::{Arrangement, Chord, MarkerKind, Note, Pitch};

/// Bars of a call; the answer takes as many.
const CALL_BARS: usize = 2;
/// The widest motif: a fifth.
const RANGE_SEMITONES: u8 = 7;
const PITCH_CLASSES: u8 = 12;
/// Drums while Kontra calls: softer, so the call stands out.
const CALL_DRUM_VELOCITY: f32 = 0.5;
const FULL_VELOCITY: f32 = 1.0;
/// Without a tuning: the lowest string of a four-string bass.
const E1: u8 = 28;

/// `arrangement` as call and response: every four bars a call of two (Kontra plays) and its
/// answer (the player plays the same); bars left over at the end go. `None` without chord
/// symbols (nothing to build a motif from) or shorter than four bars.
pub fn call_and_response(arrangement: &Arrangement) -> Option<Arrangement> {
    let unit = 2 * CALL_BARS;
    let units = arrangement.bars.len() / unit;
    let chords = arrangement
        .markers
        .iter()
        .any(|m| m.kind == MarkerKind::Chord && Chord::parse(&m.name).is_some());
    if units == 0 || !chords {
        return None;
    }
    let mut out = arrangement.clone();
    out.notes.clear();
    out.calls.clear();
    out.bars.truncate(units * unit);
    let end = out.bars.last().map_or(0, kontra_model::Bar::end_ticks);
    out.drums.retain(|d| d.tick < end);
    out.markers.retain(|m| m.tick < end);
    for u in 0..units {
        let first = u * unit;
        let call_from = arrangement.bars[first].start_ticks;
        let answer_from = arrangement.bars[first + CALL_BARS].start_ticks;
        let motif = motif(arrangement, first, u);
        let shift = answer_from - call_from;
        out.notes.extend(motif.iter().cloned());
        out.notes.extend(motif.into_iter().map(|mut n| {
            n.start_ticks += shift;
            n
        }));
        out.calls.push((call_from, answer_from));
    }
    out.normalize();
    kontra_content::assign_tab(&mut out);
    Some(out)
}

/// Velocity of a drum stroke at `tick`: softer during a call.
pub(crate) fn drum_velocity(arrangement: &Arrangement, tick: u64) -> f32 {
    if arrangement.in_call(tick) {
        CALL_DRUM_VELOCITY
    } else {
        FULL_VELOCITY
    }
}

/// The call of unit `u` starting at bar `first`: the line's rhythm in those bars (quarters where
/// it has none), each attack a chord tone within a fifth above the first chord's root; bar starts
/// on the root where it is in reach, the others step through the chord tones.
fn motif(arrangement: &Arrangement, first: usize, u: usize) -> Vec<Note> {
    let bars = &arrangement.bars[first..first + CALL_BARS];
    let base = base_pitch(arrangement, bars[0].start_ticks);
    let mut notes = Vec::new();
    for bar in bars {
        let chord = chord_at(arrangement, bar.start_ticks);
        let reach: Vec<u8> = (base..=base + RANGE_SEMITONES)
            .filter(|m| chord.is_none_or(|c| c.has(*m)))
            .collect();
        let rhythm = rhythm_of(arrangement, bar);
        for (k, (start, length)) in rhythm.into_iter().enumerate() {
            let root = chord.and_then(|c| {
                reach
                    .iter()
                    .copied()
                    .find(|m| m % PITCH_CLASSES == c.root())
            });
            let pitch = match (k, root) {
                (0, Some(root)) => root,
                _ => reach[(k + u) % reach.len().max(1)],
            };
            notes.push(Note::new(start, length, Pitch(pitch)));
        }
    }
    notes
}

/// Attacks (start, length) of the line in `bar`, ties and legato left out; none: quarters.
fn rhythm_of(arrangement: &Arrangement, bar: &kontra_model::Bar) -> Vec<(u64, u64)> {
    let written: Vec<(u64, u64)> = arrangement
        .notes_between(bar.start_ticks, bar.end_ticks())
        .iter()
        .filter(|n| n.needs_attack())
        .map(|n| {
            (
                n.start_ticks,
                n.duration_ticks.min(bar.end_ticks() - n.start_ticks),
            )
        })
        .collect();
    let mut attacks: Vec<(u64, u64)> = Vec::new();
    for (start, length) in written {
        // A chord of the line is one attack.
        if attacks.last().is_none_or(|(s, _)| *s != start) {
            attacks.push((start, length));
        }
    }
    if !attacks.is_empty() {
        return attacks;
    }
    let beat = kontra_model::TICKS_PER_QUARTER * 4 / u64::from(bar.signature.denominator.max(1));
    (0..u64::from(bar.signature.numerator))
        .map(|b| (bar.start_ticks + b * beat, beat))
        .collect()
}

/// The lowest tone of the motif: the chord's root at `tick` in the lowest octave the instrument
/// plays it, so the fifth above stays low and playable.
fn base_pitch(arrangement: &Arrangement, tick: u64) -> u8 {
    let lowest = arrangement.instrument.tuning.lowest().map_or(E1, |p| p.0);
    let root = chord_at(arrangement, tick).map_or(lowest % PITCH_CLASSES, Chord::root);
    let up = (root + PITCH_CLASSES - lowest % PITCH_CLASSES) % PITCH_CLASSES;
    lowest + up
}

/// The chord named last at or before `tick`.
fn chord_at(arrangement: &Arrangement, tick: u64) -> Option<Chord> {
    arrangement
        .markers
        .iter()
        .filter(|m| m.kind == MarkerKind::Chord && m.tick <= tick)
        .filter_map(|m| Chord::parse(&m.name))
        .next_back()
}
