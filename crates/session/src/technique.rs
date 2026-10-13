//! Technique scoring (LH-BEW-06, round 9: its own evaluation, the hit rate stays as it is).
//!
//! The score says how a note is played (Guitar Pro, MusicXML); the tracker says how it was
//! played. Each note with a scored technique gets one result per pass:
//!
//! ```text
//! score:   E1 ─sl→ G1      A1 (h)       D2 (S)
//!                   │          │            │
//! target:   G1 Slide   A1 HammerOn   D2 Slap     (a slide is judged where it arrives)
//!                   │          │            │
//! played:   Slide ✓    plucked ✗    not heard ✗  ──► "Slides 1 of 1, hammer-ons 0 of 1, …"
//!                   │
//! sign:     E1 sl ✓                               (round 10: shown where it starts, as printed)
//! ```
//!
//! Dead notes (x, `Muted`) are judged as muted; ghost notes in parentheses are soft notes with
//! a pitch and only get their sign (round 10).
//!
//! Notes that are struck (slap, pop, dead note, the arrival of a slide) are judged with the
//! matcher's result for them. Hammer-on and pull-off targets have no attack: the matcher never
//! sees them, so a detected note near one is judged here and is not an extra note.

use kontra_model::{Arrangement, InputSource, Technique, TICKS_PER_QUARTER};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

/// Techniques Kontra can tell apart, in the order the evaluation lists them.
const SCORED: [Technique; 6] = [
    Technique::Slide,
    Technique::HammerOn,
    Technique::PullOff,
    Technique::Slap,
    Technique::Pop,
    Technique::Muted,
];

/// Techniques that are shown but not judged: a ghost note ( ) is soft, not muted (round 10).
const SHOWN: [Technique; 1] = [Technique::Ghost];

/// A slide out of a note arrives at the next note on its string if that starts at most this
/// late after the slide's note ends; else it slides into nothing (a "shift slide out").
const SLIDE_GAP_TICKS: u64 = TICKS_PER_QUARTER;

/// Pitch distance in cents within which a played note can be a legato target.
const LEGATO_CENTS: f64 = 50.0;

/// How a technique was played.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TechniqueOutcome {
    /// As the score says.
    Played,
    /// The note came, with another technique (`None`: a plain pluck).
    Other(Option<Technique>),
    /// Nothing was heard for this note.
    NotHeard,
}

/// The technique result of one note in one pass.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TechniqueResult {
    pub pass: u32,
    /// Index into `Arrangement::notes`.
    pub note: usize,
    /// The note whose sign shows the result: a slide's start, else `note`.
    pub sign: usize,
    pub expected: Technique,
    pub outcome: TechniqueOutcome,
    pub position_s: f64,
}

impl TechniqueResult {
    pub fn is_played(&self) -> bool {
        self.outcome == TechniqueOutcome::Played
    }
}

/// "Slides 8 of 10": notes with the technique and how many were played with it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TechniqueTally {
    pub technique: Technique,
    pub played: u32,
    pub judged: u32,
}

/// At a microphone room noise brightens soft plucks: slaps and pops are told only on a cable
/// (corpus "room-double").
const AT_MICROPHONE: [Technique; 4] = [
    Technique::Slide,
    Technique::HammerOn,
    Technique::PullOff,
    Technique::Muted,
];

