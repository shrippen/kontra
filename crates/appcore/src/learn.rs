//! Lernen in the app core (LH-LRN-07 to -09): the course as JSON for QML, a lesson page by page,
//! its tones placed on the instrument, "try it" and "listen" through the ear-training path, and
//! practice through the normal exercise path.
//!
//! ```text
//! QML ──learn_json / lesson_json / glossary_json──▶ Core ──▶ Learning (session): resolved docs
//!     ──lesson_ear(id, page, block)──▶ EarQuiz::fixed + LiveEar ──▶ earState (as ear training)
//!     ──lesson_exercise(…)──▶ bundled file or generated one ──▶ openFile, changeSpeed
//! ```

use crate::{ear, exercise, plan_number, unix_now, Core, Language};
use kontra_model::Pitch;
use kontra_session::lesson::{Block, Doc, ExerciseRef, Lang, Practice, Span, Tones};
use kontra_session::{
    place, positions, Answer, LearnState, LearnStore, Learning, LiveEar, LiveSetup, QuestionRef,
};
use serde_json::{json, Value};
use std::path::PathBuf;

/// Questions per review round (LH-LRN-10): a few minutes, not a test.
const REVIEW_MAX: usize = 10;

/// Link target prefix of a glossary term in lesson HTML: `<a href="term:septime">`.
const TERM_LINK: &str = "term:";

/// Places outside the lessons with a glossary term behind their "?" (LH-LRN-09): generator
/// choices (`kind.`, `scale.`, `progression.`, `rhythm.`, `style.`), exercise goals (`goal.`),
/// technique signs (`technique.`, as `technique_key`) and figures of the evaluation (`summary.`).
const TOPIC_TERMS: &[(&str, &str)] = &[
    ("kind.scale", "tonleiter"),
    ("kind.arpeggio", "arpeggio"),
    ("kind.triads", "dreiklang"),
    ("kind.walking", "walking-bass"),
    ("kind.chromatic", "fingersatz"),
    ("kind.song", "form"),
    ("scale.major", "dur"),
    ("scale.minor", "moll"),
    ("scale.majorpenta", "pentatonik"),
    ("scale.minorpenta", "pentatonik"),
    ("progression.251", "zwei-fuenf-eins"),
    ("progression.minor251", "zwei-fuenf-eins"),
    ("progression.145", "stufe"),
    ("progression.blues", "stufe"),
    ("progression.pop", "akkordfolge"),
    ("progression.1625", "akkordfolge"),
    ("progression.minor", "moll"),
    ("rhythm.quarters", "viertel"),
    ("rhythm.eighths", "achtel"),
    ("rhythm.sixteenths", "sechzehntel"),
    ("rhythm.rests", "pause"),
    ("rhythm.shuffle", "shuffle"),
    ("rhythm.octaves", "oktave"),
    ("rhythm.ghost", "ghost-note"),
    ("rhythm.dead", "dead-note"),
    ("rhythm.rootfifth", "quinte"),
    ("rhythm.rootfifthoctave", "quinte"),
    ("rhythm.offbeat", "synkope"),
    ("rhythm.tresillo", "synkope"),
    ("rhythm.triplets", "triole"),
    ("rhythm.slap", "slap"),
    ("rhythm.34", "taktart"),
    ("rhythm.68", "taktart"),
    ("rhythm.54", "taktart"),
    ("rhythm.78", "taktart"),
    ("style.shuffle", "shuffle"),
    ("goal.pulse", "zaehlzeit"),
    ("goal.rests", "pause"),
    ("goal.syncopation", "synkope"),
    ("goal.chordtones", "akkord"),
    ("goal.sixteenths", "sechzehntel"),
    ("goal.triplets", "triole"),
    ("goal.meter", "taktart"),
    ("goal.slap", "slap"),
    ("goal.scale", "tonleiter"),
    ("goal.modes", "tonleiter"),
    ("goal.reading", "tonart"),
    ("goal.ramp", "bpm"),
    ("goal.walking", "walking-bass"),
    ("goal.fingers", "fingersatz"),
    ("goal.timing", "streuung"),
    ("goal.form", "form"),
    ("technique.slide", "slide"),
    ("technique.hammer", "hammer-on"),
    ("technique.pull", "pull-off"),
    ("technique.slap", "slap"),
    ("technique.pop", "slap"),
    ("technique.ghost", "ghost-note"),
    ("technique.dead", "dead-note"),
    ("lane.chord", "akkordsymbol"),
    ("summary.pocket", "pocket"),
    ("summary.spread", "streuung"),
];

