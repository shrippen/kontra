//! Notes the player is expected to strike, in arrangement seconds.

use kontra_model::Arrangement;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ExpectedNote {
    /// Index into `Arrangement::notes`.
    pub index: usize,
    /// Position in seconds at original tempo.
    pub position_s: f64,
    pub midi: u8,
    /// Index into `Arrangement::bars`, if the note lies in a bar.
    pub bar: Option<usize>,
}

/// Every note that needs an attack (ties and hammer-on/pull-off targets are skipped).
pub fn expected_notes(arrangement: &Arrangement) -> Vec<ExpectedNote> {
    arrangement
        .notes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.needs_attack())
        .map(|(index, n)| ExpectedNote {
            index,
            position_s: arrangement.tempo.seconds_at(n.start_ticks),
            midi: n.pitch.0,
            bar: arrangement.bar_at(n.start_ticks),
        })
        .collect()
}
