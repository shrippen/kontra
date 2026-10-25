//! Settings and library on disk (LH-ALL-03, LH-FOR-03): JSON in the data directory.

use kontra_model::A4_HZ;
use kontra_session::Gains;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Pieces the library keeps (LH-BIB-04 asks for 200 with fast search).
const RECENT_MAX: usize = 2000;
/// Monitoring volume when first switched on.
const MONITOR_VOLUME: f64 = 0.8;
const PROMPT_VOLUME: f64 = 0.8;
/// Free timing window before the user picks one (LH-BEW-02): as "normal".
const WINDOW_DEFAULT_MS: f64 = 60.0;

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

/// Files Kontra can make again (separated songs, LH-BIB-09): `$XDG_CACHE_HOME/kontra`, else
/// `~/.cache/kontra`, so backups skip them. A data folder of its own (`KONTRA_DATA_DIR`, tests)
/// keeps them inside, in `cache/`.
pub fn cache_dir(data: &Path) -> PathBuf {
    if data != data_dir() || std::env::var_os("KONTRA_DATA_DIR").is_some() {
        return data.join("cache");
    }
    if let Some(dir) = std::env::var_os("XDG_CACHE_HOME") {
        return PathBuf::from(dir).join("kontra");
    }
    let home = std::env::var_os("HOME").map_or_else(|| PathBuf::from("."), PathBuf::from);
    home.join(".cache/kontra")
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
    /// MIDI keyboard playing the bass line (LH-EIN-12).
    Midi,
    /// Microphone in the room, e.g. at a double bass (LH-EIN-11).
    Microphone,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub language: Option<String>,
    pub strings: u8,
    pub fretless: bool,
    pub input_kind: InputKind,
    pub input_device: Option<String>,
    /// MIDI device by name when `input_kind` is MIDI (port ids change when it is plugged in
    /// again); `None` or not connected: the first one.
    pub midi_port: Option<String>,
    pub output_device: Option<String>,
    pub latency_ms: f64,
    pub metronome: bool,
    pub reference_tones: bool,
    pub count_in_bars: u32,
    pub view: String,
    pub mode: String,
    /// The mode last chosen for an own piece; plan exercises open in "song" (round 12).
    pub piece_mode: Option<String>,
    /// "kante", "kante-light" or "system".
    pub style: String,
    pub low_string_on_top: bool,
    /// Note marks in play mode: "quiet" (default), "off" or "full". Other modes always mark fully.
    pub play_marks: String,
    /// Motion: "system" (follow the desktop's animation setting), "reduce" or "full".
    pub motion: String,
    /// Timing window (LH-BEW-02): "auto" (by input), "loose", "normal" or "strict".
    pub strictness: String,
    /// Aim against the beat (LH-BEW-08): "free", "onbeat", "laidback" or "pushed".
    pub pocket: String,
    /// The course to learn (LH-LRN-14); `None` or gone: the first for the instrument.
    pub course: Option<String>,
    /// Pitch limit in cents (LH-BEW-05); 0 = by bass kind (50 fretted, 30 fretless).
    pub pitch_cents: f64,
    /// The first-start assistant was completed or skipped (LH-EIN-01).
    pub setup_done: bool,
    /// Mixer volumes, 0..=1 (LH-WIE-03, LH-MOD-05).
    pub backing_volume: f64,
    pub metronome_volume: f64,
    pub tone_volume: f64,
    /// Drum accompaniment of grooves and imported drum tracks (LH-LRN-01).
    pub drums_volume: f64,
    /// The original bass in a backing track played without it (LH-BIB-09); 0 = removed.
    pub bass_volume: f64,
    /// Ear training prompts (LH-LRN-03, round 8: its own slider, not the reference tone's).
    pub prompt_volume: f64,
    /// Play the bass through Kontra (input monitoring); off: the interface monitors directly.
    pub monitor: bool,
    pub monitor_volume: f64,
    /// Tone shaping of the monitored bass (LH-EIN-08): bands in dB, compressor and warmth 0..=1.
    pub monitor_low_db: f64,
    pub monitor_mid_db: f64,
    pub monitor_high_db: f64,
    pub monitor_compress: f64,
    pub monitor_warmth: f64,
    /// Suggest more or fewer notes between runs (LH-MOD-06).
    pub adaptive: bool,
    /// Tuning (LH-TUN-02): "standard", "drop" (lowest string a whole step down),
    /// "half-down", "whole-down" or "custom".
    pub tuning: String,
    /// Open strings of the custom tuning, lowest first, e.g. `["C1", "G1", "C2", "F2"]`.
    pub custom_tuning: Vec<String>,
    /// Reference pitch of A4 in Hz (LH-TUN-04), 430..=450.
    pub a4_hz: f64,
    /// Timing window half width in ms when `strictness` is "custom" (LH-BEW-02).
    pub window_ms: f64,
    /// String colours (LH-ANZ-06): "kante" or a preset name the UI knows.
    pub string_colors: String,
    /// Mirrored fretboard, nut on the right (LH-ANZ-08).
    pub left_handed: bool,
}

