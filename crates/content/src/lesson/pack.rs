//! Course packages `.kontrakurs` (LH-LRN-14, `FORMAT.md`): a course with its own lessons,
//! glossary and media, written by someone else and added by the player.
//!
//! ```text
//! kurs.kontrakurs (zip)
//! ├── manifest.json            format, version, id, title, provider, license, course, hosts
//! ├── courses/<course>.json
//! ├── units/<id>/de.md, en.md
//! ├── glossary/de.md, en.md    optional: further terms
//! └── media/                   optional: videos, subtitles
//! ```
//!
//! A package is data, never code: names stay inside it, its course and lessons carry its prefix
//! (`bassbuzz.anschlag`) so nothing bundled is replaced, and videos stream only from the hosts
//! the manifest names. The same folder unpacked is what an author edits (`from_dir`).

use super::{Block, Doc, Lang, LessonError, Lessons};
use crate::zip::{self, Zip};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use thiserror::Error;

pub const COURSE_FORMAT: &str = "kontra-course";
pub const COURSE_VERSION: u32 = 1;
/// File extension of a course package.
pub const COURSE_EXT: &str = "kontrakurs";
const MANIFEST: &str = "manifest.json";
const MEDIA_DIR: &str = "media/";
const HTTPS: &str = "https://";
/// What Qt Multimedia plays on every desktop; subtitles as WebVTT.
const VIDEO_TYPES: &[&str] = &[".webm", ".mp4"];
const SUBTITLE_TYPES: &[&str] = &[".vtt"];
/// Videos belong on the web (small packages); one that must work offline may carry a few.
const MAX_COURSE_BYTES: u64 = 1024 * 1024 * 1024;
/// Manifest, courses and lessons are text; refuse more before inflating.
const MAX_TEXT_BYTES: usize = 4 * 1024 * 1024;

/// The template `new` lays out; `{id}` becomes the package id (path and text).
const TEMPLATE: &[(&str, &str)] = &[
    (
        "manifest.json",
        include_str!("../../course-template/manifest.json"),
    ),
    (
        "courses/{id}.kurs.json",
        include_str!("../../course-template/course.json"),
    ),
    (
        "units/{id}.start/de.md",
        include_str!("../../course-template/start-de.md"),
    ),
    (
        "units/{id}.start/en.md",
        include_str!("../../course-template/start-en.md"),
    ),
];
const ID: &str = "{id}";

/// `manifest.json` of a course package.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CourseManifest {
    pub format: String,
    pub version: u32,
    /// Prefix of the course and lesson ids (`bassbuzz` → `bassbuzz.anschlag`).
    pub id: String,
    /// Title per language code ("de", "en").
    pub title: BTreeMap<String, String>,
    pub provider: String,
    pub license: String,
    /// The course: `courses/<course>.json`.
    pub course: String,
    /// Hosts videos stream from; any other address is an error.
    #[serde(default)]
    pub hosts: Vec<String>,
}

/// A course package read and parsed (media only by name).
#[derive(Debug, Clone, PartialEq)]
pub struct CoursePack {
    pub manifest: CourseManifest,
    pub lessons: Lessons,
    /// Names below `media/`.
    pub media: BTreeSet<String>,
}

/// A package that cannot be read at all; what reads but does not fit is in `problems`.
#[derive(Debug, Error, PartialEq)]
pub enum PackError {
    #[error(transparent)]
    Lesson(#[from] LessonError),
    #[error("{0}")]
    Archive(String),
}

impl PackError {
    fn archive(reason: impl Into<String>) -> PackError {
        PackError::Archive(reason.into())
    }
}

impl CoursePack {
    /// Reads a `.kontrakurs` archive; media stay packed.
    pub fn read(data: &[u8]) -> Result<CoursePack, PackError> {
        if data.len() as u64 > MAX_COURSE_BYTES {
            return Err(PackError::archive(format!("{} bytes", data.len())));
        }
        let zip = Zip::open(data).map_err(|e| PackError::archive(e.to_string()))?;

        let mut texts = Vec::new();
        let mut media = Vec::new();
        for name in zip.names() {
            safe(name)?;
            if name.ends_with('/') {
                continue;
            }
            if name.starts_with(MEDIA_DIR) {
                media.push(name.to_string());
                continue;
            }
            let bytes = zip
                .read_text(name, MAX_TEXT_BYTES)
                .map_err(|e| PackError::archive(format!("{name}: {e}")))?;
            texts.push((name.to_string(), utf8(name, bytes)?));
        }
        CoursePack::parse(texts, media)
    }

