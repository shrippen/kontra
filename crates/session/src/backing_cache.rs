//! Backing tracks ready to play (QA 27.10., J2): decoding an MP3 and resampling it to the
//! output's rate took a second (20 s in a debug build) on every first play after opening. The
//! result is kept in the cache, and prepared in the background as soon as a piece with a
//! backing track is open.
//!
//! ```text
//! open piece ─► BackingCache::prepare (thread) ─► <cache>/<key>.wav (float, output rate)
//! play       ─► BackingCache::load ─ cached? ─yes─► read the WAV (no resampling)
//!                                       └─no──► decode, resample, keep
//! ```
//! The key changes with the file's path, size, modification time and the rate, like the
//! separated songs (`stems`). Only the newest `KEEP_FILES` stay.

use crate::stems::file_key;
use kontra_engine::{list_devices, BackingTrack, Direction, EngineError, NULL_DEVICE};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, TryRecvError};

const RESULT_EXT: &str = "wav";
/// Prepared tracks kept; about 46 MB for four minutes at 48 kHz.
const KEEP_FILES: usize = 4;
/// Output rate when no device tells one (also the rate of the silent output).
const FALLBACK_RATE: u32 = 48_000;

#[derive(Debug, Clone)]
pub struct BackingCache {
    dir: PathBuf,
}

impl BackingCache {
    pub fn new(dir: &Path) -> BackingCache {
        BackingCache {
            dir: dir.to_path_buf(),
        }
    }

    /// The song at `rate`: from the cache, else decoded, resampled and kept there.
    pub fn load(&self, song: &Path, rate: u32) -> Result<BackingTrack, EngineError> {
        let Some(target) = self.target(song, rate) else {
            return BackingTrack::load(song, rate);
        };
        if target.is_file() {
            if let Ok(track) = BackingTrack::load(&target, rate) {
                return Ok(track);
            }
        }
        let track = BackingTrack::load(song, rate)?;
        self.keep(&track, &target);
        Ok(track)
    }

    /// True if the song is ready at `rate`.
    pub fn cached(&self, song: &Path, rate: u32) -> bool {
        self.target(song, rate).is_some_and(|t| t.is_file())
    }

    /// Prepared files and their bytes.
    pub fn usage(&self) -> (usize, u64) {
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return (0, 0);
        };
        entries
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == RESULT_EXT))
            .filter_map(|e| e.metadata().ok())
            .fold((0, 0), |(n, bytes), m| (n + 1, bytes + m.len()))
    }

    /// Prepares `songs` in a thread for the output `device` (`None`: the default one).
    pub fn prepare(&self, songs: Vec<PathBuf>, device: Option<String>) -> Preparing {
        let (tx, rx) = channel();
        let cache = self.clone();
        let spawned = std::thread::Builder::new()
            .name("kontra-backing".into())
            .spawn(move || {
                let rate = output_rate(device.as_deref());
                for song in &songs {
                    if !cache.cached(song, rate) {
                        // A song that does not load says so when it is played.
                        let _ = cache.load(song, rate);
                    }
                }
                let _ = tx.send(());
            });
        Preparing {
            done: rx,
            running: spawned.is_ok(),
        }
    }

    fn target(&self, song: &Path, rate: u32) -> Option<PathBuf> {
        let hash = file_key(song, &format!("|{rate}"))?;
        Some(self.dir.join(format!("{hash:016x}.{RESULT_EXT}")))
    }

    /// Writes beside the target and renames, so a reader never sees half a file; then drops
    /// the oldest files beyond `KEEP_FILES`.
    fn keep(&self, track: &BackingTrack, target: &Path) {
        if std::fs::create_dir_all(&self.dir).is_err() {
            return;
        }
        let partial = target.with_extension(format!("{:?}.part", std::thread::current().id()));
        if track.save(&partial).is_err() || std::fs::rename(&partial, target).is_err() {
            let _ = std::fs::remove_file(&partial);
            return;
        }
        self.prune();
    }

    fn prune(&self) {
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return;
        };
        let mut files: Vec<(std::time::SystemTime, PathBuf)> = entries
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == RESULT_EXT))
            .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
            .collect();
        files.sort_by_key(|f| std::cmp::Reverse(f.0));
        for (_, path) in files.iter().skip(KEEP_FILES) {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// Backing tracks being prepared in the background.
#[derive(Debug)]
pub struct Preparing {
    done: Receiver<()>,
    running: bool,
}

impl Preparing {
    /// True while the thread works.
    pub fn running(&mut self) -> bool {
        if !self.running {
            return false;
        }
        self.running = matches!(self.done.try_recv(), Err(TryRecvError::Empty));
        self.running
    }
}

/// The rate the output will run at: the chosen device's, else (gone or none chosen) the
/// default device's.
fn output_rate(device: Option<&str>) -> u32 {
    if device == Some(NULL_DEVICE) {
        return FALLBACK_RATE;
    }
    let devices = list_devices(Direction::Output);
    let chosen = device.and_then(|id| devices.iter().find(|d| d.id == id));
    chosen
        .or_else(|| devices.iter().find(|d| d.is_default))
        .map_or(FALLBACK_RATE, |d| d.sample_rate)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 44_100;
    const OUTPUT_RATE: u32 = 48_000;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("kontra-backing-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn song(dir: &Path) -> PathBuf {
        let samples: Vec<f32> = (0..RATE)
            .map(|i| 0.5 * (2.0 * std::f32::consts::PI * 110.0 * i as f32 / RATE as f32).sin())
            .collect();
        let path = dir.join("song.wav");
        crate::write_mono_wav(&path, &samples, RATE).unwrap();
        path
    }

    /// QA 27.10. (J2): the second load reads the prepared file, it does not decode again.
    #[test]
    fn second_load_comes_from_the_cache() {
        let dir = temp_dir("second");
        let song = song(&dir);
        let cache = BackingCache::new(&dir.join("cache"));
        assert!(!cache.cached(&song, OUTPUT_RATE));
        let first = cache.load(&song, OUTPUT_RATE).unwrap();
        assert!(cache.cached(&song, OUTPUT_RATE));
        assert_eq!(cache.usage().0, 1);
        let again = cache.load(&song, OUTPUT_RATE).unwrap();
        assert_eq!(again.sample_rate, OUTPUT_RATE);
        assert_eq!(again.left.len(), first.left.len());
        let worst = first
            .left
            .iter()
            .zip(&again.left)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max);
        assert!(worst < 1e-4, "float: {worst}");

        // Another rate is another file.
        assert!(!cache.cached(&song, RATE));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn prepares_in_the_background_and_keeps_few() {
        let dir = temp_dir("prepare");
        let song = song(&dir);
        let cache = BackingCache::new(&dir.join("cache"));
        let mut job = cache.prepare(vec![song.clone()], Some(NULL_DEVICE.into()));
        let end = std::time::Instant::now() + std::time::Duration::from_secs(20);
        while job.running() && std::time::Instant::now() < end {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(!job.running());
        assert!(cache.cached(&song, FALLBACK_RATE));

        for rate in [22_050, 32_000, 44_100, 88_200, 96_000] {
            cache.load(&song, rate).unwrap();
        }
        assert_eq!(cache.usage().0, KEEP_FILES);
        let _ = std::fs::remove_dir_all(dir);
    }
}
