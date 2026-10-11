//! Backing track without its bass (LH-BIB-09, round 9): an external separator (Demucs) removes
//! it once per song file; the result is kept in the cache and used instead of the file.
//!
//! ```text
//! song.mp3 ─► BassRemoval::start ─► StemJob (progress, cancel) ─► <cache>/<key>.wav
//!                                                     next time ─► BassRemoval::cached
//! ```
//! The key changes with the file's path, size and modification time, so an edited song is
//! separated again.

use kontra_engine::{find_separator, in_sandbox, SeparationJob, Separator, SeparatorError};
use std::path::{Path, PathBuf};

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
/// Folder of a running separation inside the cache.
const WORK_DIR: &str = "work";

#[derive(Debug, Clone)]
pub struct BassRemoval {
    separator: Option<Separator>,
    cache_dir: PathBuf,
}

impl BassRemoval {
    /// Looks for the separator; results go to `cache_dir`.
    pub fn new(cache_dir: &Path) -> BassRemoval {
        BassRemoval::with_separator(find_separator(), cache_dir)
    }

    /// With a given separator (`None`: not installed), for tests.
    pub fn with_separator(separator: Option<Separator>, cache_dir: &Path) -> BassRemoval {
        BassRemoval {
            separator,
            cache_dir: cache_dir.to_path_buf(),
        }
    }

    /// True if the separator is installed.
    pub fn available(&self) -> bool {
        self.separator.is_some()
    }

    /// True inside a Flatpak, which may not start programs of the system (then installing the
    /// separator alone does not help).
    pub fn sandboxed(&self) -> bool {
        in_sandbox()
    }

    /// The song without bass, if it was separated before.
    pub fn cached(&self, song: &Path) -> Option<PathBuf> {
        let path = self.target(song)?;
        path.is_file().then_some(path)
    }

    /// Starts separating `song`; `None` when no separator is installed.
    pub fn start(&self, song: &Path) -> Option<Result<StemJob, SeparatorError>> {
        let separator = self.separator.as_ref()?;
        let target = self.target(song)?;
        // One folder per song: a cancelled run elsewhere cannot write into this one.
        let key = target
            .file_stem()
            .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
        let work = self.cache_dir.join(WORK_DIR).join(key);
        let _ = std::fs::remove_dir_all(&work);
        Some(
            separator
                .start(song, &work)
                .map(|job| StemJob { job, target, work }),
        )
    }

    fn target(&self, song: &Path) -> Option<PathBuf> {
        let meta = std::fs::metadata(song).ok()?;
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_secs());
        let mut hash = FNV_OFFSET;
        let key = format!("{}|{}|{modified}", song.display(), meta.len());
        for byte in key.bytes() {
            hash = (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME);
        }
        Some(self.cache_dir.join(format!("{hash:016x}.wav")))
    }
}

/// A running separation of one song.
#[derive(Debug)]
pub struct StemJob {
    job: SeparationJob,
    target: PathBuf,
    work: PathBuf,
}

impl StemJob {
    /// 0..=1 of the separator's current pass.
    pub fn progress(&self) -> f64 {
        self.job.progress()
    }

    /// `None` while it runs; the cached file without bass when done.
    pub fn poll(&mut self) -> Option<Result<PathBuf, SeparatorError>> {
        let result = self.job.poll()?.and_then(|file| {
            std::fs::rename(&file, &self.target)
                .map(|()| self.target.clone())
                .map_err(|e| SeparatorError::Failed(e.to_string()))
        });
        let _ = std::fs::remove_dir_all(&self.work);
        Some(result)
    }

    pub fn cancel(&mut self) {
        self.job.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separates_once_and_keeps_the_result() {
        let dir = std::env::temp_dir().join(format!("kontra-stems-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("demucs");
        std::fs::write(
            &script,
            "#!/bin/sh\nname=$(basename \"$7\"); name=\"${name%.*}\"\nmkdir -p \"$6/htdemucs/$name\" && cp \"$7\" \"$6/htdemucs/$name/no_bass.wav\"\n",
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let song = dir.join("Song.wav");
        std::fs::write(&song, b"RIFF").unwrap();
        let cache = dir.join("cache");
        std::fs::create_dir_all(&cache).unwrap();

        let none = BassRemoval::with_separator(None, &cache);
        assert!(!none.available() && none.start(&song).is_none());

        let stems = BassRemoval::with_separator(Some(Separator::with_program(&script)), &cache);
        assert_eq!(stems.cached(&song), None);
        let mut job = stems.start(&song).unwrap().unwrap();
        let result = loop {
            if let Some(r) = job.poll() {
                break r.unwrap();
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        };
        assert_eq!(stems.cached(&song), Some(result.clone()));
        assert_eq!(std::fs::read(result).unwrap(), b"RIFF");
        let leftover = std::fs::read_dir(cache.join(WORK_DIR)).map_or(0, |d| d.count());
        assert_eq!(leftover, 0, "the work folder is cleared");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
