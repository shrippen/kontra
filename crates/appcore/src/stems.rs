//! Backing track without its bass (LH-BIB-09, round 9): Demucs removes it, if installed; Kontra
//! ships nothing of it. Without it the choice is greyed out and says how to install it.
//!
//! ```text
//! remove_bass(true) ─► cached? ─yes─► entry.backing_without_bass = true
//!                          └─no──► StemJob ─tick─► UiEvent::Stems(progress) … done ─► cached
//! forget_stems() ─► cached file deleted, the song plays with its bass again
//! ```
//! One separation at a time; a failure belongs to its song. The separated files live in the
//! cache folder (`store::cache_dir`), about 10 MB per minute of song.

use crate::{file_name, store, Core, UiEvent};
use kontra_session::{BassRemoval, SeparatorError, StemJob};
use serde_json::json;
use std::path::{Path, PathBuf};

const CACHE_DIR: &str = "stems";
/// Progress steps the UI is told about (permille).
const PROGRESS_STEP: u32 = 10;
const PERMILLE: f64 = 1000.0;
const BYTES_PER_MB: f64 = 1_000_000.0;
/// Below this many seconds left the estimate is given in seconds, above in minutes.
const SECONDS_SHOWN: u64 = 90;
const SECONDS_PER_MINUTE: f64 = 60.0;

/// The separator, a running separation (piece, song) and the last failure (song).
#[derive(Debug, Default)]
pub(crate) struct Stems {
    removal: Option<BassRemoval>,
    job: Option<(PathBuf, PathBuf, StemJob)>,
    failed: Option<(PathBuf, Failure)>,
    told_permille: u32,
}

/// Why a separation failed; the UI says it in its language.
#[derive(Debug, Clone, PartialEq)]
enum Failure {
    /// The program did not start.
    Start,
    /// It ran and failed; its last line of output (Demucs' own words).
    Program(String),
}

impl Failure {
    fn of(error: &SeparatorError) -> Option<Failure> {
        match error {
            SeparatorError::Start { .. } => Some(Failure::Start),
            SeparatorError::Failed(line) => Some(Failure::Program(line.clone())),
            SeparatorError::Cancelled => None,
        }
    }
}

/// Whether choosing with or without bass ends a running practice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Running {
    /// The player chose: the next play uses it.
    End,
    /// A separation finished on its own: the run goes on, the next play uses it.
    Keep,
}

impl Core {
    /// The cache folder of separated songs; one from before 14.10. (in the data folder) moves
    /// there once.
    fn stems_cache(&self) -> PathBuf {
        let cache = store::cache_dir(&self.data_dir).join(CACHE_DIR);
        let old = self.data_dir.join(CACHE_DIR);
        if old.is_dir() && !cache.exists() {
            if let Some(parent) = cache.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::rename(&old, &cache);
        }
        cache
    }

    /// The song without bass, if it was separated before (no separator needed for that).
    pub(crate) fn without_bass(&self, song: &Path) -> Option<PathBuf> {
        self.cached_stems().cached(song)
    }

    /// The cache alone, without looking for the separator.
    fn cached_stems(&self) -> BassRemoval {
        BassRemoval::with_separator(None, &self.stems_cache())
    }

    /// The separator, looked for once (inside a Flatpak that asks the host, which takes a
    /// moment); `recheck_stems` looks again.
    fn separator(&mut self) -> &BassRemoval {
        let cache = self.stems_cache();
        self.stems
            .removal
            .get_or_insert_with(|| BassRemoval::new(&cache))
    }

    /// QA (journeys): a stand-in for Demucs at `program`; `None` acts as if none is installed.
    pub fn qa_separator(&mut self, program: Option<&Path>) {
        let cache = self.stems_cache();
        let separator = program.map(kontra_session::Separator::with_program);
        self.stems.removal = Some(BassRemoval::with_separator(separator, &cache));
    }

    /// Looks for the separator again, e.g. after installing it (no restart needed).
    pub fn recheck_stems(&mut self) {
        self.stems.removal = None;
    }