    /// Reads an unpacked package folder, as an author keeps it.
    pub fn from_dir(dir: &Path) -> Result<CoursePack, PackError> {
        let mut texts = Vec::new();
        let mut media = Vec::new();
        for name in files(dir)? {
            if name.starts_with(MEDIA_DIR) {
                media.push(name);
                continue;
            }
            let bytes = std::fs::read(dir.join(&name))
                .map_err(|e| PackError::archive(format!("{name}: {e}")))?;
            texts.push((name.clone(), utf8(&name, bytes)?));
        }
        CoursePack::parse(texts, media)
    }

    fn parse(texts: Vec<(String, String)>, media: Vec<String>) -> Result<CoursePack, PackError> {
        let manifest_text = texts
            .iter()
            .find(|(n, _)| n == MANIFEST)
            .map(|(_, t)| t.as_str())
            .ok_or_else(|| PackError::archive(format!("{MANIFEST} missing")))?;
        let manifest: CourseManifest = serde_json::from_str(manifest_text)
            .map_err(|e| super::parse::error(MANIFEST, e.line(), &e.to_string()))?;
        if manifest.format != COURSE_FORMAT {
            let reason = format!("format {} is not {COURSE_FORMAT}", manifest.format);
            return Err(super::parse::error(MANIFEST, 1, &reason).into());
        }
        if manifest.version > COURSE_VERSION {
            let reason = format!("version {} is newer than this Kontra", manifest.version);
            return Err(super::parse::error(MANIFEST, 1, &reason).into());
        }

        let files: Vec<(&str, &str)> = texts
            .iter()
            .map(|(n, t)| (n.as_str(), t.as_str()))
            .collect();
        Ok(CoursePack {
            manifest,
            lessons: Lessons::parse(&files)?,
            media: media
                .into_iter()
                .filter_map(|m| m.strip_prefix(MEDIA_DIR).map(str::to_string))
                .collect(),
        })
    }

    /// What does not fit, one sentence each; `bundled` is the pool the course may build on.
    pub fn problems(&self, bundled: &Lessons) -> Vec<String> {
        let m = &self.manifest;
        let mut out = Vec::new();
        let prefix = format!("{}.", m.id);

        // Manifest: an id that makes a prefix, titles in every language, plain host names.
        if m.id.is_empty() || !m.id.chars().all(|c| c.is_ascii_lowercase() || c == '-') {
            out.push(format!("id {:?}: only a-z and -", m.id));
        }
        for lang in Lang::ALL {
            if m.title.get(lang.key()).is_none_or(|t| t.trim().is_empty()) {
                out.push(format!("title lacks {}", lang.key()));
            }
        }
        for field in [("provider", &m.provider), ("license", &m.license)] {
            if field.1.trim().is_empty() {
                out.push(format!("{} is empty", field.0));
            }
        }
        for host in &m.hosts {
            if host.is_empty() || host.contains(['/', ':', '@', ' ']) {
                out.push(format!("host {host:?} is not a plain host name"));
            }
        }

        // Own ids carry the prefix: the bundled pool is named, never replaced.
        if !self.lessons.courses.iter().any(|c| c.id == m.course) {
            out.push(format!("course {} missing in courses/", m.course));
        }
        for course in &self.lessons.courses {
            if !course.id.starts_with(&prefix) {
                out.push(format!("course {} lacks the prefix {prefix}", course.id));
            }
        }
        for unit in &self.lessons.units {
            if !unit.id.starts_with(&prefix) {
                out.push(format!("unit {} lacks the prefix {prefix}", unit.id));
            }
        }

        // Videos and subtitles: from the package or a named host.
        for (name, doc) in self.texts() {
            for page in &doc.pages {
                self.media_problems(&name, &page.blocks, &mut out);
            }
        }

        // The pool as Kontra will see it: bundled lessons and the package's together.
        let pool = Lessons {
            courses: self.lessons.courses.clone(),
            units: [bundled.units.clone(), self.lessons.units.clone()].concat(),
            terms: [bundled.terms.clone(), self.lessons.terms.clone()].concat(),
        };
        out.extend(pool.check());
        out
    }

    /// Every unit's and term's text with the file it comes from ("units/<id>/de.md").
    fn texts(&self) -> Vec<(String, &Doc)> {
        let mut out = Vec::new();
        for unit in &self.lessons.units {
            for (lang, doc) in &unit.texts {
                out.push((format!("units/{}/{}.md", unit.id, lang.key()), doc));
            }
        }
        for term in &self.lessons.terms {
            for (lang, doc) in &term.texts {
                out.push((format!("glossary/{}.md ({})", lang.key(), term.id), doc));
            }
        }
        out
    }

    fn media_problems(&self, name: &str, blocks: &[Block], out: &mut Vec<String>) {
        for block in blocks {
            match block {
                Block::Video { src, subtitles } => {
                    if let Some(p) = self.source(src, VIDEO_TYPES) {
                        out.push(format!("{name}: video {src}: {p}"));
                    }
                    let Some(sub) = subtitles else {
                        continue;
                    };
                    if let Some(p) = self.source(sub, SUBTITLE_TYPES) {
                        out.push(format!("{name}: subtitles {sub}: {p}"));
                    }
                }
                Block::Variant { blocks, .. } => self.media_problems(name, blocks, out),
                _ => {}
            }
        }
    }

