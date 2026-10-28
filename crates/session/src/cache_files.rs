//! The files of a cache folder as entries the player can see and delete (Settings → Data):
//! the prepared backing tracks and the separated songs.
//!
//! ```text
//! <cache>/<key>.wav    the result (key: 16 hex digits, see `stems::file_key`)
//! <cache>/<key>.name   the song's file name, for the list (a hash says nothing)
//! <cache>/<key>.….part a result being written; left over when Kontra stopped while writing
//! ```

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const RESULT_EXT: &str = "wav";
const NAME_EXT: &str = "name";
const PART_EXT: &str = "part";
/// Digits of a key (a `u64` in hex).
const KEY_DIGITS: usize = 16;
/// A partial file untouched this long is left over, nobody writes it any more.
const STALE_PART: Duration = Duration::from_secs(60);

/// One result in a cache: its key, the song's file name (unknown for files from before the
/// list) and its bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct CacheEntry {
    pub key: String,
    pub name: Option<String>,
    pub bytes: u64,
}

/// Whether a partial file is removed by its age or always.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Partial {
    /// Only one nobody wrote for a while: another thread may be writing it.
    Stale,
    /// Every one (the player empties the cache).
    All,
}

/// Results, newest first.
pub(crate) fn entries(dir: &Path) -> Vec<CacheEntry> {
    let mut found: Vec<(SystemTime, CacheEntry)> = files(dir)
        .filter(|p| has_ext(p, RESULT_EXT))
        .filter_map(|path| {
            let meta = std::fs::metadata(&path).ok()?;
            let key = path.file_stem()?.to_string_lossy().into_owned();
            let name = std::fs::read_to_string(path.with_extension(NAME_EXT))
                .ok()
                .map(|n| n.trim().to_string())
                .filter(|n| !n.is_empty());
            let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            let entry = CacheEntry {
                key,
                name,
                bytes: meta.len(),
            };
            Some((modified, entry))
        })
        .collect();
    found.sort_by_key(|f| std::cmp::Reverse(f.0));
    found.into_iter().map(|(_, entry)| entry).collect()
}

/// Results and their bytes; partial files count to the bytes, they take room too.
pub(crate) fn usage(dir: &Path) -> (usize, u64) {
    files(dir)
        .filter(|p| has_ext(p, RESULT_EXT) || has_ext(p, PART_EXT))
        .filter_map(|p| Some((has_ext(&p, RESULT_EXT), std::fs::metadata(&p).ok()?.len())))
        .fold((0, 0), |(n, bytes), (result, len)| {
            (n + usize::from(result), bytes + len)
        })
}

/// Notes the song's file name beside the result `target`.
pub(crate) fn label(target: &Path, song: &Path) {
    let Some(name) = song.file_name() else {
        return;
    };
    let _ = std::fs::write(
        target.with_extension(NAME_EXT),
        name.to_string_lossy().as_bytes(),
    );
}

/// Deletes the result `key` and its name; false if there was none or the key is no key.
pub(crate) fn forget(dir: &Path, key: &str) -> bool {
    // Only a key: never a path the caller made up ("../x").
    if key.len() != KEY_DIGITS || !key.bytes().all(|b| b.is_ascii_hexdigit()) {
        return false;
    }
    let result = dir.join(key).with_extension(RESULT_EXT);
    let _ = std::fs::remove_file(result.with_extension(NAME_EXT));
    std::fs::remove_file(result).is_ok()
}

/// Deletes the results beyond the newest `keep`, names without a result and partial files.
pub(crate) fn prune(dir: &Path, keep: usize, partial: Partial) {
    for entry in entries(dir).iter().skip(keep) {
        forget(dir, &entry.key);
    }
    for path in files(dir) {
        let orphan = has_ext(&path, NAME_EXT) && !path.with_extension(RESULT_EXT).is_file();
        let part = has_ext(&path, PART_EXT) && (partial == Partial::All || stale(&path));
        if orphan || part {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn stale(path: &Path) -> bool {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.elapsed().ok())
        .is_some_and(|age| age >= STALE_PART)
}

fn files(dir: &Path) -> impl Iterator<Item = PathBuf> {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file())
}

fn has_ext(path: &Path, ext: &str) -> bool {
    path.extension().is_some_and(|x| x == ext)
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "00000000000000aa";

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("kontra-cache-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn lists_names_and_forgets_one() {
        let dir = temp_dir("list");
        let target = dir.join(format!("{KEY}.wav"));
        std::fs::write(&target, [0u8; 10]).unwrap();
        label(&target, Path::new("/music/Freitag.mp3"));
        std::fs::write(dir.join("00000000000000bb.wav"), [0u8; 4]).unwrap();

        let list = entries(&dir);
        assert_eq!(list.len(), 2);
        let named = list.iter().find(|e| e.key == KEY).unwrap();
        assert_eq!(named.name.as_deref(), Some("Freitag.mp3"));
        assert_eq!(named.bytes, 10);
        assert!(list.iter().any(|e| e.name.is_none()));

        assert!(!forget(&dir, "../outside"));
        assert!(forget(&dir, KEY));
        assert!(!dir.join(format!("{KEY}.name")).exists());
        assert_eq!(entries(&dir).len(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Review #162: a partial file left from a stop while writing took room nobody saw.
    #[test]
    fn partial_files_count_and_go_when_emptied() {
        let dir = temp_dir("part");
        std::fs::write(dir.join(format!("{KEY}.wav")), [0u8; 10]).unwrap();
        std::fs::write(dir.join(format!("{KEY}.ThreadId(7).part")), [0u8; 5]).unwrap();
        assert_eq!(usage(&dir), (1, 15));

        // A fresh one may be written right now.
        prune(&dir, 4, Partial::Stale);
        assert_eq!(usage(&dir), (1, 15));

        prune(&dir, 0, Partial::All);
        assert_eq!(usage(&dir), (0, 0));
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);
        let _ = std::fs::remove_dir_all(dir);
    }
}
