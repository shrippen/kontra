//! Update notice in the app core (LH-ALL-07): one look at `versions.json` a day, a hint
//! with a link when a newer release exists, dismissed until the next one.
//!
//! ```text
//! start_update_check() ─► UpdateCheck (thread) ─► update_poll() each tick ─► update_json()
//! off: setting, Flatpak (Flathub updates), QA runs with a recording, tests (nobody calls it)
//! ```

use crate::Core;
use kontra_session::{is_newer, FileHttp, Http, NewRelease, UpdateCheck, Web};
use serde_json::json;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

const DAY_S: u64 = 24 * 60 * 60;
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Set inside a Flatpak, whose store announces updates.
const FLATPAK_ENV: &str = "FLATPAK_ID";

/// The check and what it found; the web client is made on first use.
#[derive(Default)]
pub(crate) struct Updater {
    http: Option<Arc<dyn Http>>,
    check: Option<UpdateCheck>,
    found: Option<NewRelease>,
}

impl std::fmt::Debug for Updater {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Updater")
            .field("found", &self.found)
            .finish_non_exhaustive()
    }
}

impl Core {
    /// Starts the day's request, unless a day has not passed since the last answer or the setting, a Flatpak, a QA run with a
    /// recording or a screenshot run says no. `answer`: a file read as `versions.json` instead
    /// of the web (QA, screenshots); it also lets those runs ask.
    pub fn start_update_check(&mut self, answer: Option<&Path>, screenshot: Option<&Path>) {
        if let Some(answer) = answer {
            self.fake_update_answer(answer);
        }
        let fake = self.updater.http.is_some();
        let recorded = self.qa.input_file.is_some() || screenshot.is_some();
        if !self.settings.update_check || self.update_by_store() || (recorded && !fake) {
            return;
        }
        let now = unix_now();
        // A date in the future (clock set back) counts as due.
        let since = now.checked_sub(self.settings.update_checked_unix);
        if answer.is_none() && since.is_some_and(|s| s < DAY_S) {
            self.recall_update();
            return;
        }
        let http = self
            .updater
            .http
            .get_or_insert_with(|| Arc::new(Web::new()))
            .clone();
        let mut check = UpdateCheck::new(http);
        check.start(VERSION);
        self.updater.check = Some(check);
    }

    /// Takes the finished check in; `None` while it runs or when none was started.
    pub fn update_poll(&mut self) -> Option<String> {
        let outcome = self.updater.check.as_mut()?.poll()?;
        self.updater.check = None;
        // No answer (offline): nothing is remembered, the next start asks again.
        if let Ok(found) = outcome {
            self.remember_update(&found);
            self.updater.found = found;
        }
        Some(self.update_json())
    }

    /// `{"offered", "enabled", "pending", "version", "url"}`: `offered` is false where a store
    /// updates Kontra (the setting is not shown); `pending` while the request runs; `version`
    /// is empty without a notice to show.
    pub fn update_json(&self) -> String {
        let shown = self
            .updater
            .found
            .as_ref()
            .filter(|r| self.settings.update_dismissed.as_deref() != Some(&r.version));
        json!({
            "offered": !self.update_by_store(),
            "enabled": self.settings.update_check,
            "pending": self.updater.check.is_some(),
            "version": shown.map_or("", |r| &r.version),
            "url": shown.map_or("", |r| &r.url),
        })
        .to_string()
    }

    /// Hides the notice until a newer release comes.
    pub fn dismiss_update(&mut self) {
        let Some(found) = &self.updater.found else {
            return;
        };
        let version = found.version.clone();
        self.update_settings(|s| s.update_dismissed = Some(version));
    }

    /// Answers the check with the file's content (QA runs, screenshots): never the network.
    fn fake_update_answer(&mut self, answer: &Path) {
        self.set_update_http(Arc::new(FileHttp::new(answer)));
    }

    /// The web client the check uses; tests pass a fake.
    pub(crate) fn set_update_http(&mut self, http: Arc<dyn Http>) {
        self.updater.http = Some(http);
    }

    /// The answer of today's first request, for the starts that follow it.
    fn remember_update(&mut self, found: &Option<NewRelease>) {
        let (version, url) = found
            .as_ref()
            .map_or_else(Default::default, |r| (r.version.clone(), r.url.clone()));
        self.update_settings(|s| {
            s.update_checked_unix = unix_now();
            s.update_found_version = version;
            s.update_found_url = url;
        });
    }

    /// Shows what the last request found, as long as this version is still older.
    fn recall_update(&mut self) {
        let s = &self.settings;
        if s.update_found_version.is_empty() || !is_newer(&s.update_found_version, VERSION) {
            return;
        }
        self.updater.found = Some(NewRelease {
            version: s.update_found_version.clone(),
            url: s.update_found_url.clone(),
        });
    }

    fn update_by_store(&self) -> bool {
        std::env::var_os(FLATPAK_ENV).is_some()
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}
