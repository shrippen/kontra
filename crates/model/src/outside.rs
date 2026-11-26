//! What the outside world fills in, as the layers above see it: the web, the system's secret
//! store and the stem separator. The traits are the seams tests put fakes into; `kontra_drivers`
//! has the real ones.

use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;
use thiserror::Error;

/// A slow answer is no answer: the user waits for it.
pub const HTTP_TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum HttpError {
    /// No connection: offline, name not found, refused.
    #[error("no connection: {0}")]
    Offline(String),
    #[error("no answer within {} s", HTTP_TIMEOUT.as_secs())]
    Timeout,
    #[error("HTTP status {0}")]
    Status(u16),
    /// An error status with the body the provider sent back (its own explanation), cut short.
    #[error("HTTP status {0}: {1}")]
    StatusBody(u16, String),
    #[error("{0}")]
    Failed(String),
    /// This `Http` implementation does not do this kind of request (the default, for GET-only
    /// fakes); a real provider call never sees it.
    #[error("not supported")]
    NotSupported,
}

/// Fetches a URL; one call is one request.
pub trait Http: Send + Sync + std::fmt::Debug {
    fn get(&self, url: &str) -> Result<Vec<u8>, HttpError>;

    /// A file to keep (a course package): up to `max_bytes`, with time for a slow line.
    fn download(&self, url: &str, max_bytes: u64) -> Result<Vec<u8>, HttpError> {
        let _ = max_bytes;
        self.get(url)
    }

    /// A GET with extra headers (an LLM provider's model list, sent with its auth header).
    fn get_with(&self, url: &str, headers: &[(&str, &str)]) -> Result<Vec<u8>, HttpError> {
        let _ = headers;
        self.get(url)
    }

    /// A POST of a JSON body, with extra headers (an LLM provider's chat/messages call). The
    /// default errors, so a fake that only implements `get` still compiles.
    fn post_json(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: &[u8],
        timeout: Duration,
    ) -> Result<Vec<u8>, HttpError> {
        let _ = (url, headers, body, timeout);
        Err(HttpError::NotSupported)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SecretError {
    /// No secret under that account (a plain "not set", not a failure).
    #[error("no secret stored")]
    NotFound,
    /// No Secret Service (or equivalent) is running; the UI says so instead of failing silently.
    #[error("the system's secret store is not reachable")]
    Unavailable,
    #[error("{0}")]
    Failed(String),
}

/// One secret under an account name (here: an LLM provider's id).
pub trait SecretStore: Send + Sync + std::fmt::Debug {
    fn get(&self, account: &str) -> Result<String, SecretError>;
    fn set(&self, account: &str, value: &str) -> Result<(), SecretError>;
    fn delete(&self, account: &str) -> Result<(), SecretError>;
}

/// A fake store, in memory only: tests never touch the real keyring.
#[derive(Debug, Default)]
pub struct MemoryStore {
    entries: Mutex<std::collections::HashMap<String, String>>,
}

impl MemoryStore {
    pub fn new() -> MemoryStore {
        MemoryStore::default()
    }

    // A poisoned lock (a panic while held) still holds every secret set so far; one test's
    // panic must not make the next test's assertions lie about what is stored.
    fn entries(&self) -> MutexGuard<'_, std::collections::HashMap<String, String>> {
        self.entries.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl SecretStore for MemoryStore {
    fn get(&self, account: &str) -> Result<String, SecretError> {
        self.entries()
            .get(account)
            .cloned()
            .ok_or(SecretError::NotFound)
    }

    fn set(&self, account: &str, value: &str) -> Result<(), SecretError> {
        self.entries()
            .insert(account.to_string(), value.to_string());
        Ok(())
    }

    fn delete(&self, account: &str) -> Result<(), SecretError> {
        self.entries()
            .remove(account)
            .map(|_| ())
            .ok_or(SecretError::NotFound)
    }
}

#[derive(Debug, Error)]
pub enum SeparatorError {
    #[error("cannot start {program}: {source}")]
    Start {
        program: String,
        source: std::io::Error,
    },
    #[error("separation failed: {0}")]
    Failed(String),
    #[error("separation cancelled")]
    Cancelled,
}
