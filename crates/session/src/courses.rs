//! Courses added by the player (LH-LRN-14): `.kontrakurs` packages kept as they came, one file
//! per package id in the data folder.
//!
//! ```text
//! kurs.kontrakurs ──preview()──▶ manifest, problems   (shown before adding)
//!                 ──add()──────▶ <data>/courses/<id>.kontrakurs ──load()──▶ [CoursePack]
//! https://… ──fetch()──▶ worker thread ──poll()──▶ <data>/courses/fetched.part (then as a file)
//! ```

use kontra_content::lesson::{
    self, CoursePack, Cue, Lessons, PackError, COURSE_EXT, MAX_COURSE_BYTES,
};
use kontra_engine::{Http, HttpError};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;
use thiserror::Error;

const DIR: &str = "courses";
/// Where a fetched package waits to be previewed and added; not a course (`load` skips it).
const FETCHED: &str = "fetched.part";
const HTTPS: &str = "https://";

/// Why a course cannot be added.
#[derive(Debug, Error)]
pub enum CourseError {
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error(transparent)]
    Pack(#[from] PackError),
    /// It reads but does not fit; the problems as `CoursePack::problems` says them.
    #[error("{} problems", .0.len())]
    Problems(Vec<String>),
    #[error(transparent)]
    Http(#[from] HttpError),
    /// Kontra fetches packages only over https.
    #[error("not an https address")]
    NotHttps,
    /// The worker ended without an answer.
    #[error("fetch ended")]
    Ended,
}

/// A package being fetched from an address the player typed (LH-LRN-14).
#[derive(Debug)]
pub struct CourseFetch {
    running: Receiver<Result<PathBuf, CourseError>>,
}

impl CourseFetch {
    /// The fetched file, once; `None` while it runs.
    pub fn poll(&mut self) -> Option<Result<PathBuf, CourseError>> {
        match self.running.try_recv() {
            Ok(outcome) => Some(outcome),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err(CourseError::Ended)),
        }
    }
}

/// The added courses in the data folder.
#[derive(Debug)]
pub struct CourseStore {
    dir: PathBuf,
}

impl CourseStore {
    pub fn new(data_dir: &Path) -> CourseStore {
        CourseStore {
            dir: data_dir.join(DIR),
        }
    }

    /// Every added course that still reads; a broken file is left out, not fatal.
    pub fn load(&self) -> Vec<CoursePack> {
        let Ok(entries) = fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        let mut packs: Vec<CoursePack> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == COURSE_EXT))
            .filter_map(|p| CoursePack::read(&fs::read(p).ok()?).ok())
            .collect();
        packs.sort_by(|a, b| a.manifest.id.cmp(&b.manifest.id));
        packs
    }

    /// Reads the package at `path` and what does not fit, before anything is added.
    pub fn preview(path: &Path) -> Result<(CoursePack, Vec<String>), CourseError> {
        let data = read(path)?;
        let pack = CoursePack::read(&data)?;
        let problems = Lessons::bundled()
            .map(|bundled| pack.problems(&bundled))
            .map_err(PackError::from)?;
        Ok((pack, problems))
    }

    /// Adds the package at `path` if it has no problems; one with the same id is replaced (an
    /// update from its provider).
    pub fn add(&self, path: &Path) -> Result<CoursePack, CourseError> {
        let (pack, problems) = CourseStore::preview(path)?;
        if !problems.is_empty() {
            return Err(CourseError::Problems(problems));
        }
        let io = |source| CourseError::Io {
            path: self.dir.clone(),
            source,
        };
        fs::create_dir_all(&self.dir).map_err(io)?;

        // Copy beside, then rename: a half-written file never replaces a working course.
        let target = self.package(&pack.manifest.id);
        let partial = target.with_extension("part");
        fs::copy(path, &partial).map_err(io)?;
        fs::rename(&partial, &target).map_err(io)?;
        Ok(pack)
    }
}

impl CourseStore {
    /// File `src` ("media/a.webm") of added course `id` as a file to play, unpacked into
    /// `cache` on first use (videos inside a package; streamed ones need no file).
    pub fn media_file(&self, id: &str, src: &str, cache: &Path) -> Result<PathBuf, CourseError> {
        // Only names a package can hold, before the cache is looked at ("media/../../x").
        let inside = src
            .split('/')
            .all(|p| !p.is_empty() && p != ".." && p != ".");
        if !inside || id.contains(['/', '.']) {
            return Err(CourseError::Pack(PackError::Archive(format!(
                "{id}: {src}"
            ))));
        }
        let target = cache.join(DIR).join(id).join(src);
        if target.is_file() {
            return Ok(target);
        }
        let data = lesson::read_media(&read(&self.package(id))?, src)?;
        let io = |source| CourseError::Io {
            path: target.clone(),
            source,
        };
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(io)?;
        }
        fs::write(&target, data).map_err(io)?;
        Ok(target)
    }

    /// The subtitles `src` ("media/a.de.vtt") of added course `id`; none if they cannot be read.
    pub fn subtitles(&self, id: &str, src: &str) -> Vec<Cue> {
        let Ok(data) = read(&self.package(id)) else {
            return Vec::new();
        };
        let Ok(bytes) = lesson::read_media(&data, src) else {
            return Vec::new();
        };
        lesson::cues(&String::from_utf8_lossy(&bytes))
    }

    fn package(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.{COURSE_EXT}"))
    }

    /// Fetches the package at `url` (https only, nothing else is asked for) on a worker thread
    /// into the data folder; `poll` hands over the file to preview and add.
    pub fn fetch(&self, http: Arc<dyn Http>, url: &str) -> Result<CourseFetch, CourseError> {
        let url = url.trim().to_string();
        if !url.starts_with(HTTPS) {
            return Err(CourseError::NotHttps);
        }
        let dir = self.dir.clone();
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let outcome = http
                .download(&url, MAX_COURSE_BYTES)
                .map_err(CourseError::from)
                .and_then(|data| keep(&dir, &data));
            let _ = tx.send(outcome);
        });
        Ok(CourseFetch { running: rx })
    }
}

