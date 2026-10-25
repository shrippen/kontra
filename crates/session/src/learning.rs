//! Lernen (LH-LRN-07 to -13, `planung/LERNEN.md`): the course for the instrument profile, lessons
//! and glossary resolved for it, tones placed on it, and what the learner has done.
//!
//! ```text
//! Lessons (content) ──Learning::new(instrument)──▶ course(), doc(unit, lang): variants resolved
//!                                                  place(tones): pitches in the instrument's range
//! LearnStore ──load/save──▶ LearnState { units: id → pages seen, done }
//! ```

use crate::progress::ProgressError;
use kontra_content::lesson::{Block, Course, Doc, Lang, Lessons, Page, Term, Tones, Unit};
use kontra_model::{Instrument, Pitch};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

pub use kontra_content::lesson;

const ANY: &str = "any";
const STATE_FILE: &str = "learning.json";
const OCTAVE: u8 = 12;

/// Lessons seen through one instrument profile.
#[derive(Debug, Clone)]
pub struct Learning {
    lessons: Lessons,
    scopes: &'static [&'static str],
}

impl Learning {
    pub fn new(lessons: Lessons, instrument: &Instrument) -> Learning {
        Learning {
            lessons,
            scopes: lesson::scopes(instrument.kind),
        }
    }

    /// Whether something scoped `scope` is meant for this profile.
    pub fn fits(&self, scope: &str) -> bool {
        self.scopes.contains(&scope)
    }

    /// The course for this profile: the first whose scope fits.
    pub fn course(&self) -> Option<&Course> {
        self.lessons.courses.iter().find(|c| self.fits(&c.scope))
    }

    pub fn unit(&self, id: &str) -> Option<&Unit> {
        self.lessons.unit(id)
    }

    pub fn terms(&self) -> &[Term] {
        &self.lessons.terms
    }

    pub fn term(&self, id: &str) -> Option<&Term> {
        self.lessons.term(id)
    }

    /// A unit's or term's text in `lang` (or the first one there is), with only the variants for
    /// this profile left, their blocks in place of the variant.
    pub fn doc(&self, texts: &BTreeMap<Lang, Doc>, lang: Lang) -> Option<Doc> {
        let doc = texts.get(&lang).or_else(|| texts.values().next())?;
        let pages = doc
            .pages
            .iter()
            .map(|p| Page {
                title: p.title.clone(),
                blocks: self.resolve(&p.blocks),
            })
            .collect();
        Some(Doc {
            title: doc.title.clone(),
            pages,
        })
    }

    fn resolve(&self, blocks: &[Block]) -> Vec<Block> {
        let mut out = Vec::new();
        for block in blocks {
            match block {
                Block::Variant { scope, blocks } if self.fits(scope) => {
                    out.extend(self.resolve(blocks));
                }
                Block::Variant { .. } => {}
                other => out.push(other.clone()),
            }
        }
        out
    }

    /// Key of a unit's state: knowledge for every instrument (`any`) is shared across courses;
    /// what the hands learn counts per instrument (`id@e-bass`).
    pub fn state_key(&self, unit: &Unit) -> String {
        match (unit.scope.as_str(), self.scopes.first()) {
            (ANY, _) | (_, None) => unit.id.clone(),
            (_, Some(instrument)) => format!("{}@{instrument}", unit.id),
        }
    }
}

/// Pitches of `tones` on `instrument`: the root lowest in its range, the steps above it. Steps
/// outside the range are left out.
pub fn place(tones: &Tones, instrument: &Instrument) -> Vec<Pitch> {
    placed(tones, instrument)
        .into_iter()
        .map(|(_, pitch)| pitch)
        .collect()
}

/// Where to show `tones` on a string instrument: (step index, string, fret) per tone, on
/// `tones.string` if given, else the lowest fret. The index keeps each tone with its label; tones
/// that do not fit are left out; keyboards get none.
pub fn positions(tones: &Tones, instrument: &Instrument) -> Vec<(usize, u8, u8)> {
    let open = |string: u8| instrument.pitch_at(string, 0).map(|p| p.0 % OCTAVE);
    placed(tones, instrument)
        .into_iter()
        .filter_map(|(step, pitch)| {
            let all = instrument.positions(pitch);
            let at = match tones.string {
                Some(wanted) => all.into_iter().find(|&(s, _)| open(s) == Some(wanted)),
                None => all.into_iter().min_by_key(|&(_, fret)| fret),
            };
            at.map(|(string, fret)| (step, string, fret))
        })
        .collect()
}

/// Each step's index and pitch, if it lies in the instrument's range.
fn placed(tones: &Tones, instrument: &Instrument) -> Vec<(usize, Pitch)> {
    let Some((low, high)) = instrument.range() else {
        return Vec::new();
    };
    let up = (tones.root + OCTAVE - low.0 % OCTAVE) % OCTAVE;
    let root = i32::from(low.0) + i32::from(up);
    let range = i32::from(low.0)..=i32::from(high.0);
    tones
        .steps
        .iter()
        .enumerate()
        .filter(|(_, s)| range.contains(&(root + **s)))
        .filter_map(|(i, s)| u8::try_from(root + s).ok().map(|p| (i, Pitch(p))))
        .collect()
}

/// What the learner has done in one unit.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UnitState {
    /// Pages seen, by index.
    #[serde(default)]
    pub pages: BTreeSet<usize>,
    /// When the last page was reached (seconds since the Unix epoch).
    #[serde(default)]
    pub done_unix: Option<u64>,
}

