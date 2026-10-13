//! Songs from outside (LH-BIB-11): search pages opened in the browser, the files the browser
//! saved to the download folder, and files dropped onto the library.

use crate::Core;
use kontra_session::{recent_scores, search_url, FileKind, SongSite};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Downloads older than this are not new any more.
const NEW_FOR: Duration = Duration::from_secs(14 * 24 * 3600);
/// Rows "New in Downloads" shows at most.
const SHOWN: usize = 5;
const MINUTE_S: u64 = 60;
const HOUR_S: u64 = 3600;
const DAY_S: u64 = 24 * HOUR_S;

impl Core {
    /// The search page of `site` ("songsterr", "musescore") for `query`; empty when there is
    /// nothing to search.
    pub fn search_url(&self, site: &str, query: &str) -> String {
        SongSite::from_code(site)
            .and_then(|s| search_url(s, query))
            .unwrap_or_default()
    }

    /// Where downloads are looked for; tests point it elsewhere.
    pub fn set_download_dir(&mut self, dir: &Path) {
        self.downloads = Some(dir.to_path_buf());
    }

    /// Scores downloaded lately that are not in the library yet, newest first:
    /// `{"dir": "…", "files": [{"path", "name", "kind", "age"}]}`.
    pub fn downloads_json(&self) -> String {
        let Some(dir) = &self.downloads else {
            return json!({ "dir": "", "files": [] }).to_string();
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let files: Vec<_> = recent_scores(dir, NEW_FOR)
            .into_iter()
            .filter(|f| self.library.entry(&f.path).is_none())
            .take(SHOWN)
            .map(|f| {
                let name = f
                    .path
                    .file_name()
                    .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
                json!({
                    "path": f.path.to_string_lossy(),
                    "name": name,
                    "kind": self.kind_label(&f.path),
                    "age": self.age_label(now.saturating_sub(f.modified_unix)),
                })
            })
            .collect();
        json!({ "dir": dir.to_string_lossy(), "files": files }).to_string()
    }

    /// Adds dropped files to the library without playing them; like `import_folder` it returns
    /// how many were added and, per failed file, its name and the reason.
    pub fn import_files(&mut self, files: &[PathBuf]) -> (usize, Vec<(String, String)>) {
        let (added, failed) = self.add_files(files);
        self.save_library();
        (added.len(), failed)
    }

    /// What the file is, in words a beginner knows from the site (P1 does not know formats).
    fn kind_label(&self, path: &Path) -> String {
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default();
        let key = match FileKind::from_extension(extension) {
            Some(FileKind::GuitarPro | FileKind::GuitarPro6 | FileKind::GuitarPro7) => {
                "search.kind.guitarpro"
            }
            Some(FileKind::Midi) => "search.kind.midi",
            Some(FileKind::MusicXml | FileKind::MusicXmlZip) => "search.kind.musicxml",
            Some(FileKind::MuseScore | FileKind::MuseScoreZip) => "search.kind.musescore",
            None => "search.kind.package",
        };
        self.tr(key, &[])
    }

    /// "just now", "5 min ago", "3 h ago", "2 days ago".
    fn age_label(&self, seconds: u64) -> String {
        if seconds < MINUTE_S {
            return self.tr("search.age.now", &[]);
        }
        let (key, n) = match seconds {
            s if s < HOUR_S => ("search.age.minutes", s / MINUTE_S),
            s if s < DAY_S => ("search.age.hours", s / HOUR_S),
            s => ("search.age.days", s / DAY_S),
        };
        self.tr(key, &[("n", n.to_string())])
    }
}
