//! Fretboard quiz with the real instrument (LH-LRN-02): find a note on a string, or play an
//! interval above a given note. The tuner's pitch answers; nothing here touches audio.
//! Notes come from the chosen frets, weak and long unpractised places more often (LH-LRN-18).

use crate::quiz_stats::{Outcome, QuizStats};
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
    /// Fret of the asked place (notes) or of the root (intervals).
    pub fret: u8,
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
    /// A wrong note was played for the current question (counted once).
    missed: bool,
    stats: QuizStats,
    seed: u64,
}

impl Quiz {
    pub fn new(
        kind: QuizKind,
        instrument: &Instrument,
        stats: QuizStats,
        seed: u64,
        now_s: u64,
    ) -> Quiz {
        let mut quiz = Quiz {
            kind,
            question: Question {
                kind,
                string: 0,
                fret: 0,
                target: 0,
                root: None,
                interval: None,
            },
            asked: 0,
            right: 0,
            solved: false,
            missed: false,
            stats,
            seed: seed.max(1),
        };
        quiz.next(instrument, now_s);
        quiz
    }

    /// What to store: the range and the answers so far.
    pub fn stats(&self) -> &QuizStats {
        &self.stats
    }

    fn random(&mut self, below: u64) -> u64 {
        // xorshift64: enough to vary questions, reproducible in tests.
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        self.seed % below.max(1)
    }

    /// A number in [0, 1).
    fn unit(&mut self) -> f64 {
        const SCALE: u64 = 1 << 53;
        self.random(SCALE) as f64 / SCALE as f64
    }

    /// A place in the range, drawn by the weights of the stats; the previous question's places
    /// are left out.
    fn pick(&mut self, instrument: &Instrument, previous: &Question, now_s: u64) -> (usize, u8) {
        let range = self.stats.range;
        let places: Vec<(usize, u8, f64)> = instrument
            .tuning
            .strings
            .iter()
            .enumerate()
            .flat_map(|(s, open)| range.frets().map(move |f| (s, open.0, f)))
            .filter(|(s, open, f)| *s != previous.string || (open + f) % 12 != previous.target)
            .map(|(s, open, f)| (s, f, self.stats.weight(open, f, now_s)))
            .collect();
        let total: f64 = places.iter().map(|p| p.2).sum();
        let mut point = self.unit() * total;
        for &(string, fret, weight) in &places {
            if point < weight {
                return (string, fret);
            }
            point -= weight;
        }
        places.last().map_or((0, range.low), |p| (p.0, p.1))
    }

