//! Lessons (LH-LRN-07 to -13, `planung/LERNEN.md`): courses, lessons and the glossary, written as
//! a small Markdown subset with blocks for what Kontra shows, plays and hears.
//!
//! ```text
//! lessons/courses/<id>.json ──┐
//! lessons/units/<id>/de.md ───┼─parse─▶ Lessons { courses, units, terms } ──check()─▶ problems
//! lessons/glossary/de.md ─────┘
//! ```
//!
//! A course is only an order (chapters of unit ids); units live in one pool shared by every
//! course. `scope` says for which instruments a unit, term or variant is true (`any`, a family
//! like `fretted`, an instrument like `bass`); resolving it against a profile is the session's job.

mod check;
mod pack;
mod parse;

pub use pack::{
    new_course, pack, Author, CourseManifest, CoursePack, PackError, COURSE_EXT, COURSE_FORMAT,
    COURSE_VERSION,
};
pub use parse::note_class;

use kontra_model::InstrumentKind;
use std::collections::BTreeMap;
use thiserror::Error;

// The bundled files, embedded by build.rs: (path below `lessons/`, text).
include!(concat!(env!("OUT_DIR"), "/lessons.rs"));

const COURSES_DIR: &str = "courses/";
const UNITS_DIR: &str = "units/";
const GLOSSARY_DIR: &str = "glossary/";
const JSON: &str = ".json";

/// Scopes of each instrument profile, most specific first (`lessons/README.md`).
const FRETTED: &[&str] = &["e-bass", "fretted", "strings", "bass", "any"];
const FRETLESS: &[&str] = &["e-bass", "fretless", "strings", "bass", "any"];
const DOUBLE: &[&str] = &["double-bass", "strings", "bass", "any"];
const KEYS: &[&str] = &["keys", "bass", "any"];
const PROFILES: [&[&str]; 4] = [FRETTED, FRETLESS, DOUBLE, KEYS];

/// The scopes an instrument profile belongs to, most specific first.
pub fn scopes(kind: InstrumentKind) -> &'static [&'static str] {
    match kind {
        InstrumentKind::Fretted => FRETTED,
        InstrumentKind::Fretless => FRETLESS,
        InstrumentKind::Double => DOUBLE,
        InstrumentKind::Keys => KEYS,
    }
}

/// Whether something for `wide` holds wherever something for `narrow` does: every profile with
/// `narrow` also has `wide` (`strings` covers `fretted`, not the other way round).
fn covers(wide: &str, narrow: &str) -> bool {
    PROFILES
        .iter()
        .filter(|p| p.contains(&narrow))
        .all(|p| p.contains(&wide))
}

/// Whether a profile knows `scope` at all.
fn known(scope: &str) -> bool {
    PROFILES.iter().any(|p| p.contains(&scope))
}

/// A lesson file that cannot be read, with the place it went wrong.
#[derive(Debug, Error, PartialEq)]
#[error("{file}:{line}: {reason}")]
pub struct LessonError {
    pub file: String,
    pub line: usize,
    pub reason: String,
}

/// Languages lessons are written in; each unit and term has one text per language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Lang {
    De,
    En,
}

impl Lang {
    pub const ALL: [Lang; 2] = [Lang::De, Lang::En];

    /// File stem and language code, e.g. "de".
    pub fn key(self) -> &'static str {
        match self {
            Lang::De => "de",
            Lang::En => "en",
        }
    }

    pub fn from_key(key: &str) -> Option<Lang> {
        Lang::ALL.into_iter().find(|l| l.key() == key)
    }
}

/// Inline text: plain, emphasised, or a glossary term ("[[septime|Septimen]]").
#[derive(Debug, Clone, PartialEq)]
pub enum Span {
    Text(String),
    Strong(String),
    Em(String),
    Term { id: String, text: String },
}

