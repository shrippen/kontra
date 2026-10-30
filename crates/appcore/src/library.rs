//! Library: recent pieces, tags, folder import and setlists (LH-BIB-04).

use crate::{piece_name, store, Core, Opened, RecentEntry, Setlist};
use kontra_session::{import_file, ProgressStore};
use serde_json::json;
use std::path::{Path, PathBuf};

impl Core {
    pub fn recent_json(&self) -> String {
        let store = self.progress_store();
        let list: Vec<_> = self
            .library
            .recent
            .iter()
            .map(|e| {
                let progress = store.load_id(&e.song_id);
                let best = progress.as_ref().and_then(|p| p.best_hit_rate());
                let minutes = progress.as_ref().map_or(0.0, |p| p.practice_s() / 60.0);
                let title = self.library_title(e);
                json!({ "path": e.path, "title": title, "artist": e.artist, "best": best, "practiceMin": minutes, "lastUnix": e.last_unix, "tags": e.tags, "hasBacking": e.backing.is_some() })
            })
            .collect();
        serde_json::Value::Array(list).to_string()
    }

    /// Sets the tags of the piece at `path` (LH-BIB-04): trimmed, without empty or repeated ones.
    pub fn set_tags(&mut self, path: &Path, tags: &[String]) {
        let mut clean: Vec<String> = Vec::new();
        for tag in tags.iter().map(|t| t.trim()) {
            if !tag.is_empty() && !clean.iter().any(|c| c.eq_ignore_ascii_case(tag)) {
                clean.push(tag.to_string());
            }
        }
        let Some(entry) = self.library.entry_mut(path) else {
            return;
        };
        entry.tags = clean;
        self.save_library();
    }

    /// Every tag in the library, sorted, for suggestions.
    pub fn tags_json(&self) -> String {
        let mut tags: Vec<&String> = self.library.recent.iter().flat_map(|e| &e.tags).collect();
        tags.sort_by_key(|t| t.to_lowercase());
        tags.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
        json!(tags).to_string()
    }

    /// Adds every score in `dir` to the library without playing it (LH-BIB-04, J-B1); with
    /// `as_setlist` also a setlist named after the folder, in file name order. Returns how many
    /// were added and, for each file that failed, its name and the reason in the user's language.
    pub fn import_folder(
        &mut self,
        dir: &Path,
        as_setlist: bool,
    ) -> (usize, Vec<(String, String)>) {
        const EXTENSIONS: [&str; 11] = [
            "gp", "gp3", "gp4", "gp5", "gpx", "mid", "midi", "musicxml", "mxl", "mscz", "mscx",
        ];
        let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
            .map(|entries| entries.filter_map(Result::ok).map(|e| e.path()).collect())
            .unwrap_or_default();
        files.retain(|p| {
            p.extension().and_then(|e| e.to_str()).is_some_and(|e| {
                EXTENSIONS.contains(&e.to_ascii_lowercase().as_str())
                    || e.eq_ignore_ascii_case("xml")
            })
        });
        files.sort();
        let (added, failed) = self.add_files(&files);
        if as_setlist && !added.is_empty() {
            let name = dir
                .file_name()
                .map_or_else(|| "Setlist".into(), |n| n.to_string_lossy().into());
            self.library.setlists.retain(|s| s.name != name);
            self.library.setlists.push(Setlist {
                name,
                pieces: added.clone(),
            });
        }
        self.save_library();
        (added.len(), failed)
    }

