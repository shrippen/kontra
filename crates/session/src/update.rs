//! Update notice (LH-ALL-07, owner's decision round 18): asks `versions.json` of the project
//! site whether a newer release exists. Never downloads or installs anything; the app shows a
//! link. Every failure is silent: offline, a bad answer and "no newer version" look the same.
//!
//! ```text
//! start(current) ─► worker thread ─► Http::get(versions.json)
//!                                        │
//! poll() ◄───────── channel ◄── parse: Some(NewRelease) if newer than `current`, else None
//! ```
//! Format and comparison rules: shrippen.github.io/overview/VERSIONS.md.

use kontra_engine::Http;
use serde::Deserialize;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;

/// Where the latest release of every project is listed.
pub const VERSIONS_URL: &str = "https://shrippen.github.io/versions.json";
/// Kontra's key in `versions.json` (`overview/sites.json` of the site).
const PROJECT: &str = "kontra";
/// The only `format` of `versions.json` this client knows.
const FORMAT: u32 = 1;
/// A release page is a link to open in the browser: https only.
const HTTPS: &str = "https://";

/// A release newer than the running one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewRelease {
    pub version: String,
    /// The release page on GitHub.
    pub url: String,
}

#[derive(Debug, Deserialize)]
struct Listing {
    format: u32,
    projects: HashMap<String, Entry>,
}

#[derive(Debug, Deserialize)]
struct Entry {
    version: String,
    url: String,
}

/// At most one request in flight; the UI polls for its end.
#[derive(Debug)]
pub struct UpdateCheck {
    http: Arc<dyn Http>,
    running: Option<Receiver<Option<NewRelease>>>,
}

impl UpdateCheck {
    pub fn new(http: Arc<dyn Http>) -> UpdateCheck {
        UpdateCheck {
            http,
            running: None,
        }
    }

    /// Sends one request on a worker thread; does nothing while one runs.
    pub fn start(&mut self, current: &str) {
        if self.running.is_some() {
            return;
        }
        let http = Arc::clone(&self.http);
        let current = current.to_string();
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let answer = http.get(VERSIONS_URL).ok();
            let _ = tx.send(answer.and_then(|body| newer(&body, &current)));
        });
        self.running = Some(rx);
    }

    /// The finished check, once: `Some(None)` is "nothing newer"; `None` while it runs.
    pub fn poll(&mut self) -> Option<Option<NewRelease>> {
        let rx = self.running.as_ref()?;
        let outcome = match rx.try_recv() {
            Ok(outcome) => outcome,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => None,
        };
        self.running = None;
        Some(outcome)
    }
}

/// The release in a `versions.json` body if it is newer than `current`; anything unexpected
/// (not JSON, unknown format, no entry, a link that is not https) is `None`.
pub fn newer(body: &[u8], current: &str) -> Option<NewRelease> {
    let listing: Listing = serde_json::from_slice(body).ok()?;
    if listing.format != FORMAT {
        return None;
    }
    let entry = listing.projects.get(PROJECT)?;
    if !entry.url.starts_with(HTTPS) {
        return None;
    }
    if compare(&entry.version, current) != Ordering::Greater {
        return None;
    }
    Some(NewRelease {
        version: entry.version.trim_start_matches('v').to_string(),
        url: entry.url.clone(),
    })
}

/// "0.10.0" > "0.9.3": parts split at `.` compare as numbers, missing parts count 0, anything
/// after `-` or `+` is ignored.
fn compare(a: &str, b: &str) -> Ordering {
    let (a, b) = (parts(a), parts(b));
    for i in 0..a.len().max(b.len()) {
        let order = a.get(i).unwrap_or(&0).cmp(b.get(i).unwrap_or(&0));
        if order != Ordering::Equal {
            return order;
        }
    }
    Ordering::Equal
}

fn parts(version: &str) -> Vec<u64> {
    let core = version.trim_start_matches('v');
    let core = core.split(['-', '+']).next().unwrap_or("");
    core.split('.').map(|p| p.parse().unwrap_or(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use kontra_engine::HttpError;

    #[derive(Debug)]
    struct Canned(Result<Vec<u8>, HttpError>);

    impl Http for Canned {
        fn get(&self, _url: &str) -> Result<Vec<u8>, HttpError> {
            self.0.clone()
        }
    }

    fn listing(version: &str) -> Vec<u8> {
        format!(
            r#"{{"format":1,"projects":{{"kontra":{{"version":"{version}","date":"2026-11-10",
            "url":"https://github.com/shrippen/Kontra/releases/tag/v{version}"}}}}}}"#
        )
        .into_bytes()
    }

    fn check(answer: Result<Vec<u8>, HttpError>, current: &str) -> Option<NewRelease> {
        let mut check = UpdateCheck::new(Arc::new(Canned(answer)));
        check.start(current);
        loop {
            if let Some(outcome) = check.poll() {
                return outcome;
            }
            std::thread::yield_now();
        }
    }

    #[test]
    fn numbers_not_text() {
        assert_eq!(compare("0.10.0", "0.9.3"), Ordering::Greater);
        assert_eq!(compare("1.0", "1.0.0"), Ordering::Equal);
        assert_eq!(compare("v1.2.0-rc1", "1.2.0"), Ordering::Equal);
        assert_eq!(compare("0.9.9", "1.0.0"), Ordering::Less);
    }

    #[test]
    fn newer_release_is_offered() {
        let release = check(Ok(listing("0.2.0")), "0.1.0").expect("a newer release");
        assert_eq!(release.version, "0.2.0");
        assert!(release.url.ends_with("v0.2.0"));
    }

    #[test]
    fn same_or_older_is_silent() {
        assert_eq!(check(Ok(listing("0.1.0")), "0.1.0"), None);
        assert_eq!(check(Ok(listing("0.1.0")), "0.2.0"), None);
    }

    #[test]
    fn offline_is_silent() {
        let offline = HttpError::Offline("no route".into());
        assert_eq!(check(Err(offline), "0.1.0"), None);
        assert_eq!(check(Err(HttpError::Timeout), "0.1.0"), None);
        assert_eq!(check(Err(HttpError::Status(500)), "0.1.0"), None);
    }

    #[test]
    fn malformed_answers_are_silent() {
        let unknown_format = br#"{"format":2,"projects":{}}"#;
        let other_project =
            br#"{"format":1,"projects":{"andon":{"version":"9.0.0","url":"https://x"}}}"#;
        let plain_link =
            br#"{"format":1,"projects":{"kontra":{"version":"9.0.0","url":"http://x"}}}"#;
        let no_url = br#"{"format":1,"projects":{"kontra":{"version":"9.0.0"}}}"#;
        for body in [
            &b"not json"[..],
            b"",
            b"{}",
            unknown_format,
            other_project,
            plain_link,
            no_url,
        ] {
            assert_eq!(
                newer(body, "0.1.0"),
                None,
                "{}",
                String::from_utf8_lossy(body)
            );
        }
    }
}
