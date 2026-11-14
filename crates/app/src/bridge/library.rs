//! Library: import, search, setlists, tags, caches, progress, exercises and data export.

use super::{file_url, local_path, q, qobject};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use kontra_appcore::{Core, DataImport, DensityLock, Opened, PackageAudio};
use std::path::PathBuf;

impl qobject::Controller {
    pub(super) fn song_map(&self) -> QString {
        q(&self.rust().core.song_map_json())
    }

    pub(super) fn folder_url(&self, path: &QString) -> QString {
        let path = PathBuf::from(path.to_string());
        let folder = path.parent().filter(|p| !p.as_os_str().is_empty());
        folder.map_or_else(QString::default, |p| q(&file_url(p)))
    }

    pub(super) fn import_folder(mut self: Pin<&mut Self>, url: &QString, as_setlist: bool) {
        let dir = local_path(&url.to_string());
        let (added, failed) = self.as_mut().core().import_folder(&dir, as_setlist);
        let (text, recent, lists) = {
            let this = self.as_ref();
            let core = &this.rust().core;
            let mut text = core.imported_text(added);
            for (name, reason) in &failed {
                text.push_str(&format!("\n{name}: {reason}"));
            }
            (text, core.recent_json(), core.setlists_json())
        };
        self.as_mut().set_recent_json(q(&recent));
        self.as_mut().set_setlists_json(q(&lists));
        self.as_mut().set_message(q(&text));
    }

    pub(super) fn import_files(mut self: Pin<&mut Self>, urls_json: &QString) {
        let urls: Vec<String> = serde_json::from_str(&urls_json.to_string()).unwrap_or_default();
        let files: Vec<PathBuf> = urls.iter().map(|u| local_path(u)).collect();
        let (added, failed) = self.as_mut().core().import_files(&files);
        let (text, recent) = {
            let this = self.as_ref();
            let core = &this.rust().core;
            let mut text = core.imported_text(added);
            for (name, reason) in &failed {
                text.push_str(&format!("\n{name}: {reason}"));
            }
            (text, core.recent_json())
        };
        self.as_mut().set_recent_json(q(&recent));
        self.as_mut().set_message(q(&text));
    }

    pub(super) fn library_json(
        &self,
        source: &QString,
        sort: &QString,
        group: &QString,
        query: &QString,
    ) -> QString {
        q(&self.rust().core.library_json(
            &source.to_string(),
            &sort.to_string(),
            &group.to_string(),
            &query.to_string(),
        ))
    }

    pub(super) fn is_folder(&self, url: &QString) -> bool {
        local_path(&url.to_string()).is_dir()
    }

    pub(super) fn link_folder(mut self: Pin<&mut Self>, url: &QString) {
        let dir = local_path(&url.to_string());
        if let Err(e) = self.as_mut().core().link_folder(&dir) {
            self.as_mut().show_error(&e);
            return;
        }
        // The tree shows the folder before its walk ends.
        self.as_mut().bump_library();
    }

    pub(super) fn unlink_folder(mut self: Pin<&mut Self>, path: &QString) {
        self.as_mut()
            .core()
            .unlink_folder(&PathBuf::from(path.to_string()));
        self.as_mut().bump_library();
    }

    pub(super) fn bump_library(mut self: Pin<&mut Self>) {
        let revision = self.as_ref().library_revision().wrapping_add(1);
        self.as_mut().set_library_revision(revision);
    }

    pub(super) fn library_rescan(mut self: Pin<&mut Self>) {
        self.as_mut().core().library_rescan();
    }

    pub(super) fn search_url(&self, site: &QString, query: &QString) -> QString {
        q(&self
            .rust()
            .core
            .search_url(&site.to_string(), &query.to_string()))
    }

    pub(super) fn downloads_json(&self) -> QString {
        q(&self.rust().core.downloads_json())
    }

    pub(super) fn song_search(mut self: Pin<&mut Self>, query: &QString) {
        let json = self.as_mut().core().song_search(&query.to_string());
        self.as_mut().set_song_search_json(q(&json));
    }

    pub(super) fn refresh_setlists(mut self: Pin<&mut Self>) {
        let (lists, next) = {
            let this = self.as_ref();
            let core = &this.rust().core;
            (core.setlists_json(), core.next_title().unwrap_or_default())
        };
        self.as_mut().set_setlists_json(q(&lists));
        self.as_mut().set_next_title(q(&next));
    }

    pub(super) fn setlist_create(mut self: Pin<&mut Self>, name: &QString) {
        self.as_mut().core().setlist_create(&name.to_string());
        self.as_mut().refresh_setlists();
    }

