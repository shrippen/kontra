//! The effect chain on the monitored bass (LH-EIN-19): slots in free order between the tone
//! shaping and the limiter.
//!
//! ```text
//!            mono                                      stereo (from the widener on)
//! x ─► [Overdrive] ─► [Chorus] ─► [Widener] ─┬─ L ─► [Delay L] ─► [Reverb L] ─► L
//!       a b mix        a b mix       a mix   └─ R ─► [Delay R] ─► [Reverb R] ─► R
//!
//! slot  out = dry + (wet − dry) · mix       mix 0 passes the dry signal untouched
//! ```
//!
//! - Before the first widener (or plugin) the chain is mono, one instance per slot. After it,
//!   each slot has two instances, one per side (`Side::Left` and `Right` decorrelate the
//!   modulation and the reverb's tunings). A second widener works on the sum of both sides.
//! - Only switched-on slots exist. Which slots, their kinds and their order are the chain's
//!   *shape*; a change of shape needs a new `Chain` (made outside the audio callback, which
//!   fades over to it, see `Shaper`). Changes of values only (`set_values`) glide in place.
//! - A plugin (LH-EIN-20, -21) enters as an `Insert`: it takes a stereo frame, so the chain is
//!   stereo from it on.
//!
//! No allocation after `new`: one frame at a time.

use crate::effects::{Overdrive, Widener};
use crate::fx::{Chorus, Delay, EnvelopeFilter, Fuzz, Octaver, Reverb, Side};
use kontra_model::{ChainSpec, EffectKind, CHAIN_SLOTS};

/// How fast a value follows its target (a slider moved): no zipper noise.
const GLIDE_S: f64 = 0.020;
/// A value this close to its target has arrived (so a mix of 0 is exactly 0).
const SNAP: f32 = 1e-4;

/// An effect from outside Kontra (a plugin instance) in one slot. Called in the audio
/// callback: no allocation, no locks.
pub trait Insert: Send + std::fmt::Debug {
    /// One stereo frame through the effect.
    fn process(&mut self, frame: (f32, f32)) -> (f32, f32);

    /// Frames the effect delays the signal by (lookahead); 0 if none.
    fn latency_frames(&self) -> usize {
        0
    }
}

#[derive(Debug)]
enum Effect {
    Overdrive(Overdrive),
    Widener(Widener),
    Chorus(Chorus),
    EnvelopeFilter(EnvelopeFilter),
    Octaver(Octaver),
    Delay(Delay),
    Reverb(Box<Reverb>),
    Fuzz(Fuzz),
    Plugin(Box<dyn Insert>),
}

impl Effect {
    /// `None` for a plugin: that one is handed in.
    fn build(kind: EffectKind, rate: f64, side: Side) -> Option<Effect> {
        Some(match kind {
            EffectKind::Overdrive => Effect::Overdrive(Overdrive::new(rate)),
            EffectKind::Widener => Effect::Widener(Widener::new(rate)),
            EffectKind::Chorus => Effect::Chorus(Chorus::new(rate, side)),
            EffectKind::EnvelopeFilter => Effect::EnvelopeFilter(EnvelopeFilter::new(rate)),
            EffectKind::Octaver => Effect::Octaver(Octaver::new(rate)),
            EffectKind::Delay => Effect::Delay(Delay::new(rate)),
            EffectKind::Reverb => Effect::Reverb(Box::new(Reverb::new(rate, side))),
            EffectKind::Fuzz => Effect::Fuzz(Fuzz::new(rate)),
            EffectKind::Plugin => return None,
        })
    }

    fn retime(&mut self, rate: f64) {
        match self {
            Effect::Overdrive(fx) => *fx = Overdrive::new(rate),
            Effect::Widener(fx) => fx.retime(rate),
            Effect::Chorus(fx) => fx.retime(rate),
            Effect::EnvelopeFilter(fx) => fx.retime(rate),
            Effect::Octaver(fx) => fx.retime(rate),
            Effect::Delay(fx) => fx.retime(rate),
            Effect::Reverb(fx) => fx.retime(rate),
            Effect::Fuzz(fx) => fx.retime(rate),
            Effect::Plugin(_) => {}
        }
    }

