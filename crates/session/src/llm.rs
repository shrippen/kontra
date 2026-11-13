//! A piece plan from an LLM (LH-BIB-16, "Stück wünschen"): pick a provider, send the wish and
//! the prompt, validate the answer and retry once with its errors. Synchronous over `dyn Http`;
//! appcore runs it off a background thread, the way it owns `SecretStore` and settings.
//!
//! ```text
//! provider, wish, level ──► generate::generate_plan ──► PiecePlan or GenError
//!                                     │
//!                        wire::build_chat_request ──► Http::post_json ──► wire::parse_chat_answer
//! ```

use std::time::Duration;

mod generate;
mod prompt;
mod provider;
mod wire;

/// An LLM can take minutes to write a whole piece plan (mirrors `kontra_drivers`'s own).
const LLM_TIMEOUT: Duration = Duration::from_secs(180);

pub use generate::{generate_plan, list_models, GenError};
pub use provider::{provider, KeyNeed, Provider, WireFormat, CUSTOM, PROVIDERS};
pub use wire::provider_message;