    /// Adds scores to the library without playing them; returns the added paths and, per
    /// failed file, its name and the reason in the user's language.
    pub(crate) fn add_files(&mut self, files: &[PathBuf]) -> (Vec<PathBuf>, Vec<(String, String)>) {
        let mut added = Vec::new();
        let mut failed = Vec::new();
        for path in files.iter().cloned() {
            let name = path
                .file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into());
            let arrangement = import_file(&path)
                .map_err(|e| self.import_error(&e))
                .and_then(|i| {
                    i.arrangement(i.suggested().unwrap_or(0))
                        .map_err(|e| self.import_error(&e))
                });
            match arrangement {
                Ok(a) => {
                    let title = piece_name(&a.title, &path);
                    self.library.add_known(RecentEntry {
                        path: path.clone(),
                        title,
                        artist: a.artist.clone(),
                        track: 0,
                        last_unix: 0,
                        song_id: ProgressStore::song_id(&a),
                        backing: None,
                        backing_offset_s: 0.0,
                        view: None,
                        tags: Vec::new(),
                        sections: Vec::new(),
                        densities: Vec::new(),
                        transpose_semitones: 0,
                        backing_without_bass: false,
                        loops: Vec::new(),
                        target_speed: None,
                    });
                    added.push(path);
                }
                Err(reason) => failed.push((name, reason)),
            }
        }
        (added, failed)
    }

    pub fn setlist_create(&mut self, name: &str) {
        let name = name.trim();
        if !name.is_empty() && self.library.setlist_mut(name).is_none() {
            self.library.setlists.push(Setlist {
                name: name.to_string(),
                pieces: Vec::new(),
            });
            self.save_library();
        }
    }

    pub fn setlist_delete(&mut self, name: &str) {
        self.library.setlists.retain(|s| s.name != name);
        self.save_library();
    }

    /// Adds the loaded piece to the end of `name`.
    pub fn setlist_add_loaded(&mut self, name: &str) {
        let Some(path) = self.loaded_path().map(Path::to_path_buf) else {
            return;
        };
        if let Some(list) = self.library.setlist_mut(name) {
            list.pieces.push(path);
            self.save_library();
        }
    }

    /// Adds a piece of the library to a setlist without opening it (QA 14.10., P2: only the
    /// loaded piece could be added, so every song had to be opened first).
    pub fn setlist_add(&mut self, name: &str, path: &Path) {
        let Some(path) = self.library.entry(path).map(|e| e.path.clone()) else {
            return;
        };
        if let Some(list) = self.library.setlist_mut(name) {
            list.pieces.push(path);
            self.save_library();
        }
    }

    pub fn setlist_remove(&mut self, name: &str, index: usize) {
        if let Some(list) = self.library.setlist_mut(name) {
            if index < list.pieces.len() {
                list.pieces.remove(index);
                self.save_library();
            }
        }
    }

    /// Moves the piece at `from` one place up (`-1`) or down (`1`).
    pub fn setlist_move(&mut self, name: &str, from: usize, step: i32) {
        if let Some(list) = self.library.setlist_mut(name) {
            let to = from as i64 + i64::from(step);
            if from < list.pieces.len() && (0..list.pieces.len() as i64).contains(&to) {
                list.pieces.swap(from, to as usize);
                self.save_library();
            }
        }
    }

    /// Opens piece `index` of setlist `name` and remembers the position for `next_in_setlist`.
    pub fn open_from_setlist(&mut self, name: &str, index: usize) -> Result<Opened, String> {
        let path = self
            .library
            .setlists
            .iter()
            .find(|s| s.name == name)
            .and_then(|s| s.pieces.get(index))
            .cloned()
            .ok_or("no such piece")?;
        let opened = self.open(&path)?;
        self.active_setlist = Some((name.to_string(), index));
        Ok(opened)
    }

    /// Title of the next piece in the active setlist, if any.
    pub fn next_title(&self) -> Option<String> {
        let (name, index) = self.active_setlist.as_ref()?;
        let list = self.library.setlists.iter().find(|s| &s.name == name)?;
        let path = list.pieces.get(index + 1)?;
        Some(self.piece_title(path))
    }

    /// A setlist piece's title: a bundled exercise as in the plan, else the library's title or
    /// the file name.
    fn piece_title(&self, path: &Path) -> String {
        self.plan_title(path).unwrap_or_else(|| {
            self.library.entry(path).map_or_else(
                || {
                    path.file_stem()
                        .map_or_else(String::new, |s| s.to_string_lossy().into())
                },
                |e| e.title.clone(),
            )
        })
    }

    /// Opens the next piece of the active setlist.
    pub fn open_next(&mut self) -> Result<Opened, String> {
        let (name, index) = self.active_setlist.clone().ok_or("no setlist")?;
        self.open_from_setlist(&name, index + 1)
    }

    pub fn setlists_json(&self) -> String {
        let lists: Vec<serde_json::Value> = self
            .library
            .setlists
            .iter()
            .map(|l| {
                let pieces: Vec<serde_json::Value> = l
                    .pieces
                    .iter()
                    .map(|p| {
                        let e = self.library.entry(p);
                        json!({
                            "path": p,
                            "title": self.piece_title(p),
                            "artist": e.map_or_else(String::new, |e| e.artist.clone()),
                            "missing": !p.exists(),
                        })
                    })
                    .collect();
                json!({ "name": l.name, "pieces": pieces })
            })
            .collect();
        serde_json::Value::Array(lists).to_string()
    }

    pub(crate) fn save_library(&self) {
        let _ = store::save(&self.data_dir.join("library.json"), &self.library);
    }
}
