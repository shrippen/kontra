//! Instrument profiles. Until 1.0 only basses; the type is open for more.

use crate::pitch::{Pitch, A4_HZ};
use crate::tuning::Tuning;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum InstrumentError {
    #[error("tuning has {0} strings, supported are 4 to 6")]
    StringCount(usize),
    #[error("fret count {0} out of range 1..=36")]
    Frets(u8),
}

/// What kind of bass, which decides detection range and scoring defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BassKind {
    /// Electric bass with frets.
    Fretted,
    /// Electric bass without frets: intonation in cents matters.
    Fretless,
    /// Acoustic double bass (phase 4, microphone).
    Double,
    /// Bass lines played on a MIDI keyboard (phase 4).
    Keys,
}

/// An instrument profile: kind, tuning, frets and reference pitch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Instrument {
    pub kind: BassKind,
    pub tuning: Tuning,
    pub frets: u8,
    pub a4_hz: f64,
}

impl Instrument {
    pub fn new(kind: BassKind, tuning: Tuning, frets: u8) -> Result<Instrument, InstrumentError> {
        let strings = tuning.string_count();
        if !(4..=6).contains(&strings) {
            return Err(InstrumentError::StringCount(strings));
        }
        if !(1..=36).contains(&frets) {
            return Err(InstrumentError::Frets(frets));
        }
        Ok(Instrument {
            kind,
            tuning,
            frets,
            a4_hz: A4_HZ,
        })
    }

    /// Four-string fretted bass in standard tuning, 20 frets.
    pub fn default_bass() -> Instrument {
        Instrument {
            kind: BassKind::Fretted,
            tuning: Tuning::bass4_standard(),
            frets: 20,
            a4_hz: A4_HZ,
        }
    }

    /// Lowest and highest playable pitch.
    pub fn range(&self) -> Option<(Pitch, Pitch)> {
        let low = self.tuning.lowest()?;
        let high = self.tuning.highest()?.transposed(i32::from(self.frets))?;
        Some((low, high))
    }

    /// Frequency range for detection with one semitone headroom below and above.
    pub fn detection_range_hz(&self) -> Option<(f64, f64)> {
        let (low, high) = self.range()?;
        let low = low.transposed(-2).unwrap_or(low);
        let high = high.transposed(1).unwrap_or(high);
        Some((low.hz(self.a4_hz), high.hz(self.a4_hz)))
    }

    /// All (string, fret) positions that produce `pitch`; string 0 is the lowest.
    pub fn positions(&self, pitch: Pitch) -> Vec<(u8, u8)> {
        let mut found = Vec::new();
        for (string, open) in self.tuning.strings.iter().enumerate() {
            let fret = i32::from(pitch.0) - i32::from(open.0);
            if fret < 0 || fret > i32::from(self.frets) {
                continue;
            }
            found.push((string as u8, fret as u8));
        }
        found
    }

    /// Pitch at a position, `None` if string or fret are out of range.
    pub fn pitch_at(&self, string: u8, fret: u8) -> Option<Pitch> {
        if fret > self.frets {
            return None;
        }
        self.tuning
            .strings
            .get(usize::from(string))?
            .transposed(i32::from(fret))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_of_a1_on_four_string() {
        let bass = Instrument::default_bass();
        let a1 = Pitch::parse("A1").unwrap();
        assert_eq!(bass.positions(a1), vec![(0, 5), (1, 0)]);
    }

    #[test]
    fn range_and_detection() {
        let bass = Instrument::new(BassKind::Fretless, Tuning::bass5_standard(), 24).unwrap();
        let (low, high) = bass.range().unwrap();
        assert_eq!(low.to_string(), "B0");
        assert_eq!(high.to_string(), "G4");
        let (lo_hz, _) = bass.detection_range_hz().unwrap();
        assert!(lo_hz < 30.0);
    }

    #[test]
    fn validates() {
        let t = Tuning::from_names("x", &["E1", "A1"]).unwrap();
        assert_eq!(
            Instrument::new(BassKind::Fretted, t, 20),
            Err(InstrumentError::StringCount(2))
        );
    }
}
