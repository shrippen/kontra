//! "Stück wünschen" (LH-BIB-16): provider settings, key storage and the background wish/model
//! list jobs, all of it just forwarding to `Core`; the real logic is `kontra-appcore`'s.

use super::{q, qobject};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use kontra_appcore::LlmEvent;

impl qobject::Controller {
    pub(super) fn llm_providers_json(&self) -> QString {
        q(&self.rust().core.llm_providers_json())
    }

    /// `tick`'s dispatch for `UiEvent::Llm` (kept here, not in the big match in `bridge.rs`,
    /// so that file stays under its line budget).
    pub(super) fn dispatch_llm(mut self: Pin<&mut Self>, event: LlmEvent) {
        match event {
            LlmEvent::PieceReady(path) => {
                self.as_mut().set_llm_busy(false);
                if self.as_mut().open_file(&q(&path)) {
                    self.as_mut().llm_piece_opened();
                }
            }
            LlmEvent::Models(models) => {
                self.as_mut().set_llm_busy(false);
                let json = serde_json::Value::Array(
                    models.into_iter().map(serde_json::Value::String).collect(),
                )
                .to_string();
                self.as_mut().set_llm_models_json(q(&json));
            }
            LlmEvent::Failed(text) => {
                self.as_mut().set_llm_busy(false);
                // Cleared first: the same failure twice must reach the wish dialog twice
                // (`errorText` changes only on a new value).
                self.as_mut().show_error("");
                self.as_mut().show_error(&text);
            }
            LlmEvent::KeyState {
                provider,
                state,
                ready,
                text,
            } => {
                // A check queued behind this job may already run.
                let busy = self.as_mut().core().llm_key_busy();
                self.as_mut().set_llm_key_busy(busy);
                self.as_mut()
                    .llm_key_state(q(&provider), q(state.id()), ready, q(&text));
            }
            LlmEvent::KeyError { provider, text } => {
                let busy = self.as_mut().core().llm_key_busy();
                self.as_mut().set_llm_key_busy(busy);
                self.as_mut().llm_key_error(q(&provider), q(&text));
            }
        }
    }

    pub(super) fn llm_check_key(mut self: Pin<&mut Self>, provider: &QString) -> bool {
        let started = self.as_mut().core().llm_check_key(&provider.to_string());
        if started {
            self.as_mut().set_llm_key_busy(true);
        }
        started
    }

    pub(super) fn llm_set_key(
        mut self: Pin<&mut Self>,
        provider: &QString,
        value: &QString,
    ) -> bool {
        let started = self
            .as_mut()
            .core()
            .llm_set_key(&provider.to_string(), &value.to_string());
        if started {
            self.as_mut().set_llm_key_busy(true);
        }
        started
    }

    pub(super) fn llm_delete_key(mut self: Pin<&mut Self>, provider: &QString) -> bool {
        let started = self.as_mut().core().llm_delete_key(&provider.to_string());
        if started {
            self.as_mut().set_llm_key_busy(true);
        }
        started
    }

    pub(super) fn change_llm_provider(mut self: Pin<&mut Self>, provider: &QString) {
        let provider = provider.to_string();
        self.as_mut()
            .core()
            .update_settings(|s| s.llm_provider = provider.clone());
        self.as_mut().set_llm_provider(q(&provider));
        self.as_mut().set_llm_models_json(q("[]"));
    }

    pub(super) fn change_llm_base_url(mut self: Pin<&mut Self>, url: &QString) {
        let url = url.to_string();
        self.as_mut()
            .core()
            .update_settings(|s| s.llm_base_url = url.clone());
        self.as_mut().set_llm_base_url(q(&url));
    }

    pub(super) fn change_llm_model(mut self: Pin<&mut Self>, model: &QString) {
        let model = model.to_string();
        self.as_mut()
            .core()
            .update_settings(|s| s.llm_model = model.clone());
        self.as_mut().set_llm_model(q(&model));
    }

    pub(super) fn llm_wish(mut self: Pin<&mut Self>, wish: &QString, level: &QString) -> bool {
        let started = self
            .as_mut()
            .core()
            .llm_wish(&wish.to_string(), &level.to_string());
        if started {
            self.as_mut().set_llm_busy(true);
        }
        started
    }

    pub(super) fn llm_fetch_models(mut self: Pin<&mut Self>) -> bool {
        let started = self.as_mut().core().llm_fetch_models();
        if started {
            self.as_mut().set_llm_busy(true);
        }
        started
    }
}
