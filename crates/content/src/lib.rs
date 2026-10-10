//! Content layer: imports Guitar Pro (3-8), MIDI and MusicXML files into `Arrangement`s and
//! generates tabs for notes without string and fret.
//!
//! ```text
//! import_file / import_bytes ─→ ImportResult ─tracks()/suggested()─→ arrangement(track)
//!                                                                      └→ assign_tab
//! ```
//! Malformed input yields an `ImportError`, never a panic.

mod build;
mod bytes;
mod error;
pub mod exercise;
mod gp;
mod midi;
mod musicxml;
mod package;
mod score;
mod tab;
mod unroll;
mod xml;
mod zip;

pub use error::ImportError;
pub use package::{
    read_package, write_package, Manifest, Package, PackageBacking, PACKAGE_FORMAT, PACKAGE_VERSION,
};
pub use tab::assign_tab;

use kontra_model::Arrangement;
use score::Score;
use std::path::Path;

/// Files larger than this are refused before reading (scores are kilobytes to a few MB).
const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
const GP_MAGIC: &[u8] = b"FICHIER GUITAR PRO";
const MIDI_MAGIC: &[u8] = b"MThd";
const ZIP_MAGIC: &[u8] = b"PK\x03\x04";
const GP7_SCORE: &str = "Content/score.gpif";
/// Bytes searched for a MusicXML root element.
const XML_SNIFF: usize = 4096;

/// Supported file formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    /// Guitar Pro 3, 4, 5 (`.gp3`, `.gp4`, `.gp5`).
    GuitarPro,
    /// Guitar Pro 6 (`.gpx`).
    GuitarPro6,
    /// Guitar Pro 7 and 8 (`.gp`).
    GuitarPro7,
    /// Standard MIDI file (`.mid`, `.midi`).
    Midi,
    /// Uncompressed MusicXML (`.musicxml`, `.xml`).
    MusicXml,
    /// Compressed MusicXML (`.mxl`).
    MusicXmlZip,
}

impl FileKind {
    /// Kind from the file extension, case-insensitive.
    pub fn from_extension(extension: &str) -> Option<FileKind> {
        match extension.to_ascii_lowercase().as_str() {
            "gp3" | "gp4" | "gp5" => Some(FileKind::GuitarPro),
            "gpx" => Some(FileKind::GuitarPro6),
            "gp" => Some(FileKind::GuitarPro7),
            "mid" | "midi" | "kar" => Some(FileKind::Midi),
            "musicxml" | "xml" => Some(FileKind::MusicXml),
            "mxl" => Some(FileKind::MusicXmlZip),
            _ => None,
        }
    }

    /// Kind from the first bytes; zip archives are told apart by their content.
    fn sniff(data: &[u8]) -> Option<FileKind> {
        if data.get(1..).is_some_and(|d| d.starts_with(GP_MAGIC)) {
            return Some(FileKind::GuitarPro);
        }
        if data.starts_with(b"BCFZ") || data.starts_with(b"BCFS") {
            return Some(FileKind::GuitarPro6);
        }
        if data.starts_with(MIDI_MAGIC) {
            return Some(FileKind::Midi);
        }
        if data.starts_with(ZIP_MAGIC) {
            let archive = zip::Zip::open(data).ok()?;
            let gp7 = archive.names().any(|n| n == GP7_SCORE);
            return Some(if gp7 {
                FileKind::GuitarPro7
            } else {
                FileKind::MusicXmlZip
            });
        }
        let head = String::from_utf8_lossy(&data[..data.len().min(XML_SNIFF)]).into_owned();
        if head.contains("<score-partwise") || head.contains("<score-timewise") {
            return Some(FileKind::MusicXml);
        }
        None
    }
}

/// One track (part) of an imported file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackInfo {
    name: String,
    is_bass: bool,
    strings: usize,
}

impl TrackInfo {
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Guess from name, MIDI program, tuning and register.
    pub fn is_bass(&self) -> bool {
        self.is_bass
    }

    /// Strings in the file's tuning; 0 when the format has no tab information.
    pub fn strings(&self) -> usize {
        self.strings
    }
}

/// A parsed file: lists its tracks and builds an arrangement for the chosen one.
#[derive(Debug, Clone)]
pub struct ImportResult {
    score: Score,
    tracks: Vec<TrackInfo>,
    suggested: Option<usize>,
}

impl ImportResult {
    fn new(score: Score) -> Result<ImportResult, ImportError> {
        if score.parts.iter().all(|p| p.notes().next().is_none()) {
            return Err(ImportError::Empty);
        }
        let scores: Vec<i32> = score.parts.iter().map(build::bass_score).collect();
        let tracks = score
            .parts
            .iter()
            .zip(&scores)
            .map(|(part, value)| TrackInfo {
                name: part.name.clone(),
                is_bass: *value >= build::BASS_THRESHOLD,
                strings: part.tuning.len(),
            })
            .collect();

        // Highest score wins; ties go to the first track.
        let mut suggested: Option<usize> = None;
        for (index, value) in scores.iter().enumerate() {
            if *value < build::BASS_THRESHOLD {
                continue;
            }
            if suggested.is_none_or(|best| *value > scores[best]) {
                suggested = Some(index);
            }
        }
        Ok(ImportResult {
            score,
            tracks,
            suggested,
        })
    }

    pub fn title(&self) -> &str {
        &self.score.title
    }

    pub fn artist(&self) -> &str {
        &self.score.artist
    }

    pub fn tracks(&self) -> &[TrackInfo] {
        &self.tracks
    }

    /// The track most likely to be the bass, if any looks like one.
    pub fn suggested(&self) -> Option<usize> {
        self.suggested
    }

    /// Whether `track` brings its own tuning; without one the app may use the player's instrument.
    pub fn has_tuning(&self, track: usize) -> bool {
        self.score.parts.get(track).is_some_and(build::has_tuning)
    }

    /// Linear arrangement of `track` with repeats unrolled and tab filled in.
    pub fn arrangement(&self, track: usize) -> Result<Arrangement, ImportError> {
        build::arrangement(&self.score, track)
    }
}

/// Reads and parses a file; the format comes from its content, else its extension.
pub fn import_file(path: &Path) -> Result<ImportResult, ImportError> {
    let size = std::fs::metadata(path)?.len();
    if size > MAX_FILE_BYTES {
        return Err(ImportError::TooLarge(size));
    }
    let data = std::fs::read(path)?;
    let by_extension = path
        .extension()
        .and_then(|e| e.to_str())
        .and_then(FileKind::from_extension);
    let kind = FileKind::sniff(&data)
        .or(by_extension)
        .ok_or(ImportError::UnknownFormat)?;
    import_bytes(&data, kind)
}

/// Parses file content of a known kind.
pub fn import_bytes(data: &[u8], kind: FileKind) -> Result<ImportResult, ImportError> {
    let score = match kind {
        FileKind::GuitarPro => gp::binary::parse(data)?,
        FileKind::GuitarPro6 => gp::gpx(data)?,
        FileKind::GuitarPro7 => gp::gp7(data)?,
        FileKind::Midi => midi::parse(data)?,
        FileKind::MusicXml => musicxml::parse(data)?,
        FileKind::MusicXmlZip => musicxml::parse_mxl(data)?,
    };
    ImportResult::new(score)
}
