//! The bundled set (LH-BIB-08): a plan from the first groove on an open string to odd meters,
//! reading in all keys and slap. Files are numbered by their place in the plan and progress is
//! keyed by the file's path: new exercises are only ever appended, never moved or renamed.
//!
//! ```text
//! 01–45   first set (levels 1–5)
//! 46–     appended per level: 1 Einstieg … 4 Groove, 5 Walking Bass, 6 Taktarten, 7 Lesen,
//!         8 Technik (levels 4 and 5 swapped in Runde 12; numbers unchanged),
//!         9 Stücke (pieces in a form to play through, 24.10.)
//! ```

use super::*;

const LEVEL_1: &str = "1 Einstieg";
const LEVEL_2: &str = "2 Grundlagen";
const LEVEL_3: &str = "3 Fortgeschritten";
// Runde 12: Groove before Walking Bass (walking is harder); the files keep their numbers.
const LEVEL_GROOVE: &str = "4 Groove";
const LEVEL_WALKING: &str = "5 Walking Bass";
const LEVEL_6: &str = "6 Taktarten";
const LEVEL_7: &str = "7 Lesen";
const LEVEL_8: &str = "8 Technik";
const LEVEL_9: &str = "9 Stücke";

/// Key roots as pitch classes.
const C: u8 = 0;
const DB: u8 = 1;
const D: u8 = 2;
const EB: u8 = 3;
const E: u8 = 4;
const F: u8 = 5;
const G: u8 = 7;
const AB: u8 = 8;
const A: u8 = 9;
const BB: u8 = 10;
const B: u8 = 11;

/// Note value of scales and finger exercises.
#[derive(Clone, Copy)]
enum Step {
    Quarters,
    Eighths,
}

/// Learning order of level 1 where it differs from the numbers (appended files keep theirs):
/// the pulse on open strings, roots, root and fifth, the fretting hand, then scales (DIDAKTIK
/// I4; QA 27.10.: the timing-only grooves 46 and 52 came after the scales).
const LEVEL_1_ORDER: [usize; 16] = [1, 2, 3, 46, 52, 47, 4, 49, 48, 53, 50, 5, 6, 51, 7, 8];

/// Sort key of bundled exercise `number` inside its level: level 1 in learning order, the
/// others by number.
pub fn plan_place(number: usize) -> (usize, usize) {
    let place = LEVEL_1_ORDER.iter().position(|&n| n == number);
    (place.unwrap_or(LEVEL_1_ORDER.len()), number)
}

/// The bundled set (LH-BIB-08): (level folder, file name, exercise), easiest first.
pub fn bundled() -> Vec<(&'static str, String, Exercise)> {
    bundled_specs()
        .into_iter()
        .enumerate()
        .map(|(i, (level, spec))| {
            let exercise = generate(&spec);
            let file = format!("{:02} {}.mid", i + 1, file_name(&exercise.title));
            (level, file, exercise)
        })
        .collect()
}