impl Default for Settings {
    fn default() -> Self {
        let mix = Gains::default();
        Settings {
            language: None,
            strings: 4,
            fretless: false,
            input_kind: InputKind::Line,
            input_device: None,
            midi_port: None,
            output_device: None,
            latency_ms: 0.0,
            metronome: true,
            reference_tones: false,
            count_in_bars: 1,
            view: "lane".into(),
            mode: "play".into(),
            piece_mode: None,
            style: "kante".into(),
            low_string_on_top: false,
            play_marks: "quiet".into(),
            motion: "system".into(),
            strictness: "auto".into(),
            pocket: "free".into(),
            course: None,
            pitch_cents: 0.0,
            setup_done: false,
            backing_volume: f64::from(mix.backing),
            metronome_volume: f64::from(mix.click),
            tone_volume: f64::from(mix.tone),
            drums_volume: f64::from(mix.drums),
            bass_volume: f64::from(mix.bass),
            prompt_volume: PROMPT_VOLUME,
            monitor: false,
            monitor_volume: MONITOR_VOLUME,
            monitor_low_db: 0.0,
            monitor_mid_db: 0.0,
            monitor_high_db: 0.0,
            monitor_compress: 0.0,
            monitor_warmth: 0.0,
            adaptive: true,
            tuning: "standard".into(),
            custom_tuning: Vec::new(),
            a4_hz: A4_HZ,
            window_ms: WINDOW_DEFAULT_MS,
            string_colors: "kante".into(),
            left_handed: false,
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
    /// Own tags for search (LH-BIB-04), e.g. "Band", "Slap".
    #[serde(default)]
    pub tags: Vec<String>,
    /// Bars of the sections practised, so progress can name them (LH-FOR-01).
    #[serde(default)]
    pub sections: Vec<SectionBars>,
    /// Note density per section (LH-MOD-06); none stored: all notes.
    #[serde(default)]
    pub densities: Vec<SectionDensity>,
    /// Piece and backing track moved by semitones (LH-WIE-02), −6..=6.
    #[serde(default)]
    pub transpose_semitones: i32,
    /// Play the backing track with its bass removed (LH-BIB-09), once separated.
    #[serde(default)]
    pub backing_without_bass: bool,
}

/// Note density chosen for a section (`None`: the whole piece), 1 (few notes) to 3 (all).
/// A fixed one gets no suggestions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SectionDensity {
    pub section: Option<(f64, f64)>,
    pub level: u8,
    pub fixed: bool,
}

/// A practised section: its time range (as a progress run stores it) and its bars as written.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SectionBars {
    pub start_s: f64,
    pub end_s: f64,
    pub from_bar: u32,
    pub to_bar: u32,
}

/// Where the bars of a piece fall in its backing track (LH-BIB-06): the detected beats, which
/// of them is the first downbeat, and the user's corrections.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BeatMap {
    /// Beat times in the audio, seconds.
    pub beats: Vec<f64>,
    pub bpm: f64,
    /// Index into `beats` of bar 1's downbeat.
    pub first: usize,
    /// Shift of every bar, ms (fine alignment).
    pub shift_ms: f64,
    /// Per-bar corrections, ms: bar index → shift of that bar's start.
    pub nudges: std::collections::BTreeMap<usize, f64>,
}

