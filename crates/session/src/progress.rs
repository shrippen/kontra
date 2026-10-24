//! Practice progress per song, stored as one JSON file per song (LH-FOR-01, LH-FOR-03).

use crate::summary::Summary;
use kontra_model::Arrangement;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProgressError {
    #[error("cannot read or write progress in {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("progress file {path} is damaged: {source}")]
    Format {
        path: PathBuf,
        source: serde_json::Error,
    },
}

/// One finished run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Run {
    /// Seconds since the Unix epoch.
    pub finished_unix: u64,
    pub practice_s: f64,
    pub speed: f64,
    pub hit_rate: f64,
    pub mode: String,
    /// Loop start and end in arrangement seconds, if a section was practiced.
    pub section: Option<(f64, f64)>,
    /// Bars with the most mistakes, as numbered in the score (progress report, LH-FOR-04).
    #[serde(default)]
    pub hardest_bars: Vec<u32>,
    /// Timing spread of the run (ms), to compare a blind probe with the practice before it.
    #[serde(default)]
    pub spread_ms: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SongProgress {
    pub song_id: String,
    pub title: String,
    pub artist: String,
    pub runs: Vec<Run>,
}

impl SongProgress {
    pub fn best_hit_rate(&self) -> Option<f64> {
        self.runs.iter().map(|r| r.hit_rate).max_by(f64::total_cmp)
    }

    /// Highest speed reached with at least `min_hit_rate`.
    pub fn best_speed(&self, min_hit_rate: f64) -> Option<f64> {
        self.runs
            .iter()
            .filter(|r| r.hit_rate >= min_hit_rate)
            .map(|r| r.speed)
            .max_by(f64::total_cmp)
    }

    pub fn practice_s(&self) -> f64 {
        self.runs.iter().map(|r| r.practice_s).sum()
    }
}

/// Directory of progress files.
#[derive(Debug, Clone)]
pub struct ProgressStore {
    dir: PathBuf,
}

impl ProgressStore {
    pub fn new(dir: &Path) -> ProgressStore {
        ProgressStore {
            dir: dir.to_path_buf(),
        }
    }

    /// Stable id of an arrangement: FNV-1a over title, artist and notes.
    pub fn song_id(arrangement: &Arrangement) -> String {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        let mut feed = |bytes: &[u8]| {
            for b in bytes {
                hash ^= u64::from(*b);
                hash = hash.wrapping_mul(0x0100_0000_01b3);
            }
        };
        feed(arrangement.title.as_bytes());
        feed(arrangement.artist.as_bytes());
        for n in &arrangement.notes {
            feed(&n.start_ticks.to_le_bytes());
            feed(&[n.pitch.0]);
        }
        format!("{hash:016x}")
    }

    pub fn load(&self, arrangement: &Arrangement) -> Result<SongProgress, ProgressError> {
        let song_id = Self::song_id(arrangement);
        let path = self.path(&song_id);
        if !path.exists() {
            return Ok(SongProgress {
                song_id,
                title: arrangement.title.clone(),
                artist: arrangement.artist.clone(),
                runs: Vec::new(),
            });
        }
        let text = fs::read_to_string(&path).map_err(|source| ProgressError::Io {
            path: path.clone(),
            source,
        })?;
        serde_json::from_str(&text).map_err(|source| ProgressError::Format { path, source })
    }

    /// Adds a run and saves.
    pub fn record(
        &self,
        arrangement: &Arrangement,
        run: Run,
    ) -> Result<SongProgress, ProgressError> {
        let mut progress = self.load(arrangement)?;
        progress.runs.push(run);
        self.save(&progress)?;
        Ok(progress)
    }

    /// Writes `progress` under its song id atomically (temp file, then rename); replaces what
    /// was stored for that song (data import, LH-FOR-03).
    pub fn save(&self, progress: &SongProgress) -> Result<(), ProgressError> {
        fs::create_dir_all(&self.dir).map_err(|source| ProgressError::Io {
            path: self.dir.clone(),
            source,
        })?;
        let path = self.path(&progress.song_id);
        let temp = path.with_extension("json.tmp");
        let text =
            serde_json::to_string_pretty(progress).map_err(|source| ProgressError::Format {
                path: path.clone(),
                source,
            })?;
        fs::write(&temp, text).map_err(|source| ProgressError::Io {
            path: temp.clone(),
            source,
        })?;
        fs::rename(&temp, &path).map_err(|source| ProgressError::Io { path, source })
    }

