//! The LLM piece wish and provider settings (LH-BIB-16): key resolution (env beats the store),
//! `KeyState`, the background jobs through `tick`, and that a key never reaches settings, a
//! backup or an error string.

use super::*;
use kontra_session::{HttpError, MemoryStore, SecretError, SecretStore};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// `std::env::set_var` is process-global; tests that touch a provider's key variable share
/// this lock so they never see each other's value.
static ENV_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone)]
struct FakeHttp {
    chat: Result<Vec<u8>, HttpError>,
    models: Result<Vec<u8>, HttpError>,
}

impl Default for FakeHttp {
    fn default() -> FakeHttp {
        FakeHttp {
            chat: Err(HttpError::NotSupported),
            models: Err(HttpError::NotSupported),
        }
    }
}

impl kontra_session::Http for FakeHttp {
    fn get(&self, _url: &str) -> Result<Vec<u8>, HttpError> {
        Err(HttpError::NotSupported)
    }

    fn get_with(&self, _url: &str, _headers: &[(&str, &str)]) -> Result<Vec<u8>, HttpError> {
        self.models.clone()
    }

    fn post_json(
        &self,
        _url: &str,
        _headers: &[(&str, &str)],
        _body: &[u8],
        _timeout: Duration,
    ) -> Result<Vec<u8>, HttpError> {
        self.chat.clone()
    }
}

fn chat_json(text: &str) -> Vec<u8> {
    serde_json::json!({"choices": [{"message": {"content": text}}]})
        .to_string()
        .into_bytes()
}

const VALID_PLAN: &str = r#"{"title":"Testwunsch","key":"A","bpm":100,"feel":"straight","parts":[
    {"section":"verse","bars":["A"],"times":1,"groove":"quarters","line":"roots","fill":false,"drums":"full"}
]}"#;

fn core_with(name: &str, http: FakeHttp, secrets: Arc<dyn SecretStore>) -> Core {
    let mut core = Core::new(&temp_dir(name), QaOptions::default());
    core.update_settings(|s| s.llm_provider = "openai".into());
    core.llm_test_env(Arc::new(http), secrets);
    core
}

/// A `MemoryStore` with `openai`'s key already set.
fn keyed_store() -> Arc<dyn SecretStore> {
    let store = MemoryStore::new();
    store.set("openai", "sk-test").unwrap();
    Arc::new(store)
}

/// `llm_poll` until it has something, so tests never race the background thread.
fn wait(core: &mut Core) -> Vec<UiEvent> {
    loop {
        let events = core.llm_poll();
        if !events.is_empty() {
            return events;
        }
        std::thread::yield_now();
    }
}

/// Like `wait`, but fails instead of hanging forever when nothing ever arrives.
fn wait_timeout(core: &mut Core, timeout: Duration) -> Vec<UiEvent> {
    let start = Instant::now();
    loop {
        let events = core.llm_poll();
        if !events.is_empty() {
            return events;
        }
        assert!(
            start.elapsed() < timeout,
            "timed out waiting for a background job"
        );
        std::thread::yield_now();
    }
}

/// A `SecretStore` whose `get` blocks until released (review 13.11.2026: `Core::llm_key_state`
/// and the key resolution inside `llm_wish`/`llm_fetch_models` used to run such a call on the
/// caller's own thread, freezing the window for up to the keyring's own timeout).
#[derive(Debug)]
struct BlockingSecrets {
    gate: Mutex<Receiver<()>>,
    value: String,
}

impl SecretStore for BlockingSecrets {
    fn get(&self, _account: &str) -> Result<String, SecretError> {
        self.gate.lock().unwrap().recv().ok();
        Ok(self.value.clone())
    }
    fn set(&self, _account: &str, _value: &str) -> Result<(), SecretError> {
        Ok(())
    }
    fn delete(&self, _account: &str) -> Result<(), SecretError> {
        Ok(())
    }
}

#[test]
fn a_finished_wish_is_rendered_and_opened() {
    let http = FakeHttp {
        chat: Ok(chat_json(VALID_PLAN)),
        ..FakeHttp::default()
    };
    let mut core = core_with("llm-done", http, keyed_store());
    assert!(core.llm_wish("a groove in A", "medium"));
    let events = wait(&mut core);
    let UiEvent::LlmPieceReady(path) = &events[0] else {
        panic!("{events:?}");
    };
    assert!(std::path::Path::new(path).is_file(), "{path}");
}

#[test]
fn a_failed_request_is_reported_through_tick() {
    let http = FakeHttp {
        chat: Err(HttpError::Offline("no route".into())),
        ..FakeHttp::default()
    };
    let mut core = core_with("llm-failed", http, keyed_store());
    assert!(core.llm_wish("a groove in A", "medium"));
    let events = wait(&mut core);
    assert!(matches!(events[0], UiEvent::LlmFailed(_)), "{events:?}");
}

