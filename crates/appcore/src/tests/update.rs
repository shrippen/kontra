//! The update notice (LH-ALL-07): asked only when started, silent when off, dismissed per version.

use super::*;
use kontra_session::{Http, HttpError};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Answers with a fixed body and counts the requests.
#[derive(Debug)]
struct Canned(Vec<u8>, Arc<AtomicUsize>);

impl Http for Canned {
    fn get(&self, _url: &str) -> Result<Vec<u8>, HttpError> {
        self.1.fetch_add(1, Ordering::SeqCst);
        Ok(self.0.clone())
    }
}

fn answer(version: &str) -> Arc<dyn Http> {
    counted(version).0
}

fn counted(version: &str) -> (Arc<dyn Http>, Arc<AtomicUsize>) {
    let count = Arc::new(AtomicUsize::new(0));
    let body = format!(
        r#"{{"format":1,"projects":{{"kontra":{{"version":"{version}","url":"https://example.org/r"}}}}}}"#
    );
    (Arc::new(Canned(body.into_bytes(), count.clone())), count)
}

fn wait(core: &mut Core) -> serde_json::Value {
    loop {
        if let Some(json) = core.update_poll() {
            return serde_json::from_str(&json).unwrap();
        }
        std::thread::yield_now();
    }
}

#[test]
fn nothing_is_asked_unless_started() {
    let mut core = Core::new(&temp_dir("update-idle"), QaOptions::default());
    core.set_update_http(answer("99.0.0"));
    assert_eq!(core.update_poll(), None);
    assert_eq!(
        core.update_json(),
        r#"{"enabled":true,"offered":true,"pending":false,"url":"","version":""}"#
    );
}

#[test]
fn newer_release_shows_until_dismissed() {
    let dir = temp_dir("update-dismiss");
    let mut core = Core::new(&dir, QaOptions::default());
    core.set_update_http(answer("99.0.0"));
    core.start_update_check(None, None);
    assert_eq!(wait(&mut core)["version"], "99.0.0");

    core.dismiss_update();
    assert_eq!(
        core.update_json(),
        r#"{"enabled":true,"offered":true,"pending":false,"url":"","version":""}"#
    );

    // Dismissed until a newer one: the next run stays quiet, also a day later.
    let mut again = Core::new(&dir, QaOptions::default());
    again.start_update_check(None, None);
    assert!(again.update_json().contains(r#""version":"""#));
    again.update_settings(|s| s.update_checked_unix = 1);
    again.set_update_http(answer("99.0.0"));
    again.start_update_check(None, None);
    assert_eq!(wait(&mut again)["version"], "");
    again.update_settings(|s| s.update_checked_unix = 1);
    again.set_update_http(answer("99.1.0"));
    again.start_update_check(None, None);
    assert_eq!(wait(&mut again)["version"], "99.1.0");
}

#[test]
fn setting_off_asks_nothing() {
    let mut core = Core::new(&temp_dir("update-off"), QaOptions::default());
    core.update_settings(|s| s.update_check = false);
    core.set_update_http(answer("99.0.0"));
    core.start_update_check(None, None);
    assert_eq!(core.update_poll(), None);
}

#[test]
fn recorded_run_asks_nothing() {
    let qa = QaOptions {
        input_file: Some("bass.wav".into()),
        laptop_jack: false,
    };
    let mut core = Core::new(&temp_dir("update-qa"), qa);
    core.start_update_check(None, None);
    assert_eq!(core.update_poll(), None);
}

#[test]
fn screenshot_run_asks_only_with_an_answer() {
    let shot = Some(Path::new("shot.png"));
    let mut core = Core::new(&temp_dir("update-shot"), QaOptions::default());
    core.start_update_check(None, shot);
    assert_eq!(core.update_poll(), None);

    let answer = temp_dir("update-shot-answer").join("versions.json");
    let body =
        r#"{"format":1,"projects":{"kontra":{"version":"99.0.0","url":"https://example.org/r"}}}"#;
    std::fs::write(&answer, body).unwrap();
    core.start_update_check(Some(&answer), shot);
    assert_eq!(wait(&mut core)["version"], "99.0.0");
}

#[test]
fn asks_once_a_day_and_remembers_the_answer() {
    let dir = temp_dir("update-daily");
    let (http, count) = counted("99.0.0");
    let mut first = Core::new(&dir, QaOptions::default());
    first.set_update_http(http.clone());
    first.start_update_check(None, None);
    assert_eq!(wait(&mut first)["version"], "99.0.0");
    assert_eq!(count.load(Ordering::SeqCst), 1);

    // The next start within the day asks nothing, and still shows the notice.
    let mut second = Core::new(&dir, QaOptions::default());
    second.set_update_http(http.clone());
    second.start_update_check(None, None);
    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert_eq!(second.update_poll(), None);
    assert!(second.update_json().contains(r#""version":"99.0.0""#));

    // A day later it asks again.
    second.update_settings(|s| s.update_checked_unix = 1);
    second.start_update_check(None, None);
    wait(&mut second);
    assert_eq!(count.load(Ordering::SeqCst), 2);
}

#[test]
fn a_failed_request_is_tried_again_at_the_next_start() {
    let dir = temp_dir("update-offline");
    let mut core = Core::new(&dir, QaOptions::default());
    core.set_update_http(Arc::new(Offline));
    core.start_update_check(None, None);
    assert_eq!(wait(&mut core)["version"], "");

    let (http, count) = counted("99.0.0");
    let mut again = Core::new(&dir, QaOptions::default());
    again.set_update_http(http);
    again.start_update_check(None, None);
    assert_eq!(wait(&mut again)["version"], "99.0.0");
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[derive(Debug)]
struct Offline;

impl Http for Offline {
    fn get(&self, _url: &str) -> Result<Vec<u8>, HttpError> {
        Err(HttpError::Offline("no route".into()))
    }
}