impl Core {
    /// The bundled lessons and added courses seen through the current instrument profile;
    /// `None` only if the embedded files are broken (a test guards that).
    fn learning(&self) -> Option<Learning> {
        let chosen = self.settings.course.as_deref().unwrap_or_default();
        Learning::with_courses(&self.instrument(), &self.courses, chosen)
    }

    pub(crate) fn learn_lang(&self) -> Lang {
        match self.catalog.language() {
            Language::German => Lang::De,
            Language::English => Lang::En,
        }
    }

    fn learn_store(&self) -> LearnStore {
        LearnStore::new(&self.data_dir)
    }

    fn learn_state(&self) -> LearnState {
        self.learn_store().load().unwrap_or_default()
    }

    /// The course: `{"fits", "next": {id, title} | null, "chapters": [{id, title, done, total,
    /// questions, units: [{id, title, done, fits}], finale: practice | null}], "course": id,
    /// "courses": [{id, title, removable, address}]}`. `fits` false: the course is not for this instrument
    /// (LH-LRN-12); `courses` are all for it, to choose from (LH-LRN-14).
    pub fn learn_json(&self) -> String {
        let (Some(learning), lang) = (self.learning(), self.learn_lang()) else {
            return json!({ "fits": false, "chapters": [] }).to_string();
        };
        let Some(course) = learning.course() else {
            return json!({ "fits": false, "chapters": [] }).to_string();
        };
        let state = self.learn_state();
        let mut next = Value::Null;

        let chapters: Vec<Value> = course
            .chapters
            .iter()
            .map(|chapter| {
                let units: Vec<Value> = chapter
                    .units
                    .iter()
                    .filter_map(|id| learning.unit(id))
                    .map(|unit| {
                        let fits = learning.fits(&unit.scope);
                        let done = state.done(&learning.state_key(unit));
                        let title = learning
                            .doc(&unit.texts, lang)
                            .map(|d| d.title)
                            .unwrap_or_default();
                        if next.is_null() && fits && !done {
                            next = json!({ "id": unit.id, "title": title });
                        }
                        json!({ "id": unit.id, "title": title, "done": done, "fits": fits })
                    })
                    .collect();
                let done = units.iter().filter(|u| u["done"] == true).count();
                json!({
                    "id": chapter.id,
                    "questions": self.chapter_questions(&chapter.id).len(),
                    "title": chapter.titles.get(&lang).cloned().unwrap_or_default(),
                    "done": done,
                    "total": units.len(),
                    "units": units,
                    "finale": chapter.finale.as_ref().map(|p| self.practice_json(p)),
                })
            })
            .collect();
        let courses: Vec<Value> = learning
            .courses()
            .map(|c| {
                json!({ "id": c.id, "title": self.course_title(&c.id),
                        "removable": self.is_added(&c.id),
                        "address": self.course_address(&c.id) })
            })
            .collect();
        json!({ "fits": true, "next": next, "chapters": chapters, "course": course.id,
                "courses": courses })
        .to_string()
    }

    /// One lesson: `{"id", "title", "fits", "seen": [page], "done", "pages": [{"title",
    /// "blocks": [block]}]}`; blocks as `block_json`. "" for an unknown id.
    pub fn lesson_json(&self, id: &str) -> String {
        let Some(learning) = self.learning() else {
            return String::new();
        };
        let Some(unit) = learning.unit(id) else {
            return String::new();
        };
        let Some(doc) = learning.doc(&unit.texts, self.learn_lang()) else {
            return String::new();
        };
        let state = self.learn_state();
        let key = learning.state_key(unit);
        let seen: Vec<usize> = state
            .units
            .get(&key)
            .map(|u| u.pages.iter().copied().collect())
            .unwrap_or_default();
        json!({
            "id": unit.id,
            "title": doc.title,
            "fits": learning.fits(&unit.scope),
            "seen": seen,
            "done": state.done(&key),
            "pages": self.with_cues(id, self.pages_json(&doc)),
        })
        .to_string()
    }

