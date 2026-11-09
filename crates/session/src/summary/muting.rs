//! How the notes the tab wants stopped ended (LH-BEW-10), per note and in total.

use serde::{Deserialize, Serialize};

/// How the notes the tab wants stopped ended (LH-BEW-10): "6 of 8 stopped in time".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MutingTally {
    pub judged: u32,
    pub in_time: u32,
    pub late: u32,
    pub rang_on: u32,
    /// Notes to stop this input cannot judge (microphone, MIDI): said, not hidden.
    pub untold: u32,
}

/// How a note the tab wants stopped ended.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum EndHeard {
    /// Silent by its written end (within the timing window).
    InTime,
    /// Muted, but this many real ms after its written end and the window.
    Late { late_ms: f64 },
    /// Not stopped: it rang into the rest.
    RangOn,
}

/// How one note the tab wants stopped ended.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NoteEndMark {
    pub pass: u32,
    /// Index into `Arrangement::notes`.
    pub note: usize,
    pub heard: EndHeard,
}

impl MutingTally {
    /// Counts each note once, by its last verdict; `untold` as the session says.
    pub fn of(ends: &[NoteEndMark], untold: u32) -> MutingTally {
        let mut tally = MutingTally {
            untold,
            ..MutingTally::default()
        };
        for (i, e) in ends.iter().enumerate() {
            let later = ends[i + 1..]
                .iter()
                .any(|l| l.pass == e.pass && l.note == e.note);
            if later {
                continue;
            }
            tally.judged += 1;
            match e.heard {
                EndHeard::InTime => tally.in_time += 1,
                EndHeard::Late { .. } => tally.late += 1,
                EndHeard::RangOn => tally.rang_on += 1,
            }
        }
        tally
    }
}
