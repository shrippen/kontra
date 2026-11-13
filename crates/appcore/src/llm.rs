//! A wished-for piece, or a provider's model list (LH-BIB-16, "Stück wünschen"): one such
//! request at a time, answered off the UI thread. The key never crosses into `Settings`; it is
//! read fresh from the secret store (or the provider's environment variable) inside the spawned
//! thread, never on the caller's (review 13.11.2026: the keyring can block for up to
//! `KEYRING_TIMEOUT`, which froze the window when read from `Core::llm_key_state` etc. directly).
//!
//! ```text
//! wish, level ──► thread: resolve key, session::generate_plan ──► PiecePlan JSON ─┬─► tick
//! "Modelle laden" ──► thread: resolve key, session::list_models ──► model ids ────┤ (render,
//! check/save/delete key ──► thread: Keychain::get/set/delete ──► new KeyState ────┘  write,
//!                                                                                     LlmKeyState)
//! ```

use crate::{Core, UiEvent};
use kontra_session::exercise::Level;
use kontra_session::{
    self as session, GenError, Http, HttpError, Keychain, Provider, SecretError, SecretStore, Web,
};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;

/// What a key check, save or delete job answers with, once its thread is done.
enum KeyOutcome {
    /// `llm_check_key`: the state as found, no write attempted.
    Checked(KeyState),
    /// `llm_set_key`/`llm_delete_key`: the state after the write, or why the write failed.
    Written(Result<KeyState, SecretError>),
}

pub(crate) struct LlmJob {
    /// Made on first use (`Web::new` loads the system's TLS roots, not worth paying for at
    /// every start); a fake in tests.
    http: Option<Arc<dyn Http>>,
    /// Cheap to make, so kept ready from the start; a fake in tests.
    secrets: Arc<dyn SecretStore>,
    piece: Option<Receiver<Result<String, GenError>>>,
    /// The level the running piece job was asked for; needed again once its answer comes back.
    piece_level: String,
    models: Option<Receiver<Result<Vec<String>, GenError>>>,
    /// A key check, save or delete in flight; the provider id travels with it (the UI may have
    /// switched provider by the time the answer comes back, but the answer still names the one
    /// it was asked about).
    key: Option<(String, Receiver<KeyOutcome>)>,
}

impl Default for LlmJob {
    fn default() -> LlmJob {
        LlmJob {
            http: None,
            secrets: Arc::new(Keychain),
            piece: None,
            piece_level: String::new(),
            models: None,
            key: None,
        }
    }
}

impl std::fmt::Debug for LlmJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmJob")
            .field("piece_running", &self.piece.is_some())
            .field("models_running", &self.models.is_some())
            .field("key_running", &self.key.is_some())
            .finish_non_exhaustive()
    }
}

/// Whether a key is set for a provider, and where it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    /// From the provider's own environment variable (beats a stored key).
    Env,
    /// In the system's secret store.
    Stored,
    /// Neither: the provider needs one before it can be asked.
    Missing,
    /// The system's secret store could not be reached at all.
    Unavailable,
}

impl KeyState {
    /// The i18n key of its text in the settings row.
    pub(crate) fn text_key(self) -> &'static str {
        match self {
            KeyState::Env => "llm.key.env",
            KeyState::Stored => "llm.key.stored",
            KeyState::Missing => "llm.key.missing",
            KeyState::Unavailable => "llm.key.unavailable",
        }
    }
}

impl Core {
    /// Every provider Kontra offers, for the settings' picker.
    pub fn llm_providers_json(&self) -> String {
        let providers: Vec<_> = session::PROVIDERS
            .iter()
            .map(|p| {
                serde_json::json!({
                    "id": p.id,
                    "name": p.name,
                    "needsKey": !matches!(p.key, session::KeyNeed::None),
                    "docsUrl": p.docs_url,
                    "defaultModel": p.default_model.unwrap_or(""),
                })
            })
            .collect();
        serde_json::Value::Array(providers).to_string()
    }