    /// Why `src` cannot be played, if it cannot: an address on a host the manifest does not
    /// name, a file missing in `media/`, or a type Kontra does not play.
    fn source(&self, src: &str, types: &[&str]) -> Option<&'static str> {
        if let Some(rest) = src.strip_prefix(HTTPS) {
            let host = rest.split('/').next().unwrap_or_default();
            if !self.manifest.hosts.iter().any(|h| h == host) {
                return Some("host not named in the manifest");
            }
            return None;
        }
        if src.contains("://") {
            return Some("only https addresses");
        }
        let Some(file) = src.strip_prefix(MEDIA_DIR) else {
            return Some("not below media/");
        };
        if !self.media.contains(file) {
            return Some("missing in media/");
        }
        let lower = file.to_ascii_lowercase();
        if !types.iter().any(|t| lower.ends_with(t)) {
            return Some("unsupported type");
        }
        None
    }
}

/// Packs the folder `dir` as a `.kontrakurs` archive (every file, hidden ones left out). Check
/// it first; this only refuses what cannot be packed.
pub fn pack(dir: &Path) -> Result<Vec<u8>, PackError> {
    let names = files(dir)?;
    let mut contents = Vec::new();
    for name in &names {
        let bytes = std::fs::read(dir.join(name))
            .map_err(|e| PackError::archive(format!("{name}: {e}")))?;
        contents.push(bytes);
    }
    let entries: Vec<(&str, &[u8])> = names
        .iter()
        .map(String::as_str)
        .zip(contents.iter().map(Vec::as_slice))
        .collect();
    zip::write(&entries).map_err(|e| PackError::archive(e.to_string()))
}

/// Who offers a new course and under which licence (`manifest.json`).
#[derive(Debug, Clone)]
pub struct Author {
    pub provider: String,
    pub license: String,
}

/// Lays out a new course in the empty or missing folder `dir`: manifest, course, one lesson.
pub fn new_course(dir: &Path, id: &str, author: &Author) -> Result<(), PackError> {
    let io = |e: std::io::Error| PackError::archive(format!("{}: {e}", dir.display()));
    if dir.exists() && std::fs::read_dir(dir).map_err(io)?.next().is_some() {
        return Err(PackError::archive(format!(
            "{} is not empty",
            dir.display()
        )));
    }
    for (path, text) in TEMPLATE {
        let target = dir.join(path.replace(ID, id));
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(io)?;
        }
        std::fs::write(&target, text.replace(ID, id)).map_err(io)?;
    }

    // Provider and licence go in through JSON, so quotes in them stay quotes.
    let path = dir.join(MANIFEST);
    let text = std::fs::read_to_string(&path).map_err(io)?;
    let mut manifest: CourseManifest =
        serde_json::from_str(&text).map_err(|e| PackError::archive(e.to_string()))?;
    manifest.provider.clone_from(&author.provider);
    manifest.license.clone_from(&author.license);
    let text =
        serde_json::to_string_pretty(&manifest).map_err(|e| PackError::archive(e.to_string()))?;
    std::fs::write(&path, text + "\n").map_err(io)?;
    Ok(())
}

/// Paths of all files below `dir`, relative with `/`, sorted; hidden files and folders skipped.
fn files(dir: &Path) -> Result<Vec<String>, PackError> {
    let mut out = Vec::new();
    let mut todo = vec![dir.to_path_buf()];
    while let Some(at) = todo.pop() {
        let entries = std::fs::read_dir(&at)
            .map_err(|e| PackError::archive(format!("{}: {e}", at.display())))?;
        for entry in entries.flatten() {
            let path = entry.path();
            if entry.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            if path.is_dir() {
                todo.push(path);
                continue;
            }
            let Ok(rel) = path.strip_prefix(dir) else {
                continue;
            };
            let parts: Vec<String> = rel
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect();
            out.push(parts.join("/"));
        }
    }
    out.sort();
    Ok(out)
}

/// A name that stays inside the package: relative, no `..`, no backslash or NUL.
fn safe(name: &str) -> Result<(), PackError> {
    let bad = name.starts_with('/')
        || name.contains(['\\', '\0'])
        || name.split('/').any(|part| part == ".." || part == ".");
    if bad {
        return Err(PackError::archive(format!("unsafe name {name:?}")));
    }
    Ok(())
}