    /// One glossary term: `{"id", "title", "blocks": [block], "lesson": {id, title} | null}`;
    /// "" for an unknown id. Several courses explaining the term give their texts one after
    /// the other, bundled first; the first block of each carries `from` ("Aus: Bass-Kurs").
    pub fn glossary_json(&self, id: &str) -> String {
        let Some(learning) = self.learning() else {
            return String::new();
        };
        let docs = self.term_docs(&learning, id);
        let Some((_, first)) = docs.first() else {
            return String::new();
        };
        let lang = self.learn_lang();
        let lesson = learning
            .term_texts(id)
            .into_iter()
            .find_map(|(t, _)| t.lesson.as_deref())
            .and_then(|l| learning.unit(l))
            .filter(|u| learning.fits(&u.scope))
            .and_then(|u| {
                let title = learning.doc(&u.texts, lang)?.title;
                Some(json!({ "id": u.id, "title": title }))
            });

        let mut blocks: Vec<Value> = Vec::new();
        for (from, doc) in &docs {
            let start = blocks.len();
            blocks.extend(
                doc.pages
                    .iter()
                    .flat_map(|p| p.blocks.iter().map(|b| self.block_json(b))),
            );
            if let (Some(block), false) = (blocks.get_mut(start), from.is_empty()) {
                block["from"] = json!(from);
            }
        }
        json!({ "id": id, "title": first.title, "blocks": blocks, "lesson": lesson }).to_string()
    }

    /// The texts of term `id` with their origin, empty when only one course explains it.
    fn term_docs(&self, learning: &Learning, id: &str) -> Vec<(String, Doc)> {
        let lang = self.learn_lang();
        let texts = learning.term_texts(id);
        let several = texts.len() > 1;
        texts
            .into_iter()
            .filter_map(|(term, source)| {
                let doc = learning.doc(&term.texts, lang)?;
                let from = match several {
                    true => self.tr("glossary.from", &[("course", self.source_title(source))]),
                    false => String::new(),
                };
                Some((from, doc))
            })
            .collect()
    }

    /// The term behind a place's "?" (`TOPIC_TERMS`): `{"id", "title", "text"}`, the text its
    /// first paragraph for the tooltip; "" without a term or for a term of another instrument.
    pub fn term_json(&self, topic: &str) -> String {
        let Some((_, id)) = TOPIC_TERMS.iter().find(|(t, _)| *t == topic) else {
            return String::new();
        };
        let Some(learning) = self.learning() else {
            return String::new();
        };
        let Some(term) = learning.term(id).filter(|t| learning.fits(&t.scope)) else {
            return String::new();
        };
        let Some(doc) = learning.doc(&term.texts, self.learn_lang()) else {
            return String::new();
        };

        let text = doc
            .pages
            .iter()
            .flat_map(|p| p.blocks.iter())
            .find_map(|b| match b {
                Block::Text(spans) => Some(plain(spans)),
                _ => None,
            })
            .unwrap_or_default();
        json!({ "id": term.id, "title": doc.title, "text": text }).to_string()
    }

    /// All terms for this instrument, by name: `[{"id", "title"}]`.
    pub fn glossary_list_json(&self) -> String {
        let Some(learning) = self.learning() else {
            return "[]".to_string();
        };
        let lang = self.learn_lang();
        let mut terms: Vec<(String, String)> = learning
            .terms()
            .iter()
            .filter(|t| learning.fits(&t.scope))
            .filter_map(|t| Some((learning.doc(&t.texts, lang)?.title, t.id.clone())))
            .collect();
        // A term several courses explain is one entry, under its first (bundled) name.
        let mut seen = std::collections::BTreeSet::new();
        terms.retain(|(_, id)| seen.insert(id.clone()));
        terms.sort_by_key(|(title, _)| title.to_lowercase());
        let list: Vec<Value> = terms
            .into_iter()
            .map(|(title, id)| json!({ "id": id, "title": title }))
            .collect();
        json!(list).to_string()
    }

    /// The next unit to learn, for the start page: `{id, title}` or null.
    pub(crate) fn next_lesson(&self) -> Value {
        let next = self
            .learn_json()
            .parse::<Value>()
            .ok()
            .map(|v| v["next"].clone());
        next.unwrap_or(Value::Null)
    }

    /// Page `page` of lesson `id` was shown; the last page finishes the lesson.
    pub fn lesson_seen(&mut self, id: &str, page: usize) {
        let Some(learning) = self.learning() else {
            return;
        };
        let Some(unit) = learning.unit(id) else {
            return;
        };
        let pages = learning
            .doc(&unit.texts, self.learn_lang())
            .map_or(0, |d| d.pages.len());
        let store = self.learn_store();
        // A damaged file stays as it is rather than being overwritten with less.
        let Ok(mut state) = store.load() else {
            return;
        };
        state.see(&learning.state_key(unit), page, pages, unix_now());
        if let Err(e) = store.save(&state) {
            eprintln!("kontra: {e}");
        }
    }

