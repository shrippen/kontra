//! Ear training in the app (LH-LRN): the `ear_*` calls the QML bridge makes on `Core`.

use crate::input_state::InputUse;
use crate::{ear, quiz_stats, save, time_seed, unix_now, Core, UiEvent};
use kontra_model::InputSource;
use kontra_session::{LiveEar, LiveSetup, PracticeEvent};
use serde_json::json;

impl Core {
    /// Starts ear training ("echo", "interval" or "chord"), plays the first prompt and returns
    /// `ear_json`.
    pub fn ear_start(&mut self, kind: &str) -> Result<String, String> {
        let kind = match kind {
            "interval" => ear::EarKind::Interval,
            "chord" => ear::EarKind::ChordRoot,
            _ => ear::EarKind::Echo,
        };
        self.stop_live();
        self.end_tuner_and_ear();
        self.level.reset();
        let instrument = self.instrument();
        let setup = LiveSetup {
            input: self.input_choice(),
            output_device: self.output_device(),
            source: self.source(),
            backing: None,
            original: None,
            backing_cache: None,
            onset_gate_db: self.settings.onset_gate_db,
        };
        self.release_standby();
        self.live_ear = Some(self.open_ear(&setup, &instrument)?);
        self.input.remember(InputUse::Ear, setup.input);
        let stats = save::load(&self.data_dir.join(quiz_stats::FILE));
        self.ear = Some(ear::EarQuiz::new(
            kind,
            &instrument,
            stats,
            time_seed(),
            unix_now(),
        ));
        self.ear_play();
        Ok(self.ear_json())
    }

    pub fn ear_stop(&mut self) {
        self.end_ear();
        self.input.stopped(InputUse::Ear);
        self.ear = None;
        self.ear_prompting = false;
    }

    /// Keeps the interval answers (LH-LRN-18) beside the fretboard quiz's in its file.
    fn save_ear_stats(&self) {
        let Some(quiz) = self
            .ear
            .as_ref()
            .filter(|q| q.kind == ear::EarKind::Interval)
        else {
            return;
        };
        let file = self.data_dir.join(quiz_stats::FILE);
        let stats = save::load::<quiz_stats::QuizStats>(&file).with_intervals_of(quiz.stats());
        let _ = save::save(&file, &stats);
    }

    /// Plays the current prompt (again).
    pub fn ear_play(&mut self) -> String {
        let (Some(quiz), Some(live_ear)) = (&self.ear, self.live_ear.as_mut()) else {
            return self.ear_json();
        };
        live_ear.play(&quiz.question.tones);
        self.ear_prompting = live_ear.is_prompting();
        self.ear_json()
    }

    /// Next question, played at once.
    pub fn ear_next(&mut self) -> String {
        let instrument = self.instrument();
        if let Some(quiz) = self.ear.as_mut() {
            quiz.next(&instrument, unix_now());
        }
        self.ear_play()
    }

    /// Judges a played note (fractional MIDI); the UI gets the new state.
    pub fn ear_answer(&mut self, midi: f64) -> String {
        if let Some(quiz) = self.ear.as_mut() {
            quiz.answer(midi, unix_now());
        }
        self.save_ear_stats();
        self.ear_json()
    }

    /// Feeds the level readings of `events` to the advisor; returns the advice when it changed.
    pub(crate) fn advise_level(&mut self, events: &[UiEvent]) -> Option<UiEvent> {
        let now = self.started_at.elapsed().as_secs_f64();
        for event in events {
            if let UiEvent::Level {
                peak_db, clipped, ..
            } = event
            {
                self.level.push(now, *peak_db as f32, *clipped);
            }
        }
        let laptop = self.source() == InputSource::Line && self.level.suggests_laptop();
        let told = (self.level.state(), laptop);
        if told == self.level_told {
            return None;
        }
        self.level_told = told;
        Some(UiEvent::LevelAdvice {
            state: told.0.key(),
            suggest_laptop: laptop,
        })
    }

    /// Takes the answers out of `raw` and judges them; returns the new state when it changed
    /// (a note was judged or the prompt has ended).
    pub(crate) fn ear_heard(&mut self, raw: &mut Vec<PracticeEvent>) -> Option<UiEvent> {
        let mut changed = false;
        let now_s = unix_now();
        raw.retain(|event| {
            let PracticeEvent::Heard { midi } = event else {
                return true;
            };
            if let Some(quiz) = self.ear.as_mut() {
                changed |= quiz.answer(*midi, now_s) != ear::Verdict::Ignored;
            }
            false
        });
        if changed {
            self.save_ear_stats();
        }
        let prompting = self.live_ear.as_ref().is_some_and(LiveEar::is_prompting);
        changed |= prompting != self.ear_prompting;
        self.ear_prompting = prompting;
        changed.then(|| UiEvent::Ear(self.ear_json()))
    }

