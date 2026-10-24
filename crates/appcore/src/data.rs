//! All user data in one open JSON file (LH-FOR-03): settings, library with setlists, progress.
//!
//! ```text
//! { "format": "kontra-data", "version": 1, "exported_unix": …,
//!   "settings": {…settings.json}, "library": {…library.json}, "progress": [{…song}, …] }
//! ```
//!
//! Import either merges (keeps the own settings, adds unknown pieces and setlists, joins runs)
//! or replaces everything.

use crate::store::{Library, Settings};
use kontra_session::SongProgress;
use serde::{Deserialize, Serialize};

pub const FORMAT: &str = "kontra-data";
pub const VERSION: u32 = 1;
/// Song ids are FNV-1a hashes as 16 hex digits; anything else could leave the progress folder.
const SONG_ID_LEN: usize = 16;

/// How an import treats what is already there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataImport {
    Merge,
    Replace,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bundle {
    pub format: String,
    pub version: u32,
    #[serde(default)]
    pub exported_unix: u64,
    pub settings: Settings,
    pub library: Library,
    #[serde(default)]
    pub progress: Vec<SongProgress>,
}

/// Why a file cannot be imported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BundleError {
    /// Not JSON or not Kontra's structure.
    Unreadable,
    /// Written by a newer Kontra.
    Newer(u32),
    /// A progress entry with an id that is not a song id.
    BadSongId,
}

/// What a merge or replace changed, for the report.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Merged {
    pub pieces: usize,
    pub setlists: usize,
    pub runs: usize,
}

impl Bundle {
    pub fn parse(text: &str) -> Result<Bundle, BundleError> {
        let bundle: Bundle = serde_json::from_str(text).map_err(|_| BundleError::Unreadable)?;
        if bundle.format != FORMAT {
            return Err(BundleError::Unreadable);
        }
        if bundle.version > VERSION {
            return Err(BundleError::Newer(bundle.version));
        }
        let valid = |id: &str| id.len() == SONG_ID_LEN && id.bytes().all(|b| b.is_ascii_hexdigit());
        if !bundle.progress.iter().all(|p| valid(&p.song_id)) {
            return Err(BundleError::BadSongId);
        }
        Ok(bundle)
    }
}

/// Adds pieces and setlists of `from` that `into` does not know (by path and by name).
pub fn merge_library(into: &mut Library, from: Library) -> Merged {
    let mut merged = Merged::default();
    for entry in from.recent {
        if into.entry(&entry.path).is_none() {
            into.recent.push(entry);
            merged.pieces += 1;
        }
    }
    for list in from.setlists {
        if !into.setlists.iter().any(|s| s.name == list.name) {
            into.setlists.push(list);
            merged.setlists += 1;
        }
    }
    // Beat maps of backing tracks (LH-BIB-06) come along where the own library has none.
    for (path, map) in from.beat_maps {
        into.beat_maps.entry(path).or_insert(map);
    }
    merged
}

/// Joins the runs of one song; a run already there (same end time and length) is not doubled.
/// Returns how many runs were added.
pub fn merge_runs(into: &mut SongProgress, from: SongProgress) -> usize {
    let mut added = 0;
    for run in from.runs {
        let known = into
            .runs
            .iter()
            .any(|r| r.finished_unix == run.finished_unix && r.practice_s == run.practice_s);
        if !known {
            into.runs.push(run);
            added += 1;
        }
    }
    into.runs.sort_by_key(|r| r.finished_unix);
    added
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{RecentEntry, Setlist};
    use kontra_session::Run;

    fn entry(path: &str) -> RecentEntry {
        RecentEntry {
            path: path.into(),
            title: path.into(),
            artist: String::new(),
            track: 0,
            last_unix: 0,
            song_id: String::new(),
            backing: None,
            backing_offset_s: 0.0,
            view: None,
            tags: Vec::new(),
            sections: Default::default(),
            densities: Default::default(),
            transpose_semitones: 0,
            backing_without_bass: false,
        }
    }

    fn run(unix: u64) -> Run {
        Run {
            spread_ms: None,
            finished_unix: unix,
            practice_s: 60.0,
            speed: 1.0,
            hit_rate: 0.8,
            mode: "song".into(),
            section: None,
            hardest_bars: Vec::new(),
        }
    }

    #[test]
    fn rejects_foreign_newer_and_unsafe_files() {
        assert_eq!(Bundle::parse("{ nope"), Err(BundleError::Unreadable));
        let mut bundle = Bundle {
            format: FORMAT.into(),
            version: VERSION,
            exported_unix: 0,
            settings: Settings::default(),
            library: Library::default(),
            progress: Vec::new(),
        };
        let text = |b: &Bundle| serde_json::to_string(b).unwrap();
        assert!(Bundle::parse(&text(&bundle)).is_ok());
        bundle.version = VERSION + 1;
        assert_eq!(
            Bundle::parse(&text(&bundle)),
            Err(BundleError::Newer(VERSION + 1))
        );
        bundle.version = VERSION;
        bundle.progress.push(SongProgress {
            song_id: "../../evil".into(),
            title: String::new(),
            artist: String::new(),
            runs: Vec::new(),
        });
        assert_eq!(Bundle::parse(&text(&bundle)), Err(BundleError::BadSongId));
        bundle.format = "other".into();
        assert_eq!(Bundle::parse(&text(&bundle)), Err(BundleError::Unreadable));
    }

    #[test]
    fn merge_adds_only_unknown_pieces_lists_and_runs() {
        let mut own = Library {
            beat_maps: Default::default(),
            recent: vec![entry("a")],
            setlists: vec![Setlist {
                name: "Freitag".into(),
                pieces: vec!["a".into()],
            }],
        };
        let other = Library {
            beat_maps: Default::default(),
            recent: vec![entry("a"), entry("b")],
            setlists: vec![
                Setlist {
                    name: "Freitag".into(),
                    pieces: Vec::new(),
                },
                Setlist {
                    name: "Probe".into(),
                    pieces: vec!["b".into()],
                },
            ],
        };
        let merged = merge_library(&mut own, other);
        assert_eq!((merged.pieces, merged.setlists), (1, 1));
        assert_eq!(own.setlists[0].pieces.len(), 1, "own setlist kept");

        let mut song = SongProgress {
            song_id: "0123456789abcdef".into(),
            title: "Riff".into(),
            artist: String::new(),
            runs: vec![run(5)],
        };
        let incoming = SongProgress {
            runs: vec![run(1), run(5)],
            ..song.clone()
        };
        assert_eq!(merge_runs(&mut song, incoming), 1);
        let times: Vec<u64> = song.runs.iter().map(|r| r.finished_unix).collect();
        assert_eq!(times, [1, 5]);
    }
}
