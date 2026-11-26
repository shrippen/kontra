//! The outside world as the layers above get it: web access and the store of secrets.
//! The traits (`Http`, `SecretStore`) are in `kontra_model`, so tests can pass fakes.

use kontra_engine::{FileHttp, Keychain, Web};
use kontra_model::{Http, SecretStore};
use std::path::Path;
use std::sync::Arc;

/// The real web. Loads the system's TLS roots, so make it on first use.
pub fn web() -> Arc<dyn Http> {
    Arc::new(Web::new())
}

/// Answers every address with the content of `answer` and never touches the network (QA).
pub fn web_from_file(answer: &Path) -> Arc<dyn Http> {
    Arc::new(FileHttp::new(answer))
}

/// The system's keychain.
pub fn keychain() -> Arc<dyn SecretStore> {
    Arc::new(Keychain)
}