    /// The learner picked option `option` of question `block` on page `page`; true if right. The
    /// question comes back after days, sooner when it was wrong (LH-LRN-10).
    pub fn lesson_answer(&mut self, id: &str, page: usize, block: usize, option: usize) -> bool {
        self.answer_at(id, page, block, option, unix_now())
    }

    fn answer_at(&mut self, id: &str, page: usize, block: usize, option: usize, now: u64) -> bool {
        let Some(Block::Question(question)) = self.lesson_block(id, page, block) else {
            return false;
        };
        let right = question.answer == option;
        let Some(learning) = self.learning() else {
            return right;
        };
        let Some(unit) = learning.unit(id) else {
            return right;
        };
        let store = self.learn_store();
        // A damaged file stays as it is rather than being overwritten with less.
        let Ok(mut state) = store.load() else {
            return right;
        };
        let answer = if right { Answer::Right } else { Answer::Wrong };
        let at = QuestionRef::new(&learning.state_key(unit), page, block);
        state.answer(&at, answer, now);
        if let Err(e) = store.save(&state) {
            eprintln!("kontra: {e}");
        }
        right
    }

    /// Questions due again, at most `REVIEW_MAX`: `{"questions": [{"lesson": {id, title},
    /// "page", "block", "question": block}]}`. Questions of lessons that no longer fit the
    /// instrument or no longer exist wait.
    pub fn review_json(&self) -> String {
        self.review_json_at(unix_now())
    }

    fn review_json_at(&self, now: u64) -> String {
        let questions: Vec<Value> = self
            .due_questions(now)
            .into_iter()
            .take(REVIEW_MAX)
            .collect();
        json!({ "questions": questions }).to_string()
    }

    /// How many questions are due, for the start page.
    pub(crate) fn review_count(&self, now: u64) -> usize {
        self.due_questions(now).len()
    }

    fn due_questions(&self, now: u64) -> Vec<Value> {
        let Some(learning) = self.learning() else {
            return Vec::new();
        };
        let lang = self.learn_lang();
        self.learn_state()
            .due(now)
            .into_iter()
            .filter_map(|q| {
                // "greifen@e-bass": the key holds the unit's id and, if scoped, the instrument it
                // was answered on; another instrument's answers wait for it.
                let id = q.unit.split('@').next()?;
                let unit = learning
                    .unit(id)
                    .filter(|u| learning.fits(&u.scope) && learning.state_key(u) == q.unit)?;
                let doc = learning.doc(&unit.texts, lang)?;
                let block = doc.pages.get(q.page)?.blocks.get(q.block)?;
                let Block::Question(_) = block else {
                    return None;
                };
                Some(json!({
                    "lesson": { "id": unit.id, "title": doc.title },
                    "page": q.page,
                    "block": q.block,
                    "question": self.block_json(block),
                }))
            })
            .collect()
    }

    /// "Kann ich schon" (LH-LRN-10): every question of chapter `chapter` in lessons for this
    /// instrument, as `review_json`, the chapter's `title` and `since`, the time to hand back to
    /// `chapter_pass`.
    pub fn chapter_check_json(&self, chapter: &str) -> String {
        let questions: Vec<Value> = self
            .chapter_questions(chapter)
            .into_iter()
            .map(|(_, value)| value)
            .collect();
        let title = self
            .learning()
            .and_then(|l| {
                let course = l.course()?;
                let chapter = course.chapters.iter().find(|ch| ch.id == chapter)?;
                chapter.titles.get(&self.learn_lang()).cloned()
            })
            .unwrap_or_default();
        json!({ "title": title, "questions": questions, "since": unix_now() }).to_string()
    }

    /// Ticks off chapter `chapter` (each of its lessons for this instrument done) if every
    /// question was last answered right at `since` or later; true if so.
    pub fn chapter_pass(&mut self, chapter: &str, since: u64) -> bool {
        let questions = self.chapter_questions(chapter);
        let store = self.learn_store();
        let Ok(mut state) = store.load() else {
            return false;
        };
        let right = questions.iter().all(|(q, _)| state.right_since(q, since));
        if questions.is_empty() || !right {
            return false;
        }

        let Some(learning) = self.learning() else {
            return false;
        };
        let Some(units) = learning
            .course()
            .and_then(|c| c.chapters.iter().find(|ch| ch.id == chapter))
            .map(|ch| ch.units.clone())
        else {
            return false;
        };
        let now = unix_now();
        for unit in units.iter().filter_map(|id| learning.unit(id)) {
            if learning.fits(&unit.scope) {
                state.pass(&learning.state_key(unit), now);
            }
        }
        if let Err(e) = store.save(&state) {
            eprintln!("kontra: {e}");
            return false;
        }
        true
    }

