//! API keys of LLM providers (LH-BIB-16), kept in the system's own secret store, never in
//! Kontra's own settings file, so a backup or an error never carries one along.
//!
//! ```text
//! session ─► dyn SecretStore ─┬─► Keychain (keyring: Secret Service / Credential Manager)
//!                             └─► MemoryStore (tests: nothing touches the real store)
//! ```

use keyring::Entry;
use std::sync::mpsc::RecvTimeoutError;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;
use thiserror::Error;

/// Identifies Kontra to the secret store; one entry per provider id as the account name.
const SERVICE: &str = "io.github.shrippen.Kontra";
/// A locked keyring can prompt the user to unlock it (a GUI the Secret Service shows); off
/// screen, or if the person never answers, that prompt blocks the D-Bus call forever. Every
/// `Keychain` call is bounded by this, so a caller on the UI thread never freezes on it; past
/// it, the call is treated as `Unavailable` (its thread is left to finish or hang on its own).
const KEYRING_TIMEOUT: Duration = Duration::from_secs(3);

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

/// Runs `call` (a blocking keyring operation) on its own thread and waits at most
/// `KEYRING_TIMEOUT` for it; past that, `Unavailable` instead of hanging the caller.
fn bounded<T: Send + 'static>(
    call: impl FnOnce() -> Result<T, SecretError> + Send + 'static,
) -> Result<T, SecretError> {
    let (tell, answer) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        // The receiver may be gone by now (we gave up waiting); nobody is there to hear it.
        let _ = tell.send(call());
    });
    match answer.recv_timeout(KEYRING_TIMEOUT) {
        Ok(result) => result,
        Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => {
            Err(SecretError::Unavailable)
        }
    }
}

impl SecretStore for Keychain {
    fn get(&self, account: &str) -> Result<String, SecretError> {
        let account = account.to_string();
        bounded(move || Keychain::entry(&account)?.get_password().map_err(error))
    }

    fn set(&self, account: &str, value: &str) -> Result<(), SecretError> {
        let account = account.to_string();
        let value = value.to_string();
        bounded(move || {
            Keychain::entry(&account)?
                .set_password(&value)
                .map_err(error)
        })
    }

    fn delete(&self, account: &str) -> Result<(), SecretError> {
        let account = account.to_string();
        bounded(move || {
            Keychain::entry(&account)?
                .delete_credential()
                .map_err(error)
        })
    }
}

fn error(e: keyring::Error) -> SecretError {
    match e {
        keyring::Error::NoEntry => SecretError::NotFound,
        keyring::Error::NoStorageAccess(_)
        | keyring::Error::PlatformFailure(_)
        | keyring::Error::NoDefaultStore => SecretError::Unavailable,
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

    /// A locked keyring can block on a prompt that never comes (offscreen, nobody there to
    /// answer it, review 13.11.2026: opening PieceWish hung the whole app). `bounded` must
    /// still return.
    #[test]
    fn bounded_times_out_instead_of_hanging_forever() {
        let result: Result<(), SecretError> = bounded(|| {
            std::thread::sleep(KEYRING_TIMEOUT * 10);
            Ok(())
        });
        assert_eq!(result, Err(SecretError::Unavailable));
    }

    #[test]
    fn bounded_passes_through_a_quick_result() {
        let result: Result<&str, SecretError> = bounded(|| Ok("sk-x"));
        assert_eq!(result, Ok("sk-x"));
    }

    /// No Secret Service or session bus: keyring reports `NoDefaultStore`, which the UI must show
    /// as "keyring not reachable", not as "no key" or with keyring's English text.
    #[test]
    fn no_default_store_is_unavailable() {
        assert_eq!(
            error(keyring::Error::NoDefaultStore),
            SecretError::Unavailable
        );
        assert_eq!(error(keyring::Error::NoEntry), SecretError::NotFound);
    }

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
