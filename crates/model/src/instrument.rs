//! Instrument profiles. Until 1.0 only basses; the type is open for more: string instruments
//! (a tuning and frets) and keyboards (a key range, no strings).
//!
//! ```text
//! Instrument ─┬─ strings: tuning (1–12 strings) + frets ─► tab, fretboard, positions
//!             └─ keys: lowest..=highest key, no strings ─► notes only, no tab
//!   range() and detection_range_hz() come from whichever the profile has.
//! ```

use crate::pitch::{Pitch, A4_HZ};
use crate::tuning::Tuning;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum InstrumentError {
    #[error("tuning has {0} strings, supported are 1 to 12")]
    StringCount(usize),
    #[error("key range {0}..{1} is empty")]
    Keys(u8, u8),
    #[error("fret count {0} out of range 1..=36")]
    Frets(u8),
}

/// What kind of instrument, which decides scoring defaults (fretless and double bass score
/// intonation). Until 1.0 these are basses; guitars and keyboards add kinds here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstrumentKind {
    /// Electric bass with frets.
    Fretted,
    /// Electric bass without frets: intonation in cents matters.
    Fretless,
    /// Acoustic double bass (phase 4, microphone).
    Double,
    /// Bass lines played on a MIDI keyboard (phase 4).
    Keys,
}

/// Corner frequencies of the monitor tone bands (LH-EIN-08), chosen per instrument so that
/// "low", "mid" and "high" mean the same to the player on every instrument.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ToneBands {
    /// Low shelf: below this the instrument's body.
    pub low_hz: f64,
    /// Mid bell: the growl or honk of the instrument.
    pub mid_hz: f64,
    /// High shelf: string noise and attack.
    pub high_hz: f64,
}

impl ToneBands {
    /// Bass: lowest strings' fundamental, growl around 800 Hz, string attack.
    pub const BASS: ToneBands = ToneBands {
        low_hz: 100.0,
        mid_hz: 800.0,
        high_hz: 4000.0,
    };
}

/// The keys of a keyboard profile, lowest and highest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyRange {
    pub low: Pitch,
    pub high: Pitch,
}

/// log10 B of open bass strings by their MIDI pitch (`Instrument::string_log_b`).
const OPEN_LOG_B: [(u8, f64); 4] = [(28, -3.93), (33, -4.102), (38, -4.371), (43, -4.53)];

/// Fewest and most strings of a string profile (ukulele to twelve-string).
const STRINGS: std::ops::RangeInclusive<usize> = 1..=12;

/// An instrument profile: kind, strings (tuning and frets) or keys, and reference pitch.
/// A keyboard has an empty tuning and `keys`; a string instrument has no `keys`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Instrument {
    pub kind: InstrumentKind,
    pub tuning: Tuning,
    pub frets: u8,
    pub a4_hz: f64,
    #[serde(default)]
    pub keys: Option<KeyRange>,
}

