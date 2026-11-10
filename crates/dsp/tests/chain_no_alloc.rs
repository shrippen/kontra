//! Changing the effect chain while the bass is monitored (LH-EIN-19): the audio callback
//! must not allocate or free, and the output stays under the limiter's ceiling.
//!
//! A counting global allocator needs `unsafe impl GlobalAlloc`; it only forwards to the
//! system allocator, so it is sound.
#![allow(unsafe_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use kontra_dsp::{Chain, Limit, Shaper, Shaping, LIMIT_CEILING};
use kontra_model::{ChainSpec, EffectKind, SlotSpec};

struct Counting;

thread_local! {
    /// Allocations and frees made by this thread while armed.
    static COUNT: Cell<usize> = const { Cell::new(0) };
    static ARMED: Cell<bool> = const { Cell::new(false) };
}

fn note() {
    // try_with: the thread-local may already be gone at thread exit.
    let _ = ARMED.try_with(|armed| {
        if armed.get() {
            let _ = COUNT.try_with(|c| c.set(c.get() + 1));
        }
    });
}

// SAFETY: every call is forwarded unchanged to `System`; counting touches
// only const-initialised thread-locals, which never allocate.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        note();
        // SAFETY: same contract as the caller's.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        note();
        // SAFETY: same contract as the caller's.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        note();
        // SAFETY: same contract as the caller's.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        note();
        // SAFETY: same contract as the caller's.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

const SR: f64 = 48_000.0;
const BLOCK: usize = 480;
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

fn chain_of(kinds: &[EffectKind]) -> ChainSpec {
    kinds
        .iter()
        .map(|&k| SlotSpec::new(k))
        .collect::<Vec<_>>()
        .into()
}

/// A slap line: loud, far over the ceiling before the limiter.
fn input(n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| 1.6 * (std::f64::consts::TAU * 55.0 * i as f64 / SR).sin() as f32)
        .collect()
}

#[test]
fn changing_the_chain_while_running_does_not_allocate() {
    // The counter itself works.
    ARMED.with(|a| a.set(true));
    drop(std::hint::black_box(Box::new(1_u64)));
    ARMED.with(|a| a.set(false));
    assert_eq!(COUNT.with(|c| c.replace(0)), 2, "one alloc, one free");

    // Everything the control side makes comes first: chains and their settings.
    let rounds = 24;
    let specs: Vec<ChainSpec> = (0..rounds)
        .map(|i| {
            let n = 1 + i % KINDS.len();
            let rotated: Vec<EffectKind> = (0..n).map(|j| KINDS[(i + j) % KINDS.len()]).collect();
            chain_of(&rotated)
        })
        .collect();
    let mut chains: Vec<Option<Box<Chain>>> = specs
        .iter()
        .map(|s| Some(Box::new(Chain::new(s, SR))))
        .collect();
    let mut shaper = Shaper::new(SR);
    let mut shaping = Shaping {
        limit: Limit::On,
        ..Shaping::FLAT
    };
    shaper.set(shaping);
    let signal = input(BLOCK * 8);
    // Room for what comes back, so collecting it does not allocate either.
    let mut returned: Vec<Box<Chain>> = Vec::with_capacity(rounds + 1);

    ARMED.with(|a| a.set(true));
    let mut peak = 0.0f32;
    for (i, spec) in specs.iter().enumerate() {
        shaping.chain = *spec;
        shaper.set(shaping);
        if let Some(chain) = chains[i].take() {
            // A chain still fading out refuses the next one; the callback tries again later.
            if let Err(chain) = shaper.swap_chain(chain) {
                chains[i] = Some(chain);
            }
        }
        for block in signal.chunks(BLOCK) {
            for &x in block {
                let (l, r) = shaper.process(x, SR);
                peak = peak.max(l.abs()).max(r.abs());
            }
            if let Some(old) = shaper.take_retired() {
                returned.push(old);
            }
        }
    }
    ARMED.with(|a| a.set(false));
    assert_eq!(
        COUNT.with(|c| c.replace(0)),
        0,
        "the callback allocated or freed"
    );
    assert!(peak <= LIMIT_CEILING + 1e-6, "peak {peak}");
    assert!(!returned.is_empty(), "chains left the callback");
}
