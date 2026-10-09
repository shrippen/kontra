//! Format-neutral score every parser produces: bars as written (repeats not yet unrolled),
//! one note list per track and bar. `build` turns it into a linear `Arrangement`.
//!
//! ```text
//! .gp3-5 ─┐
//! .gpx/.gp┤
//! .mid ───┼─→ Score ─→ build (unroll, tempo, ties, legato) ─→ Arrangement ─→ assign_tab
//! .musicxml┘
//! ```

use kontra_model::{Pitch, Technique, TimeSignature};

/// How a note connects to its neighbour on the same string without a new attack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Legato {
    None,
    /// The next note on this string is hammered on or pulled off (Guitar Pro 3-5 style).
    Origin,
    /// This note is hammered on or pulled off (GPIF and MusicXML style).
    Target,
}

/// One note inside a bar, positioned relative to the bar start.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RawNote {
    pub(crate) offset: u64,
    pub(crate) duration: u64,
    pub(crate) pitch: Pitch,
    /// 0 = lowest string.
    pub(crate) string: Option<u8>,
    pub(crate) fret: Option<u8>,
    pub(crate) tied: bool,
    pub(crate) legato: Legato,
    pub(crate) techniques: Vec<Technique>,
}

impl RawNote {
    pub(crate) fn new(offset: u64, duration: u64, pitch: Pitch) -> RawNote {
        RawNote {
            offset,
            duration,
            pitch,
            string: None,
            fret: None,
            tied: false,
            legato: Legato::None,
            techniques: Vec::new(),
        }
    }

    pub(crate) fn add(&mut self, technique: Technique) {
        if !self.techniques.contains(&technique) {
            self.techniques.push(technique);
        }
    }
}

/// Bar properties shared by all tracks (Guitar Pro "measure header", GPIF "MasterBar").
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BarHead {
    pub(crate) signature: TimeSignature,
    pub(crate) repeat_open: bool,
    /// Total number of plays when this bar closes a repeat; 0 = no repeat close.
    pub(crate) repeat_plays: u8,
    /// Alternate endings: bit 0 = played on the first pass, bit 1 on the second, ...; 0 = always.
    pub(crate) endings: u32,
    pub(crate) marker: Option<String>,
    /// Tempo changes as (offset in ticks from the bar start, quarter notes per minute).
    pub(crate) tempos: Vec<(u64, f64)>,
}

impl BarHead {
    pub(crate) fn new(signature: TimeSignature) -> BarHead {
        BarHead {
            signature,
            repeat_open: false,
            repeat_plays: 0,
            endings: 0,
            marker: None,
            tempos: Vec::new(),
        }
    }

    /// Adds a tempo change unless one already exists at that offset (all tracks repeat it).
    pub(crate) fn add_tempo(&mut self, offset: u64, bpm: f64) {
        if !bpm.is_finite() || bpm <= 0.0 {
            return;
        }
        if self.tempos.iter().any(|(at, _)| *at == offset) {
            return;
        }
        self.tempos.push((offset, bpm));
    }
}

/// One instrument part.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Part {
    pub(crate) name: String,
    /// Open strings, lowest first; empty when the format has no tab information.
    pub(crate) tuning: Vec<Pitch>,
    pub(crate) frets: u8,
    /// General MIDI program 0..=127, if known.
    pub(crate) program: Option<u8>,
    pub(crate) percussion: bool,
    /// Format-specific hint that this is a bass (e.g. GP7 instrument type `electricBass`).
    pub(crate) bass_hint: bool,
    /// Notes per bar, indexed like `Score::bars`; may be shorter.
    pub(crate) bars: Vec<Vec<RawNote>>,
}

impl Part {
    pub(crate) fn new(name: &str) -> Part {
        Part {
            name: name.to_string(),
            tuning: Vec::new(),
            frets: DEFAULT_FRETS,
            program: None,
            percussion: false,
            bass_hint: false,
            bars: Vec::new(),
        }
    }

    /// Mutable note list of bar `index`, growing the list as needed.
    pub(crate) fn bar_mut(&mut self, index: usize) -> &mut Vec<RawNote> {
        if self.bars.len() <= index {
            self.bars.resize_with(index + 1, Vec::new);
        }
        &mut self.bars[index]
    }

    pub(crate) fn notes(&self) -> impl Iterator<Item = &RawNote> {
        self.bars.iter().flatten()
    }
}

/// Frets assumed when a file does not say.
pub(crate) const DEFAULT_FRETS: u8 = 24;

/// A whole imported song before unrolling.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Score {
    pub(crate) title: String,
    pub(crate) artist: String,
    /// Tempo at the start, quarter notes per minute.
    pub(crate) bpm: f64,
    pub(crate) bars: Vec<BarHead>,
    pub(crate) parts: Vec<Part>,
}

impl Score {
    pub(crate) fn new() -> Score {
        Score {
            title: String::new(),
            artist: String::new(),
            bpm: DEFAULT_BPM,
            bars: Vec::new(),
            parts: Vec::new(),
        }
    }
}

/// Tempo assumed when a file does not say (MIDI default).
pub(crate) const DEFAULT_BPM: f64 = 120.0;

/// Whole note in ticks (4 quarters of 960).
pub(crate) const WHOLE_TICKS: u64 = kontra_model::TICKS_PER_QUARTER * 4;
