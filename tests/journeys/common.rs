//! Shared helpers: temporary profiles, scores written as MIDI, recordings rendered on the fly
//! (no binary test material is committed) and a loop that ticks the core like the UI does.

use kontra_appcore::{Core, QaOptions, UiEvent};
use kontra_model::{Arrangement, Instrument, Note, Pitch, TempoMap, TICKS_PER_QUARTER};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const RATE: u32 = 48_000;
/// Tempo of every generated piece; one 4/4 count-in bar lasts 2 s.
pub const BPM: f64 = 120.0;
pub const COUNT_IN_S: f64 = 2.0;
/// The UI polls the core about every 16 ms.
const TICK: Duration = Duration::from_millis(10);

/// A fresh data directory per test, removed when dropped.
#[derive(Debug)]
pub struct Profile {
    pub dir: PathBuf,
}

impl Profile {
    pub fn new(name: &str) -> Profile {
        let dir =
            std::env::temp_dir().join(format!("kontra-journey-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Profile { dir }
    }

    /// A core on this profile, optionally fed by a recording instead of a device.
    pub fn core(&self, take: Option<&Path>) -> Core {
        let qa = QaOptions {
            input_file: take.map(Path::to_path_buf),
            laptop_jack: false,
        };
        let mut core = Core::new(&self.dir, qa);
        core.update_settings(|s| s.language = Some("de".into()));
        core
    }
}

impl Drop for Profile {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Quarter notes at `BPM` (4/4), one per MIDI number.
pub fn piece(title: &str, midis: &[u8]) -> Arrangement {
    let mut a = Arrangement::new(title, Instrument::default_bass());
    a.tempo = TempoMap::constant(BPM);
    for (i, midi) in midis.iter().enumerate() {
        a.notes.push(Note::new(
            i as u64 * TICKS_PER_QUARTER,
            TICKS_PER_QUARTER,
            Pitch(*midi),
        ));
    }
    a.normalize();
    a
}

/// Writes `notes` as a type-0 MIDI file with a bass program and track name.
///
/// ```text
/// MThd (format 0, 1 track, 480 ticks/quarter) │ MTrk: name, tempo, 4/4, program 33, notes, end
/// ```
pub fn write_midi(path: &Path, title: &str, midis: &[u8]) {
    const DIVISION: u16 = 480;
    let mut track = Vec::new();
    let mut meta = |kind: u8, data: &[u8]| {
        track.extend([0, 0xff, kind, data.len() as u8]);
        track.extend(data);
    };
    meta(0x03, title.as_bytes());
    let us = (60_000_000.0 / BPM) as u32;
    meta(0x51, &us.to_be_bytes()[1..]);
    meta(0x58, &[4, 2, 24, 8]);
    track.extend([0, 0xc0, 33]);
    for midi in midis {
        track.extend([0, 0x90, *midi, 100]);
        track.extend(varlen(u32::from(DIVISION)));
        track.extend([0x80, *midi, 0]);
    }
    track.extend([0, 0xff, 0x2f, 0]);
    let mut file = b"MThd".to_vec();
    file.extend(6u32.to_be_bytes());
    file.extend([0, 0, 0, 1]);
    file.extend(DIVISION.to_be_bytes());
    file.extend(b"MTrk");
    file.extend((track.len() as u32).to_be_bytes());
    file.extend(track);
    std::fs::write(path, file).unwrap();
}

fn varlen(mut value: u32) -> Vec<u8> {
    let mut out = vec![(value & 0x7f) as u8];
    value >>= 7;
    while value > 0 {
        out.insert(0, (value & 0x7f) as u8 | 0x80);
        value >>= 7;
    }
    out
}

/// Renders `a` as played after the count-in, with `mistakes`, and writes it as WAV.
pub fn render(path: &Path, a: &Arrangement, mistakes: kontra_session::Mistakes) {
    let samples = kontra_session::render_performance(a, 1.0, COUNT_IN_S, mistakes, RATE);
    kontra_session::write_mono_wav(path, &samples, RATE).unwrap();
}

/// Ticks `core` until `done` returns true for an event or `limit` passes; returns all events.
pub fn run_until(
    core: &mut Core,
    limit: Duration,
    done: impl Fn(&UiEvent) -> bool,
) -> Vec<UiEvent> {
    let end = Instant::now() + limit;
    let mut events = Vec::new();
    while Instant::now() < end {
        let batch = core.tick();
        let finished = batch.iter().any(&done);
        events.extend(batch);
        if finished {
            break;
        }
        std::thread::sleep(TICK);
    }
    events
}

/// States of all judged notes, in event order.
pub fn judged(events: &[UiEvent]) -> Vec<&'static str> {
    events
        .iter()
        .filter_map(|e| match e {
            UiEvent::Judged { state, .. } => Some(*state),
            _ => None,
        })
        .collect()
}
