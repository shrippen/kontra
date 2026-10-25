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
use kontra_session::{place, positions, LearnState, LearnStore, Learning, LiveEar, LiveSetup};
use serde_json::{json, Value};
use std::path::PathBuf;

/// Link target prefix of a glossary term in lesson HTML: `<a href="term:septime">`.
const TERM_LINK: &str = "term:";

impl Core {
    /// The bundled lessons seen through the current instrument profile; `None` only if the
    /// embedded files are broken (a test guards that).
    fn learning(&self) -> Option<Learning> {
        Learning::bundled(&self.instrument())
    }

    fn learn_lang(&self) -> Lang {
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
    /// units: [{id, title, done, fits}], finale: practice | null}]}`. `fits` false: the course is not for
    /// this instrument (LH-LRN-12).
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
                    "title": chapter.titles.get(&lang).cloned().unwrap_or_default(),
                    "done": done,
                    "total": units.len(),
                    "units": units,
                    "finale": chapter.finale.as_ref().map(|p| self.practice_json(p)),
                })
            })
            .collect();
        json!({ "fits": true, "next": next, "chapters": chapters }).to_string()
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
            "pages": self.pages_json(&doc),
        })
        .to_string()
    }

    /// One glossary term: `{"id", "title", "blocks": [block], "lesson": {id, title} | null}`;
    /// "" for an unknown id.
    pub fn glossary_json(&self, id: &str) -> String {
        let Some(learning) = self.learning() else {
            return String::new();
        };
        let Some(term) = learning.term(id) else {
            return String::new();
        };
        let lang = self.learn_lang();
        let Some(doc) = learning.doc(&term.texts, lang) else {
            return String::new();
        };
        let lesson = term
            .lesson
            .as_deref()
            .and_then(|l| learning.unit(l))
            .filter(|u| learning.fits(&u.scope))
            .and_then(|u| {
                let title = learning.doc(&u.texts, lang)?.title;
                Some(json!({ "id": u.id, "title": title }))
            });
        let blocks: Vec<Value> = doc
            .pages
            .iter()
            .flat_map(|p| p.blocks.iter().map(|b| self.block_json(b)))
            .collect();
        json!({ "id": term.id, "title": doc.title, "blocks": blocks, "lesson": lesson }).to_string()
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

    /// "Listen" plays block `block` of the page; "try" waits for the player to play it. Both
    /// go through ear training, so `earState` tells what was heard (LH-LRN-08).
    pub fn lesson_ear(&mut self, id: &str, page: usize, block: usize) -> Result<String, String> {
        let block = self.lesson_block(id, page, block);
        self.block_ear(block)
    }

    /// "Listen" or "try" in glossary entry `id`, block `block` (counted over its pages).
    pub fn term_ear(&mut self, id: &str, block: usize) -> Result<String, String> {
        let block = self.learning().and_then(|learning| {
            let term = learning.term(id)?;
            let doc = learning.doc(&term.texts, self.learn_lang())?;
            doc.pages.into_iter().flat_map(|p| p.blocks).nth(block)
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
        let started = self.start_lesson_ear(pitches, matching);

        // Without an input "try it" still runs: the learner answers by clicking (LH-LRN-08).
        // "Listen" needs the output, so its error stands.
        if let Err(error) = started {
            if play {
                self.ear = None;
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

    fn lesson_block(&self, id: &str, page: usize, block: usize) -> Option<Block> {
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

    /// One block for QML, by `kind`: text/hint `{html}`, list `{items: [html]}`, figure/listen/
    /// try `{notes, positions: [{string, fret, label}]}`, practice `{title, speeds}`, question
    /// `{html, options: [text], answer}`, tuner, video `{src}`.
    fn block_json(&self, block: &Block) -> Value {
        match block {
            Block::Text(spans) => json!({ "kind": "text", "html": html(spans) }),
            Block::Hint(spans) => json!({ "kind": "hint", "html": html(spans) }),
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
            Block::Video { src } => json!({ "kind": "video", "src": src }),
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
