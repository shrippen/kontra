//! Ear training (LH-LRN-03): Kontra plays, the player answers on the instrument.
//!
//! ```text
//! Echo       ♪ ♪ ♪      → play the same notes in order (the line grows while it goes well)
//! Interval   ♪ root ♪   → play the second note; the root is named
//! ChordRoot  ♪♪♪ chord  → play the chord's root, any octave
//! ```
//!
//! Only pitches count, never strings or frets, so the same exercises work for every instrument.
//! Nothing here touches audio; `Core` plays the prompt and passes heard notes in.

use crate::quiz::INTERVALS;
use kontra_model::{Instrument, Pitch};
use kontra_session::PromptTone;

/// What the exercise asks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EarKind {
    Echo,
    Interval,
    ChordRoot,
}

/// How a played note must match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Match {
    Exact,
    AnyOctave,
}

/// The verdict on one played note, like LH-BEW-01's hit or wrong pitch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Right,
    Wrong,
    /// The last note of the answer: the question is done.
    Solved,
    /// Already solved; further notes do not count.
    Ignored,
}

/// Prompt notes start this far above the instrument's lowest note: on a bass A1, reachable on
/// every string in the first position.
const LOW_OFFSET: i32 = 5;
/// Prompt notes stay within this span above that.
const SPAN: i32 = 12;
const NOTE_S: f64 = 0.6;
const NOTE_GAP_S: f64 = 0.75;
const CHORD_S: f64 = 1.6;
/// Steps between neighbouring notes of an echo line, in semitones (up or down).
const MAX_STEP: u64 = 5;
const MAX_LINE: usize = 4;
/// Questions right at the first try before the line grows by one note.
const STREAK_UP: u32 = 3;
const CHORDS: [(&str, [i32; 3]); 2] = [("major", [0, 4, 7]), ("minor", [0, 3, 7])];

/// A played note and how it compares with the wanted one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Played {
    pub pitch: Pitch,
    pub right: bool,
    /// Semitones above (+) or below (−) the wanted note; 0 for any-octave answers.
    pub offset: i32,
}

/// One question.
#[derive(Debug, Clone, PartialEq)]
pub struct EarQuestion {
    pub tones: Vec<PromptTone>,
    /// The pitches to play, in order.
    pub answer: Vec<Pitch>,
    pub matching: Match,
    /// Interval: the root (named in the prompt) and the interval's text key ("p5").
    pub root: Option<Pitch>,
    pub interval: Option<&'static str>,
    /// Chord: the quality's text key ("major").
    pub chord: Option<&'static str>,
}

/// State of an ear-training round.
#[derive(Debug, Clone)]
pub struct EarQuiz {
    pub kind: EarKind,
    pub question: EarQuestion,
    pub asked: u32,
    /// Questions answered without a wrong note.
    pub right: u32,
    /// Notes of the answer already played right.
    pub progress: usize,
    pub solved: bool,
    /// Played notes of this question with their verdict.
    pub played: Vec<Played>,
    /// Length of the next echo line, 1..=`MAX_LINE`.
    pub line_len: usize,
    missed: bool,
    streak: u32,
    seed: u64,
}

impl EarQuiz {
    pub fn new(kind: EarKind, instrument: &Instrument, seed: u64) -> EarQuiz {
        let mut quiz = EarQuiz {
            kind,
            question: EarQuestion {
                tones: Vec::new(),
                answer: Vec::new(),
                matching: Match::Exact,
                root: None,
                interval: None,
                chord: None,
            },
            asked: 0,
            right: 0,
            progress: 0,
            solved: false,
            played: Vec::new(),
            line_len: 1,
            missed: false,
            streak: 0,
            seed: seed.max(1),
        };
        quiz.next(instrument);
        quiz
    }

    /// One fixed question, as a lesson asks it (LH-LRN-08): play `answer` in order; the prompt
    /// sounds the same notes. Nothing adapts or moves on.
    pub fn fixed(answer: Vec<Pitch>, matching: Match) -> EarQuiz {
        EarQuiz {
            kind: EarKind::Echo,
            question: EarQuestion {
                tones: line(&answer),
                answer,
                matching,
                root: None,
                interval: None,
                chord: None,
            },
            asked: 1,
            right: 0,
            progress: 0,
            solved: false,
            played: Vec::new(),
            line_len: 1,
            missed: false,
            streak: 0,
            seed: 1,
        }
    }

    fn random(&mut self, below: u64) -> u64 {
        // xorshift64, as in the fretboard quiz: varied and reproducible in tests.
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        self.seed % below.max(1)
    }

