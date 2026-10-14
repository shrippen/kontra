//! Songsterr search from inside Kontra (LH-BIB-11, owner's decision round 12): title, artist,
//! whether a bass track exists and its tuning. A hit opens the song's page in the browser; tabs
//! are never fetched (`QUELLEN-SONGS.md`).
//!
//! ```text
//! start("seven nation") ─► worker thread ─► Http::get(/api/songs?pattern=…&size=20)
//!                                               │
//! poll() ◄──────── channel ◄── parse: [SongHit] or SearchError
//! ```
//! Polite by design: only on the user's request, one request at a time, no retries, no prefetch.

use crate::sources::percent_encode;
use kontra_engine::{Http, HttpError};
use serde::Deserialize;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;

const API: &str = "https://www.songsterr.com/api/songs?pattern=";
const PAGE: &str = "https://www.songsterr.com/a/wsa/";
/// Hits asked for: one screen of results.
const HITS: usize = 20;
/// General MIDI programs of basses (0-based): acoustic, finger, pick, fretless, slap, synth.
const BASS_PROGRAMS: std::ops::RangeInclusive<u32> = 32..=39;

/// One song found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SongHit {
    pub title: String,
    pub artist: String,
    /// The song's page, on its bass track if it has one.
    pub page_url: String,
    pub bass: Option<BassTrack>,
}

/// The song's bass track.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BassTrack {
    /// Open strings as MIDI numbers, lowest first; empty if Songsterr gives none.
    pub tuning: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchError {
    Offline,
    Timeout,
    /// The site answered with an error status (e.g. 429 too many requests).
    Status(u16),
    /// The answer was not the expected list.
    Unreadable,
}

/// What `start` did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchStart {
    Started,
    /// A request is still running; nothing new was sent.
    Busy,
    /// Nothing to search for.
    Empty,
}

type Outcome = Result<Vec<SongHit>, SearchError>;

/// At most one search in flight; the UI polls for its end.
#[derive(Debug)]
pub struct SongSearch {
    http: Arc<dyn Http>,
    running: Option<Receiver<Outcome>>,
}

impl SongSearch {
    pub fn new(http: Arc<dyn Http>) -> SongSearch {
        SongSearch {
            http,
            running: None,
        }
    }

    pub fn busy(&self) -> bool {
        self.running.is_some()
    }

    /// Sends one request on a worker thread.
    pub fn start(&mut self, query: &str) -> SearchStart {
        let query = query.trim();
        if query.is_empty() {
            return SearchStart::Empty;
        }
        if self.busy() {
            return SearchStart::Busy;
        }
        let url = format!("{API}{}&size={HITS}", percent_encode(query));
        let http = Arc::clone(&self.http);
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let outcome = http
                .get(&url)
                .map_err(SearchError::from)
                .and_then(|body| parse(&body));
            let _ = tx.send(outcome);
        });
        self.running = Some(rx);
        SearchStart::Started
    }

    /// The finished search, once; `None` while it runs or when none was started.
    pub fn poll(&mut self) -> Option<Outcome> {
        let rx = self.running.as_ref()?;
        let outcome = match rx.try_recv() {
            Ok(outcome) => outcome,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => Err(SearchError::Unreadable),
        };
        self.running = None;
        Some(outcome)
    }
}

impl From<HttpError> for SearchError {
    fn from(e: HttpError) -> SearchError {
        match e {
            HttpError::Offline(_) => SearchError::Offline,
            HttpError::Timeout => SearchError::Timeout,
            HttpError::Status(code) => SearchError::Status(code),
            HttpError::Failed(_) => SearchError::Unreadable,
        }
    }
}

/// The fields Kontra reads from Songsterr's answer.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiSong {
    song_id: u64,
    #[serde(default)]
    artist: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    tracks: Vec<ApiTrack>,
    popular_track_bass: Option<usize>,
    #[serde(default)]
    is_junk: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiTrack {
    #[serde(default)]
    instrument_id: u32,
    tuning: Option<Vec<u8>>,
}

fn parse(body: &[u8]) -> Outcome {
    let songs: Vec<ApiSong> = serde_json::from_slice(body).map_err(|_| SearchError::Unreadable)?;
    Ok(songs.into_iter().filter(|s| !s.is_junk).map(hit).collect())
}