fn utf8(name: &str, bytes: Vec<u8>) -> Result<String, PackError> {
    String::from_utf8(bytes).map_err(|_| PackError::archive(format!("{name}: not UTF-8")))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST_OK: &str = r#"{
  "format": "kontra-course", "version": 1, "id": "test",
  "title": {"de": "Test", "en": "Test"}, "provider": "Anbieter", "license": "CC-BY-4.0",
  "course": "test.kurs", "hosts": ["video.example"]
}"#;
    const COURSE: &str = r#"{"id": "test.kurs", "for": "bass", "chapters": [
  {"id": "eins", "de": "Eins", "en": "One", "units": ["viertel", "test.video"]}]}"#;

    fn unit(lang: &str, video: &str) -> String {
        format!("# Video\nid: test.video\nneeds: viertel\n\n## {lang}\nText.\n::: video {video}\n")
    }

    fn archive(video: &str, extra: &[(&str, &[u8])]) -> Vec<u8> {
        let de = unit("Sehen", video);
        let en = unit("Watch", video);
        let mut entries: Vec<(&str, &[u8])> = vec![
            ("manifest.json", MANIFEST_OK.as_bytes()),
            ("courses/test.kurs.json", COURSE.as_bytes()),
            ("units/test.video/de.md", de.as_bytes()),
            ("units/test.video/en.md", en.as_bytes()),
        ];
        entries.extend_from_slice(extra);
        zip::write(&entries).unwrap()
    }

    fn problems(data: &[u8]) -> Vec<String> {
        let bundled = Lessons::bundled().unwrap();
        CoursePack::read(data).unwrap().problems(&bundled)
    }

    #[test]
    fn a_course_builds_on_bundled_lessons() {
        let data = archive("src=media/a.webm", &[("media/a.webm", b"x")]);
        assert_eq!(problems(&data), Vec::<String>::new());
    }

    #[test]
    fn videos_come_from_the_package_or_a_named_host() {
        // The same problem in both language files.
        let both = |problem: &str| {
            Lang::ALL.map(|l| format!("units/test.video/{}.md: {problem}", l.key()))
        };

        let named = archive("src=https://video.example/a.webm", &[]);
        assert_eq!(problems(&named), Vec::<String>::new());

        let other = archive("src=https://tracker.example/a.webm", &[]);
        let host = "video https://tracker.example/a.webm: host not named in the manifest";
        assert_eq!(problems(&other), both(host));

        let missing = archive("src=media/b.webm", &[]);
        let file = "video media/b.webm: missing in media/";
        assert_eq!(problems(&missing), both(file));

        let plain = archive("src=http://video.example/a.webm", &[]);
        let http = "video http://video.example/a.webm: only https addresses";
        assert_eq!(problems(&plain), both(http));

        let subs = archive(
            "src=media/a.webm subtitles=media/a.srt",
            &[("media/a.webm", b"x"), ("media/a.srt", b"x")],
        );
        let srt = "subtitles media/a.srt: unsupported type";
        assert_eq!(problems(&subs), both(srt));
    }

    #[test]
    fn own_lessons_carry_the_prefix() {
        let de = "# Greifen\nid: greifen\n\n## Seite\nText.\n";
        let data = archive(
            "src=media/a.webm",
            &[
                ("media/a.webm", b"x"),
                ("units/greifen/de.md", de.as_bytes()),
                ("units/greifen/en.md", de.as_bytes()),
            ],
        );
        assert!(problems(&data).contains(&"unit greifen lacks the prefix test.".to_string()));
    }

    #[test]
    fn names_stay_inside_the_package() {
        let data = archive("src=media/a.webm", &[("media/../../evil", b"x")]);
        assert!(matches!(
            CoursePack::read(&data),
            Err(PackError::Archive(_))
        ));
    }

    #[test]
    fn manifest_errors_name_file_and_line() {
        let data = zip::write(&[("manifest.json", b"{\n  \"format\": 1\n}")]).unwrap();
        let Err(PackError::Lesson(e)) = CoursePack::read(&data) else {
            panic!("expected a lesson error");
        };
        assert_eq!((e.file.as_str(), e.line), ("manifest.json", 2));
    }

    #[test]
    fn a_new_course_packs_and_reads_without_problems() {
        let dir = std::env::temp_dir().join(format!("kontra-course-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let author = Author {
            provider: "Studio \"W\"".into(),
            license: "CC-BY-4.0".into(),
        };
        new_course(&dir, "probe", &author).unwrap();
        assert!(
            new_course(&dir, "probe", &author).is_err(),
            "refuses a folder with files"
        );

        let bundled = Lessons::bundled().unwrap();
        let unpacked = CoursePack::from_dir(&dir).unwrap();
        assert_eq!(unpacked.problems(&bundled), Vec::<String>::new());
        assert_eq!(unpacked.manifest.provider, "Studio \"W\"");
        let packed = CoursePack::read(&pack(&dir).unwrap()).unwrap();
        assert_eq!(packed, unpacked);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
