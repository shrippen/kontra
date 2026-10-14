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
const RESULT_EXT: &str = "wav";

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

    /// Bytes of the song's separated file; `None` if it was not separated.
    pub fn size(&self, song: &Path) -> Option<u64> {
        std::fs::metadata(self.cached(song)?).ok().map(|m| m.len())
    }

    /// Deletes the song's separated file (it plays with its bass again); false if there was
    /// none.
    pub fn remove(&self, song: &Path) -> bool {
        self.cached(song)
            .is_some_and(|path| std::fs::remove_file(path).is_ok())
    }

    /// Separated files in the cache and their bytes, of every song.
    pub fn usage(&self) -> (usize, u64) {
        let Ok(entries) = std::fs::read_dir(&self.cache_dir) else {
            return (0, 0);
        };
        entries
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == RESULT_EXT))
            .filter_map(|e| e.metadata().ok())
            .fold((0, 0), |(n, bytes), m| (n + 1, bytes + m.len()))
    }

    /// Starts separating `song`; `None` when no separator is installed.
    pub fn start(&self, song: &Path) -> Option<Result<StemJob, SeparatorError>> {
        let separator = self.separator.as_ref()?;
        let target = self.target(song)?;
        let key = target
            .file_stem()
            .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
        // One separation at a time: what lies in the work folder is left from a crash.
        let _ = std::fs::remove_dir_all(self.cache_dir.join(WORK_DIR));
        let work = self.cache_dir.join(WORK_DIR).join(key);
        Some(separator.start(song, &work).map(|job| StemJob {
            job,
            target,
            work,
            done: false,
        }))
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
        Some(self.cache_dir.join(format!("{hash:016x}.{RESULT_EXT}")))
    }
}

/// A running separation of one song.
#[derive(Debug)]
pub struct StemJob {
    job: SeparationJob,
    target: PathBuf,
    work: PathBuf,
    done: bool,
}

impl StemJob {
    /// 0..=1 of the separator's current pass.
    pub fn progress(&self) -> f64 {
        self.job.progress()
    }

    /// Time left, as the separator estimates it; `None` until it does.
    pub fn remaining(&self) -> Option<std::time::Duration> {
        self.job.remaining()
    }

    /// `None` while it runs; the cached file without bass when done.
    pub fn poll(&mut self) -> Option<Result<PathBuf, SeparatorError>> {
        let result = self.job.poll()?.and_then(|file| {
            std::fs::rename(&file, &self.target)
                .map(|()| self.target.clone())
                .map_err(|e| SeparatorError::Failed(e.to_string()))
        });
        self.done = true;
        self.clear();
        Some(result)
    }

    /// Stops it; what the separator wrote so far goes with it.
    pub fn cancel(&mut self) {
        if self.done {
            return;
        }
        self.job.cancel();
        self.done = true;
        self.clear();
    }

    /// Removes the work folder, and the folder of all work folders once empty.
    fn clear(&self) {
        let _ = std::fs::remove_dir_all(&self.work);
        if let Some(parent) = self.work.parent() {
            let _ = std::fs::remove_dir(parent);
        }
    }
}

impl Drop for StemJob {
    /// Kontra quits or the piece changes while it runs.
    fn drop(&mut self) {
        self.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One test at a time writes and starts a script: a child forked by another test can hold
    /// the file still open for writing, and starting it fails with "Text file busy" (ETXTBSY),
    /// as in `kontra_drivers::separator` (CI 14.10.: "failed" instead of "running").
    static SCRIPTS: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn scripts() -> std::sync::MutexGuard<'static, ()> {
        SCRIPTS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    #[test]
    fn separates_once_and_keeps_the_result() {
        let _scripts = scripts();
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
        assert_eq!(std::fs::read(&result).unwrap(), b"RIFF");
        assert_eq!(stems.size(&song), Some(4));
        assert_eq!(stems.usage(), (1, 4));
        let leftover = std::fs::read_dir(cache.join(WORK_DIR)).map_or(0, |d| d.count());
        assert_eq!(leftover, 0, "the work folder is cleared");

        // Deleted: it plays with its bass again and the space is free.
        assert!(stems.remove(&song));
        assert_eq!((stems.cached(&song), stems.usage()), (None, (0, 0)));
        assert!(!stems.remove(&song));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Second reader #64: work folders of an earlier run (crash, killed) stayed for good.
    #[test]
    fn start_clears_old_work() {
        let _scripts = scripts();
        let dir = std::env::temp_dir().join(format!("kontra-stems-stale-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let cache = dir.join("cache");
        let stale = cache.join(WORK_DIR).join("0000000000000000/htdemucs");
        std::fs::create_dir_all(&stale).unwrap();
        let script = dir.join("demucs");
        std::fs::write(&script, "#!/bin/sh\nsleep 30\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let song = dir.join("Song.wav");
        std::fs::write(&song, b"RIFF").unwrap();

        let stems = BassRemoval::with_separator(Some(Separator::with_program(&script)), &cache);
        let job = stems.start(&song).unwrap().unwrap();
        assert!(!stale.exists());
        drop(job);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// QA P2 (J-D1, 14.10.): a cancelled separation left Demucs' half-written files behind.
    #[test]
    fn cancel_leaves_nothing_behind() {
        let _scripts = scripts();
        let dir = std::env::temp_dir().join(format!("kontra-stems-cancel-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("demucs");
        std::fs::write(
            &script,
            "#!/bin/sh\nmkdir -p \"$6/htdemucs/x\" && echo half > \"$6/htdemucs/x/no_bass.wav\"\nsleep 30\n",
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let song = dir.join("Song.wav");
        std::fs::write(&song, b"RIFF").unwrap();
        let cache = dir.join("cache");

        let stems = BassRemoval::with_separator(Some(Separator::with_program(&script)), &cache);
        let mut job = stems.start(&song).unwrap().unwrap();
        let work = cache.join(WORK_DIR);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let started = |w: &Path| std::fs::read_dir(w).is_ok_and(|mut d| d.next().is_some());
        while !started(&work) && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        job.cancel();
        drop(job);
        assert!(!work.exists(), "work folder left: {work:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
