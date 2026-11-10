//! The user's folders: where the browser saves downloads and which files arrived there lately.
//!
//! ```text
//! $XDG_DOWNLOAD_DIR ─┐
//! user-dirs.dirs ────┼─→ download_dir() ─→ recent_files(dir, max_age) ─→ newest first
//! $HOME/Downloads ───┘
//!
//! Windows: %USERPROFILE%\Downloads ─→ download_dir()   (a moved folder: known folders, Win4)
//!
//!            data_home()                         cache_home()
//! Unix       $XDG_DATA_HOME, ~/.local/share      $XDG_CACHE_HOME, ~/.cache
//! Windows    %APPDATA% (roams with the user)     %LOCALAPPDATA% (stays on the machine)
//! ```

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(unix)]
const DOWNLOAD_KEY: &str = "XDG_DOWNLOAD_DIR";
#[cfg(unix)]
const USER_DIRS: &str = "user-dirs.dirs";
const FALLBACK: &str = "Downloads";
#[cfg(unix)]
const HOME_VARIABLE: &str = "$HOME";

/// The systems whose folders differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum System {
    Unix,
    Windows,
}

impl System {
    const HERE: System = if cfg!(windows) {
        System::Windows
    } else {
        System::Unix
    };
}

/// Where programs keep the user's data (settings, library, progress); `None` without a home.
pub fn data_home() -> Option<PathBuf> {
    data_home_in(System::HERE, &|key| std::env::var_os(key))
}

/// Where programs keep what they can make again (prepared audio); `None` without a home.
pub fn cache_home() -> Option<PathBuf> {
    cache_home_in(System::HERE, &|key| std::env::var_os(key))
}

fn data_home_in(system: System, env: &dyn Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    match system {
        System::Unix => folder(env, "XDG_DATA_HOME").or_else(|| below_home(env, ".local/share")),
        System::Windows => folder(env, "APPDATA"),
    }
}

fn cache_home_in(system: System, env: &dyn Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    match system {
        System::Unix => folder(env, "XDG_CACHE_HOME").or_else(|| below_home(env, ".cache")),
        System::Windows => folder(env, "LOCALAPPDATA"),
    }
}

/// The folder a variable names; set but empty counts as not set.
fn folder(env: &dyn Fn(&str) -> Option<OsString>, key: &str) -> Option<PathBuf> {
    env(key).filter(|v| !v.is_empty()).map(PathBuf::from)
}

fn below_home(env: &dyn Fn(&str) -> Option<OsString>, rest: &str) -> Option<PathBuf> {
    folder(env, "HOME").map(|home| home.join(rest))
}

/// A file and when it last changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentFile {
    pub path: PathBuf,
    pub modified_unix: u64,
}

/// The download folder as the desktop names it (XDG user dirs), else `~/Downloads`.
#[cfg(unix)]
pub fn download_dir() -> Option<PathBuf> {
    let home = PathBuf::from(std::env::var_os("HOME")?);
    if let Some(dir) = std::env::var_os(DOWNLOAD_KEY).filter(|d| !d.is_empty()) {
        return Some(PathBuf::from(dir));
    }
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|d| !d.is_empty())
        .map_or_else(|| home.join(".config"), PathBuf::from);
    let named = std::fs::read_to_string(config.join(USER_DIRS))
        .ok()
        .and_then(|text| user_dir(&text, &home));
    Some(named.unwrap_or_else(|| home.join(FALLBACK)))
}

/// The profile's download folder (LH-NF-17).
#[cfg(windows)]
pub fn download_dir() -> Option<PathBuf> {
    let profile = PathBuf::from(std::env::var_os("USERPROFILE")?);
    Some(profile.join(FALLBACK))
}

/// `XDG_DOWNLOAD_DIR="$HOME/Downloads"` from a `user-dirs.dirs` file.
#[cfg(unix)]
fn user_dir(text: &str, home: &Path) -> Option<PathBuf> {
    let line = text
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with(DOWNLOAD_KEY))?;
    let value = line.split_once('=')?.1.trim().trim_matches('"');
    if let Some(rest) = value.strip_prefix(HOME_VARIABLE) {
        return Some(home.join(rest.trim_start_matches('/')));
    }
    Some(PathBuf::from(value)).filter(|p| p.is_absolute())
}

