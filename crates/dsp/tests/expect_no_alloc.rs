//! The tracker with expected strikes (R2) runs on the analysis thread: `expect` and `push`
//! must not allocate once it is set up.
//!
//! A counting global allocator needs `unsafe impl GlobalAlloc`; it only
//! forwards to the system allocator, so it is sound.
#![allow(unsafe_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use kontra_dsp::synth::{render, SynthNote, ToneShape};
use kontra_dsp::{NoteTracker, TrackerConfig};
use kontra_model::{ExpectedStrike, ExpectedTone, InputSource, Instrument, Pitch, A4_HZ};

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
fn expecting_does_not_allocate() {
    const SR: f64 = 48_000.0;
    const BLOCK: usize = 480;
    const SPACING_S: f64 = 0.25;
    const AHEAD_S: f64 = 0.25;

    // The counter itself works.
    ARMED.with(|a| a.set(true));
    drop(std::hint::black_box(Box::new(1_u64)));
    ARMED.with(|a| a.set(false));
    assert_eq!(COUNT.with(|c| c.replace(0)), 1);

    // A line of notes, each expected; every third a semitone off (the wrong path too).
    let names = ["A2", "D3", "G2", "C3", "E2", "B2", "F2", "A2"];
    let notes: Vec<SynthNote> = (0..32)
        .map(|i| SynthNote {
            onset_s: 0.2 + i as f64 * SPACING_S,
            hz: Pitch::parse(names[i % names.len()])
                .expect("pitch")
                .hz(A4_HZ),
            duration_s: SPACING_S * 0.9,
            velocity: 0.7,
        })
        .collect();
    let signal = render(&notes, SR, 8.5, ToneShape::Rich, 21);
    let strikes: Vec<ExpectedStrike> = notes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let midi = kontra_model::hz_to_midi(n.hz, A4_HZ).round() as u8 + u8::from(i % 3 == 0);
            ExpectedStrike::new(n.onset_s, 0.09).with(ExpectedTone { midi, string: None })
        })
        .collect();

    let config = TrackerConfig::for_instrument(&Instrument::default_bass(), InputSource::Line);
    let mut tracker = NoteTracker::new(SR, config);
    let mut next = 0;
    let mut found = 0;
    for (i, block) in signal.chunks(BLOCK).enumerate() {
        // Warm-up: the first second runs unarmed.
        let end_s = ((i + 1) * BLOCK) as f64 / SR;
        ARMED.with(|a| a.set(end_s > 1.0));
        while let Some(s) = strikes.get(next).filter(|s| s.onset_s < end_s + AHEAD_S) {
            tracker.expect(s);
            next += 1;
        }
        if i == 500 {
            tracker.forget_expected();
        }
        found += tracker.push(block).len();
        ARMED.with(|a| a.set(false));
    }
    assert!(found >= notes.len() - 1, "{found} notes");
    assert_eq!(COUNT.with(|c| c.get()), 0, "allocations while tracking");
}

/// R8: announced double stops are judged on the analysis thread too, without allocating.
#[test]
fn double_stops_do_not_allocate() {
    const SR: f64 = 48_000.0;
    const BLOCK: usize = 480;
    const SPACING_S: f64 = 0.3;
    const AHEAD_S: f64 = 0.25;
    const STRIKES: usize = 24;

    // Fifths on E and A (every second strike an octave, which tells nothing at once), each
    // tone on its own string, mixed.
    let onset = |i: usize| 0.2 + i as f64 * SPACING_S;
    let upper = |i: usize| if i.is_multiple_of(2) { 7 } else { 12 };
    let mut signal = vec![0.0f32; (8.0 * SR) as usize];
    for voice in 0..2u8 {
        let notes: Vec<SynthNote> = (0..STRIKES)
            .map(|i| SynthNote {
                onset_s: onset(i),
                hz: Pitch(33 + voice * upper(i)).hz(A4_HZ),
                duration_s: SPACING_S * 0.9,
                velocity: 0.4,
            })
            .collect();
        let rendered = render(&notes, SR, 8.0, ToneShape::Rich, 5 + u64::from(voice));
        for (out, x) in signal.iter_mut().zip(rendered) {
            *out += x;
        }
    }
    let tone = |midi| ExpectedTone { midi, string: None };
    let strikes: Vec<ExpectedStrike> = (0..STRIKES)
        .map(|i| {
            ExpectedStrike::new(onset(i), 0.09)
                .with(tone(33))
                .with(tone(33 + upper(i)))
        })
        .collect();

    let config = TrackerConfig::for_instrument(&Instrument::default_bass(), InputSource::Line);
    let mut tracker = NoteTracker::new(SR, config);
    let mut next = 0;
    let mut told = 0;
    for (i, block) in signal.chunks(BLOCK).enumerate() {
        let end_s = ((i + 1) * BLOCK) as f64 / SR;
        ARMED.with(|a| a.set(end_s > 1.0));
        while let Some(s) = strikes.get(next).filter(|s| s.onset_s < end_s + AHEAD_S) {
            tracker.expect(s);
            next += 1;
        }
        tracker.push(block);
        told += tracker.double_stops().len();
        ARMED.with(|a| a.set(false));
    }
    assert!(told >= STRIKES - 1, "{told} verdicts");
    assert_eq!(COUNT.with(|c| c.get()), 0, "allocations while judging");
}
