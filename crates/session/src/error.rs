//! The audio side's failure, as the layers above see it: a device that does not open, a file
//! that does not read, a thread that does not start.

use thiserror::Error;

#[derive(Debug, Error)]
#[error(transparent)]
pub struct EngineError(#[from] kontra_engine::EngineError);

impl EngineError {
    /// True if the system refused the audio device (LH-EIN-15).
    pub fn is_denied(&self) -> bool {
        self.0.is_denied()
    }
}
