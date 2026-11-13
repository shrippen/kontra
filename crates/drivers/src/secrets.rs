//! API keys of LLM providers (LH-BIB-16), kept in the system's own secret store, never in
//! Kontra's own settings file, so a backup or an error never carries one along.
//!
//! ```text
//! session ─► dyn SecretStore ─┬─► Keychain (keyring: Secret Service / Credential Manager)
//!                             └─► MemoryStore (tests: nothing touches the real store)
//! ```

use keyring::Entry;
use std::sync::{Mutex, MutexGuard, PoisonError};
use thiserror::Error;

/// Identifies Kontra to the secret store; one entry per provider id as the account name.
const SERVICE: &str = "io.github.shrippen.Kontra";

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

/// The system's real secret store.
#[derive(Debug, Default)]
pub struct Keychain;

impl Keychain {
    fn entry(account: &str) -> Result<Entry, SecretError> {
        Entry::new(SERVICE, account).map_err(error)
    }
}

impl SecretStore for Keychain {
    fn get(&self, account: &str) -> Result<String, SecretError> {
        Self::entry(account)?.get_password().map_err(error)
    }

    fn set(&self, account: &str, value: &str) -> Result<(), SecretError> {
        Self::entry(account)?.set_password(value).map_err(error)
    }

    fn delete(&self, account: &str) -> Result<(), SecretError> {
        Self::entry(account)?.delete_credential().map_err(error)
    }
}

fn error(e: keyring::Error) -> SecretError {
    match e {
        keyring::Error::NoEntry => SecretError::NotFound,
        keyring::Error::NoStorageAccess(_) | keyring::Error::PlatformFailure(_) => {
            SecretError::Unavailable
        }
        other => SecretError::Failed(other.to_string()),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_store_round_trips() {
        let store = MemoryStore::new();
        assert_eq!(store.get("openrouter"), Err(SecretError::NotFound));
        store.set("openrouter", "sk-123").unwrap();
        assert_eq!(store.get("openrouter").unwrap(), "sk-123");
        store.delete("openrouter").unwrap();
        assert_eq!(store.get("openrouter"), Err(SecretError::NotFound));
    }

    #[test]
    fn memory_store_keeps_accounts_apart() {
        let store = MemoryStore::new();
        store.set("openrouter", "a").unwrap();
        store.set("openai", "b").unwrap();
        assert_eq!(store.get("openrouter").unwrap(), "a");
        assert_eq!(store.get("openai").unwrap(), "b");
    }

    #[test]
    fn deleting_missing_account_errs() {
        let store = MemoryStore::new();
        assert_eq!(store.delete("none"), Err(SecretError::NotFound));
    }
}
