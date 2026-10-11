//! Backing track without its bass (LH-BIB-09, round 9): Demucs removes it, if installed; Kontra
//! ships nothing of it. Without it the choice is greyed out and says how to install it.
//!
//! ```text
//! remove_bass(true) ─► cached? ─yes─► entry.backing_without_bass = true
//!                          └─no──► StemJob ─tick─► UiEvent::Stems(progress) … done ─► cached
//! ```

use crate::{file_name, store, Core, UiEvent};
use kontra_session::{BassRemoval, StemJob};
use serde_json::json;
use std::path::{Path, PathBuf};

const CACHE_DIR: &str = "stems";
/// Progress steps the UI is told about (permille).
const PROGRESS_STEP: u32 = 10;
const PERMILLE: f64 = 1000.0;

/// The separator, a running separation (piece, song) and the last failure.
#[derive(Debug, Default)]
pub(crate) struct Stems {
    removal: Option<BassRemoval>,
    job: Option<(PathBuf, PathBuf, StemJob)>,
    failed: Option<String>,
    told_permille: u32,
}

impl Core {
    fn stems_cache(&self) -> PathBuf {
        self.data_dir.join(CACHE_DIR)
    }

    /// The song without bass, if it was separated before (no separator needed for that).
    pub(crate) fn without_bass(&self, song: &Path) -> Option<PathBuf> {
        BassRemoval::with_separator(None, &self.stems_cache()).cached(song)
    }

    /// Looks for the separator again unless it was found (installing it needs no restart).
    fn separator(&mut self) -> &BassRemoval {
        let cache = self.stems_cache();
        let found = self
            .stems
            .removal
            .as_ref()
            .is_some_and(BassRemoval::available);
        if !found {
            self.stems.removal = Some(BassRemoval::new(&cache));
        }
        self.stems
            .removal
            .get_or_insert_with(|| BassRemoval::with_separator(None, &cache))
    }

    /// State of the bass removal for the loaded piece's backing track:
    /// `{"available", "state", "progress", "text", "installText"}`; state is "none" (no backing
    /// track), "off", "running", "on" or "failed".
    pub fn stems_json(&mut self) -> String {
        let available = self.separator().available();
        let sandboxed = self.separator().sandboxed();
        let backing = self.backing_file();
        let on = self.loaded_entry().is_some_and(|e| e.backing_without_bass);
        let running = self.stems.job.as_ref().map(|(_, _, job)| job.progress());
        let (state, text) = match (&backing, running, &self.stems.failed) {
            (None, _, _) => ("none", String::new()),
            (Some(_), Some(p), _) => (
                "running",
                self.tr("stems.running", &[("percent", format!("{:.0}", p * 100.0))]),
            ),
            (Some(_), None, Some(reason)) => (
                "failed",
                self.tr("stems.failed", &[("reason", reason.clone())]),
            ),
            (Some(song), None, None) if on && self.without_bass(song).is_some() => {
                ("on", self.tr("stems.on", &[]))
            }
            (Some(_), None, None) if available => ("off", self.tr("stems.hint", &[])),
            (Some(_), None, None) if sandboxed => ("off", self.tr("stems.sandbox", &[])),
            (Some(_), None, None) => ("off", self.tr("stems.missing", &[])),
        };
        json!({
            "available": available,
            "state": state,
            "progress": running.unwrap_or(0.0),
            "text": text,
            "installText": match (available, sandboxed) {
                (true, _) => String::new(),
                (false, true) => self.tr("stems.sandbox", &[]),
                (false, false) => self.tr("stems.install", &[]),
            },
        })
        .to_string()
    }

    /// Plays the backing track with (`false`) or without its bass (`true`); separates it first
    /// when needed. Ends a running practice.
    pub fn remove_bass(&mut self, on: bool) {
        self.stems.failed = None;
        let Some(piece) = self.loaded_path().map(Path::to_path_buf) else {
            return;
        };
        let Some(song) = self.backing_file() else {
            return;
        };
        if on && self.without_bass(&song).is_none() {
            if self.stems.job.is_some() {
                return;
            }
            match self.separator().start(&song) {
                Some(Ok(job)) => {
                    self.stems.told_permille = 0;
                    self.stems.job = Some((piece, song, job));
                }
                Some(Err(e)) => self.stems.failed = Some(e.to_string()),
                None => {}
            }
            return;
        }
        self.set_without_bass(&piece, on);
    }

    /// Stops a running separation.
    pub fn cancel_stems(&mut self) {
        if let Some((_, _, mut job)) = self.stems.job.take() {
            job.cancel();
        }
    }