/// Writes a fetched package to `dir/FETCHED`.
fn keep(dir: &Path, data: &[u8]) -> Result<PathBuf, CourseError> {
    let io = |source| CourseError::Io {
        path: dir.to_path_buf(),
        source,
    };
    fs::create_dir_all(dir).map_err(io)?;
    let path = dir.join(FETCHED);
    fs::write(&path, data).map_err(io)?;
    Ok(path)
}

fn read(path: &Path) -> Result<Vec<u8>, CourseError> {
    fs::read(path).map_err(|source| CourseError::Io {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kontra_content::lesson::{new_course, pack, Author};

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("kontra-courses-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    /// A packed new course with id `id`, written to `<dir>/<id>.kontrakurs`.
    fn packed(dir: &Path, id: &str) -> PathBuf {
        let src = dir.join(id);
        let author = Author {
            provider: "Studio Weber".into(),
            license: "CC-BY-4.0".into(),
        };
        new_course(&src, id, &author).unwrap();
        let file = dir.join(format!("{id}.{COURSE_EXT}"));
        fs::write(&file, pack(&src).unwrap()).unwrap();
        file
    }

    #[test]
    fn an_added_course_loads_and_replaces_its_older_self() {
        let dir = temp("add");
        let store = CourseStore::new(&dir.join("data"));
        assert!(store.load().is_empty());

        let file = packed(&dir, "weber");
        store.add(&file).unwrap();
        store.add(&file).unwrap();
        let loaded = store.load();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].manifest.provider, "Studio Weber");
        fs::remove_dir_all(dir).unwrap();
    }

    /// The web, answering every address with one package.
    #[derive(Debug)]
    struct Fake(Vec<u8>);

    impl Http for Fake {
        fn get(&self, _url: &str) -> Result<Vec<u8>, HttpError> {
            Ok(self.0.clone())
        }
    }

    #[test]
    fn a_course_fetched_from_an_address_can_be_added() {
        let dir = temp("fetch");
        let data = fs::read(packed(&dir, "weber")).unwrap();
        let store = CourseStore::new(&dir.join("data"));
        assert!(matches!(
            store.fetch(Arc::new(Fake(data.clone())), "http://kurs.example/w"),
            Err(CourseError::NotHttps)
        ));

        let mut fetch = store
            .fetch(Arc::new(Fake(data)), " https://kurs.example/w ")
            .unwrap();
        let path = loop {
            if let Some(outcome) = fetch.poll() {
                break outcome.unwrap();
            }
            std::thread::yield_now();
        };
        assert!(store.load().is_empty(), "fetched is not added yet");
        store.add(&path).unwrap();
        assert_eq!(store.load().len(), 1);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn media_of_an_added_course_is_unpacked_once() {
        let dir = temp("media");
        let src = dir.join("weber");
        let author = Author {
            provider: "Studio Weber".into(),
            license: "CC-BY-4.0".into(),
        };
        new_course(&src, "weber", &author).unwrap();
        fs::create_dir_all(src.join("media")).unwrap();
        fs::write(src.join("media/a.webm"), b"video").unwrap();
        fs::write(
            src.join("media/a.vtt"),
            "WEBVTT\n\n00:01.000 --> 00:02.000\nHallo\n",
        )
        .unwrap();
        let file = dir.join("weber.kontrakurs");
        fs::write(&file, pack(&src).unwrap()).unwrap();
        let store = CourseStore::new(&dir.join("data"));
        store.add(&file).unwrap();

        let cache = dir.join("cache");
        let video = store.media_file("weber", "media/a.webm", &cache).unwrap();
        assert_eq!(fs::read(&video).unwrap(), b"video");
        assert!(store.media_file("weber", "../a.webm", &cache).is_err());
        // Even where the file exists: nothing outside the cache is handed out.
        assert!(store
            .media_file("weber", "media/../../../weber.kontrakurs", &cache)
            .is_err());
        assert_eq!(store.subtitles("weber", "media/a.vtt")[0].text, "Hallo");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_course_with_problems_is_not_added() {
        let dir = temp("bad");
        let file = packed(&dir, "weber");

        // A lesson without the package prefix.
        let src = dir.join("weber");
        fs::rename(src.join("units/weber.start"), src.join("units/start")).unwrap();
        for lang in ["de", "en"] {
            let path = src.join(format!("units/start/{lang}.md"));
            let text = fs::read_to_string(&path).unwrap();
            fs::write(&path, text.replace("id: weber.start", "id: start")).unwrap();
        }
        let course = src.join("courses/weber.kurs.json");
        let text = fs::read_to_string(&course).unwrap();
        fs::write(&course, text.replace("\"weber.start\"", "\"start\"")).unwrap();
        fs::write(&file, pack(&src).unwrap()).unwrap();

        let store = CourseStore::new(&dir.join("data"));
        let Err(CourseError::Problems(problems)) = store.add(&file) else {
            panic!("expected problems");
        };
        assert_eq!(problems, ["unit start lacks the prefix weber."]);
        assert!(store.load().is_empty());
        fs::remove_dir_all(dir).unwrap();
    }
}
