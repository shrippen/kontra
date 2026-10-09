//! Import errors. Messages name the cause; the UI maps variants to translated texts.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("cannot read file: {0}")]
    Io(#[from] std::io::Error),
    #[error("unknown file format")]
    UnknownFormat,
    #[error("unsupported format: {0}")]
    Unsupported(String),
    #[error("file ends unexpectedly at byte {0}")]
    Truncated(usize),
    #[error("malformed file: {0}")]
    Malformed(String),
    #[error("track {0} does not exist")]
    NoTrack(usize),
    #[error("file too large: {0} bytes")]
    TooLarge(u64),
    #[error("file contains no notes")]
    Empty,
    #[error("song too long: more than {0} bars after unrolling repeats")]
    TooLong(usize),
}

impl ImportError {
    /// Shorthand for `Malformed` with a static reason.
    pub(crate) fn bad(reason: &str) -> ImportError {
        ImportError::Malformed(reason.to_string())
    }
}