    /// State of the bass removal for the loaded piece's backing track:
    /// `{"available", "state", "progress", "text", "installText", "cached", "detail" (of a
    /// failure), "deleteText", "cacheText"}`; state is "none" (no backing track), "off", "running", "busy" (another
    /// song is separated), "on" or "failed".
    pub fn stems_json(&mut self) -> String {
        let available = self.separator().available();
        let sandboxed = self.separator().sandboxed();
        let backing = self.backing_file();
        let on = self.loaded_entry().is_some_and(|e| e.backing_without_bass);
        // A separation started for another piece goes on, but is not this piece's state.
        let piece = self.loaded_path().map(Path::to_path_buf);
        let job = self.stems.job.as_ref();
        let running = job
            .filter(|(of, _, _)| piece.as_ref() == Some(of))
            .map(|(_, _, job)| (job.progress(), job.remaining()));
        let busy = job
            .filter(|(of, _, _)| piece.as_ref() != Some(of))
            .map(|(_, song, _)| file_name(song));
        let failed = self
            .stems
            .failed
            .as_ref()
            .filter(|(song, _)| backing.as_ref() == Some(song))
            .map(|(_, failure)| failure.clone());
        let size = backing.as_ref().and_then(|s| self.cached_stems().size(s));
        let size_text = size.map(|bytes| self.megabytes(bytes));
        let (state, text) = match (&backing, running, &failed) {
            (None, _, _) => ("none", String::new()),
            (Some(_), Some((p, left)), _) => ("running", self.running_text(p, left)),
            (Some(_), None, Some(failure)) => ("failed", self.failure_text(failure)),
            (Some(_), None, None) if on && size.is_some() => (
                "on",
                self.tr(
                    "stems.on",
                    &[("size", size_text.clone().unwrap_or_default())],
                ),
            ),
            (Some(_), None, None) if size.is_some() => (
                "off",
                self.tr(
                    "stems.ready",
                    &[("size", size_text.clone().unwrap_or_default())],
                ),
            ),
            (Some(_), None, None) if available && busy.is_some() => (
                "busy",
                self.tr("stems.busy", &[("name", busy.clone().unwrap_or_default())]),
            ),
            (Some(_), None, None) if available => ("off", self.tr("stems.hint", &[])),
            (Some(_), None, None) if sandboxed => ("off", self.tr("stems.sandbox", &[])),
            (Some(_), None, None) => ("off", self.tr("stems.missing", &[])),
        };
        let (count, bytes) = self.cached_stems().usage();
        let detail = failed
            .as_ref()
            .map(|f| self.failure_detail(f))
            .unwrap_or_default();
        json!({
            "available": available,
            "state": state,
            "progress": running.map_or(0.0, |(p, _)| p),
            "text": text,
            "installText": match (available, sandboxed) {
                (true, _) => String::new(),
                (false, true) => self.tr("stems.sandbox", &[]),
                (false, false) => self.tr("stems.install", &[]),
            },
            "cached": size.is_some(),
            "detail": detail,
            "deleteText": self.tr("stems.delete.hint", &[("size", size_text.unwrap_or_default())]),
            "cacheText": self.tr("stems.where", &[
                ("dir", self.stems_cache().display().to_string()),
                ("count", count.to_string()),
                ("size", self.megabytes(bytes)),
            ]),
        })
        .to_string()
    }

    fn megabytes(&self, bytes: u64) -> String {
        let mb = (bytes as f64 / BYTES_PER_MB).ceil();
        self.tr("stems.mb", &[("mb", format!("{mb:.0}"))])
    }

