//! `.kontra` packages in the library (LH-BIB-07, J-D2): share a piece with its loops, target
//! speed and, only when chosen, its backing audio with the alignment; a package opens like a
//! score. The sender's progress stays home (Runde 15). Format: `FORMAT.md`.
//!
//! Opening unpacks into the data folder, so the piece stays in the library:
//! `<data>/packages/<package name>/<score file>` (+ backing file).

use crate::store::{BeatMap, PracticeLoop};
use crate::{clamp_speed, file_name, Core};
use kontra_session::{
    import_file, read_package, write_package, Manifest, Package, PackageBacking, PackageLoop,
    PACKAGE_AUDIO_MAX_BYTES, PACKAGE_FORMAT, PACKAGE_VERSION,
};
use std::path::{Path, PathBuf};

const EXTENSION: &str = "kontra";
const UNPACK_DIR: &str = "packages";

const BYTES_PER_MB: u64 = 1_000_000;

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
                let size = std::fs::metadata(path).map_err(|_| read_error(path))?.len();
                if size > PACKAGE_AUDIO_MAX_BYTES {
                    let mb = |b: u64| (b / BYTES_PER_MB).to_string();
                    let args = [("size", mb(size)), ("max", mb(PACKAGE_AUDIO_MAX_BYTES))];
                    return Err(self.tr("package.error.large", &args));
                }
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
        // The alignment belongs to the backing: without one it says nothing.
        let alignment = entry
            .backing
            .as_ref()
            .and_then(|_| self.library.beat_maps.get(piece))
            .and_then(|map| serde_json::to_vec_pretty(map).ok());
        let loops = entry
            .loops
            .iter()
            .map(|l| PackageLoop {
                from: l.from,
                to: l.to,
            })
            .collect();
        let package = Package {
            manifest: Manifest {
                format: PACKAGE_FORMAT.into(),
                version: PACKAGE_VERSION,
                title: entry.title.clone(),
                artist: entry.artist.clone(),
                score: file_name(piece),
                track: entry.track,
                loops,
                target_speed: entry.target_speed,
                backing,
            },
            score,
            backing: audio_bytes,
            alignment,
        };
        write_package(out, &package)
            .map_err(|e| self.tr("data.error.write", &[("reason", e.to_string())]))?;
        let key = match package.backing {
            Some(_) => "package.saved.audio",
            None => "package.saved",
        };
        Ok(self.tr(key, &[("name", file_name(out))]))
    }

    /// Unpacks a package into the data folder, opens its piece and gives it the package's
    /// loops, target speed and aligned backing. Own progress is not touched.
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
        let alignment = package
            .alignment
            .as_deref()
            .and_then(|b| serde_json::from_slice::<BeatMap>(b).ok());

        let imported = import_file(&score).map_err(|e| self.import_error(&e))?;
        let track = m.track.min(imported.tracks().len().saturating_sub(1));
        self.load(&score, &imported, track)?;
        let Some(entry) = self.library.entry_mut(&score) else {
            return Ok(());
        };
        for l in &m.loops {
            let practice = PracticeLoop {
                from: l.from.min(l.to),
                to: l.from.max(l.to),
            };
            if !entry.loops.contains(&practice) {
                entry.loops.push(practice);
            }
        }
        entry.target_speed = m.target_speed.map(clamp_speed);
        let loops = entry.loops.len();
        if let Some((audio, offset_s)) = backing {
            entry.backing = Some(audio);
            entry.backing_offset_s = offset_s;
            match alignment {
                Some(map) => self.library.beat_maps.insert(score.clone(), map),
                None => self.library.beat_maps.remove(&score),
            };
        }
        // Saves the library and lays the bars on the backing as the sender aligned them.
        self.apply_beat_map()?;

        let title = self
            .arrangement()
            .map_or_else(String::new, |a| a.title.clone());
        let percent = (self.target_speed() * 100.0).round().to_string();
        self.notice = Some(self.tr(
            "package.opened",
            &[
                ("title", title),
                ("loops", loops.to_string()),
                ("percent", percent),
            ],
        ));
        Ok(())
    }
}
