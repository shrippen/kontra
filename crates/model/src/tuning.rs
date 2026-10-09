//! String tunings, lowest string first.

use crate::pitch::Pitch;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TuningError {
    #[error("a tuning needs at least one string")]
    Empty,
    #[error("unknown note name: {0}")]
    BadNote(String),
}

/// Open-string pitches from the lowest (thickest) to the highest string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tuning {
    pub name: String,
    pub strings: Vec<Pitch>,
}

impl Tuning {
    /// Builds a tuning from note names, e.g. `["E1", "A1", "D2", "G2"]`.
    pub fn from_names(name: &str, notes: &[&str]) -> Result<Tuning, TuningError> {
        if notes.is_empty() {
            return Err(TuningError::Empty);
        }
        let strings = notes
            .iter()
            .map(|n| Pitch::parse(n).ok_or_else(|| TuningError::BadNote((*n).to_string())))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Tuning {
            name: name.to_string(),
            strings,
        })
    }

    fn preset(name: &str, notes: &[&str]) -> Tuning {
        // Presets are constant and covered by tests; an empty fallback keeps this panic-free.
        Tuning::from_names(name, notes).unwrap_or(Tuning {
            name: name.to_string(),
            strings: Vec::new(),
        })
    }

    /// E1 A1 D2 G2.
    pub fn bass4_standard() -> Tuning {
        Tuning::preset("standard-4", &["E1", "A1", "D2", "G2"])
    }

    /// B0 E1 A1 D2 G2.
    pub fn bass5_standard() -> Tuning {
        Tuning::preset("standard-5", &["B0", "E1", "A1", "D2", "G2"])
    }

    /// B0 E1 A1 D2 G2 C3.
    pub fn bass6_standard() -> Tuning {
        Tuning::preset("standard-6", &["B0", "E1", "A1", "D2", "G2", "C3"])
    }

    /// D1 A1 D2 G2.
    pub fn bass4_drop_d() -> Tuning {
        Tuning::preset("drop-d", &["D1", "A1", "D2", "G2"])
    }

    /// Every string lowered by `semitones` (e.g. -1 for half step down).
    pub fn shifted(&self, name: &str, semitones: i32) -> Option<Tuning> {
        let strings = self
            .strings
            .iter()
            .map(|p| p.transposed(semitones))
            .collect::<Option<Vec<_>>>()?;
        Some(Tuning {
            name: name.to_string(),
            strings,
        })
    }

    pub fn string_count(&self) -> usize {
        self.strings.len()
    }

    pub fn lowest(&self) -> Option<Pitch> {
        self.strings.first().copied()
    }

    pub fn highest(&self) -> Option<Pitch> {
        self.strings.last().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_are_valid() {
        assert_eq!(Tuning::bass4_standard().string_count(), 4);
        assert_eq!(Tuning::bass5_standard().lowest().unwrap().to_string(), "B0");
        assert_eq!(
            Tuning::bass6_standard().highest().unwrap().to_string(),
            "C3"
        );
        assert_eq!(Tuning::bass4_drop_d().lowest().unwrap().to_string(), "D1");
    }

    #[test]
    fn half_step_down() {
        let t = Tuning::bass4_standard().shifted("half-down", -1).unwrap();
        assert_eq!(t.lowest().unwrap().to_string(), "D#1");
    }

    #[test]
    fn rejects_bad_names() {
        assert_eq!(Tuning::from_names("x", &[]), Err(TuningError::Empty));
        assert!(Tuning::from_names("x", &["Q1"]).is_err());
    }
}