impl Instrument {
    pub fn new(
        kind: InstrumentKind,
        tuning: Tuning,
        frets: u8,
    ) -> Result<Instrument, InstrumentError> {
        let strings = tuning.string_count();
        if !STRINGS.contains(&strings) {
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
            keys: None,
        })
    }

    /// A keyboard with keys `low..=high` (no strings, no tab).
    pub fn keyboard(
        kind: InstrumentKind,
        low: Pitch,
        high: Pitch,
    ) -> Result<Instrument, InstrumentError> {
        if low > high {
            return Err(InstrumentError::Keys(low.0, high.0));
        }
        Ok(Instrument {
            kind,
            tuning: Tuning {
                name: "keys".into(),
                strings: Vec::new(),
            },
            frets: 0,
            a4_hz: A4_HZ,
            keys: Some(KeyRange { low, high }),
        })
    }

    /// True for string instruments (tab, fretboard), false for keyboards.
    pub fn has_strings(&self) -> bool {
        self.keys.is_none() && !self.tuning.strings.is_empty()
    }

    /// Four-string fretted bass in standard tuning, 20 frets.
    pub fn default_bass() -> Instrument {
        Instrument {
            kind: InstrumentKind::Fretted,
            tuning: Tuning::bass4_standard(),
            frets: 20,
            a4_hz: A4_HZ,
            keys: None,
        }
    }

    /// Tone bands for monitoring; a guitar or keyboard profile brings its own.
    pub fn tone_bands(&self) -> ToneBands {
        ToneBands::BASS
    }

    /// Lowest and highest playable pitch.
    pub fn range(&self) -> Option<(Pitch, Pitch)> {
        if let Some(keys) = self.keys {
            return Some((keys.low, keys.high));
        }
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

    /// log10 of the inharmonicity B of an open string (R7, string detection), where known:
    /// fretted basses with the usual open pitches E1, A1, D2, G2 (so also the four upper
    /// strings of a five-string). Measured on three basses of IDMT-SMT-Bass (median of
    /// `kontra-cli korpus strings`); other strings (a low B, a drop tuning) and fretless have
    /// none yet. Example: −3.93 on the open E puts partial 10 about 0.6 % sharp.
    pub fn string_log_b(&self, string: u8) -> Option<f64> {
        if self.kind != InstrumentKind::Fretted {
            return None;
        }
        let open = self.tuning.strings.get(usize::from(string))?;
        OPEN_LOG_B
            .iter()
            .find(|(pitch, _)| *pitch == open.0)
            .map(|(_, log_b)| *log_b)
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
    fn a_keyboard_has_a_range_but_no_strings() {
        let keys = Instrument::keyboard(InstrumentKind::Keys, Pitch(28), Pitch(67)).unwrap();
        assert!(!keys.has_strings());
        assert_eq!(keys.range(), Some((Pitch(28), Pitch(67))));
        assert!(keys.positions(Pitch(40)).is_empty());
        assert!(keys.detection_range_hz().is_some());
        assert!(Instrument::keyboard(InstrumentKind::Keys, Pitch(60), Pitch(40)).is_err());
        assert!(Instrument::default_bass().has_strings());
    }

    #[test]
    fn string_counts_from_one_to_twelve() {
        let ukulele = Tuning::from_names("ukulele", &["G4", "C4", "E4", "A4"]).unwrap();
        assert!(Instrument::new(InstrumentKind::Fretted, ukulele, 15).is_ok());
        let none = Tuning {
            name: "none".into(),
            strings: Vec::new(),
        };
        assert_eq!(
            Instrument::new(InstrumentKind::Fretted, none, 20),
            Err(InstrumentError::StringCount(0))
        );
    }

    #[test]
    fn positions_of_a1_on_four_string() {
        let bass = Instrument::default_bass();
        let a1 = Pitch::parse("A1").unwrap();
        assert_eq!(bass.positions(a1), vec![(0, 5), (1, 0)]);
    }

    #[test]
    fn range_and_detection() {
        let bass = Instrument::new(InstrumentKind::Fretless, Tuning::bass5_standard(), 24).unwrap();
        let (low, high) = bass.range().unwrap();
        assert_eq!(low.to_string(), "B0");
        assert_eq!(high.to_string(), "G4");
        let (lo_hz, _) = bass.detection_range_hz().unwrap();
        assert!(lo_hz < 30.0);
    }

    #[test]
    fn validates() {
        let names = ["E1"; 13];
        let t = Tuning::from_names("x", &names).unwrap();
        assert_eq!(
            Instrument::new(InstrumentKind::Fretted, t, 20),
            Err(InstrumentError::StringCount(13))
        );
        let t = Tuning::from_names("x", &["E1", "A1"]).unwrap();
        assert_eq!(
            Instrument::new(InstrumentKind::Fretted, t, 0),
            Err(InstrumentError::Frets(0))
        );
    }
}