    /// Starts checking whether a key is set for `provider_id`, and where it came from
    /// (LH-BIB-16); the answer comes back through `tick` as `UiEvent::LlmKeyState`. `false`
    /// when a key job already runs or `provider_id` names no provider Kontra offers.
    pub fn llm_check_key(&mut self, provider_id: &str) -> bool {
        if self.llm.key.is_some() {
            return false;
        }
        let Some(provider) = session::provider(provider_id).copied() else {
            return false;
        };
        let env_present = self.llm_env_key(&provider).is_some();
        let secrets = self.llm.secrets.clone();
        let (tell, answer) = channel();
        std::thread::spawn(move || {
            let state = key_state_blocking(secrets.as_ref(), provider.id, env_present);
            let _ = tell.send(KeyOutcome::Checked(state));
        });
        self.llm.key = Some((provider.id.to_string(), answer));
        true
    }

    /// Starts storing `value` as `provider_id`'s key in the background; empty is refused (use
    /// `llm_delete_key`). The new state comes back through `tick` as `UiEvent::LlmKeyState`, a
    /// failed write as `UiEvent::LlmKeyError`. `false` when nothing was started.
    pub fn llm_set_key(&mut self, provider_id: &str, value: &str) -> bool {
        if value.is_empty() || self.llm.key.is_some() {
            return false;
        }
        let Some(provider) = session::provider(provider_id).copied() else {
            return false;
        };
        let env_present = self.llm_env_key(&provider).is_some();
        let secrets = self.llm.secrets.clone();
        let value = value.to_string();
        let (tell, answer) = channel();
        std::thread::spawn(move || {
            let result = secrets
                .set(provider.id, &value)
                .map(|()| key_state_blocking(secrets.as_ref(), provider.id, env_present));
            let _ = tell.send(KeyOutcome::Written(result));
        });
        self.llm.key = Some((provider.id.to_string(), answer));
        true
    }

    /// Starts removing a stored key in the background; not an error when none was stored. The
    /// new state comes back through `tick` as `UiEvent::LlmKeyState`, a failed delete as
    /// `UiEvent::LlmKeyError`. `false` when nothing was started.
    pub fn llm_delete_key(&mut self, provider_id: &str) -> bool {
        if self.llm.key.is_some() {
            return false;
        }
        let Some(provider) = session::provider(provider_id).copied() else {
            return false;
        };
        let env_present = self.llm_env_key(&provider).is_some();
        let secrets = self.llm.secrets.clone();
        let (tell, answer) = channel();
        std::thread::spawn(move || {
            let result = match secrets.delete(provider.id) {
                Ok(()) | Err(SecretError::NotFound) => Ok(key_state_blocking(
                    secrets.as_ref(),
                    provider.id,
                    env_present,
                )),
                Err(e) => Err(e),
            };
            let _ = tell.send(KeyOutcome::Written(result));
        });
        self.llm.key = Some((provider.id.to_string(), answer));
        true
    }

    /// Starts writing a piece from `wish` at `level` in the background; `false` when a request
    /// already runs or `llm_provider` names no provider Kontra offers.
    pub fn llm_wish(&mut self, wish: &str, level: &str) -> bool {
        if self.llm.piece.is_some() || self.llm.models.is_some() {
            return false;
        }
        let Some(provider) = session::provider(&self.settings.llm_provider).copied() else {
            return false;
        };
        let base_url = self.llm_base_url(&provider);
        let model = self.llm_model(&provider);
        let env_key = self.llm_env_key(&provider);
        let secrets = self.llm.secrets.clone();
        let http = self.llm_http();
        let wish = wish.to_string();
        let level_key = level.to_string();
        let (tell, answer) = channel();
        std::thread::spawn(move || {
            let level = Level::from_key(&level_key).unwrap_or_default();
            let key = env_key.or_else(|| secrets.get(provider.id).ok());
            let result = session::generate_plan(
                http.as_ref(),
                &provider,
                &base_url,
                key.as_deref(),
                &model,
                &wish,
                level,
            )
            .map(|plan| serde_json::to_string(&plan).unwrap_or_default());
            // The core may be gone by now; nobody waits for the answer then.
            let _ = tell.send(result);
        });
        self.llm.piece = Some(answer);
        self.llm.piece_level = level.to_string();
        true
    }

