//! Added courses in the app core (LH-LRN-14): what a package says before it is added, adding
//! it, and which course to learn.
//!
//! ```text
//! QML ──course_preview(path)──▶ {title, provider, license, hosts, problems, fits}
//!     ──course_add(path)──────▶ CourseStore (data folder) ──▶ Core::courses, chosen course
//!     ──choose_course(id)─────▶ Settings::course
//! ```

use crate::Core;
use kontra_session::lesson::{self, CoursePack, COURSE_EXT};
use kontra_session::{CourseError, CourseStore};
use serde_json::json;
use std::path::Path;

/// Problems shown before adding; the rest are counted (`kontra-cli course check` lists all).
const PREVIEW_PROBLEMS: usize = 5;

impl Core {
    /// Whether `path` is a course package (by its extension, as the file manager sees it).
    pub fn is_course(path: &Path) -> bool {
        path.extension().is_some_and(|e| e == COURSE_EXT)
    }

    /// What the package at `path` is, before adding it: `{"ok", "title", "provider", "license",
    /// "lessons", "hosts": [host], "problems": [text], "more", "fits", "replaces"}`; `ok` false
    /// with `error` when it cannot be read. `fits` false: the course is for another instrument.
    pub fn course_preview(&self, path: &Path) -> String {
        let (pack, problems) = match CourseStore::preview(path) {
            Ok(preview) => preview,
            Err(e) => return json!({ "ok": false, "error": self.course_error(&e) }).to_string(),
        };
        let m = &pack.manifest;
        let scopes = lesson::scopes(self.instrument().kind);
        let fits = pack
            .lessons
            .courses
            .iter()
            .any(|c| c.id == m.course && scopes.contains(&c.scope.as_str()));
        let shown: Vec<&String> = problems.iter().take(PREVIEW_PROBLEMS).collect();
        json!({
            "ok": true,
            "title": self.pack_title(&pack),
            "provider": m.provider,
            "license": m.license,
            "lessons": pack.lessons.units.len(),
            "hosts": m.hosts,
            "problems": shown,
            "more": problems.len() - shown.len(),
            "fits": fits,
            "replaces": self.courses.iter().any(|c| c.manifest.id == m.id),
        })
        .to_string()
    }

    /// Adds the package at `path` and makes its course the one to learn; "" when done, else
    /// what went wrong.
    pub fn course_add(&mut self, path: &Path) -> String {
        let pack = match CourseStore::new(&self.data_dir).add(path) {
            Ok(pack) => pack,
            Err(e) => return self.course_error(&e),
        };
        self.courses = CourseStore::new(&self.data_dir).load();
        self.update_settings(|s| s.course = Some(pack.manifest.course.clone()));
        String::new()
    }

    /// Learns course `id` from now on (one of `learn_json`'s `courses`).
    pub fn choose_course(&mut self, id: &str) {
        self.update_settings(|s| s.course = Some(id.to_string()));
    }

    /// A course's name: the package title in the user's language, the bundled one by key.
    pub(crate) fn course_title(&self, id: &str) -> String {
        match self.courses.iter().find(|c| c.manifest.course == id) {
            Some(pack) => self.pack_title(pack),
            None => self.tr("courses.bundled", &[]),
        }
    }

    fn pack_title(&self, pack: &CoursePack) -> String {
        let titles = &pack.manifest.title;
        titles
            .get(self.learn_lang().key())
            .or_else(|| titles.values().next())
            .cloned()
            .unwrap_or_default()
    }

    fn course_error(&self, e: &CourseError) -> String {
        match e {
            CourseError::Problems(p) => {
                self.tr("courses.problems", &[("count", p.len().to_string())])
            }
            other => self.tr("courses.unreadable", &[("reason", other.to_string())]),
        }
    }
}