    /// "Bass wird entfernt … 37 %, noch etwa 2 min".
    fn running_text(&self, progress: f64, left: Option<std::time::Duration>) -> String {
        // Before the first progress line Demucs loads (the first time downloads) its model;
        // after the last one it writes the file.
        if progress <= 0.0 && left.is_none() {
            return self.tr("stems.starting", &[]);
        }
        if progress >= 1.0 {
            return self.tr("stems.finishing", &[]);
        }
        let percent = ("percent", format!("{:.0}", progress * 100.0));
        let Some(left) = left else {
            return self.tr("stems.running", &[percent]);
        };
        let seconds = left.as_secs();
        let time = if seconds < SECONDS_SHOWN {
            self.tr("stems.seconds", &[("n", seconds.to_string())])
        } else {
            let minutes = (seconds as f64 / SECONDS_PER_MINUTE).round();
            self.tr("stems.minutes", &[("n", format!("{minutes:.0}"))])
        };
        self.tr("stems.running.left", &[percent, ("time", time)])
    }

    /// What went wrong, short for the row.
    fn failure_text(&self, failure: &Failure) -> String {
        match failure {
            Failure::Start => self.tr("stems.error.start", &[]),
            Failure::Program(_) => self.tr("stems.failed", &[]),
        }
    }

    /// Demucs' own words and what to do, for the "?" next to it.
    fn failure_detail(&self, failure: &Failure) -> String {
        match failure {
            Failure::Start => self.tr("stems.install", &[]),
            Failure::Program(line) if line.is_empty() => self.tr("stems.failed.silent", &[]),
            Failure::Program(line) => self.tr("stems.failed.detail", &[("reason", line.clone())]),
        }
    }

    /// Plays the backing track with (`false`) or without its bass (`true`); separates it first
    /// when needed. Ends a running practice.
    pub fn remove_bass(&mut self, on: bool) {
        let Some(piece) = self.loaded_path().map(Path::to_path_buf) else {
            return;
        };
        let Some(song) = self.backing_file() else {
            return;
        };
        self.stems.failed = None;
        if on && self.without_bass(&song).is_none() {
            // One at a time; `stems_json` says which song is being separated.
            if self.stems.job.is_some() {
                return;
            }
            match self.separator().start(&song) {
                Some(Ok(job)) => {
                    self.stems.told_permille = 0;
                    self.stems.job = Some((piece, song, job));
                }
                Some(Err(e)) => self.stems.failed = Failure::of(&e).map(|f| (song, f)),
                None => {}
            }
            return;
        }
        self.set_without_bass(&piece, on, Running::End);
    }

    /// Deletes the separated file of the loaded piece's backing track; it plays with its bass
    /// again (separating it again takes the time once more).
    pub fn forget_stems(&mut self) {
        let Some(piece) = self.loaded_path().map(Path::to_path_buf) else {
            return;
        };
        let Some(song) = self.backing_file() else {
            return;
        };
        let on = self.loaded_entry().is_some_and(|e| e.backing_without_bass);
        self.cached_stems().remove(&song);
        if on {
            self.set_without_bass(&piece, false, Running::End);
        }
    }

    /// Stops a running separation.
    pub fn cancel_stems(&mut self) {
        if let Some((_, _, mut job)) = self.stems.job.take() {
            job.cancel();
        }
    }

