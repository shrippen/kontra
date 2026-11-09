//! The exercise ladder of a lesson's practice block (LH-LRN-17): five stages from the figure
//! alone to mixed keys, each about a quarter more notes per second than the one before.
//!
//! ```text
//! stage     single   consolidate   vary         context        mixed
//! tempo     × 0.64   × 0.8         × 1          × 1.25         × 1.5625    (written tempo: stage 3)
//! content   1 pass   2× passes     key ± 4th    I-IV-V chords  key from the seed
//! ```
//!
//! Only the tempo raises the note rate; the subdivision never changes with it. The seed (one
//! per lesson) picks the keys, so the same lesson always builds the same ladder.

use super::{build, generate, Exercise, ExerciseKind, ExerciseSpec, Progression};

/// The stages in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Single,
    Consolidate,
    Vary,
    Context,
    Mixed,
}

impl Stage {
    pub const ALL: [Stage; 5] = [
        Stage::Single,
        Stage::Consolidate,
        Stage::Vary,
        Stage::Context,
        Stage::Mixed,
    ];

    /// Text key part: "single", "consolidate" …
    pub fn key(self) -> &'static str {
        match self {
            Stage::Single => "single",
            Stage::Consolidate => "consolidate",
            Stage::Vary => "vary",
            Stage::Context => "context",
            Stage::Mixed => "mixed",
        }
    }

    /// Hit rate from which the next stage makes sense: 60 % on the first, 80 % on the last.
    pub fn threshold(self) -> f64 {
        FIRST_THRESHOLD + THRESHOLD_STEP * self.index() as f64
    }

    fn index(self) -> usize {
        Stage::ALL.iter().position(|s| *s == self).unwrap_or(0)
    }
}

/// Note rate from stage to stage.
const RATE_STEP: f64 = 1.25;

/// The written tempo is the middle stage's (vary).
const WRITTEN_STAGE: i32 = 2;

const FIRST_THRESHOLD: f64 = 0.6;
const THRESHOLD_STEP: f64 = 0.05;

/// "Vary" moves the key a fourth up or down.
const FOURTH: i32 = 5;
const SEMITONES: u64 = 12;

/// The exercise of `stage` built from a block's `spec`; `seed` is the lesson's.
pub fn stage_spec(spec: &ExerciseSpec, stage: Stage, seed: u64) -> ExerciseSpec {
    let mut out = *spec;
    let steps = stage.index() as i32 - WRITTEN_STAGE;
    out.bpm = (spec.bpm * RATE_STEP.powi(steps)).round();
    out.ramp_bpm = 0.0;

    let repeats = spec.repeats.max(1);
    match stage {
        Stage::Single => out.repeats = 1,
        Stage::Consolidate => out.repeats = repeats.saturating_mul(2),
        Stage::Vary => out.key = moved(spec.key, fourth(seed)),
        Stage::Context => out.kind = in_context(spec.kind),
        Stage::Mixed => {
            out.kind = in_context(spec.kind);
            // Any key but the written one.
            let shift = 1 + seed.wrapping_mul(SPREAD) % (SEMITONES - 1);
            out.key = moved(spec.key, shift as i32);
        }
    }
    out
}

/// The file of `stage`: `stage_spec` generated; the mixed stage plays one pass each of the
/// figure, the figure a fourth away and the figure in context, in an order from the seed.
pub fn stage_exercise(spec: &ExerciseSpec, stage: Stage, seed: u64) -> Exercise {
    let mixed = stage_spec(spec, stage, seed);
    if stage != Stage::Mixed || matches!(spec.kind, ExerciseKind::Song(..)) {
        return generate(&mixed);
    }

    let figure = ExerciseSpec {
        kind: spec.kind,
        ..mixed
    };
    let varied = ExerciseSpec {
        key: moved(mixed.key, fourth(seed)),
        ..figure
    };
    let mut passes = [figure, varied, mixed];
    // Seed → one of the six orders: rotate, then maybe swap the last two.
    passes.rotate_left((seed % 3) as usize);
    if (seed / 3).is_multiple_of(2) {
        passes.swap(1, 2);
    }
    build(&mixed, &passes, MIXED_TITLE)
}

/// Before the title of the mixed file: its own name and progress, not the context stage's.
const MIXED_TITLE: &str = "Gemischt: ";

/// A fourth up for an even seed, down for an odd one.
fn fourth(seed: u64) -> i32 {
    if seed.is_multiple_of(2) {
        FOURTH
    } else {
        -FOURTH
    }
}

/// Spreads neighbouring seeds over the keys.
const SPREAD: u64 = 0x9E37_79B9_7F4A_7C15;

