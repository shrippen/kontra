//! The engine clock shared by input and output.

use std::sync::Arc;
use std::time::Instant;

/// Seconds since creation; cheap to clone and read on any thread.
#[derive(Debug, Clone)]
pub struct Clock {
    origin: Arc<Instant>,
}

impl Default for Clock {
    fn default() -> Self {
        Clock::new()
    }
}

impl Clock {
    pub fn new() -> Clock {
        Clock {
            origin: Arc::new(Instant::now()),
        }
    }

    pub fn now_s(&self) -> f64 {
        self.origin.elapsed().as_secs_f64()
    }
}
