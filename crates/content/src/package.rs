//! `.kontra` packages (LH-BIB-07): a zip with the score file as it was imported, the practice
//! loops and target tempo, and optionally the backing track with its alignment. Never the
//! sender's progress (Runde 15). Specified in `FORMAT.md`.
//!
//! ```text
//! song.kontra (zip)
//! ├── manifest.json     format, version, title, track, loops, target speed, backing
//! ├── score/<file>      the original Guitar Pro, MIDI or MusicXML file
//! ├── alignment.json    optional, opaque here (the backing's beat map)
//! └── audio/<file>      optional, only when the backing is embedded
//! ```

use crate::error::ImportError;
use crate::zip::{self, Zip};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const PACKAGE_FORMAT: &str = "kontra-package";
pub const PACKAGE_VERSION: u32 = 2;
/// Largest backing track a package embeds: what one zip entry may hold when read.
pub const PACKAGE_AUDIO_MAX_BYTES: u64 = zip::MAX_ENTRY;
const MANIFEST: &str = "manifest.json";
const ALIGNMENT: &str = "alignment.json";
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

/// A practice loop: bars `from..=to`, indices from 0 into the bars with repeats unrolled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageLoop {
    pub from: usize,
    pub to: usize,
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
    /// Practice loops the sender set for the piece.
    #[serde(default)]
    pub loops: Vec<PackageLoop>,
    /// Target speed of the workshop, 1.0 = as written; `None`: as written.
    #[serde(default)]
    pub target_speed: Option<f64>,
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
    /// Where the bars fall in the backing track (JSON), if aligned.
    pub alignment: Option<Vec<u8>>,
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
    if let Some(alignment) = &package.alignment {
        entries.push((ALIGNMENT.into(), alignment));
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
    let alignment = zip
        .names()
        .any(|n| n == ALIGNMENT)
        .then(|| zip.read(ALIGNMENT))
        .transpose()?;
    Ok(Package {
        manifest,
        score,
        backing,
        alignment,
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
                loops: vec![
                    PackageLoop { from: 0, to: 3 },
                    PackageLoop { from: 4, to: 7 },
                ],
                target_speed: Some(0.8),
                backing,
            },
            score: b"score bytes".to_vec(),
            backing: Some(b"audio bytes".to_vec()),
            alignment: Some(b"{\"beats\": [0.5, 1.0]}".to_vec()),
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

    /// Runde 15: loops, target speed and alignment travel; a sender's progress never does.
    #[test]
    fn carries_loops_tempo_not_progress() {
        let path = temp("practice");
        let embedded = package(Some(PackageBacking::Embedded {
            file: "song.wav".into(),
            offset_s: 0.5,
        }));
        write_package(&path, &embedded).unwrap();
        let data = std::fs::read(&path).unwrap();
        let zip = Zip::open(&data).unwrap();
        assert!(zip.names().any(|n| n == ALIGNMENT));
        assert!(!zip.names().any(|n| n.contains("progress")));
        let read = read_package(&path).unwrap();
        assert_eq!(read.manifest.loops, embedded.manifest.loops);
        assert_eq!(read.manifest.target_speed, Some(0.8));
        assert_eq!(read.alignment, embedded.alignment);
        assert_eq!(read.manifest.version, 2);
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
