//! `.kontra` packages in the library (LH-BIB-07, J-D2): share a piece with its progress and,
//! only when chosen, its backing audio; a package opens like a score. Format: `FORMAT.md`.
//!
//! Opening unpacks into the data folder, so the piece stays in the library:
//! `<data>/packages/<package name>/<score file>` (+ backing file).

use crate::{data, file_name, Core};
use kontra_session::{
    import_file, read_package, write_package, Manifest, Package, PackageBacking, ProgressStore,
    SongProgress, PACKAGE_FORMAT, PACKAGE_VERSION,
};
use std::path::{Path, PathBuf};

const EXTENSION: &str = "kontra";
const UNPACK_DIR: &str = "packages";

/// Whether the backing audio goes into the package or is only named.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageAudio {
    Embed,
    Link,
}

pub(crate) fn is_package(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(EXTENSION))
}

impl Core {
    /// Writes the library piece at `piece` as a package to `out`.
    pub fn export_package(
        &self,
        piece: &Path,
        out: &Path,
        audio: PackageAudio,
    ) -> Result<String, String> {
        let entry = self
            .library
            .entry(piece)
            .ok_or_else(|| self.tr("package.error.unknown", &[]))?;
        let read_error = |p: &Path| self.tr("package.error.missing", &[("name", file_name(p))]);
        let score = std::fs::read(piece).map_err(|_| read_error(piece))?;
        let (backing, audio_bytes) = match (&entry.backing, audio) {
            (None, _) => (None, None),
            (Some(path), PackageAudio::Embed) => {
                let bytes = std::fs::read(path).map_err(|_| read_error(path))?;
                let file = PackageBacking::Embedded {
                    file: file_name(path),
                    offset_s: entry.backing_offset_s,
                };
                (Some(file), Some(bytes))
            }
            (Some(path), PackageAudio::Link) => {
                let link = PackageBacking::Linked {
                    path: path.to_string_lossy().into(),
                    offset_s: entry.backing_offset_s,
                };
                (Some(link), None)
            }
        };
        let progress = self
            .progress_store()
            .load_id(&entry.song_id)
            .and_then(|p| serde_json::to_vec_pretty(&p).ok());
        let package = Package {
            manifest: Manifest {
                format: PACKAGE_FORMAT.into(),
                version: PACKAGE_VERSION,
                title: entry.title.clone(),
                artist: entry.artist.clone(),
                score: file_name(piece),
                track: entry.track,
                backing,
            },
            score,
            backing: audio_bytes,
            progress,
        };
        write_package(out, &package)
            .map_err(|e| self.tr("data.error.write", &[("reason", e.to_string())]))?;
        let key = match package.backing {
            Some(_) => "package.saved.audio",
            None => "package.saved",
        };
        Ok(self.tr(key, &[("name", file_name(out))]))
    }

    /// Unpacks a package into the data folder, opens its piece, sets its backing and joins
    /// its progress with the own (runs already there are not doubled).
    pub(crate) fn open_package(&mut self, path: &Path) -> Result<(), String> {
        let package = read_package(path).map_err(|e| self.import_error(&e))?;
        let m = &package.manifest;
        let stem = path
            .file_stem()
            .map_or_else(|| "package".into(), |s| s.to_string_lossy().to_string());
        let dir = self.data_dir.join(UNPACK_DIR).join(stem);
        let write_error =
            |e: std::io::Error| self.tr("data.error.write", &[("reason", e.to_string())]);
        std::fs::create_dir_all(&dir).map_err(write_error)?;
        let score = dir.join(&m.score);
        std::fs::write(&score, &package.score).map_err(write_error)?;
        let backing = match (&m.backing, &package.backing) {
            (Some(PackageBacking::Embedded { file, offset_s }), Some(bytes)) => {
                let audio = dir.join(file);
                std::fs::write(&audio, bytes).map_err(write_error)?;
                Some((audio, *offset_s))
            }
            (Some(PackageBacking::Linked { path, offset_s }), _) => {
                Some((PathBuf::from(path), *offset_s))
            }
            _ => None,
        };

        let imported = import_file(&score).map_err(|e| self.import_error(&e))?;
        let track = m.track.min(imported.tracks().len().saturating_sub(1));
        self.load(&score, &imported, track)?;
        if let Some((audio, offset_s)) = backing {
            if let Some(entry) = self.library.entry_mut(&score) {
                entry.backing = Some(audio);
                entry.backing_offset_s = offset_s;
            }
            self.save_library();
        }
        let runs = self.merge_package_progress(package.progress.as_deref());
        let title = self
            .arrangement()
            .map_or_else(String::new, |a| a.title.clone());
        self.notice = Some(self.tr(
            "package.opened",
            &[("title", title), ("runs", runs.to_string())],
        ));
        Ok(())
    }

    /// Adds the package's runs to the loaded piece's progress; returns how many were new.
    fn merge_package_progress(&self, progress: Option<&[u8]>) -> usize {
        let Some(incoming) = progress.and_then(|b| serde_json::from_slice::<SongProgress>(b).ok())
        else {
            return 0;
        };
        let Some(arrangement) = self.arrangement() else {
            return 0;
        };
        let store = self.progress_store();
        // The id comes from the piece as loaded here, not from the file (it names a path).
        let mut own = store.load(arrangement).unwrap_or_else(|_| SongProgress {
            song_id: ProgressStore::song_id(arrangement),
            title: arrangement.title.clone(),
            artist: arrangement.artist.clone(),
            runs: Vec::new(),
        });
        let added = data::merge_runs(&mut own, incoming);
        if store.save(&own).is_err() {
            return 0;
        }
        added
    }
}
