//! The goal of a lesson's practice block in the summary (LH-LRN-20): "Ziel 80 %, erreicht 72 %".
//!
//! ```text
//! lesson_exercise ──▶ (path, Goal) ──▶ summary of a run of that file ──▶ {text, met} | none
//! ```

use crate::Core;
use kontra_session::lesson::Goal;
use kontra_session::Summary;
use serde_json::{json, Value};
use std::path::PathBuf;

impl Core {
    /// Remembers the goal of the block whose exercise opens at `path`.
    pub(crate) fn set_lesson_goal(&mut self, path: &std::path::Path, goal: Goal) {
        self.lesson_goal = Some((PathBuf::from(path), goal));
    }

    /// The lesson goal of the loaded piece, if it was opened from a lesson.
    pub(crate) fn loaded_goal_of_lesson(&self) -> Option<Goal> {
        let (path, goal) = self.lesson_goal.as_ref()?;
        (self.loaded_path() == Some(path.as_path())).then_some(*goal)
    }

    /// `{text, met}` for a goal with a target, `{none: true}` for pure instruction (no figure
    /// shown as reached), null outside lessons or when nothing measured the goal.
    pub(crate) fn goal_json(&self, s: &Summary) -> Value {
        let percent = |share: f64| format!("{:.0}", share * 100.0);
        let (target, reached, met) = match self.loaded_goal_of_lesson() {
            None => return Value::Null,
            Some(Goal::None) => return json!({ "none": true }),
            Some(Goal::Hits { percent: want }) => {
                let share = s.hit_rate;
                let met = share * 100.0 >= f64::from(want);
                (format!("{want} %"), format!("{} %", percent(share)), met)
            }
            Some(Goal::Technique { percent: want }) => {
                let (played, judged) = s
                    .techniques
                    .iter()
                    .fold((0, 0), |(p, j), t| (p + t.played, j + t.judged));
                if judged == 0 {
                    return Value::Null;
                }
                let share = f64::from(played) / f64::from(judged);
                let met = share * 100.0 >= f64::from(want);
                (format!("{want} %"), format!("{} %", percent(share)), met)
            }
            Some(Goal::Timing { ms }) => {
                let Some(spread) = s.spread_ms else {
                    return Value::Null;
                };
                // As the summary shows it: whole ms.
                let met = spread.round() <= f64::from(ms);
                (format!("±{ms} ms"), format!("±{spread:.0} ms"), met)
            }
        };
        let key = if met {
            "summary.goal.met"
        } else {
            "summary.goal"
        };
        let text = self.tr(key, &[("target", target), ("reached", reached)]);
        json!({ "text": text, "met": met })
    }
}