    /// Fetches `llm_provider`'s model list in the background (also used as "test connection").
    /// `false` when a request already runs.
    pub fn llm_fetch_models(&mut self) -> bool {
        if self.llm.piece.is_some() || self.llm.models.is_some() {
            return false;
        }
        let Some(provider) = session::provider(&self.settings.llm_provider).copied() else {
            return false;
        };
        let base_url = self.llm_base_url(&provider);
        let env_key = self.llm_env_key(&provider);
        let secrets = self.llm.secrets.clone();
        let http = self.llm_http();
        let (tell, answer) = channel();
        std::thread::spawn(move || {
            let key = env_key.or_else(|| secrets.get(provider.id).ok());
            let result = session::list_models(http.as_ref(), &provider, &base_url, key.as_deref());
            let _ = tell.send(result);
        });
        self.llm.models = Some(answer);
        true
    }

    /// A request still running; the UI offers only one at a time.
    pub fn llm_busy(&self) -> bool {
        self.llm.piece.is_some() || self.llm.models.is_some()
    }

    /// A key check, save or delete still running.
    pub fn llm_key_busy(&self) -> bool {
        self.llm.key.is_some()
    }

    /// Finished background requests, as events for `tick`.
    pub(crate) fn llm_poll(&mut self) -> Vec<UiEvent> {
        let mut events = Vec::new();
        match self.llm.piece.as_ref().map(Receiver::try_recv) {
            Some(Ok(Ok(plan_json))) => {
                self.llm.piece = None;
                let level = std::mem::take(&mut self.llm.piece_level);
                events.push(match self.generate_piece(&plan_json, &level, llm_seed()) {
                    Ok(path) => UiEvent::LlmPieceReady(path.to_string_lossy().into_owned()),
                    Err(e) => UiEvent::LlmFailed(e),
                });
            }
            Some(Ok(Err(err))) => {
                self.llm.piece = None;
                events.push(UiEvent::LlmFailed(self.llm_gen_error_text(&err)));
            }
            Some(Err(TryRecvError::Disconnected)) => self.llm.piece = None,
            Some(Err(TryRecvError::Empty)) | None => {}
        }
        match self.llm.models.as_ref().map(Receiver::try_recv) {
            Some(Ok(Ok(models))) => {
                self.llm.models = None;
                events.push(UiEvent::LlmModels(models));
            }
            Some(Ok(Err(err))) => {
                self.llm.models = None;
                events.push(UiEvent::LlmFailed(self.llm_gen_error_text(&err)));
            }
            Some(Err(TryRecvError::Disconnected)) => self.llm.models = None,
            Some(Err(TryRecvError::Empty)) | None => {}
        }
        if let Some((provider, rx)) = self.llm.key.take() {
            match rx.try_recv() {
                Ok(outcome) => events.push(match outcome {
                    KeyOutcome::Checked(state) => UiEvent::LlmKeyState {
                        text: self.llm_key_state_text(&provider, state),
                        provider,
                    },
                    KeyOutcome::Written(Ok(state)) => UiEvent::LlmKeyState {
                        text: self.llm_key_state_text(&provider, state),
                        provider,
                    },
                    KeyOutcome::Written(Err(e)) => UiEvent::LlmKeyError {
                        text: self.llm_secret_error_text(&e),
                        provider,
                    },
                }),
                Err(TryRecvError::Empty) => self.llm.key = Some((provider, rx)),
                Err(TryRecvError::Disconnected) => {}
            }
        }
        events
    }

