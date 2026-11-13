//! A wished-for piece, or a provider's model list (LH-BIB-16, "Stück wünschen"): one such
//! request at a time, answered off the UI thread. The key never crosses into `Settings`; it is
//! read fresh from the secret store (or the provider's environment variable) right before the
//! request and held only inside the spawned thread.
//!
//! ```text
//! wish, level ──► thread: session::generate_plan ──► PiecePlan JSON ──┬─► tick: render, write
//! "Modelle laden" ──► thread: session::list_models ──► model ids ────┘   (LlmPieceReady/Failed)
//! ```

use crate::{Core, UiEvent};
use kontra_session::exercise::Level;
use kontra_session::{
    self as session, GenError, Http, HttpError, Keychain, Provider, SecretError, SecretStore, Web,
};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;

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
}

impl Default for LlmJob {
    fn default() -> LlmJob {
        LlmJob {
            http: None,
            secrets: Arc::new(Keychain),
            piece: None,
            piece_level: String::new(),
            models: None,
        }
    }
}

impl std::fmt::Debug for LlmJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmJob")
            .field("piece_running", &self.piece.is_some())
            .field("models_running", &self.models.is_some())
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

    /// Whether a key is set for `provider_id`, and where it came from (LH-BIB-16).
    pub fn llm_key_state(&self, provider_id: &str) -> KeyState {
        let Some(provider) = session::provider(provider_id) else {
            return KeyState::Missing;
        };
        if self.llm_env_key(provider).is_some() {
            return KeyState::Env;
        }
        match self.llm.secrets.get(provider.id) {
            Ok(_) => KeyState::Stored,
            Err(SecretError::NotFound | SecretError::Failed(_)) => KeyState::Missing,
            Err(SecretError::Unavailable) => KeyState::Unavailable,
        }
    }

    /// The i18n text of `llm_key_state`.
    pub fn llm_key_state_text(&self, provider_id: &str) -> String {
        let state = self.llm_key_state(provider_id);
        let args: &[(&str, String)] = match (state, session::provider(provider_id)) {
            (KeyState::Env, Some(p)) => &[("variable", p.key_env.to_string())],
            _ => &[],
        };
        self.tr(state.text_key(), args)
    }

    /// Stores `value` as `provider_id`'s key; empty is refused (use `llm_delete_key`).
    pub fn llm_set_key(&self, provider_id: &str, value: &str) -> Result<(), String> {
        if value.is_empty() {
            return Err(self.tr("llm.key.error.empty", &[]));
        }
        self.llm
            .secrets
            .set(provider_id, value)
            .map_err(|e| self.llm_secret_error_text(&e))
    }

    /// Removes a stored key; not an error when none was stored.
    pub fn llm_delete_key(&self, provider_id: &str) -> Result<(), String> {
        match self.llm.secrets.delete(provider_id) {
            Ok(()) | Err(SecretError::NotFound) => Ok(()),
            Err(e) => Err(self.llm_secret_error_text(&e)),
        }
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
        let key = self.llm_key(&provider);
        let http = self.llm_http();
        let wish = wish.to_string();
        let level_key = level.to_string();
        let (tell, answer) = channel();
        std::thread::spawn(move || {
            let level = Level::from_key(&level_key).unwrap_or_default();
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
        let key = self.llm_key(&provider);
        let http = self.llm_http();
        let (tell, answer) = channel();
        std::thread::spawn(move || {
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

    /// The key to send: the provider's environment variable first, else the stored one.
    fn llm_key(&self, provider: &Provider) -> Option<String> {
        self.llm_env_key(provider)
            .or_else(|| self.llm.secrets.get(provider.id).ok())
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

/// A seed for the piece's variety (LH-BIB-13): the wish names no seed, so the clock gives one.
fn llm_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64)
}