    /// Moves to a new question, never the same one twice in a row.
    pub fn next(&mut self, instrument: &Instrument, now_s: u64) {
        let strings = instrument.tuning.strings.len().max(1);
        let previous = self.question.clone();
        for _ in 0..32 {
            let string = self.random(strings as u64) as usize;
            let open = open_pitch(instrument, string);
            let question = match self.kind {
                QuizKind::Notes => {
                    let (string, fret) = self.pick(instrument, &previous, now_s);
                    Question {
                        kind: self.kind,
                        string,
                        fret,
                        target: (open_pitch(instrument, string) + fret) % 12,
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
                        fret,
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
        self.missed = false;
        self.asked += 1;
    }

    /// Checks a played pitch (MIDI, fractional). The first right answer per question counts;
    /// returns whether this note answers it. Notes questions remember the place: found, or
    /// missed once a wrong note was played.
    pub fn answer(&mut self, midi: f64, instrument: &Instrument, now_s: u64) -> bool {
        if self.solved || !midi.is_finite() {
            return false;
        }
        let played = midi.round().clamp(0.0, 127.0) as u8;
        let right = match self.kind {
            QuizKind::Notes => {
                let open = open_pitch(instrument, self.question.string);
                played % 12 == self.question.target && (open..=open + MAX_FRET).contains(&played)
            }
            QuizKind::Intervals => played == self.question.target,
        };
        if self.kind == QuizKind::Notes && !self.missed {
            let outcome = if right {
                Outcome::Right
            } else {
                Outcome::Wrong
            };
            let open = open_pitch(instrument, self.question.string);
            self.stats.record(open, self.question.fret, outcome, now_s);
            self.missed = !right;
        }
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

/// Open pitch (MIDI) of a string, E1 when the tuning lacks it.
fn open_pitch(instrument: &Instrument, string: usize) -> u8 {
    const E1: u8 = 28;
    instrument.tuning.strings.get(string).map_or(E1, |p| p.0)
}

/// Note name of a pitch class or pitch (sharps; the caller localises B/H).
pub fn note_name(pitch: u8) -> String {
    Pitch(pitch).name().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quiz_stats::FretRange;

    const NOW: u64 = 1_000_000_000;
    const A1: u8 = 33;

    fn bass() -> Instrument {
        Instrument::default_bass()
    }

    fn quiz(kind: QuizKind, stats: QuizStats, seed: u64) -> Quiz {
        Quiz::new(kind, &bass(), stats, seed, NOW)
    }

    /// LH-LRN-18: a place missed often comes measurably more often than one found often
    /// (20 places at frets 0–4: evenly, B on the A string would be asked about 1 in 20).
    #[test]
    fn missed_note_comes_more_often() {
        let bass = bass();
        let mut stats = QuizStats::default();
        for open in bass.tuning.strings.iter().map(|p| p.0) {
            for fret in 0..=4 {
                for _ in 0..5 {
                    let missed = open == A1 && fret == 2;
                    let outcome = if missed {
                        Outcome::Wrong
                    } else {
                        Outcome::Right
                    };
                    stats.record(open, fret, outcome, NOW);
                }
            }
        }
        let mut quiz = quiz(QuizKind::Notes, stats, 11);
        let draws = 2000;
        let mut weak = 0;
        for _ in 0..draws {
            quiz.next(&bass, NOW);
            if (quiz.question.string, quiz.question.fret) == (1, 2) {
                weak += 1;
            }
        }
        assert!(
            weak > 2 * draws / 20,
            "B on the A string asked {weak} of {draws}"
        );
    }

    #[test]
    fn questions_stay_in_the_range() {
        let bass = bass();
        let stats = QuizStats::new(FretRange::new(5, 7));
        let mut quiz = quiz(QuizKind::Notes, stats, 5);
        for _ in 0..200 {
            quiz.next(&bass, NOW);
            assert!((5..=7).contains(&quiz.question.fret), "{:?}", quiz.question);
        }
    }

    /// Time up or skipped is no mistake; a wrong note is one, counted once per question.
    #[test]
    fn only_wrong_notes_count_as_missed() {
        let bass = bass();
        let mut quiz = quiz(QuizKind::Notes, QuizStats::default(), 9);
        let ask = |quiz: &mut Quiz| {
            quiz.question = Question {
                kind: QuizKind::Notes,
                string: 1,
                fret: 5,
                target: 2, // D on the A string
                root: None,
                interval: None,
            };
            quiz.solved = false;
            quiz.missed = false;
        };
        ask(&mut quiz);
        quiz.next(&bass, NOW);
        assert_eq!(quiz.stats().counts(A1, 5), (0, 0), "skipped");

        ask(&mut quiz);
        assert!(!quiz.answer(40.0, &bass, NOW));
        assert!(!quiz.answer(41.0, &bass, NOW));
        assert!(quiz.answer(38.0, &bass, NOW));
        assert_eq!(
            quiz.stats().counts(A1, 5),
            (0, 1),
            "missed once, then found"
        );

        ask(&mut quiz);
        assert!(quiz.answer(38.0, &bass, NOW));
        assert_eq!(quiz.stats().counts(A1, 5), (1, 1));
    }

    #[test]
    fn note_on_the_asked_string_counts() {
        let bass = bass();
        let mut quiz = quiz(QuizKind::Notes, QuizStats::default(), 7);
        quiz.question = Question {
            kind: QuizKind::Notes,
            string: 1, // A string, open A1 = 33
            fret: 5,
            target: 2, // D
            root: None,
            interval: None,
        };
        // D2 = 38 is fret 5 on the A string.
        assert!(!quiz.answer(40.1, &bass, NOW), "E is wrong");
        assert!(quiz.answer(37.8, &bass, NOW), "D2 within a quarter tone");
        assert!(!quiz.answer(38.0, &bass, NOW), "counted once");
        assert_eq!(quiz.right, 1);
        assert_eq!(quiz.positions(&bass), [(1, 5)]);
    }

    #[test]
    fn interval_needs_the_exact_pitch() {
        let bass = bass();
        let mut quiz = quiz(QuizKind::Intervals, QuizStats::default(), 3);
        quiz.question = Question {
            kind: QuizKind::Intervals,
            string: 1,
            fret: 0,
            target: 40, // fifth above A1
            root: Some(33),
            interval: Some("p5"),
        };
        assert!(
            !quiz.answer(52.0, &bass, NOW),
            "an octave higher is another note"
        );
        assert!(quiz.answer(40.0, &bass, NOW));
        assert_eq!(quiz.root_position(&bass), Some((1, 0)));
        assert!(quiz.positions(&bass).contains(&(0, 12)));
        assert!(quiz.positions(&bass).contains(&(1, 7)));
    }

    #[test]
    fn questions_change_and_stay_on_the_neck() {
        let bass = bass();
        for kind in [QuizKind::Notes, QuizKind::Intervals] {
            let mut quiz = quiz(kind, QuizStats::default(), 42);
            for _ in 0..50 {
                let before = quiz.question.clone();
                quiz.next(&bass, NOW);
                assert_ne!(before, quiz.question);
                assert!(!quiz.positions(&bass).is_empty(), "{:?}", quiz.question);
            }
        }
    }
}