/// The plan of the bundled set: (level folder, spec); file "NN …" is entry NN − 1.
pub fn bundled_specs() -> Vec<(&'static str, ExerciseSpec)> {
    use super::Sequence as Q;
    use ExerciseKind::*;
    use Fingers as Fi;
    use Progression as P;
    use Rhythm::*;
    use ScaleKind as S;
    use Style as St;
    vec![
        // ── First set (01–45) ────────────────────────────────────────────
        (LEVEL_1, open_groove(Quarters, E, 70.0)),
        (LEVEL_1, open_groove(Quarters, A, 70.0)),
        (LEVEL_1, open_groove(Quarters, D, 70.0)),
        (LEVEL_1, groove(P::Minor, Quarters, A, 70.0)),
        (LEVEL_1, scale(S::MinorPentatonic, E, 60.0, Step::Quarters)),
        (LEVEL_1, scale(S::MinorPentatonic, A, 60.0, Step::Quarters)),
        (LEVEL_1, scale(S::Major, G, 60.0, Step::Quarters)),
        (LEVEL_1, scale(S::Major, C, 60.0, Step::Quarters)),
        (LEVEL_2, groove(P::Tonic, Eighths, E, 85.0)),
        (LEVEL_2, groove(P::Blues, Quarters, E, 85.0)),
        (LEVEL_2, groove(P::Blues, Quarters, A, 85.0)),
        (LEVEL_2, groove(P::Minor, WithRests, E, 80.0)),
        (LEVEL_2, scale(S::Major, C, 80.0, Step::Eighths)),
        (LEVEL_2, scale(S::Major, F, 80.0, Step::Eighths)),
        (LEVEL_2, scale(S::Major, G, 80.0, Step::Eighths)),
        (LEVEL_2, scale(S::MinorPentatonic, E, 80.0, Step::Eighths)),
        (LEVEL_2, scale(S::Minor, A, 80.0, Step::Eighths)),
        (LEVEL_3, groove(P::Blues, Shuffle, E, 90.0)),
        (LEVEL_3, groove(P::Blues, Shuffle, A, 90.0)),
        (LEVEL_3, groove(P::Minor, Sixteenths, E, 85.0)),
        (LEVEL_3, groove(P::Tonic, Octaves, F, 95.0)),
        (LEVEL_3, scale(S::Blues, E, 90.0, Step::Eighths)),
        (LEVEL_3, scale(S::Blues, A, 90.0, Step::Eighths)),
        (LEVEL_3, chords(Arpeggio(P::TwoFiveOne), C, 90.0)),
        (LEVEL_3, chords(Arpeggio(P::TwoFiveOne), F, 90.0)),
        (LEVEL_3, chords(Arpeggio(P::Blues), A, 90.0)),
        (LEVEL_WALKING, chords(WalkingBass(P::TwoFiveOne), C, 100.0)),
        (LEVEL_WALKING, chords(WalkingBass(P::TwoFiveOne), F, 100.0)),
        (LEVEL_WALKING, chords(WalkingBass(P::TwoFiveOne), BB, 100.0)),
        (LEVEL_WALKING, chords(WalkingBass(P::TwoFiveOne), EB, 110.0)),
        (LEVEL_WALKING, chords(WalkingBass(P::Blues), F, 110.0)),
        (LEVEL_WALKING, chords(WalkingBass(P::Blues), BB, 110.0)),
        (LEVEL_WALKING, chords(WalkingBass(P::Blues), G, 120.0)),
        (LEVEL_WALKING, chords(WalkingBass(P::Minor), D, 110.0)),
        // Groove level (LH-LRN-01): with the two levels above, twenty grooves on drums.
        (LEVEL_GROOVE, groove(P::Minor, Eighths, D, 90.0)),
        (LEVEL_GROOVE, groove(P::Blues, Eighths, G, 95.0)),
        (LEVEL_GROOVE, groove(P::Tonic, WithRests, A, 90.0)),
        (LEVEL_GROOVE, groove(P::Blues, WithRests, E, 95.0)),
        (LEVEL_GROOVE, groove(P::Blues, Shuffle, G, 100.0)),
        (LEVEL_GROOVE, groove(P::Minor, Shuffle, A, 100.0)),
        (LEVEL_GROOVE, groove(P::Tonic, Sixteenths, E, 90.0)),
        (LEVEL_GROOVE, groove(P::Minor, Octaves, D, 100.0)),
        (LEVEL_GROOVE, groove(P::TwoFiveOne, Sixteenths, F, 95.0)),
        (LEVEL_GROOVE, groove(P::Tonic, Ghost, E, 80.0)),
        (LEVEL_GROOVE, groove(P::Minor, Ghost, A, 85.0)),
        // ── 1 Einstieg: rests, first chord changes on open strings, first frets (P1) ──
        (LEVEL_1, open_groove(WithRests, E, 65.0)),
        (LEVEL_1, groove(P::OneFourFive, Quarters, A, 70.0)),
        (LEVEL_1, groove(P::Tonic, RootFifth, A, 70.0)),
        (LEVEL_1, groove(P::Pop, Quarters, G, 70.0)),
        (LEVEL_1, fingers(Fi::Straight, 5, 60.0, Step::Quarters)),
        (LEVEL_1, scale(S::MajorPentatonic, G, 60.0, Step::Quarters)),
        (LEVEL_1, open_groove(Eighths, A, 70.0)),
        (LEVEL_1, groove(P::OneFourFive, RootFifth, D, 70.0)),
        // ── 2 Grundlagen: eighths over changes, triads, dead notes ──────────
        (LEVEL_2, groove(P::OneFourFive, Eighths, E, 85.0)),
        (LEVEL_2, groove(P::Pop, Eighths, C, 85.0)),
        (LEVEL_2, groove(P::OneFourFive, RootFifthOctave, A, 80.0)),
        (LEVEL_2, chords(Triads(P::OneFourFive), G, 70.0)),
        (LEVEL_2, chords(Triads(P::Minor), A, 70.0)),
        (LEVEL_2, fingers(Fi::Straight, 5, 70.0, Step::Eighths)),
        (LEVEL_2, fingers(Fi::Crossed, 5, 60.0, Step::Quarters)),
        (LEVEL_2, groove(P::Tonic, DeadNotes, A, 80.0)),
        (LEVEL_2, scale(S::MajorPentatonic, C, 80.0, Step::Eighths)),
        (LEVEL_2, scale(S::Minor, E, 80.0, Step::Eighths)),
        (LEVEL_2, groove(P::Blues, RootFifth, A, 85.0)),
        // ── 3 Fortgeschritten: seventh chords, modes, syncopation, triplets ──
        (LEVEL_3, chords(Arpeggio(P::Turnaround), C, 85.0)),
        (LEVEL_3, chords(Arpeggio(P::MinorTwoFiveOne), A, 85.0)),
        (LEVEL_3, chords(Arpeggio(P::Pop), D, 85.0)),
        (LEVEL_3, chords(Triads(P::Pop), F, 80.0)),
        (LEVEL_3, scale(S::Dorian, D, 80.0, Step::Eighths)),
        (LEVEL_3, scale(S::Mixolydian, G, 80.0, Step::Eighths)),
        (LEVEL_3, sequence(S::Major, Q::Thirds, C, 80.0)),
        (LEVEL_3, groove(P::Pop, Tresillo, A, 90.0)),
        (LEVEL_3, groove(P::Tonic, Offbeat, E, 90.0)),
        (LEVEL_3, groove(P::Tonic, Triplets, A, 80.0)),
        (LEVEL_3, groove(P::Blues, DeadNotes, A, 90.0)),
        (LEVEL_3, groove(P::Blues, RootFifthOctave, E, 90.0)),
        (LEVEL_3, fingers(Fi::SwapTop, 5, 80.0, Step::Eighths)),
        (
            LEVEL_3,
            ramp(groove(P::OneFourFive, Eighths, E, 80.0), 4, 10.0),
        ),
        // ── 5 Walking Bass: minor II-V-I, turnaround, blues in more keys ─────
        (
            LEVEL_WALKING,
            chords(WalkingBass(P::MinorTwoFiveOne), A, 100.0),
        ),
        (
            LEVEL_WALKING,
            chords(WalkingBass(P::MinorTwoFiveOne), D, 105.0),
        ),
        (LEVEL_WALKING, chords(WalkingBass(P::Turnaround), F, 100.0)),
        (LEVEL_WALKING, chords(WalkingBass(P::Turnaround), BB, 110.0)),
        (LEVEL_WALKING, chords(WalkingBass(P::Turnaround), C, 120.0)),
        (LEVEL_WALKING, chords(WalkingBass(P::Blues), C, 110.0)),
        (LEVEL_WALKING, chords(WalkingBass(P::Blues), A, 120.0)),
        (LEVEL_WALKING, chords(WalkingBass(P::Blues), E, 120.0)),
        (LEVEL_WALKING, chords(WalkingBass(P::TwoFiveOne), G, 120.0)),
        (
            LEVEL_WALKING,
            ramp(chords(WalkingBass(P::Blues), F, 100.0), 3, 10.0),
        ),
        // ── 4 Groove: funk, syncopation, slow blues in triplets ─────────────
        (LEVEL_GROOVE, groove(P::Tonic, Funk, E, 85.0)),
        (LEVEL_GROOVE, groove(P::Minor, Funk, A, 90.0)),
        (LEVEL_GROOVE, groove(P::Blues, Funk, E, 90.0)),
        (LEVEL_GROOVE, groove(P::Pop, Offbeat, C, 100.0)),
        (LEVEL_GROOVE, groove(P::Blues, Tresillo, E, 95.0)),
        (LEVEL_GROOVE, groove(P::OneFourFive, Ghost, A, 90.0)),
        (LEVEL_GROOVE, groove(P::Pop, Sixteenths, G, 95.0)),
        (LEVEL_GROOVE, groove(P::Pop, RootFifthOctave, F, 100.0)),
        (LEVEL_GROOVE, groove(P::Blues, Triplets, G, 70.0)),
        (LEVEL_GROOVE, groove(P::Minor, DeadNotes, E, 100.0)),
        (
            LEVEL_GROOVE,
            ramp(groove(P::Tonic, Sixteenths, E, 80.0), 5, 5.0),
        ),
        // ── 6 Taktarten: 3/4, 6/8, 5/4, 7/8 (P3) ────────────────────────────
        (LEVEL_6, groove(P::Tonic, Waltz, D, 90.0)),
        (LEVEL_6, groove(P::OneFourFive, Waltz, G, 100.0)),
        (LEVEL_6, groove(P::Pop, Waltz, C, 110.0)),
        (LEVEL_6, groove(P::Tonic, SixEight, A, 90.0)),
        (LEVEL_6, groove(P::Minor, SixEight, E, 90.0)),
        (LEVEL_6, groove(P::OneFourFive, SixEight, D, 100.0)),
        (LEVEL_6, groove(P::Tonic, FiveFour, E, 100.0)),
        (LEVEL_6, groove(P::Minor, FiveFour, EB, 110.0)),
        (LEVEL_6, groove(P::TwoFiveOne, FiveFour, C, 100.0)),
        (LEVEL_6, groove(P::Tonic, SevenEight, E, 80.0)),
        (LEVEL_6, groove(P::Minor, SevenEight, A, 90.0)),
        (LEVEL_6, groove(P::OneFourFive, SevenEight, A, 100.0)),
        // ── 7 Lesen: all major keys, sequences, minor and modes (P3, P4) ────
        (LEVEL_7, scale(S::Major, BB, 90.0, Step::Eighths)),
        (LEVEL_7, scale(S::Major, EB, 90.0, Step::Eighths)),
        (LEVEL_7, scale(S::Major, AB, 90.0, Step::Eighths)),
        (LEVEL_7, scale(S::Major, DB, 90.0, Step::Eighths)),
        (LEVEL_7, scale(S::Major, D, 90.0, Step::Eighths)),
        (LEVEL_7, scale(S::Major, A, 90.0, Step::Eighths)),
        (LEVEL_7, scale(S::Major, E, 90.0, Step::Eighths)),
        (LEVEL_7, scale(S::Major, B, 90.0, Step::Eighths)),
        (LEVEL_7, sequence(S::Major, Q::Thirds, F, 90.0)),
        (LEVEL_7, sequence(S::Major, Q::Thirds, BB, 90.0)),
        (LEVEL_7, sequence(S::Major, Q::Fours, G, 90.0)),
        (LEVEL_7, sequence(S::Major, Q::Fours, EB, 90.0)),
        (LEVEL_7, sequence(S::Minor, Q::Thirds, D, 90.0)),
        (LEVEL_7, scale(S::HarmonicMinor, A, 80.0, Step::Eighths)),
        (LEVEL_7, scale(S::HarmonicMinor, D, 80.0, Step::Eighths)),
        (LEVEL_7, scale(S::Phrygian, E, 80.0, Step::Eighths)),
        (LEVEL_7, scale(S::Lydian, F, 80.0, Step::Eighths)),
        (LEVEL_7, scale(S::Locrian, B, 80.0, Step::Eighths)),
        (LEVEL_7, chords(Triads(P::TwoFiveOne), AB, 90.0)),
        (LEVEL_7, chords(Arpeggio(P::TwoFiveOne), DB, 90.0)),
        (LEVEL_7, chords(Arpeggio(P::MinorTwoFiveOne), G, 90.0)),
        (LEVEL_7, chords(Triads(P::OneFourFive), B, 90.0)),
        // ── 8 Technik: slap, finger orders, tempo ramps ──────────────────────
        (LEVEL_8, groove(P::Tonic, Slap, E, 80.0)),
        (LEVEL_8, groove(P::Minor, Slap, A, 85.0)),
        (LEVEL_8, groove(P::OneFourFive, Slap, A, 90.0)),
        (LEVEL_8, groove(P::Blues, Slap, E, 90.0)),
        (LEVEL_8, fingers(Fi::Wide, 5, 80.0, Step::Eighths)),
        (LEVEL_8, fingers(Fi::Return, 3, 80.0, Step::Eighths)),
        (LEVEL_8, fingers(Fi::Reverse, 7, 80.0, Step::Eighths)),
        (LEVEL_8, fingers(Fi::Crossed, 1, 70.0, Step::Eighths)),
        (
            LEVEL_8,
            ramp(fingers(Fi::Straight, 5, 80.0, Step::Eighths), 4, 10.0),
        ),
        (LEVEL_8, ramp(groove(P::Tonic, Funk, E, 80.0), 3, 10.0)),
        (LEVEL_8, groove(P::Minor, Ghost, D, 95.0)),
        // ── 9 Stücke: whole pieces, easiest first ────────────────────────────
        (LEVEL_9, piece(St::Ballad, E, 70.0)),
        (LEVEL_9, piece(St::Pop, G, 90.0)),
        (LEVEL_9, piece(St::Rock, A, 110.0)),
        (LEVEL_9, piece(St::Shuffle, E, 90.0)),
        (LEVEL_9, piece(St::Pop, D, 100.0)),
        (LEVEL_9, piece(St::Funk, A, 95.0)),
    ]
}