/// The bass track Songsterr names, else the first one with a bass instrument.
fn hit(song: ApiSong) -> SongHit {
    let bass = song
        .popular_track_bass
        .filter(|i| *i < song.tracks.len())
        .or_else(|| {
            song.tracks
                .iter()
                .position(|t| BASS_PROGRAMS.contains(&t.instrument_id))
        });
    let slug = slug(&format!("{} {}", song.artist, song.title));
    let track = bass.map_or_else(String::new, |i| format!("t{i}"));
    let page_url = format!("{PAGE}{slug}-tab-s{}{track}", song.song_id);
    let bass = bass.and_then(|i| song.tracks.get(i)).map(|t| {
        let mut tuning = t.tuning.clone().unwrap_or_default();
        // Songsterr lists the highest string first.
        tuning.reverse();
        BassTrack { tuning }
    });
    SongHit {
        title: song.title,
        artist: song.artist,
        page_url,
        bass,
    }
}

/// "The White Stripes Seven Nation Army" → "the-white-stripes-seven-nation-army"; the site
/// finds the song by its id, the words are for people.
fn slug(text: &str) -> String {
    let lower = text.to_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    words.join("-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Answers like Songsterr did on 14.10.2026 (shortened).
    const ANSWER: &str = r#"[
      {"songId":265,"artist":"The White Stripes","title":"Seven Nation Army","tracks":[
        {"instrumentId":29,"instrument":"Overdriven Guitar","tuning":[64,59,55,50,45,40]},
        {"instrumentId":1024,"instrument":"Drums"}],"popularTrack":0,"isJunk":false},
      {"songId":397405,"artist":"The White Stripes","title":"Seven Nation Army (With Bass Guitar)",
       "tracks":[{"instrumentId":29,"tuning":[64,59,55,50,45,40]},
        {"instrumentId":33,"instrument":"Electric Bass (finger)","tuning":[43,38,33,28]}],
       "popularTrackBass":1},
      {"songId":1,"artist":"x","title":"junk","tracks":[],"isJunk":true}
    ]"#;

    #[derive(Debug)]
    struct Fake {
        answer: Result<Vec<u8>, HttpError>,
        urls: Mutex<Vec<String>>,
    }

    impl Http for Fake {
        fn get(&self, url: &str) -> Result<Vec<u8>, HttpError> {
            self.urls.lock().unwrap().push(url.to_string());
            self.answer.clone()
        }
    }

    fn wait(search: &mut SongSearch) -> Outcome {
        loop {
            if let Some(outcome) = search.poll() {
                return outcome;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    #[test]
    fn finds_songs_with_and_without_bass() {
        let fake = Arc::new(Fake {
            answer: Ok(ANSWER.as_bytes().to_vec()),
            urls: Mutex::new(Vec::new()),
        });
        let mut search = SongSearch::new(fake.clone());
        assert_eq!(search.start("  "), SearchStart::Empty);
        assert_eq!(search.start("Seven Nation"), SearchStart::Started);
        assert_eq!(search.start("again"), SearchStart::Busy);
        let hits = wait(&mut search).unwrap();
        assert!(!search.busy());
        assert_eq!(
            *fake.urls.lock().unwrap(),
            ["https://www.songsterr.com/api/songs?pattern=Seven%20Nation&size=20"]
        );

        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].bass, None);
        assert_eq!(
            hits[0].page_url,
            "https://www.songsterr.com/a/wsa/the-white-stripes-seven-nation-army-tab-s265"
        );
        assert_eq!(
            hits[1].bass,
            Some(BassTrack {
                tuning: vec![28, 33, 38, 43]
            })
        );
        assert!(hits[1]
            .page_url
            .ends_with("-with-bass-guitar-tab-s397405t1"));
    }

    #[test]
    fn errors_are_named() {
        for (answer, expected) in [
            (Err(HttpError::Offline("dns".into())), SearchError::Offline),
            (Err(HttpError::Timeout), SearchError::Timeout),
            (Err(HttpError::Status(429)), SearchError::Status(429)),
            (Ok(b"<html>".to_vec()), SearchError::Unreadable),
        ] {
            let fake = Arc::new(Fake {
                answer,
                urls: Mutex::new(Vec::new()),
            });
            let mut search = SongSearch::new(fake);
            search.start("x");
            assert_eq!(wait(&mut search), Err(expected));
        }
    }

    #[test]
    fn bass_found_by_instrument() {
        let body = br#"[{"songId":7,"artist":"A","title":"B","tracks":[{"instrumentId":30},{"instrumentId":34}]}]"#;
        let hits = parse(body).unwrap();
        assert_eq!(hits[0].bass, Some(BassTrack { tuning: vec![] }));
        assert!(hits[0].page_url.ends_with("a-b-tab-s7t1"));
    }
}
