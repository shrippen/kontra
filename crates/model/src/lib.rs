//! Domain types shared by every Kontra layer.
//!
//! ```text
//! app ─► appcore ─┐
//!                 ├─► session ─┬─► engine ─┬─► drivers
//! cli ────────────┘            │           ├─► dsp
//!                              └─► content └─► stretch
//! ```
//! (model is used by all but app and stretch; it depends on nothing)
//!
//! Units are part of the names: `_hz`, `_cent`, `_s`, `_ms`, `_ticks`.

mod arrangement;
mod beat_grid;
mod chord;
mod device;
mod event;
mod fx;
mod instrument;
mod outside;
mod pitch;
mod plugin;
mod plugin_group;
mod shaping;
mod spelling;
mod tempo;
mod tuning;

pub use arrangement::{
    Arrangement, Bar, DrumHit, DrumKind, Feel, Judging, Marker, MarkerKind, Note, Technique,
    TimeSignature, TICKS_PER_QUARTER,
};
pub use beat_grid::BeatGrid;
pub use chord::{in_key, Chord};
pub use device::{Connection, DeviceInfo, Direction, MidiPort, Priority, NULL_DEVICE};
pub use event::{
    ChordHearing, DetectedNote, DoubleStopHeard, DoubleStopVerdict, Ending, ExpectedStrike,
    ExpectedTone, InputSource, NoteEnd, NoteRetraction, PitchRevision, StringHeard, StringVerdict,
    TechniqueRevision, Unclear,
};
pub use fx::{ChainSpec, EffectKind, SlotSpec, CHAIN_SLOTS};
pub use instrument::{Instrument, InstrumentError, InstrumentKind, KeyRange, ToneBands};
pub use outside::{
    Http, HttpError, MemoryStore, SecretError, SecretStore, SeparatorError, HTTP_TIMEOUT,
};
pub use pitch::{cents_between, hz_to_midi, midi_to_hz, NoteName, Pitch, A4_HZ};
pub use plugin::{
    BrokenFile, ParamInfo, ParamSpec, PluginFormat, PluginInfo, PluginSpec, PortLayout,
    ProbeCommand, ScanReport,
};
pub use plugin_group::{group_of_effect, group_of_plugin, PluginGroup};
pub use shaping::{Limit, Shaping, SHAPING_RANGE_DB};
pub use spelling::{read_note, ReadNote};
pub use tempo::{TempoChange, TempoMap};
pub use tuning::{Tuning, TuningError};
