//! Domain types shared by every Kontra layer.
//!
//! ```text
//! ui / cli  →  session  →  engine  →  drivers
//!                 ↓           ↓
//!              content      (model is used by all of them, depends on nothing)
//! ```
//!
//! Units are part of the names: `_hz`, `_cent`, `_s`, `_ms`, `_ticks`.

mod arrangement;
mod event;
mod instrument;
mod pitch;
mod tempo;
mod tuning;

pub use arrangement::{
    Arrangement, Bar, DrumHit, DrumKind, Judging, Marker, MarkerKind, Note, Technique,
    TimeSignature, TICKS_PER_QUARTER,
};
pub use event::{
    DetectedNote, ExpectedStrike, ExpectedTone, InputSource, NoteRetraction, PitchRevision,
    TechniqueRevision,
};
pub use instrument::{Instrument, InstrumentError, InstrumentKind, KeyRange, ToneBands};
pub use pitch::{cents_between, hz_to_midi, midi_to_hz, NoteName, Pitch, A4_HZ};
pub use tempo::{TempoChange, TempoMap};
pub use tuning::{Tuning, TuningError};
