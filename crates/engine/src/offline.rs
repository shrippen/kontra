//! Whole-buffer analysis for the CLI, tests and the synthetic corpus.

use kontra_dsp::{NoteTracker, TrackerConfig};
use kontra_model::{DetectedNote, InputSource, Instrument};

/// Detects all notes in `samples` (mono); onset times are seconds from the buffer start.
pub fn analyze(
    samples: &[f32],
    sample_rate: f64,
    instrument: &Instrument,
    source: InputSource,
) -> Vec<DetectedNote> {
    let mut tracker = NoteTracker::new(
        sample_rate,
        TrackerConfig::for_instrument(instrument, source),
    );
    let mut notes = Vec::new();
    for block in samples.chunks(512) {
        notes.extend_from_slice(tracker.push(block));
    }
    notes.extend_from_slice(tracker.flush());
    notes
}