    /// The questions of a chapter's lessons for this instrument, where they are and as
    /// `review_json` lists them.
    fn chapter_questions(&self, chapter: &str) -> Vec<(QuestionRef, Value)> {
        let Some(learning) = self.learning() else {
            return Vec::new();
        };
        let Some(chapter) = learning
            .course()
            .and_then(|c| c.chapters.iter().find(|ch| ch.id == chapter))
        else {
            return Vec::new();
        };
        let lang = self.learn_lang();
        let mut questions = Vec::new();
        for unit in chapter.units.iter().filter_map(|id| learning.unit(id)) {
            if !learning.fits(&unit.scope) {
                continue;
            }
            let Some(doc) = learning.doc(&unit.texts, lang) else {
                continue;
            };
            let key = learning.state_key(unit);

            for (p, page) in doc.pages.iter().enumerate() {
                for (b, block) in page.blocks.iter().enumerate() {
                    let Block::Question(_) = block else {
                        continue;
                    };
                    questions.push((
                        QuestionRef::new(&key, p, b),
                        json!({
                            "lesson": { "id": unit.id, "title": doc.title },
                            "page": p,
                            "block": b,
                            "question": self.block_json(block),
                        }),
                    ));
                }
            }
        }
        questions
    }

    /// "Listen" plays block `block` of the page; "try" waits for the player to play it. Both
    /// go through ear training, so `earState` tells what was heard (LH-LRN-08).
    pub fn lesson_ear(&mut self, id: &str, page: usize, block: usize) -> Result<String, String> {
        let block = self.lesson_block(id, page, block);
        self.block_ear(block)
    }

    /// "Listen" or "try" in glossary entry `id`, block `block` (counted over its pages).
    pub fn term_ear(&mut self, id: &str, block: usize) -> Result<String, String> {
        // Counted over every course's text, as `glossary_json` lists them.
        let block = self.learning().and_then(|learning| {
            self.term_docs(&learning, id)
                .into_iter()
                .flat_map(|(_, doc)| doc.pages.into_iter().flat_map(|p| p.blocks))
                .nth(block)
        });
        self.block_ear(block)
    }

    fn block_ear(&mut self, block: Option<Block>) -> Result<String, String> {
        let (tones, matching, play) = match block {
            Some(Block::Listen(tones)) => (tones, ear::Match::Exact, true),
            Some(Block::Try(task)) if task.any_octave => (task.tones, ear::Match::AnyOctave, false),
            Some(Block::Try(task)) => (task.tones, ear::Match::Exact, false),
            _ => return Err(String::new()),
        };
        let pitches = place(&tones, &self.instrument());
        let before = self.ear.take();
        let started = self.start_lesson_ear(pitches, matching);

        // Without an input "try it" still runs: the learner answers by clicking (LH-LRN-08).
        // "Listen" needs the output, so its error stands, and a round of clicks goes on.
        if let Err(error) = started {
            if play {
                self.ear = before;
                return Err(error);
            }
            return Ok(self.ear_json());
        }
        if play {
            return Ok(self.ear_play());
        }
        Ok(self.ear_json())
    }

    /// The exercise of a practice block, as a file to open (bundled or generated now).
    pub fn lesson_exercise(
        &mut self,
        id: &str,
        page: usize,
        block: usize,
    ) -> Result<PathBuf, String> {
        match self.lesson_block(id, page, block) {
            Some(Block::Practice(practice)) => self.practice_path(&practice),
            _ => Err(String::new()),
        }
    }

    /// The exercise that rounds off chapter `chapter`, as a file to open.
    pub fn finale_exercise(&mut self, chapter: &str) -> Result<PathBuf, String> {
        let finale = self
            .learning()
            .and_then(|l| l.course().cloned())
            .and_then(|c| c.chapters.into_iter().find(|ch| ch.id == chapter))
            .and_then(|ch| ch.finale);
        match finale {
            Some(practice) => self.practice_path(&practice),
            None => Err(String::new()),
        }
    }

    /// The speeds (percent) of a practice block or a chapter's finale, for the UI's buttons.
    fn practice_json(&self, practice: &Practice) -> Value {
        let title = match &practice.exercise {
            ExerciseRef::Bundled(n) => self.bundled_file(*n).and_then(|p| {
                let stem = p.file_stem()?.to_string_lossy().into_owned();
                self.bundled_title(&stem)
            }),
            ExerciseRef::Generated { .. } => self
                .generated_spec(&practice.exercise)
                .map(|spec| self.exercise_title(&exercise::describe(&spec))),
        };
        json!({ "kind": "practice", "title": title.unwrap_or_default(), "speeds": practice.speeds })
    }