    /// One sample of the effects that work on one channel.
    fn mono(&mut self, x: f32, a: f32, b: f32) -> f32 {
        match self {
            Effect::Overdrive(fx) => fx.process(x, a),
            Effect::Chorus(fx) => fx.process(x, a, b),
            Effect::EnvelopeFilter(fx) => fx.process(x, a, b),
            Effect::Octaver(fx) => fx.process(x, a, b),
            Effect::Delay(fx) => fx.process(x, a, b),
            Effect::Reverb(fx) => fx.process(x, a, b),
            Effect::Fuzz(fx) => fx.process(x, a, b),
            Effect::Widener(_) | Effect::Plugin(_) => x,
        }
    }
}

#[derive(Debug)]
struct Slot {
    /// Place among the switched-on slots of the spec: where `set_values` finds its values.
    index: usize,
    effect: Effect,
    /// The right side, when the chain is stereo here.
    twin: Option<Effect>,
    /// a, b, mix now and wanted.
    now: [f32; 3],
    target: [f32; 3],
}

impl Slot {
    fn glide(&mut self, coeff: f32) {
        for (now, target) in self.now.iter_mut().zip(self.target) {
            let apart = target - *now;
            *now = if apart.abs() < SNAP {
                target
            } else {
                *now + apart * coeff
            };
        }
    }

    fn run(&mut self, dry: (f32, f32)) -> (f32, f32) {
        let [a, b, mix] = self.now;
        let wet = match (&mut self.effect, &mut self.twin) {
            (Effect::Widener(fx), _) => fx.process(0.5 * (dry.0 + dry.1), a),
            (Effect::Plugin(fx), _) => fx.process(dry),
            (left, Some(right)) => (left.mono(dry.0, a, b), right.mono(dry.1, a, b)),
            (left, None) => {
                let y = left.mono(dry.0, a, b);
                (y, y)
            }
        };
        if mix <= 0.0 {
            return dry;
        }
        (dry.0 + (wet.0 - dry.0) * mix, dry.1 + (wet.1 - dry.1) * mix)
    }
}

/// A built chain, ready for the audio callback.
#[derive(Debug)]
pub struct Chain {
    spec: ChainSpec,
    slots: Vec<Slot>,
    glide: f32,
}

impl Chain {
    /// Chain of the built-in effects at `sample_rate`; a plugin slot passes the signal.
    pub fn new(spec: &ChainSpec, sample_rate: f64) -> Chain {
        Chain::with_plugins(spec, sample_rate, &mut |_| None)
    }

    /// As `new`; `plugin(i)` makes the instance for the plugin slot at position `i` of
    /// `spec.slots()`, or `None` for one that is not there (it passes the signal).
    pub fn with_plugins(
        spec: &ChainSpec,
        sample_rate: f64,
        plugin: &mut dyn FnMut(usize) -> Option<Box<dyn Insert>>,
    ) -> Chain {
        let spec = spec.clamped();
        let mut slots = Vec::with_capacity(CHAIN_SLOTS);
        let mut stereo = false;
        let mut index = 0;
        for (position, wanted) in spec.slots().iter().enumerate() {
            if !wanted.on {
                continue;
            }
            index += 1;
            let target = [wanted.a, wanted.b, wanted.mix];
            let built = match wanted.kind {
                EffectKind::Plugin => plugin(position).map(|p| (Effect::Plugin(p), None)),
                EffectKind::Widener => {
                    Effect::build(wanted.kind, sample_rate, Side::Mono).map(|e| (e, None))
                }
                kind if stereo => {
                    let left = Effect::build(kind, sample_rate, Side::Left);
                    let right = Effect::build(kind, sample_rate, Side::Right);
                    left.zip(right).map(|(l, r)| (l, Some(r)))
                }
                kind => Effect::build(kind, sample_rate, Side::Mono).map(|e| (e, None)),
            };
            let Some((effect, twin)) = built else {
                continue;
            };
            stereo |= matches!(wanted.kind, EffectKind::Widener | EffectKind::Plugin);
            slots.push(Slot {
                index: index - 1,
                effect,
                twin,
                now: target,
                target,
            });
        }
        let mut chain = Chain {
            spec,
            slots,
            glide: 0.0,
        };
        chain.set_rate(sample_rate);
        chain
    }