/// A seed for a lesson: FNV-1a over its id.
pub fn lesson_seed(id: &str) -> u64 {
    id.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, b| {
        (hash ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// A pitch class moved by `shift` semitones.
fn moved(key: u8, shift: i32) -> u8 {
    (i32::from(key) + shift).rem_euclid(SEMITONES as i32) as u8
}

/// One chord becomes I-IV-V: the same figure among other chords. Other kinds stay.
fn in_context(kind: ExerciseKind) -> ExerciseKind {
    let wider = |p: Progression| match p {
        Progression::Tonic => Progression::OneFourFive,
        other => other,
    };
    match kind {
        ExerciseKind::Arpeggio(p) => ExerciseKind::Arpeggio(wider(p)),
        ExerciseKind::WalkingBass(p) => ExerciseKind::WalkingBass(wider(p)),
        ExerciseKind::Groove(p, r) => ExerciseKind::Groove(wider(p), r),
        ExerciseKind::Triads(p) => ExerciseKind::Triads(wider(p)),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exercise::bundled_specs;
    use crate::{import_bytes, FileKind};

    fn arrangement(exercise: &Exercise) -> kontra_model::Arrangement {
        let imported = import_bytes(&exercise.midi, FileKind::Midi).unwrap();
        imported.arrangement(imported.suggested().unwrap()).unwrap()
    }

    /// Notes per second of a generated exercise.
    fn note_rate(exercise: &Exercise) -> f64 {
        let a = arrangement(exercise);
        let last = a.notes.iter().map(|n| n.start_ticks).max().unwrap();
        let first = a.notes.iter().map(|n| n.start_ticks).min().unwrap();
        let span_s = a.tempo.seconds_at(last) - a.tempo.seconds_at(first);
        (a.notes.len() - 1) as f64 / span_s
    }

    #[test]
    fn each_stage_has_a_quarter_more_notes_per_second() {
        // Groove on one chord (1), a scale (5), an arpeggio over a progression (11).
        let specs = bundled_specs();
        for number in [1, 5, 11] {
            let spec = specs[number - 1].1;
            let rates: Vec<f64> = Stage::ALL
                .iter()
                .map(|s| note_rate(&stage_exercise(&spec, *s, 7)))
                .collect();
            for pair in rates.windows(2) {
                let step = pair[1] / pair[0];
                assert!((1.15..=1.35).contains(&step), "{number}: {rates:?}");
            }
            assert_eq!(stage_spec(&spec, Stage::Vary, 7).bpm, spec.bpm.round());
        }
    }

    #[test]
    fn the_same_lesson_builds_the_same_ladder() {
        let spec = bundled_specs()[0].1;
        let seed = lesson_seed("viertel");
        assert_eq!(seed, lesson_seed("viertel"));
        for stage in Stage::ALL {
            let a = stage_exercise(&spec, stage, seed);
            let b = stage_exercise(&spec, stage, seed);
            assert_eq!(a, b);
        }
        // Mixed never plays the written key.
        for seed in 0..40 {
            assert_ne!(stage_spec(&spec, Stage::Mixed, seed).key, spec.key);
        }
    }

    /// The mixed stage holds the figure (one chord), a second key and the I-IV-V of the context
    /// stage in one file, each key signed where it starts.
    #[test]
    fn the_mixed_stage_mixes_the_earlier_stages() {
        let spec = bundled_specs()[0].1;
        assert!(matches!(
            spec.kind,
            ExerciseKind::Groove(Progression::Tonic, _)
        ));
        for seed in [lesson_seed("viertel"), 1, 2, 3] {
            let mixed = stage_exercise(&spec, Stage::Mixed, seed);
            let context = stage_spec(&spec, Stage::Mixed, seed);
            assert!(mixed.title.starts_with(MIXED_TITLE), "{}", mixed.title);
            let a = arrangement(&mixed);
            // Chords per bar: the one-chord figure repeats its chord, the context changes it.
            let chords: Vec<&str> = a
                .markers
                .iter()
                .filter(|m| m.kind == kontra_model::MarkerKind::Chord)
                .map(|m| m.name.as_str())
                .collect();
            let still = chords.windows(2).filter(|w| w[0] == w[1]).count();
            let moving = chords.windows(2).filter(|w| w[0] != w[1]).count();
            assert!(still >= 3 && moving >= 3, "{seed}: {chords:?}");
            let mut fifths: Vec<i8> = a.bars.iter().map(|b| b.key_fifths).collect();
            fifths.dedup();
            assert!(fifths.len() >= 2, "{seed}: {fifths:?}");
            assert!((a.tempo.bpm_at(0) - context.bpm).abs() < 0.01, "{seed}");
        }
    }

    #[test]
    fn thresholds_rise_from_60_to_80() {
        let t: Vec<f64> = Stage::ALL.iter().map(|s| s.threshold()).collect();
        assert!(
            (t[0] - 0.6).abs() < 1e-9 && (t[4] - 0.8).abs() < 1e-9,
            "{t:?}"
        );
    }
}
