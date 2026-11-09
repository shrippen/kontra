//! German names of scales, sequences, progressions and rhythms in exercise titles and file
//! names (the files keep German names; texts on screen come from the i18n keys).

use super::{Progression, Rhythm, ScaleKind, Sequence};

pub(super) fn scale_label(scale: ScaleKind) -> &'static str {
    match scale {
        ScaleKind::Major => "Dur-Tonleiter",
        ScaleKind::Minor => "Moll-Tonleiter",
        ScaleKind::MajorPentatonic => "Dur-Pentatonik",
        ScaleKind::MinorPentatonic => "Moll-Pentatonik",
        ScaleKind::Blues => "Blues-Tonleiter",
        ScaleKind::Dorian => "Dorisch",
        ScaleKind::Phrygian => "Phrygisch",
        ScaleKind::Lydian => "Lydisch",
        ScaleKind::Mixolydian => "Mixolydisch",
        ScaleKind::Locrian => "Lokrisch",
        ScaleKind::HarmonicMinor => "Harmonisch Moll",
    }
}

pub(super) fn sequence_label(sequence: Sequence) -> &'static str {
    match sequence {
        Sequence::Thirds => "in Terzen",
        Sequence::Fours => "in Vierergruppen",
    }
}

pub(super) fn progression_label(progression: Progression) -> &'static str {
    match progression {
        Progression::Tonic => "auf einem Akkord",
        Progression::TwoFiveOne => "II-V-I",
        Progression::Blues => "12-Takt-Blues",
        Progression::Minor => "Moll-Kadenz",
        Progression::OneFourFive => "I-IV-V",
        Progression::Pop => "I-V-VI-IV",
        Progression::Turnaround => "I-VI-II-V",
        Progression::MinorTwoFiveOne => "Moll-II-V-I",
    }
}

pub(super) fn rhythm_label(rhythm: Rhythm) -> &'static str {
    match rhythm {
        Rhythm::Quarters => "Viertel",
        Rhythm::Eighths => "Achtel",
        Rhythm::Sixteenths => "Sechzehntel",
        Rhythm::WithRests => "mit Pausen",
        Rhythm::Shuffle => "Shuffle",
        Rhythm::Octaves => "Oktaven",
        Rhythm::Ghost => "Ghost Notes",
        Rhythm::RootFifth => "Grundton-Quinte",
        Rhythm::RootFifthOctave => "Grundton-Quinte-Oktave",
        Rhythm::DeadNotes => "Dead Notes",
        Rhythm::Offbeat => "Offbeat",
        Rhythm::Tresillo => "Tresillo",
        Rhythm::Triplets => "Triolen",
        Rhythm::Funk => "Funk",
        Rhythm::Slap => "Slap",
        Rhythm::Waltz => "3/4",
        Rhythm::SixEight => "6/8",
        Rhythm::FiveFour => "5/4",
        Rhythm::SevenEight => "7/8",
        Rhythm::Staccato => "Staccato",
    }
}