    fn set_rate(&mut self, sample_rate: f64) {
        self.glide = 1.0 - (-1.0 / (GLIDE_S * sample_rate)).exp() as f32;
    }

    /// The device's rate changed; keeps all buffers (no allocation).
    pub fn retime(&mut self, sample_rate: f64) {
        self.set_rate(sample_rate);
        for slot in &mut self.slots {
            slot.effect.retime(sample_rate);
            if let Some(twin) = slot.twin.as_mut() {
                twin.retime(sample_rate);
            }
        }
    }

    /// New values for the same shape; they are reached gliding. False (nothing changed) if
    /// the shape differs: that needs a new chain.
    pub fn set_values(&mut self, spec: &ChainSpec) -> bool {
        if !self.spec.same_shape(spec) {
            return false;
        }
        let spec = spec.clamped();
        for slot in &mut self.slots {
            if let Some(wanted) = spec.active().nth(slot.index) {
                slot.target = [wanted.a, wanted.b, wanted.mix];
            }
        }
        self.spec = spec;
        true
    }

    /// Whether `set_values` accepts `spec`.
    pub fn fits(&self, spec: &ChainSpec) -> bool {
        self.spec.same_shape(spec)
    }

    /// Frames the chain delays the signal by: 0 for the built-in effects, plugins say their
    /// own (lookahead).
    pub fn latency_frames(&self) -> usize {
        self.slots
            .iter()
            .map(|s| match &s.effect {
                Effect::Plugin(p) => p.latency_frames(),
                _ => 0,
            })
            .sum()
    }

    /// One mono sample as (left, right).
    pub fn process(&mut self, x: f32) -> (f32, f32) {
        let mut frame = (x, x);
        for slot in &mut self.slots {
            slot.glide(self.glide);
            frame = slot.run(frame);
        }
        frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kontra_model::SlotSpec;
    use std::f64::consts::PI;

    const SR: f64 = 48_000.0;
    const KINDS: [EffectKind; 8] = [
        EffectKind::Overdrive,
        EffectKind::Widener,
        EffectKind::Chorus,
        EffectKind::EnvelopeFilter,
        EffectKind::Octaver,
        EffectKind::Delay,
        EffectKind::Reverb,
        EffectKind::Fuzz,
    ];

    fn sine(hz: f64, amplitude: f32, n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| amplitude * (2.0 * PI * hz * i as f64 / SR).sin() as f32)
            .collect()
    }

    fn spec(slots: &[SlotSpec]) -> ChainSpec {
        slots.to_vec().into()
    }

    fn render(chain: &mut Chain, input: &[f32]) -> Vec<(f32, f32)> {
        input.iter().map(|&x| chain.process(x)).collect()
    }

    #[test]
    fn empty_chain_is_the_input() {
        let mut chain = Chain::new(&ChainSpec::EMPTY, SR);
        for &x in &sine(110.0, 0.5, 1000) {
            assert_eq!(chain.process(x), (x, x));
        }
        assert_eq!(chain.latency_frames(), 0);
    }

    #[test]
    fn every_effect_at_zero_passes_dry() {
        let input = sine(110.0, 0.5, 9600);
        for kind in KINDS {
            let slot = SlotSpec {
                mix: 0.0,
                ..SlotSpec::new(kind)
            };
            let mut chain = Chain::new(&spec(&[slot]), SR);
            for (&x, (l, r)) in input.iter().zip(render(&mut chain, &input)) {
                assert_eq!((l, r), (x, x), "{kind:?}");
            }
        }
    }

    #[test]
    fn every_effect_does_something_and_stays_finite() {
        // Bass with some top: a filter has something to cut.
        let input: Vec<f32> = sine(110.0, 0.3, 48_000)
            .iter()
            .zip(sine(1760.0, 0.2, 48_000))
            .map(|(a, b)| a + b)
            .collect();
        for kind in KINDS {
            let mut chain = Chain::new(&spec(&[SlotSpec::new(kind)]), SR);
            let out = render(&mut chain, &input);
            assert!(out.iter().all(|(l, r)| l.is_finite() && r.is_finite()));
            let apart = input
                .iter()
                .zip(&out)
                .skip(24_000)
                .map(|(x, (l, r))| (l - x).abs().max((r - x).abs()))
                .fold(0.0, f32::max);
            assert!(apart > 0.01, "{kind:?} changes nothing ({apart})");
        }
    }

