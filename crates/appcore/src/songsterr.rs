//! Songsterr search in the library (LH-BIB-11, round 12): the user asks, one request runs on a
//! worker thread, `tick` collects it; a hit opens the song's page in the browser.
//!
//! ```text
//! song_search("seven") ─► SongSearch (thread) ─► song_search_poll() each tick ─► JSON for QML
//! state: idle → running → done (hits) | error (reason in the user's language)
//! ```

use crate::Core;
use kontra_model::Pitch;
use kontra_session::{FileHttp, Http, SearchError, SearchStart, SongHit, SongSearch, Web};
use serde_json::json;
use std::path::Path;
use std::sync::Arc;

/// HTTP status for "too many requests".
const TOO_MANY_REQUESTS: u16 = 429;

/// Where the search stands.
#[derive(Debug, Clone, PartialEq)]
enum State {
    Idle,
    Running(String),
    Done(String, Vec<SongHit>),
    Failed(String, SearchError),
}

/// The search and its last state; the web client is made on first use.
#[derive(Debug)]
pub(crate) struct Finder {
    search: Option<SongSearch>,
    state: State,
}

impl Default for Finder {
    fn default() -> Finder {
        Finder {
            search: None,
            state: State::Idle,
        }
    }
}

impl Core {
    /// Searches Songsterr for `query` (one request at a time); returns the state as JSON.
    pub fn song_search(&mut self, query: &str) -> String {
        let search = self
            .finder
            .search
            .get_or_insert_with(|| SongSearch::new(Arc::new(Web::new())));
        if search.start(query) == SearchStart::Started {
            self.finder.state = State::Running(query.trim().to_string());
        }
        self.song_search_json()
    }

    /// The finished search as JSON, once; `None` while nothing changed.
    pub fn song_search_poll(&mut self) -> Option<String> {
        let outcome = self.finder.search.as_mut()?.poll()?;
        let State::Running(query) = &self.finder.state else {
            return None;
        };
        let query = query.clone();
        self.finder.state = match outcome {
            Ok(hits) => State::Done(query, hits),
            Err(e) => State::Failed(query, e),
        };
        Some(self.song_search_json())
    }

    /// `{"state", "query", "message", "hits": [{"title", "artist", "url", "bass", "detail"}]}`.
    pub fn song_search_json(&self) -> String {
        let (state, query, message, hits): (&str, &str, String, &[SongHit]) =
            match &self.finder.state {
                State::Idle => ("idle", "", String::new(), &[]),
                State::Running(q) => ("running", q, String::new(), &[]),
                State::Done(q, hits) => ("done", q, String::new(), hits),
                State::Failed(q, e) => ("error", q, self.search_error(e), &[]),
            };
        let hits: Vec<_> = hits
            .iter()
            .map(|h| {
                json!({
                    "title": h.title,
                    "artist": h.artist,
                    "url": h.page_url,
                    "bass": h.bass.is_some(),
                    "detail": self.bass_detail(h),
                })
            })
            .collect();
        json!({ "state": state, "query": query, "message": message, "hits": hits }).to_string()
    }

    /// Answers every search with the file's content (QA runs, screenshots): never the network.
    pub fn fake_song_search(&mut self, answer: &Path) {
        self.set_song_http(Arc::new(FileHttp::new(answer)));
    }

    /// The web client the search uses; tests pass a fake.
    pub fn set_song_http(&mut self, http: Arc<dyn Http>) {
        self.finder.search = Some(SongSearch::new(http));
        self.finder.state = State::Idle;
    }

    /// "Bass: E A D G" or "no bass track": in words, not by colour (P7).
    fn bass_detail(&self, hit: &SongHit) -> String {
        let Some(bass) = &hit.bass else {
            return self.tr("search.web.nobass", &[]);
        };
        if bass.tuning.is_empty() {
            return self.tr("search.web.bass", &[]);
        }
        let strings: Vec<String> = bass
            .tuning
            .iter()
            .map(|m| self.note_label(&Pitch(*m).name().to_string()))
            .collect();
        self.tr("search.web.tuning", &[("strings", strings.join(" "))])
    }

    fn search_error(&self, e: &SearchError) -> String {
        match e {
            SearchError::Offline => self.tr("search.web.offline", &[]),
            SearchError::Timeout => self.tr("search.web.timeout", &[]),
            SearchError::Status(TOO_MANY_REQUESTS) => self.tr("search.web.busy", &[]),
            SearchError::Status(code) => {
                self.tr("search.web.status", &[("code", code.to_string())])
            }
            SearchError::Unreadable => self.tr("search.web.unreadable", &[]),
        }
    }
}
