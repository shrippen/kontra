//! Settings of the effect chain on the monitored bass (LH-EIN-19): which effects, in which
//! order, switched on or off, with which values. Plain data; `kontra_dsp` builds the audio
//! from it.
//!
//! ```text
//! ChainSpec = [ Slot 1 ][ Slot 2 ] … [ Slot 8 ]      free order, at most `CHAIN_SLOTS`
//! Slot      = kind + on/off + a, b, mix (each 0..=1); a plugin slot also names its plugin
//! ```
//!
//! `a` and `b` mean something different per kind (see `EffectKind`); `mix` blends the dry
//! signal (0) with the effect (1) and is 1 for the kinds that have no dry part.

use serde::{Deserialize, Serialize};

/// Most slots a chain holds.
pub const CHAIN_SLOTS: usize = 8;

/// What a slot does. Meaning of the two values per kind:
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectKind {
    /// `a` drive. Soft clipping at the same level.
    Overdrive,
    /// `a` width. Mono to stereo; slots after it work per side.
    Widener,
    /// `a` rate, `b` depth.
    Chorus,
    /// Auto-wah: `a` sensitivity, `b` resonance. The pick attack opens the filter.
    EnvelopeFilter,
    /// One octave down: `a` level of the lower octave, `b` tone (dark to bright).
    Octaver,
    /// `a` time (30 to 800 ms), `b` feedback.
    Delay,
    /// Small room: `a` size, `b` damping (dark highs).
    Reverb,
    /// `a` fuzz, `b` tone.
    Fuzz,
    /// An external plugin instance (LH-EIN-20, -21); the values belong to the plugin.
    Plugin,
}

impl EffectKind {
    /// What a new slot of this kind starts with: moderate, and audible.
    fn defaults(self) -> (f32, f32, f32) {
        match self {
            EffectKind::Overdrive => (0.5, 0.0, 1.0),
            EffectKind::Widener => (0.5, 0.0, 1.0),
            EffectKind::Chorus => (0.3, 0.5, 0.5),
            EffectKind::EnvelopeFilter => (0.5, 0.4, 1.0),
            EffectKind::Octaver => (0.7, 0.5, 0.5),
            EffectKind::Delay => (0.3, 0.35, 0.3),
            EffectKind::Reverb => (0.5, 0.5, 0.25),
            EffectKind::Fuzz => (0.6, 0.5, 1.0),
            EffectKind::Plugin => (0.0, 0.0, 1.0),
        }
    }
}

/// One slot of the chain.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SlotSpec {
    pub kind: EffectKind,
    pub on: bool,
    pub a: f32,
    pub b: f32,
    pub mix: f32,
    /// For `EffectKind::Plugin`: the `PluginSpec::id` of its plugin; else 0.
    #[serde(default)]
    pub plugin: u32,
}

impl SlotSpec {
    /// A switched-on slot with the kind's usual values.
    pub fn new(kind: EffectKind) -> SlotSpec {
        let (a, b, mix) = kind.defaults();
        SlotSpec {
            kind,
            on: true,
            a,
            b,
            mix,
            plugin: 0,
        }
    }

    /// A switched-on slot for the plugin `plugin` (`PluginSpec::id`); the mix stays at 1.
    pub fn for_plugin(plugin: u32) -> SlotSpec {
        SlotSpec {
            plugin,
            ..SlotSpec::new(EffectKind::Plugin)
        }
    }

    fn clamped(self) -> SlotSpec {
        let unit = |v: f32| {
            if v.is_finite() {
                v.clamp(0.0, 1.0)
            } else {
                0.0
            }
        };
        SlotSpec {
            a: unit(self.a),
            b: unit(self.b),
            mix: unit(self.mix),
            ..self
        }
    }
}

const IDLE_SLOT: SlotSpec = SlotSpec {
    kind: EffectKind::Overdrive,
    on: false,
    a: 0.0,
    b: 0.0,
    mix: 0.0,
    plugin: 0,
};

/// The slots in order. `Copy` (a fixed array), so it travels with the mixer's settings to the
/// audio thread. Stored as a plain list.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(from = "Vec<SlotSpec>", into = "Vec<SlotSpec>")]
pub struct ChainSpec {
    slots: [SlotSpec; CHAIN_SLOTS],
    len: usize,
}

impl ChainSpec {
    /// No effects: the neutral chain.
    pub const EMPTY: ChainSpec = ChainSpec {
        slots: [IDLE_SLOT; CHAIN_SLOTS],
        len: 0,
    };

    pub fn slots(&self) -> &[SlotSpec] {
        &self.slots[..self.len]
    }