    #[test]
    fn order_changes_the_sound() {
        let input = sine(110.0, 0.4, 24_000);
        let fuzz = SlotSpec::new(EffectKind::Fuzz);
        let delay = SlotSpec::new(EffectKind::Delay);
        let a = render(&mut Chain::new(&spec(&[fuzz, delay]), SR), &input);
        let b = render(&mut Chain::new(&spec(&[delay, fuzz]), SR), &input);
        let apart = a
            .iter()
            .zip(&b)
            .map(|(p, q)| (p.0 - q.0).abs())
            .fold(0.0, f32::max);
        assert!(apart > 0.01, "{apart}");
    }

    #[test]
    fn effects_after_the_widener_work_per_side() {
        // Chorus after the widener: the sides get different modulation, so they differ even
        // where the widener alone would leave them equal (width 0).
        let wide = SlotSpec {
            a: 0.0,
            ..SlotSpec::new(EffectKind::Widener)
        };
        let chorus = SlotSpec::new(EffectKind::Chorus);
        let input = sine(220.0, 0.4, 24_000);
        let out = render(&mut Chain::new(&spec(&[wide, chorus]), SR), &input);
        assert!(out.iter().any(|(l, r)| (l - r).abs() > 0.01));
        // Before it, the chain is mono.
        let out = render(&mut Chain::new(&spec(&[chorus, wide]), SR), &input);
        assert!(out.iter().all(|(l, r)| l == r));
    }

    #[test]
    fn off_slots_do_not_exist() {
        let mut off = SlotSpec::new(EffectKind::Fuzz);
        off.on = false;
        let mut chain = Chain::new(&spec(&[off]), SR);
        assert_eq!(chain.process(0.3), (0.3, 0.3));
    }

    #[test]
    fn values_glide_in_place() {
        let mut slot = SlotSpec::new(EffectKind::Overdrive);
        slot.mix = 0.0;
        let mut chain = Chain::new(&spec(&[slot]), SR);
        let input = sine(110.0, 0.5, 4800);
        render(&mut chain, &input);

        slot.mix = 1.0;
        assert!(chain.set_values(&spec(&[slot])));
        let out = render(&mut chain, &input);
        // Not at once: the first sample is still dry; later ones are overdriven.
        assert!((out[1].0 - input[1]).abs() < 0.01);
        assert!((out[4799].0 - input[4799]).abs() > 0.01);

        // Another shape is refused.
        assert!(!chain.set_values(&spec(&[slot, SlotSpec::new(EffectKind::Fuzz)])));
    }

    #[derive(Debug)]
    struct Gain(f32);

    impl Insert for Gain {
        fn process(&mut self, (l, r): (f32, f32)) -> (f32, f32) {
            (l * self.0, r * self.0)
        }
        fn latency_frames(&self) -> usize {
            64
        }
    }

    #[test]
    fn a_plugin_takes_a_slot() {
        let plugin = SlotSpec::new(EffectKind::Plugin);
        let spec = spec(&[plugin]);
        let mut chain = Chain::with_plugins(&spec, SR, &mut |at| {
            assert_eq!(at, 0);
            Some(Box::new(Gain(0.5)))
        });
        assert_eq!(chain.process(0.4), (0.2, 0.2));
        assert_eq!(chain.latency_frames(), 64);
        // Not found: the slot passes the signal.
        let mut missing = Chain::new(&spec, SR);
        assert_eq!(missing.process(0.4), (0.4, 0.4));
    }

    #[test]
    fn survives_a_rate_change() {
        let all: Vec<SlotSpec> = KINDS.iter().map(|&k| SlotSpec::new(k)).collect();
        let mut chain = Chain::new(&spec(&all), SR);
        let input = sine(110.0, 0.4, 4800);
        render(&mut chain, &input);
        for rate in [192_000.0, 8000.0, 44_100.0] {
            chain.retime(rate);
            let out = render(&mut chain, &input);
            assert!(out.iter().all(|(l, r)| l.is_finite() && r.is_finite()));
        }
    }
}