    /// The question in words, the latest notes played and, once solved, what it was and where
    /// it lies: `{"prompt", "prompting", "solved", "progress", "total", "played": [{note,
    /// right, direction ("high", "low" or "")}], "reveal", "score", "positions": [{string, fret,
    /// role}], "silent", "listening" (false without an input: answers come by click)}`.
    pub fn ear_json(&self) -> String {
        let Some(q) = &self.ear else {
            return String::new();
        };
        let name = |p: kontra_model::Pitch| self.note_label(&p.to_string());
        let question = &q.question;
        let prompt = match q.kind {
            ear::EarKind::Echo if question.answer.len() == 1 => self.tr("ear.echo.prompt.one", &[]),
            ear::EarKind::Echo => self.tr(
                "ear.echo.prompt.many",
                &[("count", question.answer.len().to_string())],
            ),
            ear::EarKind::Interval => self.tr(
                "ear.interval.prompt",
                &[("note", question.root.map(name).unwrap_or_default())],
            ),
            ear::EarKind::ChordRoot => self.tr("ear.chord.prompt", &[]),
        };
        let reveal = match q.kind {
            _ if !q.solved => String::new(),
            ear::EarKind::Echo => {
                let notes: Vec<String> = question.answer.iter().map(|&p| name(p)).collect();
                self.tr("ear.reveal.echo", &[("notes", notes.join(" – "))])
            }
            ear::EarKind::Interval => self.tr(
                "ear.reveal.interval",
                &[
                    (
                        "interval",
                        self.tr(
                            &format!("quiz.interval.{}", question.interval.unwrap_or("p5")),
                            &[],
                        ),
                    ),
                    ("note", question.root.map(name).unwrap_or_default()),
                ],
            ),
            ear::EarKind::ChordRoot => self.tr(
                "ear.reveal.chord",
                &[
                    (
                        "note",
                        question
                            .answer
                            .first()
                            .map(|&p| name(p))
                            .unwrap_or_default(),
                    ),
                    (
                        "quality",
                        self.tr(
                            &format!("ear.chord.{}", question.chord.unwrap_or("major")),
                            &[],
                        ),
                    ),
                ],
            ),
        };
        let played: Vec<serde_json::Value> = q
            .played
            .iter()
            .map(|p| {
                let direction = match p.offset {
                    o if o > 0 => "high",
                    o if o < 0 => "low",
                    _ => "",
                };
                json!({ "note": name(p.pitch), "right": p.right, "direction": direction })
            })
            .collect();

        // Once solved, where the answer lies; any octave for a chord's root.
        let instrument = self.instrument();
        let mut positions: Vec<serde_json::Value> = Vec::new();
        // An interval's first note is named in the prompt: shown at once where it lies (QA
        // 14.10., P1 found the start only by trying).
        if let (ear::EarKind::Interval, Some(root)) = (q.kind, question.root) {
            for (string, fret) in instrument.positions(root) {
                positions.push(json!({ "string": string, "fret": fret, "role": "root" }));
            }
        }
        if q.solved {
            let wanted: Vec<kontra_model::Pitch> = match question.matching {
                ear::Match::Exact => question.answer.clone(),
                ear::Match::AnyOctave => question
                    .answer
                    .iter()
                    .flat_map(|p| (0..4).filter_map(move |o| p.transposed(12 * o - 12)))
                    .collect(),
            };
            for pitch in wanted {
                for (string, fret) in instrument.positions(pitch) {
                    positions.push(json!({ "string": string, "fret": fret, "role": "current" }));
                }
            }
        }
        json!({
            "prompt": prompt,
            "prompting": self.ear_prompting,
            "solved": q.solved,
            "progress": q.progress,
            "total": question.answer.len(),
            "played": played,
            "reveal": reveal,
            "score": self.score_text("ear.score", q.right, q.asked, q.solved),
            "positions": positions,
            "silent": self.live_ear.as_ref().is_some_and(LiveEar::output_is_silent),
            "listening": self.live_ear.is_some(),
        })
        .to_string()
    }
}