impl BeatMap {
    /// Audio time of each bar start, for bars of `beats_per_bar` beats; as far as beats reach.
    pub fn bar_times(&self, beats_per_bar: &[u32]) -> Vec<f64> {
        let mut out = Vec::new();
        let mut beat = self.first;
        for (bar, n) in beats_per_bar.iter().enumerate() {
            let Some(t) = self.beats.get(beat) else {
                break;
            };
            let nudge = self.nudges.get(&bar).copied().unwrap_or(0.0);
            out.push(t + (self.shift_ms + nudge) / 1000.0);
            beat += (*n).max(1) as usize;
        }
        out
    }
}

/// A named order of pieces for a gig or rehearsal (LH-BIB-04).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Setlist {
    pub name: String,
    pub pieces: Vec<PathBuf>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Library {
    /// Every known piece; the most recently played first, imported but unplayed ones after.
    pub recent: Vec<RecentEntry>,
    /// Backing-track beat maps per piece (LH-BIB-06).
    pub beat_maps: std::collections::BTreeMap<PathBuf, BeatMap>,
    pub setlists: Vec<Setlist>,
}

impl Library {
    /// Moves or adds `entry` to the top; keeps a backing track chosen earlier.
    pub fn touch(&mut self, mut entry: RecentEntry) {
        entry.path = plain(&entry.path);
        if let Some(old) = self.recent.iter().find(|e| e.path == entry.path) {
            if entry.backing.is_none() {
                entry.backing = old.backing.clone();
                entry.backing_offset_s = old.backing_offset_s;
            }
            if entry.view.is_none() {
                entry.view = old.view.clone();
            }
            if entry.tags.is_empty() {
                entry.tags = old.tags.clone();
            }
            if entry.sections.is_empty() {
                entry.sections = old.sections.clone();
            }
            if entry.densities.is_empty() {
                entry.densities = old.densities.clone();
            }
            if entry.transpose_semitones == 0 {
                entry.transpose_semitones = old.transpose_semitones;
            }
            entry.backing_without_bass |= old.backing_without_bass;
        }
        self.recent.retain(|e| e.path != entry.path);
        self.recent.insert(0, entry);
        self.recent.truncate(RECENT_MAX);
    }

    /// Adds a piece without playing it (folder import): after the played ones, nothing moves.
    pub fn add_known(&mut self, mut entry: RecentEntry) {
        entry.path = plain(&entry.path);
        if self.entry(&entry.path).is_none() && self.recent.len() < RECENT_MAX {
            self.recent.push(entry);
        }
    }

    /// Paths through ".." (bundled files before 13.10.: `bin/../share`, `appcore/../..`) become
    /// plain, as the plan names them now; a duplicate this makes keeps the newer entry, which
    /// comes first. Other paths stay as stored (a symlink the user chose stays).
    pub fn plain_paths(&mut self) {
        for entry in &mut self.recent {
            entry.path = plain(&entry.path);
        }
        let mut seen = std::collections::HashSet::new();
        self.recent.retain(|e| seen.insert(e.path.clone()));
        for setlist in &mut self.setlists {
            for piece in &mut setlist.pieces {
                *piece = plain(piece);
            }
        }
    }

    /// Bundled pieces whose level folder under `base` was renamed (`moves`: old, new folder
    /// name) get their new path in the library, setlists and beat maps; progress follows the
    /// song id, so it stays. Applied on every load, a no-op once moved.
    pub fn move_folders(&mut self, base: &Path, moves: &[(&str, &str)]) {
        let moved = |path: &Path| -> Option<PathBuf> {
            let folder = path.parent()?;
            if folder.parent()? != base {
                return None;
            }
            let name = folder.file_name()?.to_str()?;
            let (_, new) = moves.iter().find(|(old, _)| *old == name)?;
            Some(base.join(new).join(path.file_name()?))
        };
        for entry in &mut self.recent {
            if let Some(path) = moved(&entry.path) {
                entry.path = path;
            }
        }
        let mut seen = std::collections::HashSet::new();
        self.recent.retain(|e| seen.insert(e.path.clone()));
        for setlist in &mut self.setlists {
            for piece in &mut setlist.pieces {
                if let Some(path) = moved(piece) {
                    *piece = path;
                }
            }
        }
        let maps = std::mem::take(&mut self.beat_maps);
        self.beat_maps = maps
            .into_iter()
            .map(|(path, map)| (moved(&path).unwrap_or(path), map))
            .collect();
    }

