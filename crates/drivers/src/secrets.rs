//! API keys of LLM providers (LH-BIB-16), kept in the system's own secret store, never in
//! Kontra's own settings file, so a backup or an error never carries one along.
//!
//! ```text
//! session ─► dyn SecretStore ─┬─► Keychain (keyring: Secret Service / Credential Manager)
//!                             └─► MemoryStore (tests: nothing touches the real store)
//! ```

use keyring::Entry;
pub use kontra_model::{MemoryStore, SecretError, SecretStore};
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;

/// Identifies Kontra to the secret store; one entry per provider id as the account name.
const SERVICE: &str = "io.github.shrippen.Kontra";
/// A locked keyring can prompt the user to unlock it (a GUI the Secret Service shows); off
/// screen, or if the person never answers, that prompt blocks the D-Bus call forever. Every
/// `Keychain` call is bounded by this, so a caller on the UI thread never freezes on it; past
/// it, the call is treated as `Unavailable` (its thread is left to finish or hang on its own).
const KEYRING_TIMEOUT: Duration = Duration::from_secs(3);

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