    pub(crate) fn lesson_block(&self, id: &str, page: usize, block: usize) -> Option<Block> {
        let learning = self.learning()?;
        let unit = learning.unit(id)?;
        let doc = learning.doc(&unit.texts, self.learn_lang())?;
        doc.pages.get(page)?.blocks.get(block).cloned()
    }

    /// Starts the ear-training audio (if not running) with one fixed question.
    fn start_lesson_ear(
        &mut self,
        pitches: Vec<Pitch>,
        matching: ear::Match,
    ) -> Result<(), String> {
        self.ear = Some(ear::EarQuiz::fixed(pitches, matching));
        if self.live_ear.is_none() {
            self.stop_live();
            self.tuner = None;
            self.level.reset();
            let setup = LiveSetup {
                input: self.input_choice(),
                output_device: self.output_device(),
                source: self.source(),
                backing: None,
                original: None,
            };
            let mut live_ear = LiveEar::start(&setup, &self.instrument())
                .map_err(|e| self.tr("error.audio", &[("reason", e.to_string())]))?;
            live_ear.set_gains(crate::ear_gains(&self.settings));
            self.live_ear = Some(live_ear);
        }
        Ok(())
    }

    fn practice_path(&mut self, practice: &Practice) -> Result<PathBuf, String> {
        match &practice.exercise {
            ExerciseRef::Bundled(n) => self
                .bundled_file(*n)
                .ok_or_else(|| self.tr("learn.missing", &[])),
            ExerciseRef::Generated { .. } => {
                let spec = self
                    .generated_spec(&practice.exercise)
                    .ok_or_else(|| self.tr("learn.missing", &[]))?;
                self.generate_exercise(&spec)
            }
        }
    }

    /// Bundled exercise number `n` ("NN …" in a level folder).
    fn bundled_file(&self, n: u16) -> Option<PathBuf> {
        let levels = std::fs::read_dir(Self::exercises_dir()).ok()?;
        levels
            .flatten()
            .filter_map(|level| std::fs::read_dir(level.path()).ok())
            .flat_map(|files| files.flatten().map(|f| f.path()))
            .find(|f| plan_number(f).0 == usize::from(n))
    }

    fn generated_spec(&self, exercise: &ExerciseRef) -> Option<exercise::ExerciseSpec> {
        let ExerciseRef::Generated {
            kind,
            variant,
            rhythm,
            key,
            bpm,
            judging,
        } = exercise
        else {
            return None;
        };
        Core::exercise_spec(kind, variant, rhythm, *key, f64::from(*bpm), judging)
    }

    fn pages_json(&self, doc: &Doc) -> Vec<Value> {
        doc.pages
            .iter()
            .map(|p| {
                let blocks: Vec<Value> = p.blocks.iter().map(|b| self.block_json(b)).collect();
                json!({ "title": p.title, "blocks": blocks })
            })
            .collect()
    }

    /// One block for QML, by `kind`: text `{html}`, hint `{text}`, list `{items: [html]}`, figure/listen/
    /// try `{notes, positions: [{string, fret, label}]}`, practice `{title, speeds}`, question
    /// `{html, options: [text], answer}`, tuner, video `{src, subtitles}` (`lesson_json` adds
    /// the cues).
    fn block_json(&self, block: &Block) -> Value {
        match block {
            Block::Text(spans) => json!({ "kind": "text", "html": html(spans) }),
            Block::Hint(spans) => json!({ "kind": "hint", "text": plain(spans) }),
            Block::List(items) => {
                let items: Vec<String> = items.iter().map(|i| html(i)).collect();
                json!({ "kind": "list", "items": items })
            }
            Block::Figure(tones) => self.tones_json("figure", tones),
            Block::Listen(tones) => self.tones_json("listen", tones),
            Block::Try(task) => {
                let mut value = self.tones_json("try", &task.tones);
                value["anyOctave"] = json!(task.any_octave);
                value
            }
            Block::Practice(practice) => self.practice_json(practice),
            Block::Question(q) => {
                let options: Vec<String> = q.options.iter().map(|o| plain(o)).collect();
                json!({ "kind": "question", "html": html(&q.prompt), "options": options,
                        "answer": q.answer })
            }
            Block::Tuner => json!({ "kind": "tuner" }),
            Block::Video { src, subtitles } => {
                json!({ "kind": "video", "src": src, "subtitles": subtitles })
            }
            // Resolved away by `Learning::doc`.
            Block::Variant { .. } => json!({ "kind": "none" }),
        }
    }

