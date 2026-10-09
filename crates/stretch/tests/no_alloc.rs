//! `push`, `pull`, `set_speed` and `reset` must not allocate (audio thread).
//!
//! A counting global allocator needs `unsafe impl GlobalAlloc`; it only
//! forwards to the system allocator, so it is sound.
#![allow(unsafe_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use kontra_stretch::Stretcher;

struct Counting;

thread_local! {
    /// Allocations made by this thread while armed.
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

#[test]
fn processing_does_not_allocate() {
    const SR: u32 = 48_000;
    const BLOCK: usize = 480;

    // The counter itself works.
    ARMED.with(|a| a.set(true));
    drop(std::hint::black_box(Box::new(1_u64)));
    ARMED.with(|a| a.set(false));
    assert_eq!(COUNT.with(|c| c.replace(0)), 1);

    // Stereo test signal with clicks so transient runs and resets happen.
    let len = SR as usize * 6;
    let left: Vec<f32> = (0..len)
        .map(|n| {
            let t = n as f32 / SR as f32;
            let click = if n % 12_000 == 0 { 0.8 } else { 0.0 };
            0.3 * (std::f32::consts::TAU * 55.0 * t).sin() + click
        })
        .collect();
    let right: Vec<f32> = left.iter().map(|v| 0.5 * v).collect();

    let mut st = Stretcher::new(2, SR, BLOCK);
    let mut out_l = vec![0.0; BLOCK];
    let mut out_r = vec![0.0; BLOCK];
    let speeds = [0.5, 0.75, 0.25, 1.0, 1.5, 0.6];

    let mut fed = 0;
    let mut made = 0;
    let mut armed_blocks = 0;
    for block in 0.. {
        if fed + BLOCK > len {
            break;
        }

        // Warm-up: the first second runs unarmed.
        let armed = fed > SR as usize;
        ARMED.with(|a| a.set(armed));

        st.set_speed(speeds[(block / 100) % speeds.len()]);
        if block == 400 {
            st.reset();
        }

        // One audio callback: fill one output block.
        let mut done = 0;
        while done < BLOCK {
            let got = st.pull(&mut [&mut out_l[done..], &mut out_r[done..]]);
            done += got;
            if got > 0 {
                continue;
            }
            let need = st.required_input().clamp(1, BLOCK).min(len - fed);
            if need == 0 {
                break;
            }
            fed += st.push(&[&left[fed..fed + need], &right[fed..fed + need]]);
        }
        made += done;

        ARMED.with(|a| a.set(false));
        if armed {
            armed_blocks += 1;
        }
    }

    let count = COUNT.with(Cell::get);
    println!("{armed_blocks} armed blocks, {made} samples out, {count} allocations");
    assert!(armed_blocks > 100);
    assert_eq!(count, 0);
}