    /// Moves the runs of `old_id` to `new_id` (a file changed at the same path, e.g. a
    /// regenerated exercise); runs already under `new_id` stay. Nothing stored: nothing to do.
    pub fn adopt(&self, old_id: &str, new_id: &str) -> Result<(), ProgressError> {
        if old_id == new_id {
            return Ok(());
        }
        let Some(old) = self.load_id(old_id) else {
            return Ok(());
        };
        let mut progress = self.load_id(new_id).unwrap_or(SongProgress {
            song_id: new_id.to_string(),
            runs: Vec::new(),
            ..old.clone()
        });
        progress.runs.extend(old.runs);
        progress.runs.sort_by_key(|r| r.finished_unix);
        self.save(&progress)?;
        let path = self.path(old_id);
        fs::remove_file(&path).map_err(|source| ProgressError::Io { path, source })
    }

    /// Progress stored under `song_id`, if any (for lists that only know the id).
    pub fn load_id(&self, song_id: &str) -> Option<SongProgress> {
        let text = fs::read_to_string(self.path(song_id)).ok()?;
        serde_json::from_str(&text).ok()
    }

    /// Every recorded song, for the progress view (LH-FOR-01); unreadable files are skipped.
    pub fn all(&self) -> Vec<SongProgress> {
        let Ok(entries) = fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .filter_map(|p| fs::read_to_string(p).ok())
            .filter_map(|t| serde_json::from_str(&t).ok())
            .collect()
    }

    fn path(&self, song_id: &str) -> PathBuf {
        self.dir.join(format!("{song_id}.json"))
    }
}

/// Builds a run record from a summary.
pub fn run_from(
    summary: &Summary,
    practice_s: f64,
    speed: f64,
    mode: &str,
    section: Option<(f64, f64)>,
    finished_unix: u64,
) -> Run {
    Run {
        finished_unix,
        practice_s,
        speed,
        hit_rate: summary.hit_rate,
        mode: mode.to_string(),
        section,
        // Bar indices count from 0; without repeats they are the printed numbers minus one.
        hardest_bars: summary
            .hardest_bars
            .iter()
            .map(|b| u32::try_from(*b + 1).unwrap_or(u32::MAX))
            .collect(),
        spread_ms: summary.spread_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kontra_model::{Instrument, Note, Pitch};

    #[test]
    fn adopt_moves_runs_to_the_new_id() {
        let dir = std::env::temp_dir().join(format!("kontra-adopt-{}", std::process::id()));
        let store = ProgressStore::new(&dir);
        let mut a = Arrangement::new("Riff", Instrument::default_bass());
        a.notes.push(Note::new(0, 960, Pitch(28)));
        let run = |at| Run {
            spread_ms: None,
            finished_unix: at,
            practice_s: 60.0,
            speed: 1.0,
            hit_rate: 0.9,
            mode: "song".into(),
            section: None,
            hardest_bars: Vec::new(),
        };
        store.record(&a, run(1)).unwrap();
        let old = ProgressStore::song_id(&a);
        a.notes.push(Note::new(960, 960, Pitch(33)));
        store.record(&a, run(2)).unwrap();
        let new = ProgressStore::song_id(&a);

        store.adopt(&old, &new).unwrap();
        let moved = store.load_id(&new).unwrap();
        let at: Vec<u64> = moved.runs.iter().map(|r| r.finished_unix).collect();
        assert_eq!(at, [1, 2]);
        assert!(store.load_id(&old).is_none());
        // Nothing under the old id: nothing happens.
        store.adopt("missing", &new).unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn records_and_reloads() {
        let dir = std::env::temp_dir().join(format!("kontra-progress-{}", std::process::id()));
        let store = ProgressStore::new(&dir);
        let mut a = Arrangement::new("Riff", Instrument::default_bass());
        a.notes.push(Note::new(0, 960, Pitch(28)));
        let run = Run {
            spread_ms: None,
            finished_unix: 1,
            practice_s: 60.0,
            speed: 0.8,
            hit_rate: 0.9,
            mode: "song".into(),
            section: None,
            hardest_bars: Vec::new(),
        };
        store.record(&a, run.clone()).unwrap();
        let p = store
            .record(
                &a,
                Run {
                    speed: 1.0,
                    hit_rate: 0.7,
                    ..run
                },
            )
            .unwrap();
        assert_eq!(p.runs.len(), 2);
        assert_eq!(store.load(&a).unwrap(), p);
        assert_eq!(p.best_speed(0.85), Some(0.8));
        assert_eq!(p.practice_s(), 120.0);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