    /// Tones as note names and, on string instruments, where they lie.
    fn tones_json(&self, kind: &str, tones: &Tones) -> Value {
        let instrument = self.instrument();
        let notes: Vec<String> = place(tones, &instrument)
            .iter()
            .map(|p| self.note_label(&p.to_string()))
            .collect();
        let at: Vec<Value> = positions(tones, &instrument)
            .into_iter()
            .map(|(step, string, fret)| {
                let label = tones.labels.get(step).cloned().unwrap_or_default();
                json!({ "string": string, "fret": fret, "label": label })
            })
            .collect();
        json!({ "kind": kind, "notes": notes, "positions": at })
    }
}

/// Spans as Qt rich text: bold, italics, terms as links.
fn html(spans: &[Span]) -> String {
    let mut out = String::new();
    for span in spans {
        match span {
            Span::Text(t) => out.push_str(&escape(t)),
            Span::Strong(t) => out.push_str(&format!("<b>{}</b>", escape(t))),
            Span::Em(t) => out.push_str(&format!("<i>{}</i>", escape(t))),
            Span::Term { id, text } => {
                out.push_str(&format!(
                    "<a href=\"{TERM_LINK}{}\">{}</a>",
                    escape(id),
                    escape(text)
                ));
            }
        }
    }
    out
}