#[test]
fn a_second_wish_is_refused_while_one_runs() {
    let http = FakeHttp {
        chat: Ok(chat_json(VALID_PLAN)),
        ..FakeHttp::default()
    };
    let mut core = core_with("llm-busy", http, keyed_store());
    assert!(core.llm_wish("first", "medium"));
    assert!(!core.llm_wish("second", "medium"));
    assert!(core.llm_busy());
    wait(&mut core);
}

#[test]
fn an_unknown_provider_starts_nothing() {
    let mut core = Core::new(&temp_dir("llm-unknown"), QaOptions::default());
    core.update_settings(|s| s.llm_provider = "nope".into());
    assert!(!core.llm_wish("a groove", "medium"));
    assert!(!core.llm_fetch_models());
}

#[test]
fn model_list_comes_back_sorted_through_tick() {
    let http = FakeHttp {
        models: Ok(br#"{"data":[{"id":"b"},{"id":"a"}]}"#.to_vec()),
        ..FakeHttp::default()
    };
    let mut core = core_with("llm-models", http, keyed_store());
    assert!(core.llm_fetch_models());
    let events = wait(&mut core);
    assert_eq!(
        events,
        vec![UiEvent::LlmModels(vec!["a".into(), "b".into()])]
    );
}

/// Runs `core.llm_check_key(provider)` to completion and returns its `LlmKeyState` text.
fn checked_text(core: &mut Core, provider: &str) -> String {
    assert!(core.llm_check_key(provider));
    let events = wait(core);
    let UiEvent::LlmKeyState { text, .. } = &events[0] else {
        panic!("{events:?}");
    };
    text.clone()
}

#[test]
fn env_variable_beats_a_stored_key() {
    let _guard = ENV_LOCK.lock().unwrap();
    let secrets = Arc::new(MemoryStore::new());
    secrets.set("openai", "sk-stored").unwrap();
    std::env::set_var("OPENAI_API_KEY", "sk-env");
    let mut core = core_with("llm-env", FakeHttp::default(), secrets);
    assert!(checked_text(&mut core, "openai").contains("OPENAI_API_KEY"));
    std::env::remove_var("OPENAI_API_KEY");
}

#[test]
fn a_stored_key_is_reported_without_an_env_variable() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::remove_var("OPENAI_API_KEY");
    let secrets = Arc::new(MemoryStore::new());
    secrets.set("openai", "sk-stored").unwrap();
    let mut core = core_with("llm-stored", FakeHttp::default(), secrets);
    assert_eq!(checked_text(&mut core, "openai"), "saved in the keyring");
}

#[test]
fn a_missing_key_is_reported_as_missing() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::remove_var("OPENAI_API_KEY");
    let mut core = core_with(
        "llm-missing",
        FakeHttp::default(),
        Arc::new(MemoryStore::new()),
    );
    assert_eq!(checked_text(&mut core, "openai"), "no key");
}

#[derive(Debug, Default)]
struct UnavailableSecrets;

impl SecretStore for UnavailableSecrets {
    fn get(&self, _account: &str) -> Result<String, SecretError> {
        Err(SecretError::Unavailable)
    }
    fn set(&self, _account: &str, _value: &str) -> Result<(), SecretError> {
        Err(SecretError::Unavailable)
    }
    fn delete(&self, _account: &str) -> Result<(), SecretError> {
        Err(SecretError::Unavailable)
    }
}

#[test]
fn an_unreachable_keyring_is_reported_as_such() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::remove_var("OPENAI_API_KEY");
    let mut core = core_with(
        "llm-unavailable",
        FakeHttp::default(),
        Arc::new(UnavailableSecrets),
    );
    assert_eq!(checked_text(&mut core, "openai"), "keyring not reachable");
    assert!(core.llm_set_key("openai", "sk-x"));
    let events = wait(&mut core);
    assert!(
        matches!(events[0], UiEvent::LlmKeyError { .. }),
        "{events:?}"
    );
}

#[test]
fn an_empty_key_is_refused() {
    let mut core = core_with(
        "llm-empty-key",
        FakeHttp::default(),
        Arc::new(MemoryStore::new()),
    );
    assert!(!core.llm_set_key("openai", ""));
}

#[test]
fn the_key_never_reaches_settings_json_or_a_backup() {
    let secrets = Arc::new(MemoryStore::new());
    secrets.set("openai", "sk-very-secret").unwrap();
    let dir = temp_dir("llm-no-leak");
    let mut core = Core::new(&dir, QaOptions::default());
    core.update_settings(|s| s.llm_provider = "openai".into());
    core.llm_test_env(Arc::new(FakeHttp::default()), secrets);

    let settings_text = serde_json::to_string(core.settings()).unwrap();
    assert!(!settings_text.contains("sk-very-secret"), "{settings_text}");

    let backup = dir.join("backup.json");
    core.export_data(&backup).unwrap();
    let backup_text = std::fs::read_to_string(&backup).unwrap();
    assert!(!backup_text.contains("sk-very-secret"), "{backup_text}");
}

