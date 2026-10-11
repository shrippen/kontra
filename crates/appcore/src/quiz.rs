//! Fretboard quiz with the real instrument (LH-LRN-02): find a note on a string, or play an
//! interval above a given note. The tuner's pitch answers; nothing here touches audio.

use kontra_model::{Instrument, Pitch};

/// What the quiz asks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuizKind {
    /// "Play a D on the A string": any fret of that string with the note counts.
    Notes,
    /// "Play the fifth above A (A string, open)": the exact pitch counts.
    Intervals,
}

/// Intervals asked, in semitones, with their text key suffix.
pub(crate) const INTERVALS: [(i32, &str); 7] = [
    (3, "m3"),
    (4, "M3"),
    (5, "p4"),
    (7, "p5"),
    (9, "M6"),
    (10, "m7"),
    (12, "octave"),
];
/// Frets a question may use: the first position up to the octave.
const MAX_FRET: u8 = 12;

/// One question.
#[derive(Debug, Clone, PartialEq)]
pub struct Question {
    pub kind: QuizKind,
    /// String index (0 = lowest) the note is asked on, or the root sits on.
    pub string: usize,
    /// Notes: the pitch class wanted; intervals: the exact pitch wanted.
    pub target: u8,
    /// Intervals: the root and the interval's text key ("p5").
    pub root: Option<u8>,
    pub interval: Option<&'static str>,
}

/// Quiz state: the current question, answers so far and a small deterministic random source.
#[derive(Debug, Clone)]
pub struct Quiz {
    pub kind: QuizKind,
    pub question: Question,
    /// Questions shown so far, the current one included.
    pub asked: u32,
    pub right: u32,
    /// The current question was already answered right (further notes are ignored).
    pub solved: bool,
    seed: u64,
}

impl Quiz {
    pub fn new(kind: QuizKind, instrument: &Instrument, seed: u64) -> Quiz {
        let mut quiz = Quiz {
            kind,
            question: Question {
                kind,
                string: 0,
                target: 0,
                root: None,
                interval: None,
            },
            asked: 0,
            right: 0,
            solved: false,
            seed: seed.max(1),
        };
        quiz.next(instrument);
        quiz
    }

    fn random(&mut self, below: u64) -> u64 {
        // xorshift64: enough to vary questions, reproducible in tests.
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        self.seed % below.max(1)
    }

    /// Moves to a new question, never the same one twice in a row.
    pub fn next(&mut self, instrument: &Instrument) {
        let strings = instrument.tuning.strings.len().max(1);
        let previous = self.question.clone();
        for _ in 0..32 {
            let string = self.random(strings as u64) as usize;
            let open = instrument.tuning.strings.get(string).map_or(28, |p| p.0);
            let question = match self.kind {
                QuizKind::Notes => {
                    let fret = self.random(u64::from(MAX_FRET)) as u8;
                    Question {
                        kind: self.kind,
                        string,
                        target: (open + fret) % 12,
                        root: None,
                        interval: None,
                    }
                }
                QuizKind::Intervals => {
                    let fret = self.random(8) as u8;
                    let (semitones, key) = INTERVALS[self.random(INTERVALS.len() as u64) as usize];
                    let root = open + fret;
                    Question {
                        kind: self.kind,
                        string,
                        target: (i32::from(root) + semitones).clamp(0, 127) as u8,
                        root: Some(root),
                        interval: Some(key),
                    }
                }
            };
            // An interval above a high root may leave the first twelve frets: ask another.
            let top = instrument
                .tuning
                .strings
                .iter()
                .map(|p| p.0)
                .max()
                .unwrap_or(43);
            let reachable = question.root.is_none() || question.target <= top + MAX_FRET;
            if question != previous && reachable {
                self.question = question;
                break;
            }
        }
        self.solved = false;
        self.asked += 1;
    }

