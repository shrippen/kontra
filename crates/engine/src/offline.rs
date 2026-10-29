//! Whole-buffer analysis for the CLI, tests and the synthetic corpus.

use kontra_dsp::{EchoCanceller, NoteTracker, TrackerConfig, ECHO_BLOCK};
use kontra_model::{DetectedNote, InputSource, Instrument, PitchRevision, TechniqueRevision};

/// Samples per analysis block, as the engine feeds them (480 at 48 kHz, close to it).
const BLOCK: usize = 512;

/// Detects all notes in `samples` (mono); onset times are seconds from the buffer start.
pub fn analyze(
    samples: &[f32],
    sample_rate: f64,
    instrument: &Instrument,
    source: InputSource,
) -> Vec<DetectedNote> {
    analyze_reported(samples, sample_rate, instrument, source).0
}

/// Like `analyze`, with the time each note was first reported, in seconds from the buffer start:
/// the end of the block that told it (LH-NF-15). Example: an onset at 1.000 s told in the block
/// ending at 1.045 s took 45 ms.
pub(crate) fn analyze_reported(
    samples: &[f32],
    sample_rate: f64,
    instrument: &Instrument,
    source: InputSource,
) -> (Vec<DetectedNote>, Vec<f64>) {
    let mut tracker = NoteTracker::new(
        sample_rate,
        TrackerConfig::for_instrument(instrument, source),
    );
    let mut notes = Vec::new();
    let mut reported = Vec::new();
    for (i, block) in samples.chunks(BLOCK).enumerate() {
        notes.extend_from_slice(tracker.push(block));
        let end_s = (i * BLOCK + block.len()) as f64 / sample_rate;
        reported.resize(notes.len(), end_s);
        revise(&mut notes, tracker.revised());
        revise_pitch(&mut notes, tracker.pitch_revised());
    }
    notes.extend_from_slice(tracker.flush());
    reported.resize(notes.len(), samples.len() as f64 / sample_rate);
    revise(&mut notes, tracker.revised());
    revise_pitch(&mut notes, tracker.pitch_revised());
    (notes, reported)
}

/// Applies techniques told late to the notes they belong to (same onset).
pub fn revise(notes: &mut [DetectedNote], revisions: &[TechniqueRevision]) {
    for r in revisions {
        if let Some(n) = notes
            .iter_mut()
            .rev()
            .find(|n| (n.onset_s - r.onset_s).abs() < 1e-9)
        {
            n.technique = Some(r.technique);
        }
    }
}

/// Applies pitches told once the tone settled to the notes they belong to (same onset).
pub fn revise_pitch(notes: &mut [DetectedNote], revisions: &[PitchRevision]) {
    for r in revisions {
        if let Some(n) = notes
            .iter_mut()
            .rev()
            .find(|n| (n.onset_s - r.onset_s).abs() < 1e-9)
        {
            n.midi = r.midi;
        }
    }
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
    analyze(
        &cancel_echo(samples, reference),
        sample_rate,
        instrument,
        source,
    )
}

/// `samples` without what `reference` put into them (the band from the speakers).
pub(crate) fn cancel_echo(samples: &[f32], reference: &[f32]) -> Vec<f32> {
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
    cleaned
}
