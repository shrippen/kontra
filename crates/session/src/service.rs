//! One-shot operations for user interfaces (CLI, app): import, analysis, offline scoring,
//! corpus checks, device lists. UIs call these instead of reaching into lower layers.

use crate::session::{Mode, Session};
use crate::settings::ScoreSettings;
use crate::summary::Summary;
use crate::NoteResult;
use kontra_engine::{Clock, EngineError, Event, Input, InputKind, Pace};
use kontra_model::{Arrangement, DetectedNote, InputSource, Instrument};
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;

pub use kontra_content::exercise;
pub use kontra_content::{
    assign_tab, import_file, read_package, transpose, write_package, FileKind, ImportError,
    ImportResult, Manifest, Package, PackageBacking, TrackInfo, PACKAGE_FORMAT, PACKAGE_VERSION,
    TRANSPOSE_MAX,
};
pub use kontra_engine::{
    audio_priority, evaluate as evaluate_korpus,
    evaluate_laptop_jack as evaluate_korpus_laptop_jack, evaluate_with as evaluate_korpus_with,
    generate as generate_korpus, list_devices, list_midi_inputs, render_performance, same_pitch,
    track_beats, Annotation, BackingTrack, BeatGrid, Connection, DeviceInfo, Direction, Evaluation,
    KorpusItem, KorpusSet, Matching, MidiPort, Mistakes, Playing, Priority, StreamConfig,
    NULL_DEVICE,
};

/// Writes mono audio as WAV (corpus export).
pub fn write_mono_wav(path: &Path, samples: &[f32], sample_rate: u32) -> Result<(), EngineError> {
    kontra_engine::write_wav(
        path,
        &kontra_engine::AudioBuffer {
            sample_rate,
            channels: vec![samples.to_vec()],
        },
    )?;
    Ok(())
}

/// Reads a corpus set written by `write_mono_wav` plus its JSON annotation.
pub fn read_korpus(dir: &Path, set: KorpusSet) -> Result<KorpusItem, String> {
    let json = dir.join(format!("{}.json", set.name()));
    let text = std::fs::read_to_string(&json).map_err(|e| format!("{}: {e}", json.display()))?;
    let mut item: KorpusItem =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", json.display()))?;
    let wav = dir.join(format!("{}.wav", set.name()));
    let audio = kontra_engine::read_audio(&wav).map_err(|e| e.to_string())?;
    item.sample_rate = audio.sample_rate;
    item.samples = audio.mono();
    Ok(item)
}

/// Reads one corpus item from `json` and the WAV of the same name next to it (any name, for
/// real recordings converted by `scripts/korpus-import.py`).
pub fn read_korpus_file(json: &Path) -> Result<KorpusItem, String> {
    let text = std::fs::read_to_string(json).map_err(|e| format!("{}: {e}", json.display()))?;
    let mut item: KorpusItem =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", json.display()))?;
    let wav = json.with_extension("wav");
    let audio = kontra_engine::read_audio(&wav).map_err(|e| e.to_string())?;
    item.sample_rate = audio.sample_rate;
    item.samples = audio.mono();
    Ok(item)
}

/// Detects all notes in an audio file (as fast as possible).
pub fn analyze_file(
    path: &Path,
    instrument: &Instrument,
    source: InputSource,
    laptop_jack: bool,
) -> Result<Vec<DetectedNote>, EngineError> {
    let clock = Clock::new();
    let (tx, rx) = channel();
    let kind = InputKind::File {
        path: PathBuf::from(path),
        pace: Pace::Fast,
        laptop_jack,
    };
    let input = Input::start(&kind, instrument, source, &clock, tx, None)?;
    let mut notes = Vec::new();
    for event in rx.iter() {
        match event {
            Event::Note(n) => notes.push(n),
            Event::Technique(r) => kontra_engine::revise(&mut notes, &[r]),
            Event::Pitch(r) => kontra_engine::revise_pitch(&mut notes, &[r]),
            Event::Ended => break,
            _ => {}
        }
    }
    drop(input);
    Ok(notes)
}

/// Result of scoring a recording against an arrangement.
#[derive(Debug, Clone, PartialEq)]
pub struct Scored {
    pub summary: Summary,
    pub results: Vec<NoteResult>,
    pub detected: Vec<DetectedNote>,
}

/// Scores a recording made while the arrangement played at `speed`, with arrangement time 0 at
/// `offset_s` seconds into the recording.
pub fn score_notes(
    arrangement: &Arrangement,
    detected: &[DetectedNote],
    mode: Mode,
    settings: ScoreSettings,
    speed: f64,
    offset_s: f64,
) -> Scored {
    let mut session = Session::new(arrangement.clone(), mode, settings);
    session.set_speed(0.0, speed);
    session.play(offset_s);
    let end = offset_s + arrangement.duration_s() / speed + 1.0;
    let mut pending: Vec<&DetectedNote> = detected.iter().collect();
    pending.sort_by(|a, b| a.onset_s.total_cmp(&b.onset_s));
    let mut next = 0;
    let mut t = 0.0;
    while t <= end {
        while next < pending.len() && pending[next].onset_s <= t {
            session.on_note(pending[next]);
            next += 1;
        }
        session.update(t);
        t += 0.01;
    }
    Scored {
        summary: session.summary(),
        results: session.results().to_vec(),
        detected: detected.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kontra_engine::{generate, KorpusSet};
    use kontra_model::{Note, Pitch, TempoMap, TICKS_PER_QUARTER};

    #[test]
    fn scores_generated_corpus_as_perfect() {
        // The sixteenths corpus as an arrangement: every annotated note at its time.
        let item = generate(KorpusSet::Sixteenths);
        let mut a = Arrangement::new("Korpus", KorpusSet::Sixteenths.instrument());
        a.tempo = TempoMap::constant(60.0);
        for note in &item.notes {
            let tick = (note.onset_s * TICKS_PER_QUARTER as f64).round() as u64;
            a.notes.push(Note::new(
                tick,
                TICKS_PER_QUARTER / 8,
                Pitch(note.midi.round() as u8),
            ));
        }
        a.normalize();
        let detected = kontra_engine::analyze(
            &item.samples,
            f64::from(item.sample_rate),
            &a.instrument,
            InputSource::Line,
        );
        let scored = score_notes(
            &a,
            &detected,
            Mode::Song,
            ScoreSettings::default(),
            1.0,
            0.0,
        );
        assert_eq!(scored.summary.judged as usize, item.notes.len());
        assert_eq!(scored.summary.hits, scored.summary.judged);
        assert!(scored.summary.mean_offset_ms.unwrap().abs() < 3.0);
    }
}
