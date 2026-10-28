//! The caches in Settings → Data (round 16, P3, P4): prepared backing tracks and separated
//! songs, each listed by its song with its size, deleted alone or all at once.
//!
//! ```text
//! caches_json ─► {"backing": {bytes, total, entries: [{key, name, size}]}, "stems": {…}}
//! forget_cached("backing", key) ─► BackingCache::forget ─► the open piece is prepared again
//! forget_cached("stems", key)   ─► BassRemoval::forget  ─► the song plays with its bass
//! ```
//! A run keeps its tracks in memory: deleting what it plays changes nothing until the next play.

use crate::Core;
use kontra_session::{BackingCache, CacheEntry};
use serde_json::{json, Value};

/// Which cache the UI means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cache {
    Backing,
    Stems,
}

impl Cache {
    fn parse(name: &str) -> Option<Cache> {
        match name {
            "backing" => Some(Cache::Backing),
            "stems" => Some(Cache::Stems),
            _ => None,
        }
    }
}

impl Core {
    /// Both caches for Settings → Data: total size and the entries, newest first.
    pub fn caches_json(&self) -> String {
        json!({
            "backing": self.cache_json(Cache::Backing),
            "stems": self.cache_json(Cache::Stems),
        })
        .to_string()
    }

    /// Deletes one entry of `cache` ("backing" or "stems").
    pub fn forget_cached(&mut self, cache: &str, key: &str) {
        let Some(cache) = Cache::parse(cache) else {
            return;
        };
        match cache {
            Cache::Backing => BackingCache::new(&self.backing_cache_dir()).forget(key),
            Cache::Stems => self.cached_stems().forget(key),
        };
        self.cache_changed(cache);
    }

    /// Empties `cache` ("backing" or "stems").
    pub fn clear_cache(&mut self, cache: &str) {
        let Some(cache) = Cache::parse(cache) else {
            return;
        };
        match cache {
            Cache::Backing => BackingCache::new(&self.backing_cache_dir()).clear(),
            Cache::Stems => self.cached_stems().clear(),
        }
        self.cache_changed(cache);
    }

    /// The open piece's backing track may be gone: prepare it again, so play does not decode
    /// it while the player waits (J2). Separated songs need nothing: the original plays.
    fn cache_changed(&mut self, cache: Cache) {
        if cache == Cache::Backing {
            self.prepare_again();
        }
    }

    fn cache_json(&self, cache: Cache) -> Value {
        let (entries, (_, bytes)) = match cache {
            Cache::Backing => {
                let backing = BackingCache::new(&self.backing_cache_dir());
                (backing.entries(), backing.usage())
            }
            Cache::Stems => {
                let stems = self.cached_stems();
                (stems.entries(), stems.usage())
            }
        };
        let entries: Vec<Value> = entries.iter().map(|e| self.entry_json(e)).collect();
        json!({ "bytes": bytes, "total": self.megabytes(bytes), "entries": entries })
    }

    fn entry_json(&self, entry: &CacheEntry) -> Value {
        // Files from before the list have no name.
        let name = entry
            .name
            .clone()
            .unwrap_or_else(|| self.tr("cache.unknown", &[]));
        json!({ "key": entry.key, "name": name, "size": self.megabytes(entry.bytes) })
    }
}

#[cfg(test)]
mod tests {
    use crate::{Core, QaOptions};
    use serde_json::Value;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant};

    /// The engine's `NULL_DEVICE`.
    const SILENT_OUTPUT: &str = "kontra:null";

    fn demo() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/demo/etude-1.mid")
    }

    fn caches(core: &Core) -> Value {
        serde_json::from_str(&core.caches_json()).unwrap()
    }

    fn settle(core: &mut Core) {
        let end = Instant::now() + Duration::from_secs(20);
        core.tick();
        while core.backing_loading() && Instant::now() < end {
            core.tick();
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn core_with_backing(dir: &Path) -> Core {
        let _ = std::fs::remove_dir_all(dir);
        std::fs::create_dir_all(dir).unwrap();
        let song = dir.join("Freitag.wav");
        kontra_session::write_mono_wav(&song, &vec![0.1; 48_000], 48_000).unwrap();
        // The engine's timer instead of the speakers, a recording instead of the microphone.
        let data = dir.join("data");
        std::fs::create_dir_all(&data).unwrap();
        let silent = serde_json::json!({ "output_device": SILENT_OUTPUT });
        std::fs::write(data.join("settings.json"), silent.to_string()).unwrap();
        let take = dir.join("take.wav");
        kontra_session::write_mono_wav(&take, &vec![0.0; 48_000], 48_000).unwrap();
        let qa = QaOptions {
            input_file: Some(take),
            laptop_jack: false,
        };
        let mut core = Core::new(&data, qa);
        core.open(&demo()).unwrap();
        core.set_backing(Some(song), 0.0);
        core
    }

    /// Round 16 (P3, P4): the prepared track is listed by its song; deleted, the open piece is
    /// prepared again, emptied while preparing, nothing breaks.
    #[test]
    fn lists_by_song_and_prepares_again() {
        let dir = std::env::temp_dir().join(format!("kontra-caches-{}", std::process::id()));
        let mut core = core_with_backing(&dir);
        settle(&mut core);
        let list = caches(&core);
        let entries = list["backing"]["entries"].as_array().unwrap().clone();
        assert_eq!(entries.len(), 1, "{list}");
        assert_eq!(entries[0]["name"], "Freitag.wav");
        assert!(list["stems"]["entries"].as_array().unwrap().is_empty());

        core.forget_cached("backing", entries[0]["key"].as_str().unwrap());
        assert!(caches(&core)["backing"]["entries"]
            .as_array()
            .unwrap()
            .is_empty());
        settle(&mut core);
        assert_eq!(
            caches(&core)["backing"]["entries"]
                .as_array()
                .unwrap()
                .len(),
            1
        );

        // Emptied right after preparing started: the job ends, play works.
        core.forget_cached("backing", "0000000000000000");
        core.forget_cached("nonsense", "x");
        core.clear_cache("backing");
        core.tick();
        core.clear_cache("backing");
        settle(&mut core);
        assert!(!core.backing_loading());
        let _ = core.play();
        assert!(core.has_run());
        core.stop_live();
        let _ = std::fs::remove_dir_all(dir);
    }
}