    /// Stops the separation of `piece`'s backing track (it gets another one).
    pub(crate) fn cancel_stems_of(&mut self, piece: &Path) {
        if self
            .stems
            .job
            .as_ref()
            .is_some_and(|(of, _, _)| of == piece)
        {
            self.cancel_stems();
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
                self.set_without_bass(&piece, true, Running::Keep);
                events.push(UiEvent::Message(
                    self.tr("stems.done", &[("name", file_name(&song))]),
                ));
            }
            Err(e) => self.stems.failed = Failure::of(&e).map(|f| (song, f)),
        }
        events.push(UiEvent::Stems(self.stems_json()));
        events
    }

    fn set_without_bass(&mut self, piece: &Path, on: bool, running: Running) {
        if let Some(entry) = self.library.entry_mut(piece) {
            entry.backing_without_bass = on;
        }
        let _ = store::save(&self.data_dir.join("library.json"), &self.library);
        if running == Running::End {
            self.stop_live();
        }
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
        core.qa_separator(Some(&fake_demucs(&dir)));
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
        assert_eq!(state(&mut core)["cached"], true);

        // Deleted: back to the original, and the next time separates again.
        core.forget_stems();
        assert_eq!(core.backing().unwrap().0, song);
        assert_eq!(state(&mut core)["state"], "off");
        assert_eq!(state(&mut core)["cached"], false);
    }

    /// A stand-in for Demucs from a shell body.
    fn script(dir: &Path, body: &str) -> PathBuf {
        let script = dir.join("demucs");
        std::fs::write(&script, format!("#!/bin/sh\n{body}\n")).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        script
    }

    /// Jens with two pieces, each with its own backing song, and `separator` as Demucs.
    fn two_pieces(name: &str, body: &str) -> (Core, PathBuf, PathBuf) {
        let dir = temp_dir(name);
        std::fs::create_dir_all(&dir).unwrap();
        let mut core = Core::new(&dir.join("jens"), QaOptions::default());
        core.update_settings(|s| s.language = Some("de".into()));
        let pieces = [demo(), demo().with_file_name("clicks.mid")];
        for (i, piece) in pieces.iter().enumerate() {
            let song = dir.join(format!("Song{i}.wav"));
            kontra_session::write_mono_wav(&song, &vec![0.0; 4_800], 48_000).unwrap();
            core.open(piece).unwrap();
            core.set_backing(Some(song), 0.0);
        }
        core.qa_separator(Some(&script(&dir, body)));
        let [first, second] = pieces;
        (core, first, second)
    }

    fn state(core: &mut Core) -> serde_json::Value {
        serde_json::from_str(&core.stems_json()).unwrap()
    }

    fn finish(core: &mut Core) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while core.stems.job.is_some() && std::time::Instant::now() < deadline {
            core.tick();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    /// QA P2 (J-D1, 14.10.): a failure with one song showed on every other piece.
    #[test]
    fn failure_stays_with_its_song() {
        let (mut core, first, second) = two_pieces("stems-fail", "echo 'kaputt' >&2; exit 1");
        core.remove_bass(true);
        finish(&mut core);
        let json = state(&mut core);
        assert_eq!(json["state"], "failed");
        assert!(
            json["detail"].as_str().unwrap().contains("kaputt"),
            "{json}"
        );

        core.open(&first).unwrap();
        assert_eq!(state(&mut core)["state"], "off");
        core.open(&second).unwrap();
        assert_eq!(state(&mut core)["state"], "failed", "still its own failure");
    }

    /// QA P2 (J-D1, 14.10.): while another song was separated, "without bass" did nothing and
    /// said nothing.
    #[test]
    fn says_when_another_song_is_separated() {
        let (mut core, first, _) = two_pieces("stems-busy", "sleep 30");
        core.remove_bass(true);
        assert_eq!(state(&mut core)["state"], "running");

        core.open(&first).unwrap();
        core.remove_bass(true);
        let json = state(&mut core);
        assert_eq!(json["state"], "busy");
        assert!(
            json["text"].as_str().unwrap().contains("Song1.wav"),
            "{json}"
        );
        core.cancel_stems();
    }

    /// QA P2 (J-D1, 14.10.): another backing track for the piece kept Demucs busy with the old
    /// one for minutes.
    #[test]
    fn new_backing_stops_the_old_separation() {
        let (mut core, _, _) = two_pieces("stems-swap", "sleep 30");
        core.remove_bass(true);
        assert!(core.stems.job.is_some());

        core.set_backing(None, 0.0);
        assert!(core.stems.job.is_none());
    }

    /// Before 14.10. separated songs lay in the data folder; they move to the cache once.
    #[test]
    fn old_separations_move_to_the_cache() {
        let dir = temp_dir("stems-move");
        let old = dir.join("jens/stems");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(old.join("0123456789abcdef.wav"), b"RIFF").unwrap();
        let mut core = Core::new(&dir.join("jens"), QaOptions::default());
        core.stems_json();
        assert!(dir.join("jens/cache/stems/0123456789abcdef.wav").is_file());
        assert!(!old.exists());
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
        core.qa_separator(None);
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
