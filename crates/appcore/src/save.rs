//! Library and settings on disk: reading JSON files and the one place that writes them.

use crate::store::Settings;
use crate::{Core, Library, UiEvent};
use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;

/// File names in the data directory.
pub(crate) const LIBRARY_FILE: &str = "library.json";
pub(crate) const SETTINGS_FILE: &str = "settings.json";

/// Whether a file read as it should.
#[derive(PartialEq)]
enum Disk {
    Fine,
    /// Kept aside as `.broken`.
    Damaged,
}

/// Reads JSON or returns the default (a damaged file is kept aside as `.broken`).
pub fn load<T: DeserializeOwned + Default>(path: &Path) -> T {
    read(path).0
}

/// Settings and library of `dir`, and the names of the files that were damaged.
pub(crate) fn load_data(dir: &Path) -> (Settings, Library, Vec<&'static str>) {
    let mut damaged = Vec::new();
    let settings = load_named(dir, SETTINGS_FILE, &mut damaged);
    let library = load_named(dir, LIBRARY_FILE, &mut damaged);
    (settings, library, damaged)
}

/// As `load` for `name` in `dir`; a damaged file adds `name` to `damaged`.
fn load_named<T: DeserializeOwned + Default>(
    dir: &Path,
    name: &'static str,
    damaged: &mut Vec<&'static str>,
) -> T {
    let (value, disk) = read(&dir.join(name));
    if disk == Disk::Damaged {
        damaged.push(name);
    }
    value
}

/// Reads JSON or returns the default; leaves a damaged file where it is.
pub(crate) fn peek<T: DeserializeOwned + Default>(path: &Path) -> T {
    parse(path).and_then(Result::ok).unwrap_or_default()
}

/// None for a file that is not there.
fn parse<T: DeserializeOwned>(path: &Path) -> Option<serde_json::Result<T>> {
    let text = std::fs::read_to_string(path).ok()?;
    Some(serde_json::from_str(&text))
}

fn read<T: DeserializeOwned + Default>(path: &Path) -> (T, Disk) {
    match parse(path) {
        None => (T::default(), Disk::Fine),
        Some(Ok(value)) => (value, Disk::Fine),
        Some(Err(_)) => {
            let _ = std::fs::rename(path, path.with_extension("json.broken"));
            (T::default(), Disk::Damaged)
        }
    }
}

/// Writes JSON atomically.
pub fn save<T: Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(value).map_err(std::io::Error::other)?;
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, text)?;
    std::fs::rename(temp, path)
}

impl Core {
    pub(crate) fn save_library(&mut self) {
        let saved = save(&self.data_dir.join(LIBRARY_FILE), &self.library);
        self.tell_save(LIBRARY_FILE, saved);
    }

    pub(crate) fn save_settings(&mut self) {
        let saved = save(&self.data_dir.join(SETTINGS_FILE), &self.settings);
        self.tell_save(SETTINGS_FILE, saved);
    }

    /// The notice, if there is one, for the next tick: a failed save or a damaged file at start
    /// (an open that nobody took leaves its notice too).
    pub(crate) fn notice_event(&mut self) -> Option<UiEvent> {
        self.take_notice().map(UiEvent::Message)
    }

    /// A failed save becomes a notice; the app goes on with what is in memory.
    fn tell_save(&mut self, file: &str, saved: std::io::Result<()>) {
        let Err(error) = saved else {
            return;
        };
        let args = [("file", file.to_string()), ("reason", error.to_string())];
        let text = self.tr("save.failed", &args);
        self.notice.push_back(text);
    }

    /// What `new` changed while loading: pieces moved (the next start must not find the old
    /// paths in the file) and plugins switched off after a crash; and a notice for files that
    /// were damaged.
    pub(crate) fn save_after_start(&mut self, moved_pieces: bool, damaged: &[&str]) {
        if moved_pieces {
            self.save_library();
        }
        if !self.plugins.crashed.is_empty() {
            self.save_settings();
        }
        if damaged.is_empty() {
            return;
        }
        let text = self.tr("save.broken", &[("files", damaged.join(", "))]);
        self.notice.push_back(text);
    }
}
