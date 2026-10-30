//! Journey tests (planung/QA.md, marked ⚙): each plays a journey through the app core the way the UI
//! does, with recordings rendered on the fly instead of an instrument. No sound card needed;
//! without an output device Kontra runs silently. Run: `cargo test -p kontra-appcore --test journeys`.

// Test helpers outside #[test] functions may fail loudly too.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

mod a1_first_start;
mod a2_tuning;
mod a3_latency;
mod b1_song;
mod b2_wait;
mod b3_workshop;
mod b4_fretless;
mod b6_downloads;
mod c10_fret_quiz;
mod c2_tendency;
mod c4_ear;
mod c5_techniques;
mod c6_probe;
mod c8_pulse;
mod c9_blocks;
mod d1_backing;
mod d2_package;
mod e1_midi;
mod e3_settling;
mod f4_input;
mod f_restart;
mod g1_lesson;
mod g2_glossary;
mod g3_review;