/// Spans as plain text, for answers on buttons.
fn plain(spans: &[Span]) -> String {
    spans
        .iter()
        .map(|span| match span {
            Span::Text(t) | Span::Strong(t) | Span::Em(t) => t.as_str(),
            Span::Term { text, .. } => text.as_str(),
        })
        .collect()
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::QaOptions;

    fn core(name: &str) -> (Core, PathBuf) {
        let dir = std::env::temp_dir().join(format!("kontra-learn-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut core = Core::new(&dir, QaOptions::default());
        core.update_settings(|s| s.language = Some("de".into()));
        (core, dir)
    }

    fn parse(text: &str) -> Value {
        text.parse().unwrap()
    }

    #[test]
    fn the_course_starts_with_the_first_lesson_and_moves_on() {
        let (mut core, dir) = core("course");
        let course = parse(&core.learn_json());
        assert_eq!(course["fits"], true);
        assert_eq!(course["chapters"][0]["title"], "Loslegen");
        let first = course["next"]["id"].as_str().unwrap().to_string();
        assert_eq!(first, "bass-teile");

        let lesson = parse(&core.lesson_json(&first));
        let pages = lesson["pages"].as_array().unwrap().len();
        for page in 0..pages {
            core.lesson_seen(&first, page);
        }
        let course = parse(&core.learn_json());
        assert_eq!(course["chapters"][0]["done"], 1);
        assert_ne!(course["next"]["id"], first.as_str());
        assert_eq!(parse(&core.home_json())["lesson"], course["next"]);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_lesson_places_its_tones_on_the_bass() {
        let (core, dir) = core("tones");
        let lesson = parse(&core.lesson_json("greifen"));
        let blocks = &lesson["pages"][0]["blocks"];
        let figure = blocks
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["kind"] == "figure")
            .unwrap();
        // Frets 2 to 5 on the A string (string 1), fingers 1 to 4.
        assert_eq!(
            figure["positions"][0],
            json!({ "string": 1, "fret": 2, "label": "1" })
        );
        assert_eq!(figure["notes"][0], "H1", "German names B as H");
        let text = blocks
            .as_array()
            .unwrap()
            .iter()
            .filter(|b| b["kind"] == "text");
        assert!(
            text.clone()
                .any(|b| b["html"].as_str().unwrap().contains("href=\"term:bund\"")),
            "the fretted variant links the fret"
        );
        assert!(!text
            .clone()
            .any(|b| b["html"].as_str().unwrap().contains("Bundlinie")));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn practice_opens_the_bundled_exercise() {
        let (mut core, dir) = core("practice");
        let lesson = parse(&core.lesson_json("viertel"));
        let pages = lesson["pages"].as_array().unwrap();
        let (page, block) = pages
            .iter()
            .enumerate()
            .find_map(|(p, page)| {
                let blocks = page["blocks"].as_array()?;
                let b = blocks.iter().position(|b| b["kind"] == "practice")?;
                Some((p, b))
            })
            .unwrap();
        assert_eq!(pages[page]["blocks"][block]["speeds"], json!([70, 85, 100]));
        let path = core.lesson_exercise("viertel", page, block).unwrap();
        assert!(path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("01 "));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_glossary_names_its_lesson() {
        let (core, dir) = core("glossary");
        let term = parse(&core.glossary_json("bund"));
        assert_eq!(term["title"], "Bund");
        assert_eq!(term["lesson"]["id"], "greifen");
        let list = parse(&core.glossary_list_json());
        assert!(list
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["id"] == "viertel"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn every_topic_has_its_term() {
        let (core, dir) = core("topics");
        for (topic, id) in TOPIC_TERMS {
            let term = parse(&core.term_json(topic));
            assert_eq!(term["id"], *id, "{topic}");
            assert_ne!(term["text"], "", "{topic}");
        }
        let walking = parse(&core.term_json("kind.walking"));
        assert_eq!(walking["title"], "Walking Bass");
        assert_eq!(core.term_json("kind.groove"), "", "no term, no \"?\"");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn every_bundled_goal_has_a_term() {
        let (core, dir) = core("goals");
        for (_, spec) in exercise::bundled_specs() {
            let goal = exercise::describe(&spec).goal;
            assert_ne!(core.term_json(&format!("goal.{goal}")), "", "{goal}");
        }
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn answered_questions_come_back() {
        const DAY: u64 = 86_400;
        let (mut core, dir) = core("review");
        let lesson = parse(&core.lesson_json("septime"));
        let (page, block, answer) = lesson["pages"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .find_map(|(p, page)| {
                let blocks = page["blocks"].as_array()?;
                let b = blocks.iter().position(|b| b["kind"] == "question")?;
                Some((p, b, blocks[b]["answer"].as_u64()? as usize))
            })
            .unwrap();
        assert!(core.answer_at("septime", page, block, answer, 0));
        assert_eq!(parse(&core.review_json_at(DAY))["questions"], json!([]));

        let due = parse(&core.review_json_at(3 * DAY));
        let first = &due["questions"][0];
        assert_eq!(first["lesson"]["id"], "septime");
        assert_eq!(
            (first["page"].as_u64(), first["block"].as_u64()),
            (Some(page as u64), Some(block as u64))
        );
        assert_eq!(first["question"]["kind"], "question");
        assert_eq!(parse(&core.home_json_at(3 * DAY))["review"], 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_chapter_is_ticked_off_by_its_questions() {
        let (mut core, dir) = core("check");
        let course = parse(&core.learn_json());
        let chapter = course["chapters"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["questions"].as_u64() > Some(1))
            .unwrap()
            .clone();
        let id = chapter["id"].as_str().unwrap();
        let check = parse(&core.chapter_check_json(id));
        let since = check["since"].as_u64().unwrap();
        assert_eq!(check["title"], chapter["title"]);
        let questions = check["questions"].as_array().unwrap();
        assert_eq!(
            questions.len() as u64,
            chapter["questions"].as_u64().unwrap()
        );

        // One wrong: the chapter stays open.
        let answer = |core: &mut Core, q: &Value, right: bool| {
            let correct = q["question"]["answer"].as_u64().unwrap() as usize;
            let option = if right { correct } else { (correct + 1) % 2 };
            let lesson = q["lesson"]["id"].as_str().unwrap();
            let (page, block) = (q["page"].as_u64().unwrap(), q["block"].as_u64().unwrap());
            core.lesson_answer(lesson, page as usize, block as usize, option)
        };
        for (i, q) in questions.iter().enumerate() {
            assert_eq!(answer(&mut core, q, i > 0), i > 0);
        }
        assert!(!core.chapter_pass(id, since));

        // All right: each lesson of the chapter is done.
        for q in questions {
            assert!(answer(&mut core, q, true));
        }
        assert!(core.chapter_pass(id, since));
        let course = parse(&core.learn_json());
        let chapter = course["chapters"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == id)
            .unwrap();
        assert_eq!(chapter["done"], chapter["total"]);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn markup_is_escaped() {
        let spans = [
            Span::Text("a < b & ".into()),
            Span::Term {
                id: "x".into(),
                text: "\"y\"".into(),
            },
        ];
        assert_eq!(
            html(&spans),
            "a &lt; b &amp; <a href=\"term:x\">&quot;y&quot;</a>"
        );
    }

    #[test]
    fn answers_are_plain_text() {
        // Answers sit on buttons, which show text as it is: no markup, no escapes.
        let spans = [
            Span::Strong("a & b".into()),
            Span::Text(" or ".into()),
            Span::Term {
                id: "x".into(),
                text: "c".into(),
            },
        ];
        assert_eq!(plain(&spans), "a & b or c");
    }
}