    pub(super) fn setlist_delete(mut self: Pin<&mut Self>, name: &QString) {
        self.as_mut().core().setlist_delete(&name.to_string());
        self.as_mut().refresh_setlists();
    }

    pub(super) fn setlist_add_loaded(mut self: Pin<&mut Self>, name: &QString) {
        self.as_mut().core().setlist_add_loaded(&name.to_string());
        self.as_mut().refresh_setlists();
    }

    pub(super) fn setlist_add(mut self: Pin<&mut Self>, name: &QString, path: &QString) {
        let path = PathBuf::from(path.to_string());
        self.as_mut().core().setlist_add(&name.to_string(), &path);
        self.as_mut().refresh_setlists();
    }

    pub(super) fn setlist_remove(mut self: Pin<&mut Self>, name: &QString, index: i32) {
        let index = usize::try_from(index).unwrap_or(usize::MAX);
        self.as_mut()
            .core()
            .setlist_remove(&name.to_string(), index);
        self.as_mut().refresh_setlists();
    }

    pub(super) fn setlist_move(mut self: Pin<&mut Self>, name: &QString, index: i32, step: i32) {
        let index = usize::try_from(index).unwrap_or(usize::MAX);
        self.as_mut()
            .core()
            .setlist_move(&name.to_string(), index, step.signum());
        self.as_mut().refresh_setlists();
    }

    pub(super) fn open_from_setlist(mut self: Pin<&mut Self>, name: &QString, index: i32) -> bool {
        let index = usize::try_from(index).unwrap_or(usize::MAX);
        let result = self
            .as_mut()
            .core()
            .open_from_setlist(&name.to_string(), index);
        self.as_mut().after_open(result)
    }

    pub(super) fn switch_track(mut self: Pin<&mut Self>, track: i32) {
        let track = usize::try_from(track).unwrap_or(usize::MAX);
        match self.as_mut().core().switch_track(track) {
            Ok(()) => self.as_mut().refresh_arrangement(),
            Err(e) => self.as_mut().show_error(&e),
        }
    }

    pub(super) fn open_next(mut self: Pin<&mut Self>) -> bool {
        let result = self.as_mut().core().open_next();
        self.as_mut().after_open(result)
    }

    pub(super) fn set_tags(mut self: Pin<&mut Self>, path: &QString, tags_json: &QString) {
        let tags: Vec<String> = serde_json::from_str(&tags_json.to_string()).unwrap_or_default();
        self.as_mut()
            .core()
            .set_tags(&local_path(&path.to_string()), &tags);
        let recent = self.as_ref().rust().core.recent_json();
        self.as_mut().set_recent_json(q(&recent));
    }

    pub(super) fn tags_json(&self) -> QString {
        q(&self.rust().core.tags_json())
    }

    pub(super) fn licences_json(&self) -> QString {
        q(&self.rust().core.licences_json())
    }

    pub(super) fn caches_json(&self) -> QString {
        q(&self.rust().core.caches_json())
    }

    // The practice page's bass choice follows: a deleted separated song plays with its bass.
    pub(super) fn forget_cached(mut self: Pin<&mut Self>, cache: &QString, key: &QString) {
        self.as_mut()
            .core()
            .forget_cached(&cache.to_string(), &key.to_string());
        self.as_mut().show_stems();
    }

    pub(super) fn clear_cache(mut self: Pin<&mut Self>, cache: &QString) {
        self.as_mut().core().clear_cache(&cache.to_string());
        self.as_mut().show_stems();
    }

    pub(super) fn progress_json(&self) -> QString {
        q(&self.rust().core.progress_json())
    }

    /// Start page; the streak counts days in local time (LH-FOR-05).
    pub(super) fn home_json(&self) -> QString {
        let offset_s = cxx_qt_lib::QDateTime::current_date_time().offset_from_utc();
        q(&self.rust().core.home_json(offset_s))
    }

    pub(super) fn exercises_json(&self) -> QString {
        q(&self.rust().core.bundled_json())
    }

    pub(super) fn generate_exercise(
        mut self: Pin<&mut Self>,
        kind: &QString,
        variant: &QString,
        rhythm: &QString,
        key: i32,
        bpm: f64,
        judging: &QString,
    ) -> bool {
        let Some(spec) = Core::exercise_spec(
            &kind.to_string(),
            &variant.to_string(),
            &rhythm.to_string(),
            u8::try_from(key.rem_euclid(12)).unwrap_or(0),
            bpm,
            &judging.to_string(),
            kontra_appcore::PieceVariety::default(),
        ) else {
            return false;
        };
        match self.as_mut().core().generate_exercise(&spec) {
            Ok(path) => self.as_mut().open_file(&q(&path.to_string_lossy())),
            Err(e) => {
                self.as_mut().show_error(&e);
                false
            }
        }
    }

