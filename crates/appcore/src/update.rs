//! Update notice in the app core (LH-ALL-07): one look at `versions.json` per start, a hint
//! with a link when a newer release exists, dismissed until the next one.
//!
//! ```text
//! start_update_check() ─► UpdateCheck (thread) ─► update_poll() each tick ─► update_json()
//! off: setting, Flatpak (Flathub updates), QA runs with a recording, tests (nobody calls it)
//! ```

use crate::Core;
use kontra_session::{FileHttp, Http, NewRelease, UpdateCheck, Web};
use serde_json::json;
use std::path::Path;
use std::sync::Arc;

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
    /// Starts the one request of this run, unless the setting, a Flatpak, a QA run with a
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
        let http = self
            .updater
            .http
            .get_or_insert_with(|| Arc::new(Web::new()))
            .clone();
        let mut check = UpdateCheck::new(http);
        check.start(env!("CARGO_PKG_VERSION"));
        self.updater.check = Some(check);
    }

    /// Takes the finished check in; `None` while it runs or when none was started.
    pub fn update_poll(&mut self) -> Option<String> {
        let outcome = self.updater.check.as_mut()?.poll()?;
        self.updater.check = None;
        self.updater.found = outcome;
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

    fn update_by_store(&self) -> bool {
        std::env::var_os(FLATPAK_ENV).is_some()
    }
}