/// Tones relative to a root: `root` a pitch class (C = 0), `steps` semitones above it. The
/// instrument profile places the root lowest in its range ("A, 0 10" is A1 → G2 on a bass).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Tones {
    pub root: u8,
    pub steps: Vec<i32>,
    /// One label per step ("1", "b7"), or none.
    pub labels: Vec<String>,
    /// On string instruments: the open string (pitch class) to play them on, if it matters.
    pub string: Option<u8>,
}

/// Something to play on the instrument, checked by ear: the tones in order, or any octave of
/// them with `any_octave` ("play an E").
#[derive(Debug, Clone, PartialEq)]
pub struct Task {
    pub tones: Tones,
    pub any_octave: bool,
}

/// An exercise to practise: one of the bundled plan (its number) or a generator choice.
#[derive(Debug, Clone, PartialEq)]
pub enum ExerciseRef {
    /// Number NN of "NN …" in `testdata/exercises` (stable, never renumbered).
    Bundled(u16),
    /// The generator's choice, as `Core::exercise_spec` takes it.
    Generated {
        kind: String,
        variant: String,
        rhythm: String,
        key: u8,
        bpm: u16,
        judging: String,
    },
}

/// A practice step: the exercise and the speeds in percent to play it at (slow, medium, full).
#[derive(Debug, Clone, PartialEq)]
pub struct Practice {
    pub exercise: ExerciseRef,
    pub speeds: Vec<u8>,
}

/// A multiple-choice question; `answer` is the index of the right option.
#[derive(Debug, Clone, PartialEq)]
pub struct Question {
    pub prompt: Vec<Span>,
    pub options: Vec<Vec<Span>>,
    pub answer: usize,
}

/// One block of a page.
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Text(Vec<Span>),
    List(Vec<Vec<Span>>),
    Hint(Vec<Span>),
    /// The tones drawn on the instrument (fretboard, keyboard: the profile decides).
    Figure(Tones),
    /// The tones played to listen to.
    Listen(Tones),
    Try(Task),
    Practice(Practice),
    Question(Question),
    /// The tuner, for the lesson about tuning.
    Tuner,
    /// A video: a path inside a course package or an https address. Only loaded courses have
    /// videos (LH-LRN-14); bundled ones never.
    Video {
        src: String,
        /// Subtitles (WebVTT) in this file's language, like `src` a path or an address.
        subtitles: Option<String>,
    },
    /// Blocks only for instruments in `scope`.
    Variant {
        scope: String,
        blocks: Vec<Block>,
    },
}

/// A page of a lesson: one step on screen.
#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    pub title: String,
    pub blocks: Vec<Block>,
}

/// A unit's or term's text in one language.
#[derive(Debug, Clone, PartialEq)]
pub struct Doc {
    pub title: String,
    pub pages: Vec<Page>,
}

/// A lesson in the shared pool.
#[derive(Debug, Clone, PartialEq)]
pub struct Unit {
    pub id: String,
    pub scope: String,
    /// Units to learn first (ids, not chapters: every course orders differently).
    pub needs: Vec<String>,
    /// Glossary terms this unit teaches.
    pub terms: Vec<String>,
    pub texts: BTreeMap<Lang, Doc>,
}

/// A glossary entry; its text is the first page of `texts` (the title is the term's name).
#[derive(Debug, Clone, PartialEq)]
pub struct Term {
    pub id: String,
    pub scope: String,
    /// The unit that teaches it, if any.
    pub lesson: Option<String>,
    pub texts: BTreeMap<Lang, Doc>,
}

/// A chapter of a course: its units in order and the piece that rounds it off.
#[derive(Debug, Clone, PartialEq)]
pub struct Chapter {
    pub id: String,
    pub titles: BTreeMap<Lang, String>,
    pub units: Vec<String>,
    pub finale: Option<Practice>,
}

/// A course for one instrument: only an order of units.
#[derive(Debug, Clone, PartialEq)]
pub struct Course {
    pub id: String,
    pub scope: String,
    pub chapters: Vec<Chapter>,
}