    pub(super) fn generate_piece(
        mut self: Pin<&mut Self>,
        style: &QString,
        level: &QString,
        key: i32,
        bpm: f64,
        seed: i32,
        length: &QString,
    ) -> bool {
        let length = length.to_string();
        let Some(spec) = Core::exercise_spec(
            "song",
            &style.to_string(),
            &level.to_string(),
            u8::try_from(key.rem_euclid(12)).unwrap_or(0),
            bpm,
            "",
            kontra_appcore::PieceVariety {
                seed: u64::try_from(seed).unwrap_or(0),
                length: &length,
            },
        ) else {
            return false;
        };
        match self.as_mut().core().generate_exercise(&spec) {
            Ok(path) => self.as_mut().open_file(&q(&path.to_string_lossy())),
            Err(e) => {
                self.as_mut().show_error(&e);
                false
            }
        }
    }

    pub(super) fn export_data(mut self: Pin<&mut Self>, url: &QString) {
        let path = local_path(&url.to_string());
        match self.as_ref().rust().core.export_data(&path) {
            Ok(text) => self.as_mut().set_message(q(&text)),
            Err(e) => self.as_mut().show_error(&e),
        }
    }

    pub(super) fn import_data(mut self: Pin<&mut Self>, url: &QString, how: &QString) {
        let path = local_path(&url.to_string());
        let how = match how.to_string().as_str() {
            "replace" => DataImport::Replace,
            _ => DataImport::Merge,
        };
        match self.as_mut().core().import_data(&path, how) {
            Ok(text) => {
                self.as_mut().sync_settings();
                self.as_mut().set_message(q(&text));
            }
            Err(e) => self.as_mut().show_error(&e),
        }
    }

    pub(super) fn export_report(mut self: Pin<&mut Self>, song_id: &QString, url: &QString) {
        let path = local_path(&url.to_string());
        let result = self
            .as_ref()
            .rust()
            .core
            .export_report(&song_id.to_string(), &path);
        match result {
            Ok(text) => self.as_mut().set_message(q(&text)),
            Err(e) => self.as_mut().show_error(&e),
        }
    }

    pub(super) fn export_package(
        mut self: Pin<&mut Self>,
        piece: &QString,
        url: &QString,
        audio: &QString,
    ) {
        let piece = local_path(&piece.to_string());
        let out = local_path(&url.to_string());
        let audio = match audio.to_string().as_str() {
            "embed" => PackageAudio::Embed,
            _ => PackageAudio::Link,
        };
        let result = self
            .as_ref()
            .rust()
            .core
            .export_package(&piece, &out, audio);
        match result {
            Ok(text) => self.as_mut().set_message(q(&text)),
            Err(e) => self.as_mut().show_error(&e),
        }
    }
}

impl super::ControllerRust {
    /// Opening at start-up, before Qt owns the object (fields set directly).
    pub(super) fn open_now(&mut self, path: &std::path::Path) {
        match self.core.open(path) {
            Ok(Opened::Ready) => {
                self.loaded = true;
                self.title = q(&self.core.loaded_title());
                self.artist = q(&self
                    .core
                    .arrangement()
                    .map_or(String::new(), |a| a.artist.clone()));
                self.notes_json = q(&self.core.notes_json());
                self.bars_json = q(&self.core.bars_json());
                self.staff_json = q(&self.core.staff_json());
                self.string_names_json = q(&self.core.string_names_json());
                self.key_range_json = q(&self.core.key_range_json());
                self.duration = self.core.duration_s();
                self.recent_json = q(&self.core.recent_json());
                self.backing_name = q(&self.core.backing_name());
                self.stems_json = q(&self.core.stems_json());
                self.transpose = self.core.transpose();
                self.saved_loops_json = q(&self.core.saved_loops_json());
                self.target_speed = self.core.target_speed();
                self.view = q(&self.core.view());
                self.tracks_json = q(&self.core.tracks_json());
                let (density, lock) = self.core.density();
                self.density = i32::from(density.percent());
                self.density_fixed = lock == DensityLock::Fixed;
                if let Some(notice) = self.core.take_notice() {
                    self.message = q(&notice);
                }
            }
            Ok(Opened::ChooseTrack(names)) => {
                self.track_choices_json = q(&serde_json::to_string(&names).unwrap_or_default());
            }
            Err(e) => self.error_text = q(&e),
        }
    }
}