    /// The base URL to ask: the provider's own, or the user's for "custom".
    fn llm_base_url(&self, provider: &Provider) -> String {
        if provider.id == session::CUSTOM {
            self.settings.llm_base_url.trim_end_matches('/').to_string()
        } else {
            provider.base_url.to_string()
        }
    }

    /// The model to ask for: chosen, else the provider's default (Anthropic only).
    fn llm_model(&self, provider: &Provider) -> String {
        if !self.settings.llm_model.is_empty() {
            self.settings.llm_model.clone()
        } else {
            provider.default_model.unwrap_or_default().to_string()
        }
    }

    /// The i18n text of a `KeyState` found for `provider_id`.
    fn llm_key_state_text(&self, provider_id: &str, state: KeyState) -> String {
        let args: &[(&str, String)] = match (state, session::provider(provider_id)) {
            (KeyState::Env, Some(p)) => &[("variable", p.key_env.to_string())],
            _ => &[],
        };
        self.tr(state.text_key(), args)
    }

    fn llm_env_key(&self, provider: &Provider) -> Option<String> {
        if provider.key_env.is_empty() {
            return None;
        }
        std::env::var(provider.key_env)
            .ok()
            .filter(|v| !v.is_empty())
    }

    /// The web client to ask with; made on first use.
    fn llm_http(&mut self) -> Arc<dyn Http> {
        self.llm
            .http
            .get_or_insert_with(|| Arc::new(Web::new()))
            .clone()
    }

    fn llm_secret_error_text(&self, e: &SecretError) -> String {
        match e {
            SecretError::Unavailable => self.tr("llm.key.error.unavailable", &[]),
            SecretError::NotFound | SecretError::Failed(_) => {
                self.tr("llm.key.error.failed", &[("reason", e.to_string())])
            }
        }
    }

    /// `err`'s translated text, with the provider's own explanation appended when it sent one.
    fn llm_gen_error_text(&self, err: &GenError) -> String {
        let mut text = match err {
            GenError::Http(HttpError::Offline(_)) => self.tr("llm.error.offline", &[]),
            GenError::Http(HttpError::Timeout) => self.tr("llm.error.timeout", &[]),
            GenError::Http(HttpError::Status(code) | HttpError::StatusBody(code, _)) => {
                self.tr("llm.error.status", &[("code", code.to_string())])
            }
            GenError::Http(HttpError::Failed(_) | HttpError::NotSupported) => {
                self.tr("llm.error.failed", &[])
            }
            GenError::Declined => self.tr("llm.error.declined", &[]),
            GenError::Truncated => self.tr("llm.error.truncated", &[]),
            GenError::Unreadable(_) => self.tr("llm.error.unreadable", &[]),
            GenError::Unparseable => self.tr("llm.error.unparseable", &[]),
            GenError::Invalid(detail) => {
                self.tr("llm.error.invalid", &[("detail", detail.clone())])
            }
        };
        if let GenError::Http(e) = err {
            if let Some(extra) = session::provider_message(e) {
                text = format!("{text}: {extra}");
            }
        }
        text
    }

    #[cfg(test)]
    pub(crate) fn llm_test_env(&mut self, http: Arc<dyn Http>, secrets: Arc<dyn SecretStore>) {
        self.llm.http = Some(http);
        self.llm.secrets = secrets;
    }
}

/// Where a key stands: `env_present` (checked up front, cheap) wins; else the store itself is
/// asked. Runs inside the spawned thread, never on the caller's.
fn key_state_blocking(secrets: &dyn SecretStore, account: &str, env_present: bool) -> KeyState {
    if env_present {
        return KeyState::Env;
    }
    match secrets.get(account) {
        Ok(_) => KeyState::Stored,
        Err(SecretError::NotFound | SecretError::Failed(_)) => KeyState::Missing,
        Err(SecretError::Unavailable) => KeyState::Unavailable,
    }
}

/// A seed for the piece's variety (LH-BIB-13): the wish names no seed, so the clock gives one.
fn llm_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64)
}
