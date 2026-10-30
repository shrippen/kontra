//! Whole-buffer analysis for the CLI, tests and the synthetic corpus.

use kontra_dsp::{EchoCanceller, NoteTracker, TrackerConfig, ECHO_BLOCK};
use kontra_model::{
    DetectedNote, DoubleStopVerdict, ExpectedStrike, InputSource, Instrument, PitchRevision,
    StringVerdict, TechniqueRevision,
};

/// Samples per analysis block, as the engine feeds them (480 at 48 kHz, close to it).
const BLOCK: usize = 512;

/// Detects all notes in `samples` (mono); onset times are seconds from the buffer start.
pub fn analyze(
    samples: &[f32],
    sample_rate: f64,
    instrument: &Instrument,
    source: InputSource,
) -> Vec<DetectedNote> {
    analyze_reported(samples, sample_rate, instrument, source, &[]).notes
}

/// Notes with when and how they were first told.
pub(crate) struct Reported {
    /// With every revision applied.
    pub notes: Vec<DetectedNote>,
    /// When each note was first reported, in seconds from the buffer start: the end of the
    /// block that told it (LH-NF-15). Example: an onset at 1.000 s told in the block ending at
    /// 1.045 s took 45 ms.
    pub at_s: Vec<f64>,
    /// The pitch each note was first reported with, before revisions.
    pub first_midi: Vec<f64>,
    /// The strings told for notes the tab put on one (R7).
    pub strings: Vec<StringVerdict>,
    /// Verdicts on the double stops `expected` announced (R8).
    pub double_stops: Vec<DoubleStopVerdict>,
}

/// Like `analyze`, with the strikes the tab expects (`expected`, seconds from the buffer
/// start) and when and how each note was first told.
pub(crate) fn analyze_reported(
    samples: &[f32],
    sample_rate: f64,
    instrument: &Instrument,
    source: InputSource,
    expected: &[ExpectedStrike],
) -> Reported {
    let mut tracker = NoteTracker::new(
        sample_rate,
        TrackerConfig::for_instrument(instrument, source),
    );
    let mut told = Reported {
        notes: Vec::new(),
        at_s: Vec::new(),
        first_midi: Vec::new(),
        strings: Vec::new(),
        double_stops: Vec::new(),
    };
    let mut next = 0;
    for (i, block) in samples.chunks(BLOCK).enumerate() {
        let end_s = (i * BLOCK + block.len()) as f64 / sample_rate;

        // Told about a quarter second ahead, as the session does.
        while let Some(strike) = expected.get(next).filter(|s| s.onset_s < end_s + AHEAD_S) {
            tracker.expect(strike);
            next += 1;
        }
        let found = tracker.push(block);
        told.add(found, end_s);
        told.revise(tracker.revised(), tracker.pitch_revised());
        told.strings.extend_from_slice(tracker.string_verdicts());
        told.double_stops.extend_from_slice(tracker.double_stops());
    }
    let found = tracker.flush();
    told.add(found, samples.len() as f64 / sample_rate);
    told.revise(tracker.revised(), tracker.pitch_revised());
    told.strings.extend_from_slice(tracker.string_verdicts());
    told.double_stops.extend_from_slice(tracker.double_stops());
    told
}

/// How far ahead `analyze_reported` tells expected strikes.
const AHEAD_S: f64 = 0.25;

impl Reported {
    fn add(&mut self, found: &[DetectedNote], at_s: f64) {
        self.notes.extend_from_slice(found);
        self.at_s.resize(self.notes.len(), at_s);
        self.first_midi.extend(found.iter().map(|n| n.midi));
    }

    fn revise(&mut self, techniques: &[TechniqueRevision], pitches: &[PitchRevision]) {
        revise(&mut self.notes, techniques);
        revise_pitch(&mut self.notes, pitches);
    }
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
