//! The update notice (LH-ALL-07): asked only when started, silent when off, dismissed per version.

use super::*;
use kontra_session::{Http, HttpError};
use std::sync::Arc;

#[derive(Debug)]
struct Canned(Vec<u8>);

impl Http for Canned {
    fn get(&self, _url: &str) -> Result<Vec<u8>, HttpError> {
        Ok(self.0.clone())
    }
}

fn answer(version: &str) -> Arc<dyn Http> {
    let body = format!(
        r#"{{"format":1,"projects":{{"kontra":{{"version":"{version}","url":"https://example.org/r"}}}}}}"#
    );
    Arc::new(Canned(body.into_bytes()))
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

    // Dismissed until a newer one: the next run with the same answer stays quiet.
    let mut again = Core::new(&dir, QaOptions::default());
    again.set_update_http(answer("99.0.0"));
    again.start_update_check(None, None);
    assert_eq!(wait(&mut again)["version"], "");
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