    /// Moves to a new question, never the same one twice in a row.
    pub fn next(&mut self, instrument: &Instrument) {
        let low = instrument.range().map_or(28, |(low, _)| i32::from(low.0)) + LOW_OFFSET;
        for _ in 0..32 {
            let question = match self.kind {
                EarKind::Echo => self.echo(low),
                EarKind::Interval => self.interval(low),
                EarKind::ChordRoot => self.chord(low),
            };
            if question.answer != self.question.answer {
                self.question = question;
                break;
            }
        }
        self.asked += 1;
        self.progress = 0;
        self.solved = false;
        self.missed = false;
        self.played.clear();
    }

    /// A line of `line_len` notes in small steps, no note repeated right after itself.
    fn echo(&mut self, low: i32) -> EarQuestion {
        let mut pitch = low + self.random(SPAN as u64 + 1) as i32;
        let mut notes = vec![pitch];
        while notes.len() < self.line_len {
            let step = 1 + self.random(MAX_STEP) as i32;
            let down = self.random(2) == 0;
            let moved = if down { pitch - step } else { pitch + step };
            // Bounce back into the range instead of leaving it.
            pitch = if (low..=low + SPAN).contains(&moved) {
                moved
            } else if down {
                pitch + step
            } else {
                pitch - step
            };
            notes.push(pitch);
        }
        let answer: Vec<Pitch> = notes.iter().map(|&p| pitch_of(p)).collect();
        let tones = line(&answer);
        EarQuestion {
            tones,
            answer,
            matching: Match::Exact,
            root: None,
            interval: None,
            chord: None,
        }
    }

    /// Root, then the interval above it; the second note is the answer.
    fn interval(&mut self, low: i32) -> EarQuestion {
        let (semitones, key) = INTERVALS[self.random(INTERVALS.len() as u64) as usize];
        let root = pitch_of(low + self.random(8) as i32);
        let upper = pitch_of(i32::from(root.0) + semitones);
        EarQuestion {
            tones: vec![
                PromptTone {
                    pitch: root,
                    at_s: 0.0,
                    duration_s: NOTE_S,
                },
                PromptTone {
                    pitch: upper,
                    at_s: NOTE_GAP_S,
                    duration_s: NOTE_S,
                },
            ],
            answer: vec![upper],
            matching: Match::Exact,
            root: Some(root),
            interval: Some(key),
            chord: None,
        }
    }

    /// A triad an octave above its root, in a random inversion; the root is the answer.
    /// Example: A minor, first inversion: C3 E3 A3; play any A.
    fn chord(&mut self, low: i32) -> EarQuestion {
        let (key, steps) = CHORDS[self.random(CHORDS.len() as u64) as usize];
        let root = low + self.random(SPAN as u64) as i32;
        let inversion = self.random(3) as usize;
        let tones = steps
            .iter()
            .enumerate()
            .map(|(i, step)| {
                let lifted = if i < inversion { 12 } else { 0 };
                PromptTone {
                    pitch: pitch_of(root + 12 + step + lifted),
                    at_s: 0.0,
                    duration_s: CHORD_S,
                }
            })
            .collect();
        EarQuestion {
            tones,
            answer: vec![pitch_of(root)],
            matching: Match::AnyOctave,
            root: None,
            interval: None,
            chord: Some(key),
        }
    }

    /// Judges a played note (fractional MIDI). A wrong note starts the answer over.
    pub fn answer(&mut self, midi: f64) -> Verdict {
        if self.solved || !midi.is_finite() {
            return Verdict::Ignored;
        }
        let played = pitch_of(midi.round() as i32);
        let Some(&wanted) = self.question.answer.get(self.progress) else {
            return Verdict::Ignored;
        };
        let right = match self.question.matching {
            Match::Exact => played == wanted,
            Match::AnyOctave => played.0 % 12 == wanted.0 % 12,
        };
        let offset = match self.question.matching {
            Match::Exact => i32::from(played.0) - i32::from(wanted.0),
            Match::AnyOctave => 0,
        };
        self.played.push(Played {
            pitch: played,
            right,
            offset,
        });

        if !right {
            self.missed = true;
            self.progress = 0;
            return Verdict::Wrong;
        }
        self.progress += 1;
        if self.progress < self.question.answer.len() {
            return Verdict::Right;
        }
        self.solved = true;
        self.adapt();
        Verdict::Solved
    }

    /// Echo lines grow after `STREAK_UP` clean answers and shrink after one with mistakes.
    fn adapt(&mut self) {
        if self.missed {
            self.streak = 0;
            self.line_len = self.line_len.saturating_sub(1).max(1);
            return;
        }
        self.right += 1;
        self.streak += 1;
        if self.kind == EarKind::Echo && self.streak >= STREAK_UP && self.line_len < MAX_LINE {
            self.line_len += 1;
            self.streak = 0;
        }
    }
}