    /// Progress and the end of a separation, for `tick`.
    pub(crate) fn poll_stems(&mut self) -> Vec<UiEvent> {
        let Some((_, _, job)) = self.stems.job.as_mut() else {
            return Vec::new();
        };
        let Some(result) = job.poll() else {
            let permille = (job.progress() * PERMILLE) as u32;
            if permille.abs_diff(self.stems.told_permille) < PROGRESS_STEP {
                return Vec::new();
            }
            self.stems.told_permille = permille;
            return vec![UiEvent::Stems(self.stems_json())];
        };
        let Some((piece, song, _)) = self.stems.job.take() else {
            return Vec::new();
        };
        let mut events = Vec::new();
        match result {
            Ok(_) => {
                self.set_without_bass(&piece, true);
                events.push(UiEvent::Message(
                    self.tr("stems.done", &[("name", file_name(&song))]),
                ));
            }
            Err(kontra_session::SeparatorError::Cancelled) => {}
            Err(e) => self.stems.failed = Some(e.to_string()),
        }
        events.push(UiEvent::Stems(self.stems_json()));
        events
    }

    fn set_without_bass(&mut self, piece: &Path, on: bool) {
        if let Some(entry) = self.library.entry_mut(piece) {
            entry.backing_without_bass = on;
        }
        let _ = store::save(&self.data_dir.join("library.json"), &self.library);
        self.stop_live();
    }

    fn loaded_entry(&self) -> Option<&store::RecentEntry> {
        self.library.entry(self.loaded_path()?)
    }

    /// The backing file chosen for the loaded piece, as chosen (with its bass).
    fn backing_file(&self) -> Option<PathBuf> {
        self.loaded_entry()?.backing.clone().filter(|p| p.exists())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::QaOptions;

    fn demo() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/demo/etude-1.mid")
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("kontra-core-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    /// A stand-in for Demucs that copies the song as its result.
    fn fake_demucs(dir: &Path) -> PathBuf {
        let script = dir.join("demucs");
        std::fs::write(
            &script,
            "#!/bin/sh\nname=$(basename \"$7\"); name=\"${name%.*}\"\nprintf ' 50%%|##   |\\r' >&2\nmkdir -p \"$6/htdemucs/$name\" && cp \"$7\" \"$6/htdemucs/$name/no_bass.wav\"\n",
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        script
    }

    #[test]
    fn backing_plays_without_bass_once_separated() {
        let dir = temp_dir("stems");
        std::fs::create_dir_all(&dir).unwrap();
        let song = dir.join("Song.wav");
        kontra_session::write_mono_wav(&song, &vec![0.0; 4_800], 48_000).unwrap();
        let mut core = Core::new(&dir.join("jens"), QaOptions::default());
        core.open(&demo()).unwrap();
        core.set_backing(Some(song.clone()), 0.5);
        core.stems.removal = Some(kontra_session::BassRemoval::with_separator(
            Some(kontra_session::Separator::with_program(&fake_demucs(&dir))),
            &dir.join("jens/stems"),
        ));
        let state = |core: &mut Core| -> serde_json::Value {
            serde_json::from_str(&core.stems_json()).unwrap()
        };
        assert_eq!(state(&mut core)["state"], "off");

        core.remove_bass(true);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while core.stems.job.is_some() && std::time::Instant::now() < deadline {
            core.tick();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(state(&mut core)["state"], "on");
        let (played, offset) = core.backing().unwrap();
        assert_ne!(played, song, "the separated file plays");
        assert_eq!(offset, 0.5, "with the same offset");
        assert_eq!(core.backing_name(), "Song.wav");

        core.remove_bass(false);
        assert_eq!(core.backing().unwrap().0, song);
        assert_eq!(state(&mut core)["state"], "off");
        // Cached: on again without a new separation.
        core.remove_bass(true);
        assert!(core.stems.job.is_none());
        assert_eq!(state(&mut core)["state"], "on");
    }

    #[test]
    fn missing_separator_explains_the_install() {
        let dir = temp_dir("stems-missing");
        std::fs::create_dir_all(&dir).unwrap();
        let song = dir.join("Song.wav");
        kontra_session::write_mono_wav(&song, &vec![0.0; 4_800], 48_000).unwrap();
        let mut core = Core::new(&dir.join("jens"), QaOptions::default());
        core.update_settings(|s| s.language = Some("de".into()));
        core.open(&demo()).unwrap();
        core.set_backing(Some(song), 0.0);
        core.stems.removal = Some(kontra_session::BassRemoval::with_separator(
            None,
            &dir.join("jens/stems"),
        ));
        // `stems_json` looks again; a test machine may have Demucs, so only check the texts.
        let json: serde_json::Value = serde_json::from_str(&core.stems_json()).unwrap();
        if json["available"] == false {
            assert!(json["installText"]
                .as_str()
                .unwrap()
                .contains("pipx install demucs"));
            assert!(json["text"].as_str().unwrap().contains("nicht installiert"));
        }
    }
}
