//! Whole-buffer analysis for the CLI, tests and the synthetic corpus.

use kontra_dsp::{EchoCanceller, NoteTracker, TrackerConfig, ECHO_BLOCK};
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

/// Like `analyze`, after removing `reference` (what played from the speakers at the same time)
/// as heard through the room (LH-EIN-11). `reference` is as long as `samples` or shorter.
pub fn analyze_without(
    samples: &[f32],
    reference: &[f32],
    sample_rate: f64,
    instrument: &Instrument,
    source: InputSource,
) -> Vec<DetectedNote> {
    let mut canceller = EchoCanceller::new();
    let mut cleaned = samples.to_vec();
    let silence = [0.0f32; ECHO_BLOCK];
    for (i, block) in cleaned
        .as_chunks_mut::<ECHO_BLOCK>()
        .0
        .iter_mut()
        .enumerate()
    {
        let from = i * ECHO_BLOCK;
        let played = reference.get(from..from + ECHO_BLOCK).unwrap_or(&silence);
        canceller.process(block, played);
    }
    analyze(&cleaned, sample_rate, instrument, source)
}