/// Everything read from a lesson folder.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Lessons {
    pub courses: Vec<Course>,
    pub units: Vec<Unit>,
    pub terms: Vec<Term>,
}

impl Lessons {
    /// The lessons that come with Kontra.
    pub fn bundled() -> Result<Lessons, LessonError> {
        Lessons::parse(FILES)
    }

    /// Reads `(path, text)` pairs laid out like `lessons/`: `courses/<id>.json`,
    /// `units/<id>/<lang>.md`, `glossary/<lang>.md`. Other paths are ignored.
    pub fn parse(files: &[(&str, &str)]) -> Result<Lessons, LessonError> {
        let mut lessons = Lessons::default();
        let mut units: BTreeMap<String, Unit> = BTreeMap::new();
        let mut terms: BTreeMap<String, Term> = BTreeMap::new();

        for &(path, text) in files {
            // Courses: plain JSON.
            if let Some(name) = path.strip_prefix(COURSES_DIR) {
                if name.ends_with(JSON) {
                    lessons.courses.push(parse::course(path, text)?);
                }
                continue;
            }

            // Units: one Markdown file per language; the folder is the id.
            if let Some(rest) = path.strip_prefix(UNITS_DIR) {
                let Some((folder, lang)) = lang_file(rest) else {
                    continue;
                };
                let (meta, doc) = parse::unit(path, text)?;
                if meta.id != folder {
                    return Err(parse::error(
                        path,
                        1,
                        &format!("id {} in folder {folder}", meta.id),
                    ));
                }
                let unit = units.entry(meta.id.clone()).or_insert_with(|| Unit {
                    id: meta.id.clone(),
                    scope: meta.scope.clone(),
                    needs: meta.needs.clone(),
                    terms: meta.terms.clone(),
                    texts: BTreeMap::new(),
                });
                if unit.scope != meta.scope || unit.needs != meta.needs || unit.terms != meta.terms
                {
                    return Err(parse::error(
                        path,
                        meta.line,
                        "header differs between languages",
                    ));
                }
                unit.texts.insert(lang, doc);
                continue;
            }

            // Glossary: one file per language, a section per term.
            if let Some(rest) = path.strip_prefix(GLOSSARY_DIR) {
                let Some(lang) = rest.strip_suffix(".md").and_then(Lang::from_key) else {
                    continue;
                };
                for (meta, doc) in parse::glossary(path, text)? {
                    let term = terms.entry(meta.id.clone()).or_insert_with(|| Term {
                        id: meta.id.clone(),
                        scope: meta.scope.clone(),
                        lesson: meta.lesson.clone(),
                        texts: BTreeMap::new(),
                    });
                    if term.scope != meta.scope || term.lesson != meta.lesson {
                        let reason = format!("{} differs between languages", meta.id);
                        return Err(parse::error(path, meta.line, &reason));
                    }
                    if term.texts.insert(lang, doc).is_some() {
                        let reason = format!("{} twice", meta.id);
                        return Err(parse::error(path, meta.line, &reason));
                    }
                }
            }
        }

        lessons.courses.sort_by(|a, b| a.id.cmp(&b.id));
        lessons.units = units.into_values().collect();
        lessons.terms = terms.into_values().collect();
        Ok(lessons)
    }

    pub fn unit(&self, id: &str) -> Option<&Unit> {
        self.units.iter().find(|u| u.id == id)
    }

    pub fn term(&self, id: &str) -> Option<&Term> {
        self.terms.iter().find(|t| t.id == id)
    }

    /// What does not fit together (missing language, unknown ids, needs after their unit …),
    /// one sentence per problem; empty when all is well.
    pub fn check(&self) -> Vec<String> {
        check::problems(self)
    }
}

/// "septime/de.md" → ("septime", De).
fn lang_file(rest: &str) -> Option<(&str, Lang)> {
    let (folder, file) = rest.split_once('/')?;
    let lang = Lang::from_key(file.strip_suffix(".md")?)?;
    Some((folder, lang))
}
