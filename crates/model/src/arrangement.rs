//! An arrangement: one bass part on a linear timeline (repeats already unrolled by the importer).

use crate::instrument::Instrument;
use crate::pitch::Pitch;
use crate::tempo::TempoMap;
use serde::{Deserialize, Serialize};

/// Resolution of the timeline: ticks per quarter note.
pub const TICKS_PER_QUARTER: u64 = 960;

/// Playing techniques that change what the score shows and, from phase 4, what is detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Technique {
    Slide,
    HammerOn,
    PullOff,
    Slap,
    Pop,
    Ghost,
    Harmonic,
    Muted,
    Vibrato,
    Bend,
    Staccato,
    Accent,
}

/// One note to play.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Note {
    pub start_ticks: u64,
    pub duration_ticks: u64,
    pub pitch: Pitch,
    /// String index, 0 = lowest string; `None` until the tab generator assigns one.
    pub string: Option<u8>,
    pub fret: Option<u8>,
    /// Continues the previous note (tie): no new attack expected.
    pub tied: bool,
    pub techniques: Vec<Technique>,
}

impl Note {
    pub fn new(start_ticks: u64, duration_ticks: u64, pitch: Pitch) -> Note {
        Note {
            start_ticks,
            duration_ticks,
            pitch,
            string: None,
            fret: None,
            tied: false,
            techniques: Vec::new(),
        }
    }

    pub fn end_ticks(&self) -> u64 {
        self.start_ticks + self.duration_ticks
    }

    /// True if the player has to strike this note (not tied, not a hammer-on/pull-off target).
    pub fn needs_attack(&self) -> bool {
        !self.tied
            && !self
                .techniques
                .iter()
                .any(|t| matches!(t, Technique::HammerOn | Technique::PullOff))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeSignature {
    pub numerator: u8,
    pub denominator: u8,
}

impl TimeSignature {
    pub const COMMON: TimeSignature = TimeSignature {
        numerator: 4,
        denominator: 4,
    };

    pub fn bar_ticks(self) -> u64 {
        let denominator = u64::from(self.denominator.max(1));
        u64::from(self.numerator) * TICKS_PER_QUARTER * 4 / denominator
    }
}

/// One bar on the linear timeline. `source_index` points to the bar in the original score
/// (before repeats were unrolled), so the UI can show "bar 12 (2nd time)".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bar {
    pub start_ticks: u64,
    pub signature: TimeSignature,
    pub source_index: u32,
    /// Key signature in effect (concert pitch): 1..7 sharps, -1..-7 flats, 0 = C major.
    #[serde(default)]
    pub key_fifths: i8,
}

impl Bar {
    pub fn end_ticks(&self) -> u64 {
        self.start_ticks + self.signature.bar_ticks()
    }
}

/// Named position, e.g. "Chorus" or a rehearsal mark.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Marker {
    pub tick: u64,
    pub name: String,
}

/// A bass part ready to be played and scored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Arrangement {
    pub title: String,
    pub artist: String,
    pub instrument: Instrument,
    pub tempo: TempoMap,
    pub bars: Vec<Bar>,
    pub markers: Vec<Marker>,
    /// Sorted by `start_ticks`.
    pub notes: Vec<Note>,
}

impl Arrangement {
    pub fn new(title: &str, instrument: Instrument) -> Arrangement {
        Arrangement {
            title: title.to_string(),
            artist: String::new(),
            instrument,
            tempo: TempoMap::default(),
            bars: Vec::new(),
            markers: Vec::new(),
            notes: Vec::new(),
        }
    }

    /// Sorts notes and fills missing bars in 4/4 up to the last note.
    pub fn normalize(&mut self) {
        self.notes.sort_by_key(|n| (n.start_ticks, n.pitch));
        let end = self.end_ticks();
        let mut next = self.bars.last().map_or(0, Bar::end_ticks);
        let signature = self
            .bars
            .last()
            .map_or(TimeSignature::COMMON, |b| b.signature);
        let key_fifths = self.bars.last().map_or(0, |b| b.key_fifths);
        let mut index = self.bars.last().map_or(0, |b| b.source_index + 1);
        while next < end {
            self.bars.push(Bar {
                start_ticks: next,
                signature,
                source_index: index,
                key_fifths,
            });
            next += signature.bar_ticks();
            index += 1;
        }
    }

    pub fn end_ticks(&self) -> u64 {
        let notes = self.notes.iter().map(Note::end_ticks).max().unwrap_or(0);
        let bars = self.bars.last().map_or(0, Bar::end_ticks);
        notes.max(bars)
    }

    pub fn duration_s(&self) -> f64 {
        self.tempo.seconds_at(self.end_ticks())
    }

    /// Index of the bar containing `tick`.
    pub fn bar_at(&self, tick: u64) -> Option<usize> {
        let index = self.bars.partition_point(|b| b.start_ticks <= tick);
        index.checked_sub(1)
    }

    /// Notes starting in `[from, to)`.
    pub fn notes_between(&self, from: u64, to: u64) -> &[Note] {
        let start = self.notes.partition_point(|n| n.start_ticks < from);
        let end = self.notes.partition_point(|n| n.start_ticks < to);
        &self.notes[start..end.max(start)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Arrangement {
        let mut a = Arrangement::new("Test", Instrument::default_bass());
        let e1 = Pitch::parse("E1").unwrap();
        for i in 0..8 {
            a.notes
                .push(Note::new(i * TICKS_PER_QUARTER, TICKS_PER_QUARTER, e1));
        }
        a.normalize();
        a
    }

    #[test]
    fn normalize_adds_bars() {
        let a = sample();
        assert_eq!(a.bars.len(), 2);
        assert_eq!(a.bar_at(TICKS_PER_QUARTER * 5), Some(1));
        assert!((a.duration_s() - 4.0).abs() < 1e-9);
    }

    #[test]
    fn notes_between_is_half_open() {
        let a = sample();
        assert_eq!(a.notes_between(0, TICKS_PER_QUARTER * 2).len(), 2);
        assert_eq!(a.notes_between(TICKS_PER_QUARTER * 7, u64::MAX).len(), 1);
    }

    #[test]
    fn time_signature_ticks() {
        assert_eq!(
            TimeSignature {
                numerator: 6,
                denominator: 8
            }
            .bar_ticks(),
            3 * TICKS_PER_QUARTER
        );
    }

    #[test]
    fn json_roundtrip() {
        let a = sample();
        let text = serde_json::to_string(&a).unwrap();
        let back: Arrangement = serde_json::from_str(&text).unwrap();
        assert_eq!(a, back);
    }
}