    pub fn slots_mut(&mut self) -> &mut [SlotSpec] {
        &mut self.slots[..self.len]
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Appends a slot (values clamped to 0..=1); false when the chain is full.
    pub fn push(&mut self, slot: SlotSpec) -> bool {
        self.insert(self.len, slot)
    }

    /// Puts a slot at `at` (values clamped), the later ones move on; false when the chain is
    /// full or `at` is past the end.
    pub fn insert(&mut self, at: usize, slot: SlotSpec) -> bool {
        if self.len == CHAIN_SLOTS || at > self.len {
            return false;
        }
        self.slots.copy_within(at..self.len, at + 1);
        self.slots[at] = slot.clamped();
        self.len += 1;
        true
    }

    /// Takes the slot at `at` out; `None` past the end.
    pub fn remove(&mut self, at: usize) -> Option<SlotSpec> {
        if at >= self.len {
            return None;
        }
        let slot = self.slots[at];
        self.slots.copy_within(at + 1..self.len, at);
        self.len -= 1;
        Some(slot)
    }

    /// Moves the slot at `from` to `to`; false if either is past the end.
    pub fn move_slot(&mut self, from: usize, to: usize) -> bool {
        match self.remove(from) {
            Some(slot) => self.insert(to, slot),
            None => false,
        }
    }

    /// Values clamped to 0..=1.
    pub fn clamped(self) -> ChainSpec {
        let mut spec = self;
        spec.slots_mut().iter_mut().for_each(|s| *s = s.clamped());
        spec
    }

    /// The switched-on slots in order: what actually sounds.
    pub fn active(&self) -> impl Iterator<Item = &SlotSpec> {
        self.slots().iter().filter(|s| s.on)
    }

    /// Same effects in the same order (off slots do not count): only the values may differ,
    /// so a chain built from one fits the other after `set_values`.
    pub fn same_shape(&self, other: &ChainSpec) -> bool {
        let mut a = self.active();
        let mut b = other.active();
        loop {
            match (a.next(), b.next()) {
                (None, None) => return true,
                (Some(x), Some(y)) if x.kind == y.kind && x.plugin == y.plugin => {}
                _ => return false,
            }
        }
    }
}

impl Default for ChainSpec {
    fn default() -> Self {
        ChainSpec::EMPTY
    }
}

impl From<Vec<SlotSpec>> for ChainSpec {
    /// Slots past `CHAIN_SLOTS` are dropped.
    fn from(slots: Vec<SlotSpec>) -> ChainSpec {
        let mut spec = ChainSpec::EMPTY;
        for slot in slots {
            if !spec.push(slot) {
                break;
            }
        }
        spec
    }
}

impl From<ChainSpec> for Vec<SlotSpec> {
    fn from(spec: ChainSpec) -> Vec<SlotSpec> {
        spec.slots().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(kinds: &[EffectKind]) -> ChainSpec {
        kinds
            .iter()
            .map(|&k| SlotSpec::new(k))
            .collect::<Vec<_>>()
            .into()
    }

    #[test]
    fn order_is_free_and_capped() {
        let mut chain = spec(&[EffectKind::Fuzz, EffectKind::Delay, EffectKind::Reverb]);
        assert!(chain.move_slot(2, 0));
        let kinds: Vec<_> = chain.slots().iter().map(|s| s.kind).collect();
        assert_eq!(
            kinds,
            [EffectKind::Reverb, EffectKind::Fuzz, EffectKind::Delay]
        );
        while chain.push(SlotSpec::new(EffectKind::Chorus)) {}
        assert_eq!(chain.slots().len(), CHAIN_SLOTS);
        assert!(!chain.insert(0, SlotSpec::new(EffectKind::Fuzz)));
        assert!(chain.remove(CHAIN_SLOTS).is_none());
    }

    #[test]
    fn shape_ignores_values_and_off_slots() {
        let a = spec(&[EffectKind::Fuzz, EffectKind::Delay]);
        let mut b = a;
        b.slots_mut()[1].a = 0.9;
        assert!(a.same_shape(&b));
        b.slots_mut()[1].on = false;
        assert!(!a.same_shape(&b));
        assert!(b.same_shape(&spec(&[EffectKind::Fuzz])));
    }

    #[test]
    fn stored_as_a_list_and_clamped_on_push() {
        let mut chain = ChainSpec::EMPTY;
        chain.push(SlotSpec {
            a: 3.0,
            ..SlotSpec::new(EffectKind::Octaver)
        });
        assert_eq!(chain.slots()[0].a, 1.0);
        let text = serde_json::to_string(&chain).unwrap();
        assert!(text.starts_with("[{"), "{text}");
        assert_eq!(serde_json::from_str::<ChainSpec>(&text).unwrap(), chain);
    }
}