/// The notes one after the other, as an echo line sounds.
fn line(pitches: &[Pitch]) -> Vec<PromptTone> {
    pitches
        .iter()
        .enumerate()
        .map(|(i, &pitch)| PromptTone {
            pitch,
            at_s: i as f64 * NOTE_GAP_S,
            duration_s: NOTE_S,
        })
        .collect()
}

fn pitch_of(midi: i32) -> Pitch {
    Pitch(midi.clamp(0, 127) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bass() -> Instrument {
        Instrument::default_bass()
    }

    fn solve(quiz: &mut EarQuiz) {
        for pitch in quiz.question.answer.clone() {
            quiz.answer(f64::from(pitch.0));
        }
    }

    #[test]
    fn a_fixed_question_takes_repeated_notes() {
        let mut quiz = EarQuiz::fixed(vec![Pitch(33); 4], Match::Exact);
        assert_eq!(quiz.question.tones.len(), 4);
        for _ in 0..3 {
            assert_eq!(quiz.answer(33.0), Verdict::Right);
        }
        assert_eq!(quiz.answer(33.2), Verdict::Solved);
    }

    #[test]
    fn echo_counts_notes_in_order() {
        let mut quiz = EarQuiz::new(EarKind::Echo, &bass(), 5);
        quiz.question.answer = vec![Pitch(33), Pitch(36), Pitch(38)];
        assert_eq!(quiz.answer(33.1), Verdict::Right);
        assert_eq!(quiz.answer(35.0), Verdict::Wrong);
        assert_eq!(quiz.played[1].offset, -1, "B1 is a semitone below C2");
        assert_eq!(quiz.progress, 0, "a wrong note starts over");
        assert_eq!(quiz.answer(33.0), Verdict::Right);
        assert_eq!(quiz.answer(36.0), Verdict::Right);
        assert_eq!(quiz.answer(37.8), Verdict::Solved);
        assert_eq!(quiz.answer(40.0), Verdict::Ignored);
        assert_eq!(quiz.right, 0, "not at the first try");
        assert_eq!(quiz.played.len(), 5);
    }

    #[test]
    fn interval_wants_the_exact_upper_note() {
        let mut quiz = EarQuiz::new(EarKind::Interval, &bass(), 9);
        let upper = quiz.question.answer[0];
        let root = quiz.question.root.unwrap();
        assert!(upper.0 > root.0);
        assert_eq!(quiz.answer(f64::from(upper.0 + 12)), Verdict::Wrong);
        assert_eq!(quiz.answer(f64::from(upper.0)), Verdict::Solved);
    }

    #[test]
    fn chord_root_counts_in_any_octave() {
        let mut quiz = EarQuiz::new(EarKind::ChordRoot, &bass(), 11);
        let root = quiz.question.answer[0];
        assert_eq!(quiz.question.tones.len(), 3);
        assert_eq!(quiz.answer(f64::from(root.0 + 12)), Verdict::Solved);
        assert_eq!(quiz.right, 1);
    }

    #[test]
    fn echo_line_grows_and_shrinks() {
        let bass = bass();
        let mut quiz = EarQuiz::new(EarKind::Echo, &bass, 21);
        for _ in 0..STREAK_UP {
            assert_eq!(quiz.question.answer.len(), 1);
            solve(&mut quiz);
            quiz.next(&bass);
        }
        assert_eq!(quiz.question.answer.len(), 2);

        // One mistake: the next line is shorter again.
        quiz.answer(0.0);
        solve(&mut quiz);
        quiz.next(&bass);
        assert_eq!(quiz.question.answer.len(), 1);
    }

    #[test]
    fn prompts_stay_in_reach() {
        let bass = bass();
        let low = 28 + LOW_OFFSET as u8;
        for kind in [EarKind::Echo, EarKind::Interval, EarKind::ChordRoot] {
            let mut quiz = EarQuiz::new(kind, &bass, 42);
            quiz.line_len = MAX_LINE;
            for _ in 0..50 {
                let before = quiz.question.answer.clone();
                quiz.next(&bass);
                assert_ne!(before, quiz.question.answer);
                for pitch in &quiz.question.answer {
                    assert!(pitch.0 >= low && pitch.0 <= low + 24, "{pitch:?}");
                    assert!(!bass.positions(*pitch).is_empty(), "{pitch:?} playable");
                }
                let line = &quiz.question.answer;
                assert!(line.windows(2).all(|w| w[0] != w[1]), "{line:?}");
            }
        }
    }
}