    pub fn setlist_mut(&mut self, name: &str) -> Option<&mut Setlist> {
        self.setlists.iter_mut().find(|s| s.name == name)
    }

    pub fn entry_mut(&mut self, path: &Path) -> Option<&mut RecentEntry> {
        let path = plain(path);
        self.recent.iter_mut().find(|e| e.path == path)
    }

    pub fn entry(&self, path: &Path) -> Option<&RecentEntry> {
        let path = plain(path);
        self.recent.iter().find(|e| e.path == path)
    }
}

/// `path` without ".." if it has some and exists; else as it is.
fn plain(path: &Path) -> PathBuf {
    let dotted = path
        .components()
        .any(|c| c == std::path::Component::ParentDir);
    if !dotted {
        return path.to_path_buf();
    }
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
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

    /// Files from before the mixer load with its defaults: monitoring off, volumes as before.
    #[test]
    fn old_settings_get_mixer_defaults() {
        let dir = std::env::temp_dir().join(format!("kontra-store-old-{}", std::process::id()));
        let path = dir.join("settings.json");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&path, r#"{ "strings": 5, "metronome": false }"#).unwrap();
        let settings: Settings = load(&path);
        assert_eq!(settings.strings, 5);
        assert!(!settings.monitor);
        assert_eq!(settings.backing_volume, f64::from(Gains::default().backing));
        assert_eq!(settings.metronome_volume, f64::from(Gains::default().click));
        assert_eq!(settings.tone_volume, f64::from(Gains::default().tone));
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// Files from before tunings, reference pitch and display options load with defaults.
    #[test]
    fn old_settings_get_tuning_defaults() {
        let dir = std::env::temp_dir().join(format!("kontra-store-tun-{}", std::process::id()));
        let path = dir.join("settings.json");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&path, r#"{ "strings": 5, "strictness": "strict" }"#).unwrap();
        let settings: Settings = load(&path);
        assert_eq!(settings.strings, 5);
        assert_eq!(settings.strictness, "strict");
        assert_eq!(settings.tuning, "standard");
        assert!(settings.custom_tuning.is_empty());
        assert_eq!(settings.a4_hz, A4_HZ);
        assert_eq!(settings.window_ms, WINDOW_DEFAULT_MS);
        assert_eq!(settings.string_colors, "kante");
        assert!(!settings.left_handed);
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// Entries stored before 13.10. name bundled files through "../.."; they keep their place
    /// and progress under the plain path, a duplicate collapses into the newer one.
    #[test]
    fn library_paths_become_plain() {
        let dir = std::env::temp_dir().join(format!("kontra-store-plain-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("a")).unwrap();
        let file = dir.join("f.mid");
        std::fs::write(&file, b"MThd").unwrap();
        let plain = std::fs::canonicalize(&file).unwrap();
        let old = dir.join("a/../f.mid");
        let text = serde_json::json!({
            "recent": [
                { "path": plain, "title": "new", "artist": "", "track": 0, "last_unix": 2 },
                { "path": old, "title": "old", "artist": "", "track": 0, "last_unix": 1 },
            ],
            "setlists": [{ "name": "Probe", "pieces": [old] }],
        });
        let path = dir.join("library.json");
        std::fs::write(&path, text.to_string()).unwrap();

        let mut library: Library = load(&path);
        library.plain_paths();
        assert_eq!(library.recent.len(), 1);
        assert_eq!(library.recent[0].title, "new");
        assert_eq!(library.setlists[0].pieces, vec![plain]);
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
            tags: Vec::new(),
            sections: Vec::new(),
            densities: Vec::new(),
            transpose_semitones: 0,
            backing_without_bass: false,
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
