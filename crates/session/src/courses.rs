//! Courses added by the player (LH-LRN-14): `.kontrakurs` packages kept as they came, one file
//! per package id in the data folder.
//!
//! ```text
//! kurs.kontrakurs ──preview()──▶ manifest, problems   (shown before adding)
//!                 ──add()──────▶ <data>/courses/<id>.kontrakurs ──load()──▶ [CoursePack]
//! ```

use kontra_content::lesson::{CoursePack, Lessons, PackError, COURSE_EXT};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

const DIR: &str = "courses";

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
        let target = self.dir.join(format!("{}.{COURSE_EXT}", pack.manifest.id));
        let partial = target.with_extension("part");
        fs::copy(path, &partial).map_err(io)?;
        fs::rename(&partial, &target).map_err(io)?;
        Ok(pack)
    }
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
