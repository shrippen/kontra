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
//! ```
//!
//! Notes that are struck (slap, pop, ghost, the arrival of a slide) are judged with the
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
    Technique::Ghost,
];

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
    Technique::Ghost,
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
    position_s: f64,
    midi: u8,
    technique: Technique,
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
        for (note, technique) in technique_targets(arrangement) {
            if !told.contains(&technique) {
                if !untold.contains(&technique) {
                    untold.push(technique);
                }
                continue;
            }
            let n = &arrangement.notes[note];
            let target = Target {
                note,
                position_s: arrangement.tempo.seconds_at(n.start_ticks),
                midi: n.pitch.0,
                technique,
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
            Some(t) => outcome_of(target.technique, t),
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
        Some(self.push(pass, target, outcome_of(target.technique, technique)))
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
            expected: target.technique,
            outcome,
            position_s: target.position_s,
        };
        self.results.push(result);
        result
    }
}

fn outcome_of(expected: Technique, played: Option<Technique>) -> TechniqueOutcome {
    if played == Some(expected) {
        TechniqueOutcome::Played
    } else {
        TechniqueOutcome::Other(played)
    }
}

/// Every note with a scored technique and that technique (the first scored one it has). A slide
/// marked on a note is judged on the note it slides to; views mark it there too.
pub fn technique_targets(arrangement: &Arrangement) -> Vec<(usize, Technique)> {
    let notes = &arrangement.notes;
    let mut out: BTreeMap<usize, Technique> = BTreeMap::new();
    for (i, n) in notes.iter().enumerate() {
        let Some(technique) = SCORED.iter().copied().find(|t| n.techniques.contains(t)) else {
            continue;
        };
        if technique != Technique::Slide {
            out.entry(i).or_insert(technique);
            continue;
        }
        let arrival = notes.iter().enumerate().find(|(_, m)| {
            m.start_ticks > n.start_ticks
                && m.start_ticks <= n.end_ticks() + SLIDE_GAP_TICKS
                && m.string == n.string
        });
        if let Some((j, _)) = arrival {
            out.entry(j).or_insert(Technique::Slide);
        }
    }
    out.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use kontra_model::{Instrument, Note, Pitch, TempoMap};

    /// At 120 BPM, one quarter apart: E1 sl→ G1, A1 (h), D2 (S), G2 (ghost).
    fn piece() -> Arrangement {
        let mut a = Arrangement::new("Techniken", Instrument::default_bass());
        a.tempo = TempoMap::constant(120.0);
        let notes: [(u8, u8, &[Technique]); 5] = [
            (28, 0, &[Technique::Slide]),
            (31, 0, &[]),
            (33, 0, &[Technique::HammerOn]),
            (38, 2, &[Technique::Slap]),
            (43, 3, &[Technique::Ghost]),
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
        let t = technique_targets(&piece());
        assert_eq!(
            t,
            vec![
                (1, Technique::Slide),
                (2, Technique::HammerOn),
                (3, Technique::Slap),
                (4, Technique::Ghost)
            ]
        );
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
        assert_eq!(tally.len(), 3, "the ghost was not judged yet");
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
