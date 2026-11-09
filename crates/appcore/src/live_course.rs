//! A course folder opened live (LH-LRN-21, P4): the author edits the unpacked course, Kontra
//! checks it again on every change, shows the findings and the open lesson anew. Nothing is
//! stored: no package in the data folder, no learning progress while it is open.
//!
//! ```text
//! open_course_folder(dir) ──▶ CoursePack::from_dir ──▶ Core::courses (in memory, chosen)
//! tick ──course_folder_poll() every 0.5 s──▶ folder_stamp changed? ──▶ read again, revision + 1
//!                                              └─▶ {dir, revision, ok, error, problems}
//! ```

use crate::Core;
use kontra_session::lesson::{folder_stamp, CoursePack, Lessons};
use kontra_session::CourseStore;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// How often the folder is looked at: an editor's save shows within half a second.
const LOOK_EVERY: Duration = Duration::from_millis(500);

/// The open folder and what its last reading said.
#[derive(Debug)]
pub(crate) struct LiveCourse {
    dir: PathBuf,
    stamp: Option<u64>,
    looked: Instant,
    /// Counts readings, so QML knows when to show the lesson again.
    revision: u32,
    /// The course id of the last good reading, chosen while the folder is open.
    course: Option<String>,
    error: Option<String>,
    problems: Vec<String>,
}

impl Core {
    /// Opens `dir` as a live course, or closes the live course when `dir` is empty. Returns
    /// `course_folder_json`.
    pub fn open_course_folder(&mut self, dir: &Path) -> String {
        if dir.as_os_str().is_empty() {
            self.live_course = None;
            self.courses = CourseStore::new(&self.data_dir).load();
            return Value::Null.to_string();
        }
        self.live_course = Some(LiveCourse {
            dir: dir.to_path_buf(),
            stamp: None,
            looked: Instant::now(),
            revision: 0,
            course: None,
            error: None,
            problems: Vec::new(),
        });
        self.read_live_course();
        self.course_folder_json()
    }

    /// After a change in the folder (looked at every `LOOK_EVERY`): read again, then
    /// `Some(course_folder_json)`; `None` when nothing changed or no folder is open.
    pub fn course_folder_poll(&mut self) -> Option<String> {
        let live = self.live_course.as_mut()?;
        if live.looked.elapsed() < LOOK_EVERY {
            return None;
        }
        live.looked = Instant::now();
        if folder_stamp(&live.dir).ok() == live.stamp {
            return None;
        }
        self.read_live_course();
        Some(self.course_folder_json())
    }

    /// `{"dir", "revision", "ok", "error", "problems": [text]}`, or null without a folder.
    pub fn course_folder_json(&self) -> String {
        let Some(live) = &self.live_course else {
            return Value::Null.to_string();
        };
        json!({
            "dir": live.dir.to_string_lossy(),
            "revision": live.revision,
            "ok": live.error.is_none() && live.problems.is_empty(),
            "error": live.error,
            "problems": live.problems,
        })
        .to_string()
    }

    /// The course id to learn while a folder is open (its last good reading).
    pub(crate) fn live_course_id(&self) -> Option<&str> {
        self.live_course.as_ref()?.course.as_deref()
    }

    /// Whether learning progress is held back: a live folder is open.
    pub(crate) fn previewing_course(&self) -> bool {
        self.live_course.is_some()
    }

    /// Reads the folder; a good reading replaces the course in memory, a broken one keeps the
    /// last good one on screen and names the error.
    fn read_live_course(&mut self) {
        let Some(live) = self.live_course.as_mut() else {
            return;
        };
        live.stamp = folder_stamp(&live.dir).ok();
        live.revision += 1;
        let pack = CoursePack::from_dir(&live.dir);
        let pack = match pack {
            Ok(pack) => pack,
            Err(e) => {
                live.error = Some(e.to_string());
                live.problems.clear();
                return;
            }
        };
        live.error = None;
        live.problems = Lessons::bundled()
            .map_or_else(|e| vec![e.to_string()], |bundled| pack.problems(&bundled));
        live.course = Some(pack.manifest.course.clone());

        let id = pack.manifest.id.clone();
        let mut courses = CourseStore::new(&self.data_dir).load();
        courses.retain(|c| c.manifest.id != id);
        courses.push(pack);
        self.courses = courses;
    }
}

#[cfg(test)]
mod tests {
    use crate::{Core, QaOptions};
    use kontra_session::lesson::{new_course, Author};
    use serde_json::Value;

    /// LH-LRN-21 (J-G6): an edited lesson shows without packing, a broken one names the place
    /// and keeps the last good one; nothing learned is stored meanwhile.
    #[test]
    fn a_course_folder_reloads_on_change() {
        let base = std::env::temp_dir().join(format!("kontra-live-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let course = base.join("kurs");
        let author = Author {
            provider: "Studio Weber".into(),
            license: "CC-BY-4.0".into(),
        };
        new_course(&course, "studio-weber", &author).unwrap();
        let mut core = Core::new(&base.join("data"), QaOptions::default());
        core.update_settings(|s| s.language = Some("de".into()));

        let opened: Value = core.open_course_folder(&course).parse().unwrap();
        assert_eq!(opened["ok"], true, "{opened}");
        let learn: Value = core.learn_json().parse().unwrap();
        assert_eq!(learn["course"], "studio-weber.kurs", "chosen while open");
        let id = "studio-weber.start";
        let file = course.join(format!("units/{id}/de.md"));
        assert_eq!(core.course_folder_poll(), None, "nothing changed");

        // Edited: read again after the pause, the lesson shows the new title.
        let text = std::fs::read_to_string(&file)
            .unwrap()
            .replacen('\n', " neu\n", 1);
        std::fs::write(&file, &text).unwrap();
        core.live_course.as_mut().unwrap().looked -= super::LOOK_EVERY;
        let changed: Value = core.course_folder_poll().unwrap().parse().unwrap();
        assert_eq!(changed["revision"], 2);
        let shown: Value = core.lesson_json(id).parse().unwrap();
        assert!(
            shown["title"].as_str().unwrap().ends_with(" neu"),
            "{shown}"
        );

        // Broken: the place is named, the last good lesson stays.
        std::fs::write(&file, text + "::: practice goal=x\n").unwrap();
        core.live_course.as_mut().unwrap().looked -= super::LOOK_EVERY;
        let broken: Value = core.course_folder_poll().unwrap().parse().unwrap();
        assert!(
            broken["error"].as_str().unwrap().contains("de.md:"),
            "{broken}"
        );
        assert!(!core.lesson_json(id).is_empty());

        // Seen pages are not stored, and closing leaves no course behind.
        core.lesson_seen(id, 0);
        assert!(!base.join("data/learning.json").exists());
        core.open_course_folder(std::path::Path::new(""));
        assert!(core.lesson_json(id).is_empty());
        let _ = std::fs::remove_dir_all(base);
    }
}