/// Techniques the tracker tells for an input; none over MIDI.
pub fn told_techniques(source: InputSource) -> &'static [Technique] {
    match source {
        InputSource::Midi => &[],
        InputSource::Microphone => &AT_MICROPHONE,
        InputSource::Line | InputSource::LaptopJack => &SCORED,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Target {
    note: usize,
    sign: usize,
    position_s: f64,
    midi: u8,
    technique: Technique,
    /// Semitones from the note it comes from (a slide's start, else the note before).
    step: u8,
}

/// Up to this many semitones a slide and a hammer-on or pull-off sound alike (round 11).
const ONE_FRET: u8 = 1;

impl Target {
    /// What counts as played: the technique, and over one fret a slide for a hammer-on or
    /// pull-off and back (the pitch jumps as fast; nothing tells them apart, round 11).
    fn outcome(&self, played: Option<Technique>) -> TechniqueOutcome {
        let Some(technique) = played else {
            return TechniqueOutcome::Other(None);
        };
        let legato = |t: Technique| matches!(t, Technique::HammerOn | Technique::PullOff);
        let twins = self.step <= ONE_FRET
            && ((self.technique == Technique::Slide && legato(technique))
                || (legato(self.technique) && technique == Technique::Slide));
        if technique == self.technique || twins {
            return TechniqueOutcome::Played;
        }
        TechniqueOutcome::Other(played)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TechniqueJudge {
    /// Struck notes with a technique, by note index.
    struck: BTreeMap<usize, Target>,
    /// Legato targets (no attack), by position.
    legato: Vec<Target>,
    /// Techniques in the score this input cannot tell (shown as "not judged").
    untold: Vec<Technique>,
    resolved: HashSet<(u32, usize)>,
    results: Vec<TechniqueResult>,
    /// Next legato target to expire: (pass, index into `legato`).
    cursor: (u32, usize),
}

impl TechniqueJudge {
    pub(crate) fn new(arrangement: &Arrangement, source: InputSource) -> TechniqueJudge {
        let told = told_techniques(source);
        let mut struck = BTreeMap::new();
        let mut legato = Vec::new();
        let mut untold = Vec::new();
        for TechniqueTarget {
            note,
            sign,
            technique,
        } in technique_targets(arrangement)
        {
            if !told.contains(&technique) {
                if !untold.contains(&technique) {
                    untold.push(technique);
                }
                continue;
            }
            let n = &arrangement.notes[note];
            let from = if sign != note {
                Some(sign)
            } else {
                note.checked_sub(1)
            };
            let step = from.map_or(u8::MAX, |f| {
                n.pitch.0.abs_diff(arrangement.notes[f].pitch.0)
            });
            let target = Target {
                note,
                sign,
                position_s: arrangement.tempo.seconds_at(n.start_ticks),
                midi: n.pitch.0,
                technique,
                step,
            };
            if n.needs_attack() {
                struck.insert(note, target);
            } else {
                legato.push(target);
            }
        }
        legato.sort_by(|a, b| a.position_s.total_cmp(&b.position_s));
        untold.sort_by_key(|t| SCORED.iter().position(|s| s == t));
        TechniqueJudge {
            struck,
            legato,
            untold,
            resolved: HashSet::new(),
            results: Vec::new(),
            cursor: (0, 0),
        }
    }

    pub(crate) fn results(&self) -> &[TechniqueResult] {
        &self.results
    }

    /// The same piece transposed while playing: targets get their new pitches, what was judged
    /// stays (it is kept by target slot). False, and nothing changed, if the transposed piece
    /// has its techniques on other notes (a slide's arrival moved to another string).
    pub(crate) fn retarget(&mut self, arrangement: &Arrangement, source: InputSource) -> bool {
        let fresh = TechniqueJudge::new(arrangement, source);
        let notes = |j: &TechniqueJudge| -> Vec<usize> {
            j.struck
                .keys()
                .chain(j.legato.iter().map(|t| &t.note))
                .copied()
                .collect()
        };
        if notes(&fresh) != notes(self) {
            return false;
        }
        self.struck = fresh.struck;
        self.legato = fresh.legato;
        true
    }

    pub(crate) fn untold(&self) -> &[Technique] {
        &self.untold
    }

    /// Judges a struck note the matcher decided on; `played` is `None` when it was missed.
    pub(crate) fn judge_struck(
        &mut self,
        pass: u32,
        note: usize,
        played: Option<Option<Technique>>,
    ) -> Option<TechniqueResult> {
        let target = *self.struck.get(&note)?;
        let outcome = match played {
            None => TechniqueOutcome::NotHeard,
            Some(t) => target.outcome(t),
        };
        Some(self.push(pass, target, outcome))
    }

    /// A played note near an open legato target of `pass` judges it; true if it did (the note is
    /// then not an extra one).
    pub(crate) fn judge_legato(
        &mut self,
        pass: u32,
        position_s: f64,
        midi: f64,
        technique: Option<Technique>,
        window_s: f64,
    ) -> Option<TechniqueResult> {
        let resolved = &self.resolved;
        let slot = self
            .legato
            .iter()
            .enumerate()
            .filter(|(_, t)| (t.position_s - position_s).abs() <= window_s)
            .filter(|(_, t)| ((midi - f64::from(t.midi)) * 100.0).abs() < LEGATO_CENTS)
            .filter(|(slot, _)| !resolved.contains(&(pass, *slot)))
            .min_by(|a, b| {
                let d = |t: &Target| (t.position_s - position_s).abs();
                d(a.1).total_cmp(&d(b.1))
            })
            .map(|(slot, _)| slot)?;
        self.resolved.insert((pass, slot));
        let target = self.legato[slot];
        Some(self.push(pass, target, target.outcome(technique)))
    }

    /// Restarts expiring at `position_s` of `pass` (seek, loop).
    pub(crate) fn rewind(&mut self, pass: u32, position_s: f64) {
        self.cursor = (pass, self.first_from(position_s));
    }

    /// Legato targets the clock has passed without a note: not heard. `range`: the loop.
    pub(crate) fn expire(
        &mut self,
        pass: u32,
        position_s: f64,
        window_s: f64,
        range: (f64, f64),
    ) -> Vec<TechniqueResult> {
        let mut out = Vec::new();
        loop {
            let (cursor_pass, slot) = self.cursor;
            if cursor_pass > pass {
                return out;
            }
            let in_range = self
                .legato
                .get(slot)
                .is_some_and(|t| t.position_s >= range.0 && t.position_s < range.1);
            if !in_range {
                if cursor_pass == pass {
                    return out;
                }
                self.cursor = (cursor_pass + 1, self.first_from(range.0));
                continue;
            }
            let target = self.legato[slot];
            if cursor_pass == pass && target.position_s + window_s >= position_s {
                return out;
            }
            if self.resolved.insert((cursor_pass, slot)) {
                out.push(self.push(cursor_pass, target, TechniqueOutcome::NotHeard));
            }
            self.cursor.1 += 1;
        }
    }

    /// Per technique, the last result per pass and note.
    pub(crate) fn tally(&self) -> Vec<TechniqueTally> {
        let mut last: BTreeMap<(u32, usize), TechniqueResult> = BTreeMap::new();
        for r in &self.results {
            last.insert((r.pass, r.note), *r);
        }
        SCORED
            .iter()
            .filter_map(|&technique| {
                let of: Vec<_> = last.values().filter(|r| r.expected == technique).collect();
                if of.is_empty() {
                    return None;
                }
                Some(TechniqueTally {
                    technique,
                    played: of.iter().filter(|r| r.is_played()).count() as u32,
                    judged: of.len() as u32,
                })
            })
            .collect()
    }

    fn first_from(&self, position_s: f64) -> usize {
        self.legato.partition_point(|t| t.position_s < position_s)
    }

    fn push(&mut self, pass: u32, target: Target, outcome: TechniqueOutcome) -> TechniqueResult {
        let result = TechniqueResult {
            pass,
            note: target.note,
            sign: target.sign,
            expected: target.technique,
            outcome,
            position_s: target.position_s,
        };
        self.results.push(result);
        result
    }
}

/// A note judged for a technique and the note that shows its sign.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TechniqueTarget {
    /// The note judged: where a slide arrives, else the marked note.
    pub note: usize,
    /// The note the sign and result stand at: the marked one (a slide where it starts).
    pub sign: usize,
    pub technique: Technique,
}

/// Every note with a scored technique and that technique (the first scored one it has), by
/// judged note. A slide marked on a note is judged on the note it slides to.
pub fn technique_targets(arrangement: &Arrangement) -> Vec<TechniqueTarget> {
    let notes = &arrangement.notes;
    let mut out: BTreeMap<usize, TechniqueTarget> = BTreeMap::new();
    for (i, n) in notes.iter().enumerate() {
        let Some(technique) = SCORED.iter().copied().find(|t| n.techniques.contains(t)) else {
            continue;
        };
        let target = |note| TechniqueTarget {
            note,
            sign: i,
            technique,
        };
        if technique != Technique::Slide {
            out.entry(i).or_insert(target(i));
            continue;
        }
        let arrival = notes.iter().enumerate().find(|(_, m)| {
            m.start_ticks > n.start_ticks
                && m.start_ticks <= n.end_ticks() + SLIDE_GAP_TICKS
                && m.string == n.string
        });
        if let Some((j, _)) = arrival {
            out.entry(j).or_insert(target(j));
        }
    }
    out.into_values().collect()
}

/// The technique sign of every note that has one, by note: judged techniques at their sign
/// note, then shown-only ones (a ghost note's "( )").
pub fn technique_signs(arrangement: &Arrangement) -> BTreeMap<usize, Technique> {
    let mut out: BTreeMap<usize, Technique> = technique_targets(arrangement)
        .into_iter()
        .map(|t| (t.sign, t.technique))
        .collect();
    for (i, n) in arrangement.notes.iter().enumerate() {
        if let Some(t) = SHOWN.iter().find(|t| n.techniques.contains(t)) {
            out.entry(i).or_insert(*t);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use kontra_model::{Instrument, Note, Pitch, TempoMap};

    /// At 120 BPM, one quarter apart: E1 sl→ G1, A1 (h), D2 (S), G2 (x).
    fn piece() -> Arrangement {
        let mut a = Arrangement::new("Techniken", Instrument::default_bass());
        a.tempo = TempoMap::constant(120.0);
        let notes: [(u8, u8, &[Technique]); 5] = [
            (28, 0, &[Technique::Slide]),
            (31, 0, &[]),
            (33, 0, &[Technique::HammerOn]),
            (38, 2, &[Technique::Slap]),
            (43, 3, &[Technique::Muted]),
        ];
        for (i, (midi, string, techniques)) in notes.iter().enumerate() {
            let mut n = Note::new(
                i as u64 * TICKS_PER_QUARTER,
                TICKS_PER_QUARTER,
                Pitch(*midi),
            );
            n.string = Some(*string);
            n.techniques = techniques.to_vec();
            a.notes.push(n);
        }
        a.normalize();
        a
    }

    #[test]
    fn slide_is_judged_where_it_arrives() {
        let t: Vec<_> = technique_targets(&piece())
            .into_iter()
            .map(|t| (t.note, t.sign, t.technique))
            .collect();
        assert_eq!(
            t,
            vec![
                (1, 0, Technique::Slide),
                (2, 2, Technique::HammerOn),
                (3, 3, Technique::Slap),
                (4, 4, Technique::Muted)
            ]
        );
    }

    /// Round 10: the slide's sign stays where it starts; a ghost note ( ) gets its sign only.
    #[test]
    fn signs_stand_where_the_score_marks_them() {
        let mut a = piece();
        a.notes[1].techniques = vec![Technique::Ghost];
        let signs: Vec<_> = technique_signs(&a).into_iter().collect();
        assert_eq!(
            signs,
            vec![
                (0, Technique::Slide),
                (1, Technique::Ghost),
                (2, Technique::HammerOn),
                (3, Technique::Slap),
                (4, Technique::Muted)
            ]
        );
        let judged: Vec<_> = technique_targets(&a).iter().map(|t| t.note).collect();
        assert_eq!(judged, vec![1, 2, 3, 4], "the ghost note is not judged");
    }

    #[test]
    fn struck_and_legato_are_judged_and_tallied() {
        let mut j = TechniqueJudge::new(&piece(), InputSource::Line);
        let slide = j.judge_struck(0, 1, Some(Some(Technique::Slide))).unwrap();
        assert!(slide.is_played());
        // Plucked instead of a hammer-on: judged here, so the matcher does not see an extra.
        let h = j.judge_legato(0, 1.01, 33.1, None, 0.06).unwrap();
        assert_eq!(h.outcome, TechniqueOutcome::Other(None));
        assert!(
            j.judge_legato(0, 1.01, 33.0, None, 0.06).is_none(),
            "judged once"
        );
        let slap = j.judge_struck(0, 3, None).unwrap();
        assert_eq!(slap.outcome, TechniqueOutcome::NotHeard);
        assert!(
            j.judge_struck(0, 0, Some(None)).is_none(),
            "no technique on note 0"
        );
        let tally = j.tally();
        assert_eq!(
            tally[0],
            TechniqueTally {
                technique: Technique::Slide,
                played: 1,
                judged: 1
            }
        );
        assert_eq!(tally[1].played, 0);
        assert_eq!(tally.len(), 3, "the dead note was not judged yet");
    }

    /// Round 11: a slide over one fret sounds like a hammer-on (0–22 ms to the next note); each
    /// counts for the other. Over more frets they stay apart.
    #[test]
    fn one_fret_slide_and_hammer_on_count_for_each_other() {
        let mut a = piece();
        a.notes[1].pitch = Pitch(29);
        a.notes[2].pitch = Pitch(30);
        let mut j = TechniqueJudge::new(&a, InputSource::Line);
        let slide = j
            .judge_struck(0, 1, Some(Some(Technique::HammerOn)))
            .unwrap();
        assert!(slide.is_played(), "{slide:?}");
        let h = j
            .judge_legato(0, 1.0, 30.0, Some(Technique::Slide), 0.06)
            .unwrap();
        assert!(h.is_played(), "{h:?}");

        let mut j = TechniqueJudge::new(&piece(), InputSource::Line);
        let far = j
            .judge_struck(0, 1, Some(Some(Technique::HammerOn)))
            .unwrap();
        assert_eq!(
            far.outcome,
            TechniqueOutcome::Other(Some(Technique::HammerOn))
        );
    }

    #[test]
    fn legato_targets_expire_once_passed() {
        let mut j = TechniqueJudge::new(&piece(), InputSource::Line);
        let all = (0.0, f64::INFINITY);
        assert!(
            j.expire(0, 1.0, 0.06, all).is_empty(),
            "still in its window"
        );
        let gone = j.expire(0, 1.2, 0.06, all);
        assert_eq!(gone.len(), 1);
        assert_eq!(gone[0].outcome, TechniqueOutcome::NotHeard);
        assert!(j.expire(0, 3.0, 0.06, all).is_empty());
    }

    #[test]
    fn microphone_does_not_judge_slap_and_midi_nothing() {
        let mic = TechniqueJudge::new(&piece(), InputSource::Microphone);
        assert!(!mic.struck.contains_key(&3));
        assert_eq!(mic.untold(), &[Technique::Slap]);
        let midi = TechniqueJudge::new(&piece(), InputSource::Midi);
        assert!(midi.struck.is_empty() && midi.legato.is_empty());
        assert_eq!(midi.untold().len(), 4);
    }
}
