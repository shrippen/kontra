//! Fretboard quiz in the app (LH-LRN-18): the `quiz_*` calls the QML bridge makes on `Core`.

use crate::{instrument_from, quiz, quiz_stats, save, time_seed, unix_now, Core};
use serde_json::json;

impl Core {
    /// Starts the fretboard quiz ("notes" or "intervals") on frets `low`–`high` (kept for the
    /// next start, LH-LRN-18) and returns `quiz_json`.
    pub fn quiz_start(&mut self, kind: &str, low: i32, high: i32) -> String {
        let kind = match kind {
            "intervals" => quiz::QuizKind::Intervals,
            _ => quiz::QuizKind::Notes,
        };
        let mut stats: quiz_stats::QuizStats = save::load(&self.data_dir.join(quiz_stats::FILE));
        stats.range = quiz_stats::FretRange::new(low, high);
        let instrument = instrument_from(&self.settings);
        let quiz = quiz::Quiz::new(kind, &instrument, stats, time_seed(), unix_now());
        let saved = save::save(&self.data_dir.join(quiz_stats::FILE), quiz.stats());
        self.tell_save(quiz_stats::FILE, saved);
        self.quiz = Some(quiz);
        self.quiz_json()
    }

    /// Next question.
    pub fn quiz_next(&mut self) -> String {
        let instrument = instrument_from(&self.settings);
        if let Some(q) = &mut self.quiz {
            q.next(&instrument, unix_now());
        }
        self.quiz_json()
    }

    /// Answers with a played pitch; true when it was the first right answer. Found and missed
    /// places are stored at once.
    pub fn quiz_answer(&mut self, midi: f64) -> bool {
        let instrument = instrument_from(&self.settings);
        let Some(q) = self.quiz.as_mut() else {
            return false;
        };
        let right = q.answer(midi, &instrument, unix_now());
        let file = self.data_dir.join(quiz_stats::FILE);
        let stats = save::load::<quiz_stats::QuizStats>(&file).with_places_of(q.stats());
        let saved = save::save(&file, &stats);
        self.tell_save(quiz_stats::FILE, saved);
        right
    }

    /// "3 of 4 found" over finished questions only: the open one is not lost yet (QA 13.10.: it
    /// read "0 of 1 found" before the first answer). Nothing before the first one is finished.
    /// "3 of 4 found" (quiz: solved at all) or "3 of 4 without a mistake" (ear training: clean
    /// answers only; QA 14.10.: "0 of 1 found" right after a correct answer on the fifth try).
    /// Quiz: found of found and lost; skipped questions are no miss (LH-LRN-18).
    fn quiz_score(&self, right: u32, lost: u32) -> String {
        self.score_text("quiz.score", right, right + lost, true)
    }

    pub(crate) fn score_text(&self, key: &str, right: u32, asked: u32, solved: bool) -> String {
        let done = asked - u32::from(!solved && asked > 0);
        if done == 0 {
            return String::new();
        }
        self.tr(
            key,
            &[("right", right.to_string()), ("asked", done.to_string())],
        )
    }

    /// The question in words, the score, the range, and once solved where the answer lies:
    /// `{"prompt", "right", "asked", "solved", "range": {low, high}, "positions": [{string,
    /// fret, role}]}`. Before a start only the stored range.
    pub fn quiz_json(&self) -> String {
        let Some(q) = &self.quiz else {
            let stats: quiz_stats::QuizStats = save::load(&self.data_dir.join(quiz_stats::FILE));
            return json!({ "range": stats.range }).to_string();
        };
        let instrument = instrument_from(&self.settings);
        let string_name = |s: usize| {
            instrument
                .tuning
                .strings
                .get(s)
                .map_or(String::new(), |p| self.note_label(&p.name().to_string()))
        };
        let question = &q.question;
        let prompt = match q.kind {
            quiz::QuizKind::Notes => self.tr(
                "quiz.notes.prompt",
                &[
                    ("note", self.note_label(&quiz::note_name(question.target))),
                    ("string", string_name(question.string)),
                ],
            ),
            quiz::QuizKind::Intervals => {
                let (_, fret) = q.root_position(&instrument).unwrap_or((0, 0));
                self.tr(
                    "quiz.intervals.prompt",
                    &[
                        (
                            "interval",
                            self.tr(
                                &format!("quiz.interval.{}", question.interval.unwrap_or("p5")),
                                &[],
                            ),
                        ),
                        (
                            "note",
                            self.note_label(&quiz::note_name(question.root.unwrap_or(0))),
                        ),
                        ("string", string_name(question.string)),
                        ("fret", fret.to_string()),
                    ],
                )
            }
        };
        let mut positions: Vec<serde_json::Value> = Vec::new();
        if let Some((string, fret)) = q.root_position(&instrument) {
            positions.push(json!({ "string": string, "fret": fret, "role": "root" }));
        }
        if q.solved {
            positions.extend(q.positions(&instrument).into_iter().map(
                |(string, fret)| json!({ "string": string, "fret": fret, "role": "current" }),
            ));
        }
        json!({
            "prompt": prompt,
            "right": q.right,
            "asked": q.asked,
            "solved": q.solved,
            "score": self.quiz_score(q.right, q.lost),
            "range": q.stats().range,
            "positions": positions,
            "heat": q.heat(&instrument),
        })
        .to_string()
    }
}
