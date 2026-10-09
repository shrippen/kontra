//! Settings and library on disk (LH-ALL-03, LH-FOR-03): JSON in the data directory.

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::path::{Path, PathBuf};

const RECENT_MAX: usize = 50;

/// `KONTRA_DATA_DIR`, else `$XDG_DATA_HOME/kontra`, else `~/.local/share/kontra`.
pub fn data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("KONTRA_DATA_DIR") {
        return PathBuf::from(dir);
    }
    if let Some(dir) = std::env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(dir).join("kontra");
    }
    let home = std::env::var_os("HOME").map_or_else(|| PathBuf::from("."), PathBuf::from);
    home.join(".local/share/kontra")
}

/// Reads JSON or returns the default (a damaged file is kept aside as `.broken`).
pub fn load<T: DeserializeOwned + Default>(path: &Path) -> T {
    let Ok(text) = std::fs::read_to_string(path) else {
        return T::default();
    };
    match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(_) => {
            let _ = std::fs::rename(path, path.with_extension("json.broken"));
            T::default()
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputKind {
    /// Interface or Hi-Z instrument cable.
    Line,
    /// Laptop combo jack.
    LaptopJack,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub language: Option<String>,
    pub strings: u8,
    pub fretless: bool,
    pub input_kind: InputKind,
    pub input_device: Option<String>,
    pub output_device: Option<String>,
    pub latency_ms: f64,
    pub metronome: bool,
    pub reference_tones: bool,
    pub count_in_bars: u32,
    pub view: String,
    pub mode: String,
    /// "kante", "kante-light" or "system".
    pub style: String,
    pub low_string_on_top: bool,
    /// Note marks in play mode: "quiet" (default), "off" or "full". Other modes always mark fully.
    pub play_marks: String,
    /// Timing window (LH-BEW-02): "auto" (by input), "loose", "normal" or "strict".
    pub strictness: String,
    /// Pitch limit in cents (LH-BEW-05); 0 = by bass kind (50 fretted, 30 fretless).
    pub pitch_cents: f64,
    /// The first-start assistant was completed or skipped (LH-EIN-01).
    pub setup_done: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            language: None,
            strings: 4,
            fretless: false,
            input_kind: InputKind::Line,
            input_device: None,
            output_device: None,
            latency_ms: 0.0,
            metronome: true,
            reference_tones: false,
            count_in_bars: 1,
            view: "lane".into(),
            mode: "play".into(),
            style: "kante".into(),
            low_string_on_top: false,
            play_marks: "quiet".into(),
            strictness: "auto".into(),
            pitch_cents: 0.0,
            setup_done: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecentEntry {
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    pub track: usize,
    pub last_unix: u64,
    /// Backing track for this piece (LH-WIE-01) and where the score starts in it.
    /// Progress key (`ProgressStore::song_id`).
    #[serde(default)]
    pub song_id: String,
    #[serde(default)]
    pub backing: Option<PathBuf>,
    #[serde(default)]
    pub backing_offset_s: f64,
    /// View last used for this piece (LH-ANZ-04); `None`: the default from the settings.
    #[serde(default)]
    pub view: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Library {
    pub recent: Vec<RecentEntry>,
}

impl Library {
    /// Moves or adds `entry` to the top; keeps a backing track chosen earlier.
    pub fn touch(&mut self, mut entry: RecentEntry) {
        if let Some(old) = self.recent.iter().find(|e| e.path == entry.path) {
            if entry.backing.is_none() {
                entry.backing = old.backing.clone();
                entry.backing_offset_s = old.backing_offset_s;
            }
            if entry.view.is_none() {
                entry.view = old.view.clone();
            }
        }
        self.recent.retain(|e| e.path != entry.path);
        self.recent.insert(0, entry);
        self.recent.truncate(RECENT_MAX);
    }

    pub fn entry_mut(&mut self, path: &Path) -> Option<&mut RecentEntry> {
        self.recent.iter_mut().find(|e| e.path == path)
    }

    pub fn entry(&self, path: &Path) -> Option<&RecentEntry> {
        self.recent.iter().find(|e| e.path == path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_broken_file() {
        let dir = std::env::temp_dir().join(format!("kontra-store-{}", std::process::id()));
        let path = dir.join("settings.json");
        let settings = Settings {
            strings: 5,
            fretless: true,
            ..Settings::default()
        };
        save(&path, &settings).unwrap();
        assert_eq!(load::<Settings>(&path), settings);
        std::fs::write(&path, "{ broken").unwrap();
        assert_eq!(load::<Settings>(&path), Settings::default());
        assert!(path.with_extension("json.broken").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn library_keeps_latest_first() {
        let mut library = Library::default();
        let entry = |p: &str| RecentEntry {
            path: p.into(),
            title: p.into(),
            artist: String::new(),
            track: 0,
            last_unix: 0,
            song_id: String::new(),
            backing: None,
            backing_offset_s: 0.0,
            view: None,
        };
        library.touch(entry("a"));
        library.touch(entry("b"));
        library.touch(entry("a"));
        assert_eq!(
            library
                .recent
                .iter()
                .map(|e| e.title.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b"]
        );
    }
}