    /// Checks a played pitch (MIDI, fractional). The first right answer per question counts;
    /// returns whether this note answers it.
    pub fn answer(&mut self, midi: f64, instrument: &Instrument) -> bool {
        if self.solved || !midi.is_finite() {
            return false;
        }
        let played = midi.round().clamp(0.0, 127.0) as u8;
        let right = match self.kind {
            QuizKind::Notes => {
                let open = instrument
                    .tuning
                    .strings
                    .get(self.question.string)
                    .map_or(28, |p| p.0);
                played % 12 == self.question.target && (open..=open + MAX_FRET).contains(&played)
            }
            QuizKind::Intervals => played == self.question.target,
        };
        if right {
            self.solved = true;
            self.right += 1;
        }
        right
    }

    /// Where the answer lies: (string, fret) for every place up to fret 15.
    pub fn positions(&self, instrument: &Instrument) -> Vec<(usize, u8)> {
        let strings = &instrument.tuning.strings;
        strings
            .iter()
            .enumerate()
            .flat_map(|(s, open)| (0..=15u8).map(move |f| (s, open.0, f)))
            .filter(|(s, open, f)| {
                let pitch = open + f;
                match self.kind {
                    QuizKind::Notes => {
                        *s == self.question.string
                            && pitch % 12 == self.question.target
                            && *f <= MAX_FRET
                    }
                    QuizKind::Intervals => pitch == self.question.target,
                }
            })
            .map(|(s, _, f)| (s, f))
            .collect()
    }

    /// The root's place for interval questions (string, fret).
    pub fn root_position(&self, instrument: &Instrument) -> Option<(usize, u8)> {
        let root = self.question.root?;
        let open = instrument.tuning.strings.get(self.question.string)?.0;
        root.checked_sub(open).map(|f| (self.question.string, f))
    }
}

/// Note name of a pitch class or pitch (sharps; the caller localises B/H).
pub fn note_name(pitch: u8) -> String {
    Pitch(pitch).name().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bass() -> Instrument {
        Instrument::default_bass()
    }

    #[test]
    fn note_on_the_asked_string_counts() {
        let bass = bass();
        let mut quiz = Quiz::new(QuizKind::Notes, &bass, 7);
        quiz.question = Question {
            kind: QuizKind::Notes,
            string: 1, // A string, open A1 = 33
            target: 2, // D
            root: None,
            interval: None,
        };
        // D2 = 38 is fret 5 on the A string.
        assert!(!quiz.answer(40.1, &bass), "E is wrong");
        assert!(quiz.answer(37.8, &bass), "D2 within a quarter tone");
        assert!(!quiz.answer(38.0, &bass), "counted once");
        assert_eq!(quiz.right, 1);
        assert_eq!(quiz.positions(&bass), [(1, 5)]);
    }

    #[test]
    fn interval_needs_the_exact_pitch() {
        let bass = bass();
        let mut quiz = Quiz::new(QuizKind::Intervals, &bass, 3);
        quiz.question = Question {
            kind: QuizKind::Intervals,
            string: 1,
            target: 40, // fifth above A1
            root: Some(33),
            interval: Some("p5"),
        };
        assert!(
            !quiz.answer(52.0, &bass),
            "an octave higher is another note"
        );
        assert!(quiz.answer(40.0, &bass));
        assert_eq!(quiz.root_position(&bass), Some((1, 0)));
        assert!(quiz.positions(&bass).contains(&(0, 12)));
        assert!(quiz.positions(&bass).contains(&(1, 7)));
    }

    #[test]
    fn questions_change_and_stay_on_the_neck() {
        let bass = bass();
        for kind in [QuizKind::Notes, QuizKind::Intervals] {
            let mut quiz = Quiz::new(kind, &bass, 42);
            for _ in 0..50 {
                let before = quiz.question.clone();
                quiz.next(&bass);
                assert_ne!(before, quiz.question);
                assert!(!quiz.positions(&bass).is_empty(), "{:?}", quiz.question);
            }
        }
    }
}
