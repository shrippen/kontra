//! "Stück wünschen" (LH-BIB-16): provider settings, key storage and the background wish/model
//! list jobs, all of it just forwarding to `Core`; the real logic is `kontra-appcore`'s.

use super::{q, qobject};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;

impl qobject::Controller {
    pub(super) fn llm_providers_json(&self) -> QString {
        q(&self.rust().core.llm_providers_json())
    }

    pub(super) fn llm_key_state_text(&self, provider: &QString) -> QString {
        q(&self.rust().core.llm_key_state_text(&provider.to_string()))
    }

    pub(super) fn llm_set_key(
        mut self: Pin<&mut Self>,
        provider: &QString,
        value: &QString,
    ) -> QString {
        match self
            .as_mut()
            .core()
            .llm_set_key(&provider.to_string(), &value.to_string())
        {
            Ok(()) => QString::default(),
            Err(e) => q(&e),
        }
    }

    pub(super) fn llm_delete_key(mut self: Pin<&mut Self>, provider: &QString) -> QString {
        match self.as_mut().core().llm_delete_key(&provider.to_string()) {
            Ok(()) => QString::default(),
            Err(e) => q(&e),
        }
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
