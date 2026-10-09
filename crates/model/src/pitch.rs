//! Pitches as MIDI note numbers, conversion to and from Hz, note names.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Concert pitch A4 in Hz (MIDI note 69).
pub const A4_HZ: f64 = 440.0;
const A4_MIDI: f64 = 69.0;
const CENTS_PER_OCTAVE: f64 = 1200.0;
const SEMITONES_PER_OCTAVE: i32 = 12;

/// Frequency of a (fractional) MIDI note number, e.g. 28 → 41.2 Hz (E1).
pub fn midi_to_hz(midi: f64, a4_hz: f64) -> f64 {
    a4_hz * 2f64.powf((midi - A4_MIDI) / 12.0)
}

/// Fractional MIDI note number of a frequency, e.g. 41.2 Hz → 28.0.
pub fn hz_to_midi(hz: f64, a4_hz: f64) -> f64 {
    A4_MIDI + 12.0 * (hz / a4_hz).log2()
}

/// Distance from `reference_hz` to `hz` in cents (positive = sharp).
pub fn cents_between(reference_hz: f64, hz: f64) -> f64 {
    CENTS_PER_OCTAVE * (hz / reference_hz).log2()
}

/// A pitch on the equal-tempered scale, stored as MIDI note number (0..=127).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Pitch(pub u8);

impl Pitch {
    /// Frequency at the given concert pitch.
    pub fn hz(self, a4_hz: f64) -> f64 {
        midi_to_hz(f64::from(self.0), a4_hz)
    }

    /// Nearest pitch to a frequency and the deviation in cents.
    pub fn nearest(hz: f64, a4_hz: f64) -> Option<(Pitch, f64)> {
        if !hz.is_finite() || hz <= 0.0 {
            return None;
        }
        let midi = hz_to_midi(hz, a4_hz).round();
        if !(0.0..=127.0).contains(&midi) {
            return None;
        }
        let pitch = Pitch(midi as u8);
        Some((pitch, cents_between(pitch.hz(a4_hz), hz)))
    }

    /// Pitch shifted by semitones, `None` outside the MIDI range.
    pub fn transposed(self, semitones: i32) -> Option<Pitch> {
        let value = i32::from(self.0) + semitones;
        u8::try_from(value).ok().filter(|v| *v <= 127).map(Pitch)
    }

    /// Octave in scientific notation (C4 = middle C = MIDI 60).
    pub fn octave(self) -> i32 {
        i32::from(self.0) / SEMITONES_PER_OCTAVE - 1
    }

    /// Pitch class name with sharps.
    pub fn name(self) -> NoteName {
        NoteName::ALL[usize::from(self.0) % 12]
    }

    /// Parses names like `E1`, `C#2`, `Bb0`.
    pub fn parse(text: &str) -> Option<Pitch> {
        let text = text.trim();
        let split = text.find(|c: char| c.is_ascii_digit() || c == '-')?;
        let (name, octave) = text.split_at(split);
        let class = NoteName::parse(name)?;
        let octave: i32 = octave.parse().ok()?;
        let midi = (octave + 1) * SEMITONES_PER_OCTAVE + class as i32;
        u8::try_from(midi).ok().filter(|v| *v <= 127).map(Pitch)
    }
}

impl fmt::Display for Pitch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.name(), self.octave())
    }
}

/// The twelve pitch classes, named with sharps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NoteName {
    C = 0,
    CSharp,
    D,
    DSharp,
    E,
    F,
    FSharp,
    G,
    GSharp,
    A,
    ASharp,
    B,
}

impl NoteName {
    pub const ALL: [NoteName; 12] = [
        NoteName::C,
        NoteName::CSharp,
        NoteName::D,
        NoteName::DSharp,
        NoteName::E,
        NoteName::F,
        NoteName::FSharp,
        NoteName::G,
        NoteName::GSharp,
        NoteName::A,
        NoteName::ASharp,
        NoteName::B,
    ];

    fn parse(text: &str) -> Option<NoteName> {
        let mut chars = text.chars();
        let base = match chars.next()?.to_ascii_uppercase() {
            'C' => 0,
            'D' => 2,
            'E' => 4,
            'F' => 5,
            'G' => 7,
            'A' => 9,
            'B' | 'H' => 11,
            _ => return None,
        };
        let shift: i32 = match chars.as_str() {
            "" => 0,
            "#" | "♯" => 1,
            "b" | "♭" => -1,
            _ => return None,
        };
        Some(NoteName::ALL[(base + shift).rem_euclid(12) as usize])
    }
}

impl fmt::Display for NoteName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            NoteName::C => "C",
            NoteName::CSharp => "C#",
            NoteName::D => "D",
            NoteName::DSharp => "D#",
            NoteName::E => "E",
            NoteName::F => "F",
            NoteName::FSharp => "F#",
            NoteName::G => "G",
            NoteName::GSharp => "G#",
            NoteName::A => "A",
            NoteName::ASharp => "A#",
            NoteName::B => "B",
        };
        f.write_str(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_strings_have_expected_frequencies() {
        let b0 = Pitch::parse("B0").unwrap();
        let e1 = Pitch::parse("E1").unwrap();
        assert_eq!(b0.0, 23);
        assert_eq!(e1.0, 28);
        assert!((b0.hz(A4_HZ) - 30.868).abs() < 0.01);
        assert!((e1.hz(A4_HZ) - 41.203).abs() < 0.01);
    }

    #[test]
    fn nearest_reports_cents() {
        let (pitch, cents) = Pitch::nearest(41.203 * 2f64.powf(20.0 / 1200.0), A4_HZ).unwrap();
        assert_eq!(pitch.to_string(), "E1");
        assert!((cents - 20.0).abs() < 0.1);
    }

    #[test]
    fn parses_flats_and_german_h() {
        assert_eq!(Pitch::parse("Bb1"), Pitch::parse("A#1"));
        assert_eq!(Pitch::parse("H1"), Pitch::parse("B1"));
        assert_eq!(Pitch::parse("X1"), None);
    }
}
