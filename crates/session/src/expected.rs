//! Notes the player is expected to strike, in arrangement seconds.

use kontra_model::{Arrangement, Technique};
use serde::{Deserialize, Serialize};

const CENTS_PER_OCTAVE: f64 = 1200.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ExpectedNote {
    /// Index into `Arrangement::notes`.
    pub index: usize,
    /// Position in seconds at original tempo.
    pub position_s: f64,
    pub midi: u8,
    /// Index into `Arrangement::bars`, if the note lies in a bar.
    pub bar: Option<usize>,
    /// Further notes struck together with this one (a chord); `midi` is the lowest.
    pub chord: ChordTones,
    /// A ghost or dead note: muted, without a pitch to judge (LH-BEW-06).
    #[serde(default)]
    pub ghost: bool,
    /// A harmonic: its pitch counts in any octave (the tracker often hears it lower).
    #[serde(default)]
    pub harmonic: bool,
}

impl ExpectedNote {
    /// Pitches this attack accepts: the note and every other chord tone.
    pub fn pitches(&self) -> impl Iterator<Item = u8> + '_ {
        std::iter::once(self.midi).chain(self.chord.iter().map(|(_, midi)| midi))
    }

    /// Cents from `midi` to the nearest accepted pitch (for a harmonic: in any octave).
    pub fn cents_from(&self, midi: f64) -> f64 {
        let harmonic = self.harmonic;
        self.pitches()
            .map(|p| (midi - f64::from(p)) * 100.0)
            .map(|c| {
                if harmonic {
                    c - (c / CENTS_PER_OCTAVE).round() * CENTS_PER_OCTAVE
                } else {
                    c
                }
            })
            .fold(
                f64::INFINITY,
                |best, c| if c.abs() < best.abs() { c } else { best },
            )
    }
}

/// Up to `ChordTones::MAX` further notes of a chord, as (note index, midi).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct ChordTones {
    len: u8,
    tones: [(usize, u8); ChordTones::MAX],
}

impl ChordTones {
    /// One bass can strike at most six strings at once.
    pub const MAX: usize = 5;

    fn push(&mut self, index: usize, midi: u8) {
        if let Some(slot) = self.tones.get_mut(usize::from(self.len)) {
            *slot = (index, midi);
            self.len += 1;
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (usize, u8)> + '_ {
        self.tones.iter().take(usize::from(self.len)).copied()
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// One entry per attack: every note that needs one (ties and hammer-on/pull-off targets are
/// skipped), with notes that start together merged into a chord. The pitch detector hears one
/// note, so any tone of a chord counts for the whole chord.
pub fn expected_notes(arrangement: &Arrangement) -> Vec<ExpectedNote> {
    let mut attacks: Vec<(usize, &kontra_model::Note)> = arrangement
        .notes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.needs_attack())
        .collect();
    attacks.sort_by_key(|(index, n)| (n.start_ticks, n.pitch.0, *index));
    let mut out: Vec<ExpectedNote> = Vec::with_capacity(attacks.len());
    let mut last_tick = None;
    for (index, n) in attacks {
        if last_tick == Some(n.start_ticks) {
            if let Some(chord) = out.last_mut() {
                chord.chord.push(index, n.pitch.0);
                continue;
            }
        }
        last_tick = Some(n.start_ticks);
        out.push(ExpectedNote {
            index,
            position_s: arrangement.tempo.seconds_at(n.start_ticks),
            midi: n.pitch.0,
            bar: arrangement.bar_at(n.start_ticks),
            chord: ChordTones::default(),
            ghost: n
                .techniques
                .iter()
                .any(|t| matches!(t, Technique::Ghost | Technique::Muted)),
            harmonic: n.techniques.contains(&Technique::Harmonic),
        });
    }
    out
}
