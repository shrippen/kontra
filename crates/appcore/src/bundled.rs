//! Bundled and generated exercises (LH-BIB-08, LH-LRN-04): levels, titles, goals, mastery.

use crate::{exercise, Core, RecentEntry, PULSE_SPREAD_MS};
use kontra_model::Judging;
use serde_json::json;
use std::path::{Path, PathBuf};

/// Generator choices of `Core::exercise_spec`: judged by pitch and time, or by time only.
const JUDGING_PITCH: &str = "pitch";
const JUDGING_TIME: &str = "time";

impl Core {
    /// Folder of the bundled exercises (LH-BIB-08): installed share dir, else the repo's.
    pub fn exercises_dir() -> PathBuf {
        let installed = std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(|p| p.join("../share/kontra/exercises")));
        let dir = match installed {
            Some(path) if path.is_dir() => path,
            _ => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/exercises"),
        };
        // Without "..": progress is keyed by the path, and the same file opened from the file
        // manager has its plain one.
        std::fs::canonicalize(&dir).unwrap_or(dir)
    }

    /// Bundled exercises by level: `[{"level", "mastered": 3, "items": [{"title", "path", "best",
    /// "mastered"}]}]`; mastered: see `sits`.
    pub fn bundled_json(&self) -> String {
        json!(self.bundled_levels()).to_string()
    }

    pub(crate) fn bundled_levels(&self) -> Vec<serde_json::Value> {
        let mut levels: Vec<PathBuf> = std::fs::read_dir(Self::exercises_dir())
            .map(|d| {
                d.flatten()
                    .map(|e| e.path())
                    .filter(|p| p.is_dir())
                    .collect()
            })
            .unwrap_or_default();
        levels.sort();
        let list: Vec<_> = levels
            .iter()
            .map(|dir| {
                let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
                    .map(|d| {
                        d.flatten()
                            .map(|e| e.path())
                            .filter(|p| p.extension().is_some_and(|x| x == "mid"))
                            .collect()
                    })
                    .unwrap_or_default();
                // By plan place: "100 …" comes after "99 …" (a plain sort puts it first), level 1
                // in learning order.
                files.sort_by_key(|f| {
                    let (number, path) = plan_number(f);
                    (exercise::plan_place(number), path)
                });
                let items: Vec<_> = files
                    .iter()
                    .map(|f| {
                        let stem = f
                            .file_stem()
                            .map_or(String::new(), |s| s.to_string_lossy().into_owned());
                        // "07 Dur-Tonleiter in G (60 BPM)": the number picks the plan entry,
                        // whose title is told in the UI's language.
                        let title = self.bundled_title(&stem).unwrap_or_else(|| {
                            stem.split_once(' ')
                                .map_or(stem.clone(), |(_, t)| t.to_string())
                        });
                        let best = self.best_rate(f);
                        json!({
                            "title": title,
                            "goal": self.bundled_goal(&stem),
                            "goalTopic": bundled_goal_key(&stem)
                                .map_or(String::new(), |g| format!("goal.{g}")),
                            "path": f.to_string_lossy(),
                            "best": best,
                            "mastered": self.sits(f, self.bundled_judging(&stem)),
                        })
                    })
                    .collect();
                let mastered = items.iter().filter(|i| i["mastered"] == true).count();
                let name = dir
                    .file_name()
                    .map_or(String::new(), |n| n.to_string_lossy().into_owned());
                // "1 Einstieg": the number names the level in the UI's language.
                let level = name.split_once(' ').map_or(name.clone(), |(n, _)| {
                    self.tr(&format!("exercise.level.{n}"), &[])
                });
                json!({ "level": level, "items": items, "mastered": mastered })
            })
            .collect();
        list
    }

    /// A library entry's title; bundled exercises in the UI's language.
    pub(crate) fn library_title(&self, entry: &RecentEntry) -> String {
        self.plan_title(&entry.path)
            .unwrap_or_else(|| entry.title.clone())
    }

    /// A file of the bundled plan (exercises dir / level / file).
    pub(crate) fn is_bundled(&self, path: &Path) -> bool {
        path.parent()
            .and_then(Path::parent)
            .is_some_and(|dir| dir == Self::exercises_dir())
    }

    /// The title of a bundled exercise in the UI's language; `None` for any other file.
    pub(crate) fn plan_title(&self, path: &Path) -> Option<String> {
        let bundled = self.is_bundled(path);
        let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned());
        bundled
            .then_some(stem)
            .flatten()
            .and_then(|s| self.bundled_title(&s))
    }

    /// The title of bundled exercise "NN …" in the UI's language, from its plan entry.
    pub(crate) fn bundled_title(&self, stem: &str) -> Option<String> {
        let number: usize = stem.split_once(' ')?.0.parse().ok()?;
        let (_, spec) = exercise::bundled_specs()
            .into_iter()
            .nth(number.checked_sub(1)?)?;
        Some(self.exercise_title(&exercise::describe(&spec)))
    }

    /// How bundled exercise "NN …" is judged; pitch and time when unknown.
    fn bundled_judging(&self, stem: &str) -> Judging {
        stem.split_once(' ')
            .and_then(|(n, _)| n.parse::<usize>().ok())
            .and_then(|n| exercise::bundled_specs().into_iter().nth(n.checked_sub(1)?))
            .map_or(Judging::PitchAndTime, |(_, spec)| spec.judging)
    }

    /// Whether a run sits: hits above the workshop band (`MASTERED_RATE`); judged by time only,
    /// also a spread of at most `PULSE_SPREAD_MS` (DIDAKTIK I4: then the frets come in), in
    /// whole ms as the summary shows it.
    pub(crate) fn run_sits(judging: Judging, hit_rate: f64, spread_ms: Option<f64>) -> bool {
        let pulse = match judging {
            Judging::TimeOnly => spread_ms.is_some_and(|s| s.round() <= PULSE_SPREAD_MS),
            Judging::PitchAndTime => true,
        };
        hit_rate >= MASTERED_RATE && pulse
    }

    /// Whether any run of the piece at `path` sits (`run_sits`).
    fn sits(&self, path: &Path, judging: Judging) -> bool {
        self.library
            .entry(path)
            .and_then(|e| self.progress_store().load_id(&e.song_id))
            .is_some_and(|p| {
                p.runs
                    .iter()
                    .any(|r| Self::run_sits(judging, r.hit_rate, r.spread_ms))
            })
    }

    /// What bundled exercise "NN …" trains, in the UI's language ("" for none).
    pub(crate) fn bundled_goal(&self, stem: &str) -> String {
        bundled_goal_key(stem)
            .map(|goal| self.tr(&format!("exercise.goal.{goal}"), &[]))
            .unwrap_or_default()
    }

    /// "Dur-Tonleiter in G (60 BPM)" / "Major scale in G (60 BPM)"; finger exercises
    /// "Fingerübung 1-3-2-4 ab Bund 5 (60 BPM)"; a tempo ramp "(80–110 BPM)".
    pub fn exercise_title(&self, d: &exercise::Description) -> String {
        let key = self.note_label(&format!("{}{}", d.key, if d.minor { "m" } else { "" }));
        let bpm = match d.end_bpm {
            Some(end) => format!("{:.0}-{end:.0}", d.bpm),
            None => format!("{:.0}", d.bpm),
        };
        let part = |group: &str, key: &str| self.tr(&format!("exercise.name.{group}.{key}"), &[]);
        let name = match d.kind {
            "scale" => part("scale", d.variant),
            "sequence" => self.tr(
                "exercise.name.sequence",
                &[
                    ("scale", part("scale", d.variant)),
                    (
                        "sequence",
                        d.rhythm.map(|r| part("sequence", r)).unwrap_or_default(),
                    ),
                ],
            ),
            "song" => match d.rhythm {
                Some(level) => self.tr(
                    "exercise.name.song.level",
                    &[
                        ("style", part("style", d.variant)),
                        ("level", part("level", level)),
                    ],
                ),
                None => self.tr("exercise.name.song", &[("style", part("style", d.variant))]),
            },
            "chromatic" => self.tr(
                "exercise.name.chromatic",
                &[(
                    "fingers",
                    d.variant
                        .chars()
                        .map(String::from)
                        .collect::<Vec<_>>()
                        .join("-"),
                )],
            ),
            _ => self.tr(
                &format!("exercise.name.{}", d.kind),
                &[
                    ("progression", part("progression", d.variant)),
                    (
                        "rhythm",
                        d.rhythm.map(|r| part("rhythm", r)).unwrap_or_default(),
                    ),
                ],
            ),
        };
        let title = match d.fret {
            Some(fret) => self.tr(
                "exercise.title.fret",
                &[("name", name), ("fret", fret.to_string()), ("bpm", bpm)],
            ),
            None => self.tr(
                "exercise.title",
                &[("name", name), ("key", key), ("bpm", bpm)],
            ),
        };
        // Judged by time only: another exercise with its own file and progress (DIDAKTIK I4).
        match d.judging {
            Judging::TimeOnly => self.tr("exercise.title.timing", &[("title", title)]),
            Judging::PitchAndTime => title,
        }
    }

    /// Best hit rate (0..1) of a piece, or -1 when never played.
    fn best_rate(&self, path: &Path) -> f64 {
        self.library
            .entry(path)
            .and_then(|e| self.progress_store().load_id(&e.song_id))
            .and_then(|p| p.best_hit_rate())
            .unwrap_or(-1.0)
    }

    /// An exercise from the UI's choice: `kind` "scale" | "arpeggio" | "triads" | "walking" |
    /// "groove" | "chromatic"; `variant` the scale ("major", "dorian" …), the progression ("tonic",
    /// "251", "blues", "minor", "145", "pop", "1625", "minor251") or the finger order ("1324");
    /// `rhythm` the groove's rhythm or a scale's pattern ("thirds", "fours", "" for up and down).
    /// Finger exercises take `key` as the start fret. `judging` "time" judges only the attack
    /// (grooves, DIDAKTIK I4); "pitch" or "" pitch and time.
    pub fn exercise_spec(
        kind: &str,
        variant: &str,
        rhythm: &str,
        key: u8,
        bpm: f64,
        judging: &str,
    ) -> Option<exercise::ExerciseSpec> {
        let judging = match judging {
            "" | JUDGING_PITCH => Judging::PitchAndTime,
            JUDGING_TIME => Judging::TimeOnly,
            _ => return None,
        };
        use exercise::{ExerciseKind, Fingers, Progression, Rhythm, ScaleKind, Sequence};
        let progression = || Progression::from_key(variant);
        let kind = match kind {
            "scale" if rhythm.is_empty() => ExerciseKind::Scale(ScaleKind::from_key(variant)?),
            "scale" => {
                ExerciseKind::Sequence(ScaleKind::from_key(variant)?, Sequence::from_key(rhythm)?)
            }
            "arpeggio" => ExerciseKind::Arpeggio(progression()?),
            "triads" => ExerciseKind::Triads(progression()?),
            "walking" => ExerciseKind::WalkingBass(progression()?),
            "groove" => ExerciseKind::Groove(progression()?, Rhythm::from_key(rhythm)?),
            "chromatic" => ExerciseKind::Chromatic(Fingers::from_key(variant)?),
            // A piece takes its level where grooves take the rhythm; none is medium.
            "song" => ExerciseKind::Song(
                exercise::Style::from_key(variant)?,
                match rhythm {
                    "" => exercise::Level::default(),
                    level => exercise::Level::from_key(level)?,
                },
            ),
            _ => return None,
        };
        // A piece's form repeats its parts itself; a 12-bar blues is long enough once.
        let once = matches!(kind, ExerciseKind::Song(..)) || variant == "blues";
        Some(exercise::ExerciseSpec {
            kind,
            key: key % 12,
            bpm: bpm.clamp(40.0, 200.0),
            slow: false,
            // Short patterns (scales, two-bar grooves) get more passes.
            repeats: if once { 1 } else { 2 },
            ramp_bpm: 0.0,
            judging,
        })
    }

    /// Writes a generated exercise (LH-LRN-04) into the data folder and returns its path.
    pub fn generate_exercise(&mut self, spec: &exercise::ExerciseSpec) -> Result<PathBuf, String> {
        let generated = exercise::generate(spec);
        let dir = self.data_dir.join("exercises");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        // The file is named in the UI's language; the title inside stays German (progress key).
        let name: String = self
            .exercise_title(&exercise::describe(spec))
            .chars()
            // ♯ and ♭ as before the printed signs, so a file made again keeps its name.
            .map(|c| match c {
                '/' | ':' | '#' | '♯' => '-',
                '♭' => 'b',
                _ => c,
            })
            .collect();
        let path = dir.join(format!("{name}.mid"));
        std::fs::write(&path, &generated.midi).map_err(|e| e.to_string())?;
        Ok(path)
    }

    /// Path of the bundled first exercise: next to the binary when installed, else in the repo.
    pub fn demo_path() -> PathBuf {
        let installed = std::env::current_exe().ok().and_then(|e| {
            e.parent()
                .map(|p| p.join("../share/kontra/demo/etude-1.mid"))
        });
        let path = match installed {
            Some(path) if path.exists() => path,
            _ => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/demo/etude-1.mid"),
        };
        // Without "..", as `exercises_dir`: progress is keyed by the path.
        std::fs::canonicalize(&path).unwrap_or(path)
    }
}

/// A bundled exercise sits above the workshop's target band (80–90 %, round 5).
const MASTERED_RATE: f64 = 0.9;

/// Place of a bundled exercise in the plan, from its file name "NN …" (unnumbered last).
pub(crate) fn plan_number(path: &Path) -> (usize, PathBuf) {
    let number = path
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.split_once(' '))
        .and_then(|(n, _)| n.parse().ok())
        .unwrap_or(usize::MAX);
    (number, path.to_path_buf())
}

/// The goal key of a bundled exercise ("07 …" is plan entry 7), e.g. "scale".
fn bundled_goal_key(stem: &str) -> Option<&'static str> {
    let number = stem
        .split_once(' ')
        .and_then(|(n, _)| n.parse::<usize>().ok())?;
    exercise::bundled_specs()
        .into_iter()
        .nth(number.saturating_sub(1))
        .map(|(_, spec)| exercise::describe(&spec).goal)
}