/// A spec played twice, a 12-bar blues once (it is long enough).
fn spec(kind: ExerciseKind, key: u8, bpm: f64) -> ExerciseSpec {
    let blues = matches!(
        kind,
        ExerciseKind::Groove(Progression::Blues, _)
            | ExerciseKind::Arpeggio(Progression::Blues)
            | ExerciseKind::WalkingBass(Progression::Blues)
            | ExerciseKind::Triads(Progression::Blues)
    );
    ExerciseSpec {
        kind,
        key,
        bpm,
        slow: false,
        repeats: if blues { 1 } else { 2 },
        ramp_bpm: 0.0,
        judging: Judging::PitchAndTime,
    }
}

/// A piece is played once: its form already repeats verse and chorus.
fn piece(style: Style, key: u8, bpm: f64) -> ExerciseSpec {
    ExerciseSpec {
        repeats: 1,
        ..spec(ExerciseKind::Song(style, Level::Medium), key, bpm)
    }
}

fn groove(progression: Progression, rhythm: Rhythm, key: u8, bpm: f64) -> ExerciseSpec {
    spec(ExerciseKind::Groove(progression, rhythm), key, bpm)
}

/// A groove on the open string judged by time only (DIDAKTIK I4): the pulse before the frets.
fn open_groove(rhythm: Rhythm, key: u8, bpm: f64) -> ExerciseSpec {
    ExerciseSpec {
        judging: Judging::TimeOnly,
        ..groove(Progression::Tonic, rhythm, key, bpm)
    }
}

fn chords(kind: ExerciseKind, key: u8, bpm: f64) -> ExerciseSpec {
    spec(kind, key, bpm)
}

fn scale(scale: ScaleKind, key: u8, bpm: f64, step: Step) -> ExerciseSpec {
    paced(spec(ExerciseKind::Scale(scale), key, bpm), step)
}

fn sequence(scale: ScaleKind, sequence: Sequence, key: u8, bpm: f64) -> ExerciseSpec {
    spec(ExerciseKind::Sequence(scale, sequence), key, bpm)
}

fn fingers(fingers: Fingers, fret: u8, bpm: f64, step: Step) -> ExerciseSpec {
    paced(spec(ExerciseKind::Chromatic(fingers), fret, bpm), step)
}

fn paced(spec: ExerciseSpec, step: Step) -> ExerciseSpec {
    ExerciseSpec {
        slow: matches!(step, Step::Quarters),
        ..spec
    }
}

/// `passes` passes, each `step_bpm` faster than the one before.
fn ramp(spec: ExerciseSpec, passes: u8, step_bpm: f64) -> ExerciseSpec {
    ExerciseSpec {
        repeats: passes,
        ramp_bpm: step_bpm,
        ..spec
    }
}
