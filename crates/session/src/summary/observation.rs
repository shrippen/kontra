//! The one observation of a run (LH-BEW-09).

use super::Side;
use kontra_model::Technique;

/// The one thing a run most wants the player to hear (LH-BEW-09, after Harmonicon's coaching).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Observation {
    /// A technique trails the plain notes: "Slide: 2 of 6".
    Technique {
        technique: Technique,
        played: u32,
        judged: u32,
    },
    /// Notes the tab wants stopped rang into the rest (LH-BEW-10): "31 of 31".
    Muting { rang_on: u32, judged: u32 },
    /// Most notes missed the window but were played beside it, by this many ms.
    Beside { offset_ms: f64 },
    /// Many notes did not sit (wrong or missed).
    Missed { notes: u32, judged: u32 },
    /// Hits lean to one side of the beat: `share` of them.
    Lean { side: Side, share: f64 },
    /// The mean missed the chosen aim (LH-BEW-08) on this side, by this many ms.
    Aim { side: Side, off_ms: f64 },
    /// Enough was played and nothing stands out.
    Solid,
}
