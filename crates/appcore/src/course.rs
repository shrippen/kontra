//! Added courses in the app core (LH-LRN-14): what a package says before it is added, adding
//! it, and which course to learn.
//!
//! ```text
//! QML ──course_preview(path)──▶ {title, provider, license, hosts, problems, fits}
//!     ──course_add(path)──────▶ CourseStore (data folder) ──▶ Core::courses, chosen course
//!     ──choose_course(id)─────▶ Settings::course
//!     ──course_fetch(url)─────▶ CourseFetch (thread) ──course_fetch_poll() each tick──▶ preview
//!     ──lesson_video(…)───────▶ https address as written, or the video unpacked into the cache
//! ```

use crate::Core;
use kontra_session::lesson::{self, Block, CoursePack, COURSE_EXT};
use kontra_session::{CourseError, CourseFetch, CourseStore, FileHttp, Http, Web};
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;

const HTTPS: &str = "https://";
const FILE_URL: &str = "file://";

/// Problems shown before adding; the rest are counted (`kontra-cli course check` lists all).
const PREVIEW_PROBLEMS: usize = 5;

/// A package being fetched from an address.
#[derive(Debug, Default)]
pub(crate) struct Fetcher {
    /// The web, made on first use; tests and QA runs put a fake here.
    http: Option<Arc<dyn Http>>,
    running: Option<CourseFetch>,
}

impl Core {
    /// Fetches the package at `url` for a preview: `{"state": "running"}`, or
    /// `{"state": "error", "message"}` when it cannot start (not https).
    pub fn course_fetch(&mut self, url: &str) -> String {
        let http = self
            .fetcher
            .http
            .get_or_insert_with(|| Arc::new(Web::new()))
            .clone();
        match CourseStore::new(&self.data_dir).fetch(http, url) {
            Ok(fetch) => {
                self.fetcher.running = Some(fetch);
                json!({ "state": "running" }).to_string()
            }
            Err(e) => json!({ "state": "error", "message": self.course_error(&e) }).to_string(),
        }
    }

    /// The finished fetch, once: `{"state": "done", "path"}` (preview and add it like a file)
    /// or `{"state": "error", "message"}`; `None` while it runs or none was started.
    pub fn course_fetch_poll(&mut self) -> Option<String> {
        let outcome = self.fetcher.running.as_mut()?.poll()?;
        self.fetcher.running = None;
        let json = match outcome {
            Ok(path) => json!({ "state": "done", "path": path.to_string_lossy() }),
            Err(e) => json!({ "state": "error", "message": self.course_error(&e) }),
        };
        Some(json.to_string())
    }

    /// Answers every course address with the file's content (QA runs): never the network.
    pub fn fake_course_web(&mut self, answer: &Path) {
        self.set_course_http(Arc::new(FileHttp::new(answer)));
    }

    /// The web client fetching uses; tests pass a fake.
    pub fn set_course_http(&mut self, http: Arc<dyn Http>) {
        self.fetcher.http = Some(http);
    }

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

    /// What to play for video block `block` on page `page` of lesson `id`, only when asked
    /// (a stream starts on a click, never on opening the page): an https address as written,
    /// or the video unpacked from its course as a file URL; "" when there is none.
    pub fn lesson_video(&self, id: &str, page: usize, block: usize) -> String {
        let Some(Block::Video { src, .. }) = self.lesson_block(id, page, block) else {
            return String::new();
        };
        if src.starts_with(HTTPS) {
            return src;
        }
        let Some(pack) = self.unit_pack(id) else {
            return String::new();
        };
        let cache = crate::store::cache_dir(&self.data_dir);
        match CourseStore::new(&self.data_dir).media_file(&pack.manifest.id, &src, &cache) {
            Ok(path) => format!("{FILE_URL}{}", path.display()),
            Err(e) => {
                eprintln!("kontra: {e}");
                String::new()
            }
        }
    }

    /// `pages` of lesson `id` with the subtitle cues of its videos (`cues: [{startMs, endMs,
    /// text}]`), read from the lesson's course.
    pub(crate) fn with_cues(&self, id: &str, mut pages: Vec<Value>) -> Vec<Value> {
        let Some(pack) = self.unit_pack(id) else {
            return pages;
        };
        let store = CourseStore::new(&self.data_dir);
        let blocks = pages.iter_mut().filter_map(|p| p["blocks"].as_array_mut());
        for block in blocks.flatten() {
            let Some(src) = block["subtitles"].as_str() else {
                continue;
            };
            let cues: Vec<Value> = store
                .subtitles(&pack.manifest.id, src)
                .into_iter()
                .map(|c| json!({ "startMs": c.start_ms, "endMs": c.end_ms, "text": c.text }))
                .collect();
            block["cues"] = json!(cues);
        }
        pages
    }

    /// The added course a lesson belongs to; none for bundled lessons.
    fn unit_pack(&self, id: &str) -> Option<&CoursePack> {
        self.courses
            .iter()
            .find(|c| c.lessons.units.iter().any(|u| u.id == id))
    }

    /// A course's name: the package title in the user's language, the bundled one by key.
    pub(crate) fn course_title(&self, id: &str) -> String {
        match self.courses.iter().find(|c| c.manifest.course == id) {
            Some(pack) => self.pack_title(pack),
            None => self.tr("courses.bundled", &[]),
        }
    }

    /// The name of a term's source: package `id`, or the bundled course for "".
    pub(crate) fn source_title(&self, id: &str) -> String {
        match self.courses.iter().find(|c| c.manifest.id == id) {
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
            CourseError::NotHttps => self.tr("courses.https", &[]),
            CourseError::Http(h) => self.tr("courses.offline", &[("reason", h.to_string())]),
            other => self.tr("courses.unreadable", &[("reason", other.to_string())]),
        }
    }
}
