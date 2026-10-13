//! Where songs come from besides the user's own folders (LH-BIB-11, `QUELLEN-SONGS.md`):
//! a search page on Songsterr or MuseScore.com, opened in the user's browser, and the score
//! files the browser saved to the download folder since.
//!
//! ```text
//! "seven nation" ─→ search_url(Songsterr) ─→ browser ─→ user downloads .gp
//!                                                            │
//! Library ◄── open ◄── recent_scores(download_dir()) ◄───────┘
//! ```
//!
//! Kontra itself fetches nothing: both sites forbid automated access (terms, robots.txt).

use kontra_content::FileKind;
use kontra_engine::RecentFile;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Package extension (LH-BIB-07), opened like a score.
const PACKAGE_EXTENSION: &str = "kontra";
/// Any XML file could be MusicXML; in a download folder most are not.
const PLAIN_XML: &str = "xml";
const SONGSTERR_SEARCH: &str = "https://www.songsterr.com/?pattern=";
const MUSESCORE_SEARCH: &str = "https://musescore.com/sheetmusic?text=";

/// A site with a search page for songs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SongSite {
    Songsterr,
    MuseScore,
}

impl SongSite {
    /// Site from its code in the UI ("songsterr", "musescore").
    pub fn from_code(code: &str) -> Option<SongSite> {
        match code {
            "songsterr" => Some(SongSite::Songsterr),
            "musescore" => Some(SongSite::MuseScore),
            _ => None,
        }
    }
}

/// The site's own search page for `query`; `None` for an empty query.
///
/// `"AC/DC Back in Black"` → `https://www.songsterr.com/?pattern=AC%2FDC%20Back%20in%20Black`
pub fn search_url(site: SongSite, query: &str) -> Option<String> {
    let query = query.trim();
    if query.is_empty() {
        return None;
    }
    let base = match site {
        SongSite::Songsterr => SONGSTERR_SEARCH,
        SongSite::MuseScore => MUSESCORE_SEARCH,
    };
    Some(format!("{base}{}", percent_encode(query)))
}

/// URL query encoding: unreserved characters stay, every other byte becomes `%XX`.
fn percent_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 3);
    for byte in text.bytes() {
        let unreserved = byte.is_ascii_alphanumeric() || b"-_.~".contains(&byte);
        if unreserved {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// The browser's download folder.
pub fn download_dir() -> Option<PathBuf> {
    kontra_engine::download_dir()
}

/// Files Kontra can open that arrived in `dir` within `max_age`, newest first.
pub fn recent_scores(dir: &Path, max_age: Duration) -> Vec<RecentFile> {
    let mut files = kontra_engine::recent_files(dir, max_age);
    files.retain(|f| {
        f.path
            .extension()
            .and_then(|e| e.to_str())
            .filter(|e| !e.eq_ignore_ascii_case(PLAIN_XML))
            .is_some_and(|e| {
                FileKind::from_extension(e).is_some() || e.eq_ignore_ascii_case(PACKAGE_EXTENSION)
            })
    });
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_urls() {
        assert_eq!(
            search_url(SongSite::Songsterr, " AC/DC Back in Black ").as_deref(),
            Some("https://www.songsterr.com/?pattern=AC%2FDC%20Back%20in%20Black")
        );
        assert_eq!(
            search_url(SongSite::MuseScore, "Mañana & más").as_deref(),
            Some("https://musescore.com/sheetmusic?text=Ma%C3%B1ana%20%26%20m%C3%A1s")
        );
        assert_eq!(search_url(SongSite::Songsterr, "  "), None);
        assert_eq!(SongSite::from_code("musescore"), Some(SongSite::MuseScore));
        assert_eq!(SongSite::from_code("youtube"), None);
    }

    #[test]
    fn only_scores_count() {
        let dir = std::env::temp_dir().join(format!("kontra-sources-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for name in [
            "song.gp5",
            "chart.mscz",
            "photo.jpg",
            "lesson.kontra",
            "notes.txt",
            "feed.xml",
        ] {
            std::fs::write(dir.join(name), b"x").unwrap();
        }
        let mut names: Vec<String> = recent_scores(&dir, Duration::from_secs(60))
            .iter()
            .filter_map(|f| f.path.file_name().map(|n| n.to_string_lossy().into_owned()))
            .collect();
        names.sort();
        assert_eq!(names, ["chart.mscz", "lesson.kontra", "song.gp5"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
