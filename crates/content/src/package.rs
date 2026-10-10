//! `.kontra` packages (LH-BIB-07): a zip with the score file as it was imported, the practice
//! progress and optionally the backing track. Specified in `FORMAT.md`.
//!
//! ```text
//! song.kontra (zip)
//! ├── manifest.json     format, version, title, track, backing
//! ├── score/<file>      the original Guitar Pro, MIDI or MusicXML file
//! ├── progress.json     optional, opaque here (the session's progress)
//! └── audio/<file>      optional, only when the backing is embedded
//! ```

use crate::error::ImportError;
use crate::zip::{self, Zip};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const PACKAGE_FORMAT: &str = "kontra-package";
pub const PACKAGE_VERSION: u32 = 1;
const MANIFEST: &str = "manifest.json";
const PROGRESS: &str = "progress.json";
const SCORE_DIR: &str = "score/";
const AUDIO_DIR: &str = "audio/";
/// Packages may carry a backing track; refuse anything far beyond a long song in WAV.
const MAX_PACKAGE_BYTES: u64 = 1024 * 1024 * 1024;

/// The backing track: inside the package or a path on the author's computer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum PackageBacking {
    /// `audio/<file>` in the package.
    Embedded { file: String, offset_s: f64 },
    /// Not included; the receiver chooses their own copy.
    Linked { path: String, offset_s: f64 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub format: String,
    pub version: u32,
    pub title: String,
    #[serde(default)]
    pub artist: String,
    /// File name of the score under `score/`.
    pub score: String,
    /// Track of the score that is played.
    #[serde(default)]
    pub track: usize,
    #[serde(default)]
    pub backing: Option<PackageBacking>,
}

/// A package in memory.
#[derive(Debug, Clone, PartialEq)]
pub struct Package {
    pub manifest: Manifest,
    pub score: Vec<u8>,
    /// Content of the embedded backing file, if any.
    pub backing: Option<Vec<u8>>,
    /// The progress JSON, if any.
    pub progress: Option<Vec<u8>>,
}

/// Writes `package` to `path`.
pub fn write_package(path: &Path, package: &Package) -> Result<(), ImportError> {
    let m = &package.manifest;
    check_name(&m.score)?;
    let manifest =
        serde_json::to_vec_pretty(m).map_err(|e| ImportError::Malformed(e.to_string()))?;
    let score = format!("{SCORE_DIR}{}", m.score);
    let mut entries: Vec<(String, &[u8])> =
        vec![(MANIFEST.into(), &manifest), (score, &package.score)];
    if let Some(progress) = &package.progress {
        entries.push((PROGRESS.into(), progress));
    }
    if let (Some(PackageBacking::Embedded { file, .. }), Some(audio)) =
        (&m.backing, &package.backing)
    {
        check_name(file)?;
        entries.push((format!("{AUDIO_DIR}{file}"), audio));
    }
    let named: Vec<(&str, &[u8])> = entries.iter().map(|(n, d)| (n.as_str(), *d)).collect();
    std::fs::write(path, zip::write(&named)?)?;
    Ok(())
}

/// Reads and checks a package: format, version, safe names, every named file present.
pub fn read_package(path: &Path) -> Result<Package, ImportError> {
    let size = std::fs::metadata(path)?.len();
    if size > MAX_PACKAGE_BYTES {
        return Err(ImportError::TooLarge(size));
    }
    let data = std::fs::read(path)?;
    let zip = Zip::open(&data)?;
    let manifest: Manifest = serde_json::from_slice(&zip.read(MANIFEST)?)
        .map_err(|e| ImportError::Malformed(format!("manifest: {e}")))?;
    if manifest.format != PACKAGE_FORMAT {
        return Err(ImportError::UnknownFormat);
    }
    if manifest.version > PACKAGE_VERSION {
        return Err(ImportError::Unsupported(format!(
            "package version {}",
            manifest.version
        )));
    }
    check_name(&manifest.score)?;
    let score = zip.read(&format!("{SCORE_DIR}{}", manifest.score))?;
    let backing = match &manifest.backing {
        Some(PackageBacking::Embedded { file, .. }) => {
            check_name(file)?;
            Some(zip.read(&format!("{AUDIO_DIR}{file}"))?)
        }
        _ => None,
    };
    let progress = zip
        .names()
        .any(|n| n == PROGRESS)
        .then(|| zip.read(PROGRESS))
        .transpose()?;
    Ok(Package {
        manifest,
        score,
        backing,
        progress,
    })
}

/// A plain file name: no directories, no `..`, not empty (it becomes a file on disk).
fn check_name(name: &str) -> Result<(), ImportError> {
    let plain =
        !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\\', '\0']);
    if plain {
        return Ok(());
    }
    Err(ImportError::Malformed(format!("bad file name {name:?}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn package(backing: Option<PackageBacking>) -> Package {
        Package {
            manifest: Manifest {
                format: PACKAGE_FORMAT.into(),
                version: PACKAGE_VERSION,
                title: "Riff".into(),
                artist: "Band".into(),
                score: "riff.gp5".into(),
                track: 2,
                backing,
            },
            score: b"score bytes".to_vec(),
            backing: Some(b"audio bytes".to_vec()),
            progress: Some(b"{\"runs\": []}".to_vec()),
        }
    }

    fn temp(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("kontra-pkg-{name}-{}.kontra", std::process::id()))
    }

    #[test]
    fn roundtrip_with_and_without_audio() {
        let path = temp("rt");
        let embedded = package(Some(PackageBacking::Embedded {
            file: "song.mp3".into(),
            offset_s: 1.5,
        }));
        write_package(&path, &embedded).unwrap();
        assert_eq!(read_package(&path).unwrap(), embedded);

        // Linked: the audio is not written, even if bytes were handed over.
        let linked = package(Some(PackageBacking::Linked {
            path: "/home/thomas/song.mp3".into(),
            offset_s: 0.0,
        }));
        write_package(&path, &linked).unwrap();
        let read = read_package(&path).unwrap();
        assert_eq!(read.backing, None);
        assert_eq!(read.manifest, linked.manifest);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn refuses_unsafe_names_and_newer_versions() {
        let path = temp("bad");
        let mut evil = package(None);
        evil.manifest.score = "../../.bashrc".into();
        assert!(write_package(&path, &evil).is_err());

        // A hand-made archive with a traversal name in the manifest.
        let manifest = br#"{"format":"kontra-package","version":1,"title":"x","score":"../x"}"#;
        let data = zip::write(&[("manifest.json", manifest), ("score/../x", b"x")]).unwrap();
        std::fs::write(&path, data).unwrap();
        assert!(read_package(&path).is_err());

        let manifest = br#"{"format":"kontra-package","version":99,"title":"x","score":"x"}"#;
        let data = zip::write(&[("manifest.json", manifest), ("score/x", b"x")]).unwrap();
        std::fs::write(&path, data).unwrap();
        assert!(matches!(
            read_package(&path),
            Err(ImportError::Unsupported(_))
        ));
        std::fs::remove_file(path).unwrap();
    }
}