/// The learner's state over all units, keyed by `Learning::state_key`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LearnState {
    #[serde(default)]
    pub units: BTreeMap<String, UnitState>,
}

impl LearnState {
    /// Marks page `page` of `pages` seen; the last one finishes the unit.
    pub fn see(&mut self, key: &str, page: usize, pages: usize, now_unix: u64) {
        let unit = self.units.entry(key.to_string()).or_default();
        unit.pages.insert(page);
        if page + 1 >= pages && unit.done_unix.is_none() {
            unit.done_unix = Some(now_unix);
        }
    }

    pub fn done(&self, key: &str) -> bool {
        self.units.get(key).is_some_and(|u| u.done_unix.is_some())
    }
}

/// `learning.json` in the data folder.
#[derive(Debug, Clone)]
pub struct LearnStore {
    path: PathBuf,
}

impl LearnStore {
    pub fn new(dir: &Path) -> LearnStore {
        LearnStore {
            path: dir.join(STATE_FILE),
        }
    }

    /// The stored state; nothing stored yet is an empty one.
    pub fn load(&self) -> Result<LearnState, ProgressError> {
        let text = match fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(LearnState::default()),
            Err(source) => {
                return Err(ProgressError::Io {
                    path: self.path.clone(),
                    source,
                })
            }
        };
        serde_json::from_str(&text).map_err(|source| ProgressError::Format {
            path: self.path.clone(),
            source,
        })
    }

    /// Writes atomically (temp file, then rename).
    pub fn save(&self, state: &LearnState) -> Result<(), ProgressError> {
        let io = |path: &Path| {
            let path = path.to_path_buf();
            move |source| ProgressError::Io { path, source }
        };
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir).map_err(io(dir))?;
        }
        let text = serde_json::to_string_pretty(state).map_err(|source| ProgressError::Format {
            path: self.path.clone(),
            source,
        })?;
        let temp = self.path.with_extension("json.tmp");
        fs::write(&temp, text).map_err(io(&temp))?;
        fs::rename(&temp, &self.path).map_err(io(&self.path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kontra_model::InstrumentKind;

    const DE: &str = "# Greifen\nid: greifen\nfor: strings\n\n## Seite\nAlle.\n::: variant for=fretted\nBünde.\n:::\n::: variant for=fretless\nOhne.\n:::\n";
    const COURSE: &str = r#"{"id": "bass", "for": "bass", "chapters": [{"id": "a", "de": "A", "en": "A", "units": ["greifen"]}]}"#;

    fn lessons() -> Lessons {
        Lessons::parse(&[("courses/bass.json", COURSE), ("units/greifen/de.md", DE)]).unwrap()
    }

    fn tones(root: u8, steps: &[i32], string: Option<u8>) -> Tones {
        Tones {
            root,
            steps: steps.to_vec(),
            labels: Vec::new(),
            string,
        }
    }

    #[test]
    fn keeps_only_the_profiles_variants() {
        let learning = Learning::new(lessons(), &Instrument::default_bass());
        assert_eq!(learning.course().map(|c| c.id.as_str()), Some("bass"));
        let unit = learning.unit("greifen").unwrap();
        let doc = learning.doc(&unit.texts, Lang::En).unwrap();
        assert_eq!(
            doc.pages[0].blocks.len(),
            2,
            "text and the fretted variant's text"
        );
        assert_eq!(learning.state_key(unit), "greifen@e-bass");
    }

    #[test]
    fn keys_bass_gets_the_course_but_not_string_lessons() {
        let keys = Instrument::keyboard(InstrumentKind::Keys, Pitch(28), Pitch(67)).unwrap();
        let learning = Learning::new(lessons(), &keys);
        assert!(learning.course().is_some());
        assert!(!learning.fits("strings"));
    }

    #[test]
    fn places_the_root_lowest_in_range() {
        let bass = Instrument::default_bass();
        // A with a minor seventh: A1 (33) and G2 (43).
        assert_eq!(
            place(&tones(9, &[0, 10], None), &bass),
            [Pitch(33), Pitch(43)]
        );
        // E with 15 steps: G2, the open G string.
        assert_eq!(positions(&tones(4, &[15], Some(7)), &bass), [(0, 3, 0)]);
        // Frets 2 to 5 on the A string, not the E string.
        let a = positions(&tones(9, &[2, 3, 4, 5], Some(9)), &bass);
        assert_eq!(a, [(0, 1, 2), (1, 1, 3), (2, 1, 4), (3, 1, 5)]);
    }

    #[test]
    fn keeps_each_tone_with_its_step() {
        let bass = Instrument::default_bass();
        // Five octaves up lies above the bass: no tone, not an unplayable one.
        assert_eq!(place(&tones(9, &[0, 60], None), &bass), [Pitch(33)]);
        // G1 is not on the A string; A1 and B1 keep steps 1 and 2 (and their labels).
        let a = positions(&tones(9, &[-2, 0, 2], Some(9)), &bass);
        assert_eq!(a, [(1, 1, 0), (2, 1, 2)]);
    }

    #[test]
    fn remembers_what_was_done() {
        let dir = std::env::temp_dir().join(format!("kontra-learn-{}", std::process::id()));
        let store = LearnStore::new(&dir);
        let mut state = store.load().unwrap();
        state.see("viertel", 0, 3, 10);
        assert!(!state.done("viertel"));
        state.see("viertel", 2, 3, 20);
        store.save(&state).unwrap();

        let back = store.load().unwrap();
        assert!(back.done("viertel"));
        assert_eq!(back.units["viertel"].pages.len(), 2);
        let _ = fs::remove_dir_all(dir);
    }
}
