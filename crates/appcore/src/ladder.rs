//! The exercise ladder of a lesson's practice block (LH-LRN-17) for QML: five stages, each with
//! its best hit rate as a measurement; nothing locked, the first stage below its threshold is
//! the suggested one.
//!
//! ```text
//! lesson_json ──with_ladders──▶ practice block + "ladder": [{stage, best, threshold, suggested}]
//! lesson_stage(id, page, block, stage) ──▶ stage_exercise ──▶ generated file (goal of the block)
//! ```

use crate::{exercise, Core};
use exercise::{lesson_seed, stage_exercise, stage_spec, ExerciseSpec, Stage};
use kontra_session::lesson::{Block, ExerciseRef, Practice};
use serde_json::{json, Value};
use std::path::PathBuf;

impl Core {
    /// Adds `ladder` to each practice block of lesson `id` whose exercise the generator knows.
    pub(crate) fn with_ladders(&self, id: &str, mut pages: Vec<Value>) -> Vec<Value> {
        for (p, page) in pages.iter_mut().enumerate() {
            let Some(blocks) = page["blocks"].as_array_mut() else {
                continue;
            };
            for (b, block) in blocks.iter_mut().enumerate() {
                if block["kind"] != "practice" {
                    continue;
                }
                let Some(Block::Practice(practice)) = self.lesson_block(id, p, b) else {
                    continue;
                };
                let Some(spec) = self.block_spec(&practice) else {
                    continue;
                };
                block["ladder"] = self.ladder_json(&spec, lesson_seed(id));
            }
        }
        pages
    }

    /// `[{stage: "single", best: 0.72 | null, threshold: 0.6, suggested}]`.
    fn ladder_json(&self, spec: &ExerciseSpec, seed: u64) -> Value {
        let store = self.progress_store();
        let mut suggested = false;
        let stages: Vec<Value> = Stage::ALL
            .iter()
            .map(|stage| {
                let path = self.stage_path(spec, *stage, seed);
                let best = self
                    .library
                    .entry(&path)
                    .and_then(|e| store.load_id(&e.song_id))
                    .and_then(|p| p.best_hit_rate());
                // The first stage that is not reached yet: one suggestion, no lock.
                let open = best.is_none_or(|b| b < stage.threshold());
                let here = open && !suggested;
                suggested |= here;
                json!({
                    "stage": stage.key(),
                    "best": best,
                    "threshold": stage.threshold(),
                    "suggested": here,
                })
            })
            .collect();
        json!(stages)
    }

    /// The file of stage `stage` (0–4) of a practice block, written now; it is checked by the
    /// block's goal (LH-LRN-20).
    pub fn lesson_stage(
        &mut self,
        id: &str,
        page: usize,
        block: usize,
        stage: usize,
    ) -> Result<PathBuf, String> {
        let Some(Block::Practice(practice)) = self.lesson_block(id, page, block) else {
            return Err(String::new());
        };
        let (Some(spec), Some(stage)) = (self.block_spec(&practice), Stage::ALL.get(stage)) else {
            return Err(self.tr("learn.missing", &[]));
        };
        let seed = lesson_seed(id);
        let path = self.stage_path(&spec, *stage, seed);
        let path = Self::write_exercise(&stage_exercise(&spec, *stage, seed), path)?;
        self.set_lesson_goal(&path, practice.goal);
        Ok(path)
    }

    /// Where stage `stage` of `spec` is written: the generator's name, the mixed stage's behind
    /// its stage name ("Gemischt – Groove … .mid"), so it never shares the plain file's.
    fn stage_path(&self, spec: &ExerciseSpec, stage: Stage, seed: u64) -> PathBuf {
        let path = self.generated_path(&stage_spec(spec, stage, seed));
        if stage != Stage::Mixed {
            return path;
        }
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let mixed = self.tr("learn.stage.mixed", &[]);
        path.with_file_name(format!("{mixed} – {name}"))
    }

    /// What the generator makes for a block: the plan's entry of a bundled exercise, or the
    /// block's own choice.
    fn block_spec(&self, practice: &Practice) -> Option<ExerciseSpec> {
        match &practice.exercise {
            ExerciseRef::Bundled(n) => {
                let index = usize::from(*n).checked_sub(1)?;
                exercise::bundled_specs().get(index).map(|(_, spec)| *spec)
            }
            ExerciseRef::Generated { .. } => self.generated_spec(&practice.exercise),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Core, QaOptions};
    use serde_json::Value;

    /// LH-LRN-17: a bundled lesson's block offers five stages, the first suggested; a stage
    /// played above its threshold passes the suggestion on, and the same stage is the same file.
    #[test]
    fn a_practice_block_has_a_ladder() {
        let dir = std::env::temp_dir().join(format!("kontra-ladder-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut core = Core::new(&dir, QaOptions::default());
        let lesson: Value = core.lesson_json("viertel").parse().unwrap();
        let find = |lesson: &Value| {
            lesson["pages"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
                .find_map(|(p, page)| {
                    let blocks = page["blocks"].as_array()?;
                    let b = blocks.iter().position(|b| b["kind"] == "practice")?;
                    Some((p, b, blocks[b]["ladder"].clone()))
                })
        };
        let (page, block, ladder) = find(&lesson).unwrap();
        let stages = ladder.as_array().unwrap();
        assert_eq!(stages.len(), 5);
        assert_eq!(stages[0]["suggested"], true);
        assert!(stages[1..].iter().all(|s| s["suggested"] == false));

        let path = core.lesson_stage("viertel", page, block, 0).unwrap();
        assert_eq!(
            core.lesson_stage("viertel", page, block, 0),
            Ok(path.clone())
        );
        core.open(&path).unwrap();
        let a = core.arrangement().unwrap().clone();
        let run = kontra_session::Run {
            finished_unix: 1,
            practice_s: 60.0,
            speed: 1.0,
            hit_rate: 0.7,
            mode: "song".into(),
            section: None,
            hardest_bars: Vec::new(),
            hesitant_bars: Vec::new(),
            spread_ms: None,
            muted_in_time: None,
            bars: Vec::new(),
        };
        core.progress_store().record(&a, run).unwrap();
        let lesson: Value = core.lesson_json("viertel").parse().unwrap();
        let (_, _, ladder) = find(&lesson).unwrap();
        assert_eq!(ladder[0]["best"], 0.7);
        assert_eq!(ladder[1]["suggested"], true);
        assert!(core.lesson_stage("viertel", page, block, 5).is_err());
        // The mixed stage has its own file next to the context stage's.
        let context = core.lesson_stage("viertel", page, block, 3).unwrap();
        let mixed = core.lesson_stage("viertel", page, block, 4).unwrap();
        assert_eq!(context.parent(), mixed.parent());
        assert_ne!(context, mixed);
        let _ = std::fs::remove_dir_all(dir);
    }
}