/// Files directly in `dir` changed within `max_age`, newest first; unreadable entries are
/// skipped (a download still being written may vanish).
pub fn recent_files(dir: &Path, max_age: Duration) -> Vec<RecentFile> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let oldest = SystemTime::now().checked_sub(max_age).unwrap_or(UNIX_EPOCH);
    let mut found: Vec<RecentFile> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let meta = entry.metadata().ok()?;
            let modified = meta.modified().ok()?;
            if !meta.is_file() || modified < oldest {
                return None;
            }
            let modified_unix = modified.duration_since(UNIX_EPOCH).ok()?.as_secs();
            Some(RecentFile {
                path: entry.path(),
                modified_unix,
            })
        })
        .collect();
    found.sort_by(|a, b| {
        b.modified_unix
            .cmp(&a.modified_unix)
            .then(a.path.cmp(&b.path))
    });
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env<'a>(set: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<OsString> + 'a {
        move |key| {
            set.iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| OsString::from(v))
        }
    }

    /// LH-NF-17 (Win4): data and cache went below `HOME` and XDG folders on every system; on
    /// Windows that was `.\.local\share\kontra` in whatever folder Kontra started in.
    #[test]
    fn program_folders_on_both_systems() {
        let linux = [("HOME", "/home/lea")];
        assert_eq!(
            data_home_in(System::Unix, &env(&linux)),
            Some(PathBuf::from("/home/lea/.local/share"))
        );
        assert_eq!(
            cache_home_in(System::Unix, &env(&linux)),
            Some(PathBuf::from("/home/lea/.cache"))
        );
        let xdg = [
            ("HOME", "/home/lea"),
            ("XDG_DATA_HOME", "/data"),
            ("XDG_CACHE_HOME", "/fast"),
        ];
        assert_eq!(
            data_home_in(System::Unix, &env(&xdg)),
            Some(PathBuf::from("/data"))
        );
        assert_eq!(
            cache_home_in(System::Unix, &env(&xdg)),
            Some(PathBuf::from("/fast"))
        );
        // Set but empty counts as not set (XDG base directory specification).
        let empty = [("HOME", "/home/lea"), ("XDG_DATA_HOME", "")];
        assert_eq!(
            data_home_in(System::Unix, &env(&empty)),
            Some(PathBuf::from("/home/lea/.local/share"))
        );

        let windows = [
            ("APPDATA", r"C:\Users\Noah B\AppData\Roaming"),
            ("LOCALAPPDATA", r"C:\Users\Noah B\AppData\Local"),
            ("HOME", "/not/used"),
        ];
        assert_eq!(
            data_home_in(System::Windows, &env(&windows)),
            Some(PathBuf::from(r"C:\Users\Noah B\AppData\Roaming"))
        );
        assert_eq!(
            cache_home_in(System::Windows, &env(&windows)),
            Some(PathBuf::from(r"C:\Users\Noah B\AppData\Local"))
        );
        assert_eq!(data_home_in(System::Windows, &env(&[])), None);
        assert_eq!(cache_home_in(System::Unix, &env(&[])), None);
    }

    #[cfg(unix)]
    #[test]
    fn reads_user_dirs() {
        let home = Path::new("/home/lea");
        let text = "# comment\nXDG_DESKTOP_DIR=\"$HOME/Desktop\"\nXDG_DOWNLOAD_DIR=\"$HOME/Herunterladen\"\n";
        assert_eq!(
            user_dir(text, home),
            Some(PathBuf::from("/home/lea/Herunterladen"))
        );
        let absolute = "XDG_DOWNLOAD_DIR=\"/data/dl\"";
        assert_eq!(user_dir(absolute, home), Some(PathBuf::from("/data/dl")));
        assert_eq!(user_dir("XDG_DOWNLOAD_DIR=\"dl\"", home), None);
        assert_eq!(user_dir("", home), None);
    }

    #[test]
    fn lists_recent_files_newest_first() {
        let dir = std::env::temp_dir().join(format!("kontra-folder-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("a.gp"), b"a").unwrap();
        std::fs::write(dir.join("b.mid"), b"b").unwrap();
        let old = std::fs::File::options()
            .write(true)
            .open(dir.join("a.gp"))
            .unwrap();
        old.set_modified(SystemTime::now() - Duration::from_secs(60))
            .unwrap();

        let names: Vec<String> = recent_files(&dir, Duration::from_secs(3600))
            .iter()
            .map(|f| f.path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["b.mid", "a.gp"]);
        assert!(recent_files(&dir, Duration::from_secs(30)).len() == 1);
        assert!(recent_files(&dir.join("missing"), Duration::from_secs(30)).is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
