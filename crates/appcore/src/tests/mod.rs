//! Tests of `Core`, one file per area.

use super::*;

mod coaching;
mod exercises;
mod library;
mod notation;
mod note_list;
mod playback;
mod settings;

fn demo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/demo/etude-1.mid")
}

/// A fresh data folder whose settings play to the engine's timer, never to the speakers of
/// whoever runs the tests (QA 14.10.: test runs clicked on the default output).
fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kontra-core-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);
    // The engine's NULL_DEVICE: a timer instead of a sound card.
    let silent = serde_json::json!({ "output_device": "kontra:null" });
    let _ = std::fs::write(dir.join("settings.json"), silent.to_string());
    dir
}

const Q: u64 = kontra_model::TICKS_PER_QUARTER;

/// One bar of 4/4 or more, notes as (start, duration, MIDI) in ticks.
fn arrangement(notes: &[(u64, u64, u8)]) -> Arrangement {
    let mut a = Arrangement::new("Staff", kontra_model::Instrument::default_bass());
    for &(start, duration, midi) in notes {
        a.notes.push(kontra_model::Note::new(
            start,
            duration,
            kontra_model::Pitch(midi),
        ));
    }
    a.normalize();
    a
}