#[test]
fn the_key_never_reaches_an_error_string() {
    let http = FakeHttp {
        chat: Err(HttpError::Failed("provider boom".into())),
        ..FakeHttp::default()
    };
    let secrets = Arc::new(MemoryStore::new());
    secrets.set("openai", "sk-very-secret").unwrap();
    let mut core = core_with("llm-no-leak-error", http, secrets);
    assert!(core.llm_wish("a groove", "medium"));
    let events = wait(&mut core);
    let UiEvent::LlmFailed(text) = &events[0] else {
        panic!("{events:?}");
    };
    assert!(!text.contains("sk-very-secret"), "{text}");
}

/// Bound a would-be freeze: before the fix, `llm_check_key` read the keyring itself (its logic
/// lived in the sync `Core::llm_key_state`), so this test hung until a timeout instead of
/// returning. Now the read happens inside the spawned thread.
#[test]
fn checking_a_key_never_blocks_the_caller() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::remove_var("OPENAI_API_KEY");
    let (release, gate) = channel();
    let secrets = Arc::new(BlockingSecrets {
        gate: Mutex::new(gate),
        value: "sk-blocked".into(),
    });
    let mut core = core_with("llm-check-nonblock", FakeHttp::default(), secrets);

    let start = Instant::now();
    assert!(core.llm_check_key("openai"));
    assert!(
        start.elapsed() < Duration::from_millis(100),
        "{:?}",
        start.elapsed()
    );

    release.send(()).unwrap();
    let events = wait_timeout(&mut core, Duration::from_secs(2));
    assert!(
        matches!(events[0], UiEvent::LlmKeyState { .. }),
        "{events:?}"
    );
    assert!(!format!("{events:?}").contains("sk-blocked"), "{events:?}");
}

/// Same freeze, for the key read `llm_wish` used to do on the caller before spawning.
#[test]
fn a_wish_never_reads_its_key_on_the_caller() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::remove_var("OPENAI_API_KEY");
    let (release, gate) = channel();
    let secrets = Arc::new(BlockingSecrets {
        gate: Mutex::new(gate),
        value: "sk-blocked".into(),
    });
    let http = FakeHttp {
        chat: Ok(chat_json(VALID_PLAN)),
        ..FakeHttp::default()
    };
    let mut core = core_with("llm-wish-nonblock", http, secrets);

    let start = Instant::now();
    assert!(core.llm_wish("a groove", "medium"));
    assert!(
        start.elapsed() < Duration::from_millis(100),
        "{:?}",
        start.elapsed()
    );

    release.send(()).unwrap();
    let events = wait_timeout(&mut core, Duration::from_secs(2));
    assert!(matches!(events[0], UiEvent::LlmPieceReady(_)), "{events:?}");
    assert!(!format!("{events:?}").contains("sk-blocked"), "{events:?}");
}

/// Same freeze, for the key read `llm_fetch_models` used to do on the caller before spawning.
#[test]
fn fetching_models_never_reads_its_key_on_the_caller() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::remove_var("OPENAI_API_KEY");
    let (release, gate) = channel();
    let secrets = Arc::new(BlockingSecrets {
        gate: Mutex::new(gate),
        value: "sk-blocked".into(),
    });
    let http = FakeHttp {
        models: Ok(br#"{"data":[{"id":"a"}]}"#.to_vec()),
        ..FakeHttp::default()
    };
    let mut core = core_with("llm-models-nonblock", http, secrets);

    let start = Instant::now();
    assert!(core.llm_fetch_models());
    assert!(
        start.elapsed() < Duration::from_millis(100),
        "{:?}",
        start.elapsed()
    );

    release.send(()).unwrap();
    let events = wait_timeout(&mut core, Duration::from_secs(2));
    assert!(matches!(events[0], UiEvent::LlmModels(_)), "{events:?}");
    assert!(!format!("{events:?}").contains("sk-blocked"), "{events:?}");
}

/// `llm_set_key`/`llm_delete_key` answer at once too; the write itself (on `BlockingSecrets`,
/// which never blocks on `set`/`delete`) still runs off the caller's thread.
#[test]
fn saving_a_key_never_blocks_the_caller() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::remove_var("OPENAI_API_KEY");
    let mut core = core_with(
        "llm-set-nonblock",
        FakeHttp::default(),
        Arc::new(MemoryStore::new()),
    );

    let start = Instant::now();
    assert!(core.llm_set_key("openai", "sk-fresh"));
    assert!(
        start.elapsed() < Duration::from_millis(100),
        "{:?}",
        start.elapsed()
    );

    let events = wait_timeout(&mut core, Duration::from_secs(2));
    assert!(
        matches!(events[0], UiEvent::LlmKeyState { .. }),
        "{events:?}"
    );
    assert!(!format!("{events:?}").contains("sk-fresh"), "{events:?}");
}
