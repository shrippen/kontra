//! The five built-in pieces (Harmonicon: "Etüden wie Lieder", 24.10.) as `PiecePlan`s: a form
//! of intro, verses, choruses, a bridge and an outro, each part with its own chords and groove.
//! `Style::plan` builds the plan for a key; `render` hands it to `piece_render`.
//!
//! ```text
//! Style ──plan(key, length, seed)──▶ PiecePlan (parts: section, bars, groove, times, drums)
//!                                       └─▶ piece_render::pass_of ──▶ Pass, drums, marks
//! ```

use super::chord::Chord;
use super::piece_plan::{
    Checked, CheckedPart, Drums, PieceFeel, PiecePlan, PlanLine, PlanPart, Section,
};
use super::piece_render::Rendered;
use super::variety::Rng;
use super::*;

/// Character of a piece: its form, grooves and tempo range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    /// Slow, quarters and half notes: the first piece to play through (P1).
    Ballad,
    /// I-V-VI-IV verse in eighths, octave chorus, tresillo bridge.
    Pop,
    /// Driving eighths, octave chorus.
    Rock,
    /// Minor, sixteenths with ghost and dead notes.
    Funk,
    /// Twelve-bar blues in shuffle.
    Shuffle,
}

/// How hard a piece is to play: the style's grooves one step simpler or denser, e.g. a rock
/// verse in quarters (easy), eighths (medium) or sixteenths (hard). The form stays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Level {
    Easy,
    #[default]
    Medium,
    Hard,
}

impl Level {
    pub const ALL: [Level; 3] = [Level::Easy, Level::Medium, Level::Hard];

    /// Text key ("easy", "medium", "hard"), the inverse of `from_key`.
    pub fn key(self) -> &'static str {
        match self {
            Level::Easy => "easy",
            Level::Medium => "medium",
            Level::Hard => "hard",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|l| l.key() == key)
    }

    /// The German suffix of the file's title; progress is keyed by it, so each level has its own.
    pub(super) fn label(self) -> &'static str {
        match self {
            Level::Easy => " (leicht)",
            Level::Medium => "",
            Level::Hard => " (schwer)",
        }
    }

    /// `rhythm` at this level. Easy drops subdivisions and syncopation, hard adds them; a
    /// rhythm keeps its meter (all are 4/4 here), so the drums and the form fit.
    pub(super) fn rhythm(self, rhythm: Rhythm) -> Rhythm {
        use Rhythm::*;
        match (self, rhythm) {
            (Level::Medium, r) => r,
            (Level::Easy, Quarters) => RootFifth,
            (Level::Easy, Eighths | Tresillo | RootFifthOctave | DeadNotes) => Quarters,
            (Level::Easy, Octaves | Ghost | Funk) => Eighths,
            (Level::Easy, Shuffle) => Quarters,
            (Level::Hard, RootFifth) => RootFifthOctave,
            (Level::Hard, Quarters) => Eighths,
            (Level::Hard, Eighths) => Sixteenths,
            (Level::Hard, Tresillo | Octaves) => Offbeat,
            (Level::Hard, RootFifthOctave) => Octaves,
            (Level::Hard, DeadNotes) => Funk,
            (Level::Hard, Shuffle) => Triplets,
            (_, r) => r,
        }
    }
}

/// How long the form plays: fewer or more repeats of its verses and choruses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Length {
    /// About one minute.
    Short,
    #[default]
    Normal,
    /// Three to four minutes.
    Long,
}

impl Length {
    pub const ALL: [Length; 3] = [Length::Short, Length::Normal, Length::Long];

    pub fn key(self) -> &'static str {
        match self {
            Length::Short => "short",
            Length::Normal => "normal",
            Length::Long => "long",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|l| l.key() == key)
    }

    /// `base` repeats at `Length::Normal`, one fewer (never below one) or one more.
    fn times(self, base: u32) -> u32 {
        match self {
            Length::Short => base.saturating_sub(1).max(1),
            Length::Normal => base,
            Length::Long => base + 1,
        }
    }

    /// Choruses of the shuffle's blues form (its parts are not repeated, so `times` has no
    /// part to shorten or lengthen).
    fn choruses(self) -> usize {
        match self {
            Length::Short => 2,
            Length::Normal => 4,
            Length::Long => 6,
        }
    }
}

impl Style {
    pub const ALL: [Style; 5] = [
        Style::Ballad,
        Style::Pop,
        Style::Rock,
        Style::Funk,
        Style::Shuffle,
    ];

    pub fn minor(self) -> bool {
        self == Style::Funk
    }

    /// How the chord names read: rock plays root, fifth and octave, so power chords (QA 27.10.:
    /// "Rock in E" with maj7 chords); pop and ballad as triads (round 16: maj7 read like jazz to
    /// a beginner); funk and shuffle keep their seventh chords.
    fn voicing(self) -> Voicing {
        match self {
            Style::Rock => Voicing::Power,
            Style::Pop | Style::Ballad => Voicing::Triad,
            Style::Funk | Style::Shuffle => Voicing::Seventh,
        }
    }

    /// Text key ("ballad", "pop" …), the inverse of `from_key`.
    pub fn key(self) -> &'static str {
        match self {
            Style::Ballad => "ballad",
            Style::Pop => "pop",
            Style::Rock => "rock",
            Style::Funk => "funk",
            Style::Shuffle => "shuffle",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.key() == key)
    }

    /// The German label of the file's title ("Stück Rock").
    fn label(self) -> &'static str {
        match self {
            Style::Ballad => "Stück Ballade",
            Style::Pop => "Stück Pop",
            Style::Rock => "Stück Rock",
            Style::Funk => "Stück Funk",
            Style::Shuffle => "Stück Shuffle",
        }
    }

    /// The bar-by-bar chords of `progression` in the key of `root` (pitch class), named as
    /// this style reads them (`voicing`); built directly, not through a written symbol, so
    /// it never fails to parse (unlike a plan an LLM writes).
    fn chords(self, root: u8, progression: Progression) -> Vec<Chord> {
        let fifths = key_fifths(root, self.minor());
        progression
            .bars()
            .into_iter()
            .map(|(offset, quality)| {
                let pc =
                    u8::try_from((i32::from(root) + offset).rem_euclid(OCTAVE)).unwrap_or_default();
                let spelled = name(i32::from(pc), fifths);
                Chord::built(pc, spelled, quality, self.voicing())
            })
            .collect()
    }

    /// One part with its chords, groove and the rehearsal mark the plan writes for it.
    fn part(
        self,
        root: u8,
        section: Section,
        progression: Progression,
        rhythm: Rhythm,
        times: u32,
        drums: Drums,
    ) -> Built {
        let chords = self.chords(root, progression);
        let bars = chords.iter().map(|c| c.name.clone()).collect();
        Built {
            part: PlanPart {
                section,
                bars,
                times,
                groove: rhythm.key().to_string(),
                line: PlanLine::Roots,
                fill: false,
                drums,
            },
            rhythm,
            chords: chords.into_iter().map(|c| vec![c]).collect(),
        }
    }

    /// The piece's parts in the key of `root` (pitch class); `seed` picks between a part's
    /// progression alternatives where the style has more than one (only for `seed != 0`, so
    /// the plain piece, seed 0, always keeps the first).
    fn built(self, root: u8, length: Length, seed: u64) -> Vec<Built> {
        use Progression as P;
        let mut rng = Rng::new(seed, &[u64::from(root), length as u64]);
        let mut pick = |options: &[Progression]| -> Progression {
            if seed == 0 {
                options[0]
            } else {
                options[rng.below(options.len())]
            }
        };
        let times = |base| length.times(base);
        let part = |section, progression, rhythm, times, drums| {
            self.part(root, section, progression, rhythm, times, drums)
        };

        use Rhythm::*;
        match self {
            Style::Ballad => {
                let verse = pick(&[P::Pop, P::Turnaround]);
                vec![
                    part(Section::Intro, P::OneFourFive, Quarters, 1, Drums::Light),
                    part(Section::Verse, verse, RootFifth, times(2), Drums::Full),
                    part(
                        Section::Chorus,
                        P::OneFourFive,
                        Quarters,
                        times(2),
                        Drums::Full,
                    ),
                    part(Section::Verse, verse, RootFifth, times(2), Drums::Full),
                    part(
                        Section::Chorus,
                        P::OneFourFive,
                        Quarters,
                        times(2),
                        Drums::Full,
                    ),
                    part(Section::Outro, P::OneFourFive, RootFifth, 1, Drums::Full),
                ]
            }
            Style::Pop => {
                let verse = pick(&[P::Pop, P::Turnaround]);
                vec![
                    part(Section::Intro, P::Pop, Quarters, 1, Drums::Light),
                    part(Section::Verse, verse, Eighths, times(2), Drums::Full),
                    part(
                        Section::Chorus,
                        P::OneFourFive,
                        Octaves,
                        times(2),
                        Drums::Full,
                    ),
                    part(Section::Verse, verse, Eighths, times(2), Drums::Full),
                    part(
                        Section::Chorus,
                        P::OneFourFive,
                        Octaves,
                        times(2),
                        Drums::Full,
                    ),
                    part(
                        Section::Bridge,
                        P::Turnaround,
                        Tresillo,
                        times(2),
                        Drums::Full,
                    ),
                    part(
                        Section::Chorus,
                        P::OneFourFive,
                        Octaves,
                        times(2),
                        Drums::Full,
                    ),
                    part(Section::Outro, P::Pop, Quarters, 1, Drums::Full),
                ]
            }
            Style::Rock => {
                let verse = pick(&[P::OneFourFive, P::Turnaround]);
                vec![
                    part(Section::Intro, P::OneFourFive, Eighths, 1, Drums::Light),
                    part(Section::Verse, verse, Eighths, times(2), Drums::Full),
                    part(Section::Chorus, P::Pop, Octaves, times(2), Drums::Full),
                    part(Section::Verse, verse, Eighths, times(2), Drums::Full),
                    part(Section::Chorus, P::Pop, Octaves, times(2), Drums::Full),
                    part(
                        Section::Bridge,
                        P::Turnaround,
                        RootFifthOctave,
                        times(2),
                        Drums::Full,
                    ),
                    part(Section::Chorus, P::Pop, Octaves, times(2), Drums::Full),
                    part(Section::Outro, P::OneFourFive, Quarters, 1, Drums::Full),
                ]
            }
            Style::Funk => {
                let verse = pick(&[P::Minor, P::MinorTwoFiveOne]);
                vec![
                    part(Section::Intro, P::Minor, Ghost, 1, Drums::Light),
                    part(Section::Verse, verse, Ghost, times(2), Drums::Full),
                    part(
                        Section::Chorus,
                        P::MinorTwoFiveOne,
                        Funk,
                        times(2),
                        Drums::Full,
                    ),
                    part(Section::Verse, verse, Ghost, times(2), Drums::Full),
                    part(
                        Section::Chorus,
                        P::MinorTwoFiveOne,
                        Funk,
                        times(2),
                        Drums::Full,
                    ),
                    part(Section::Bridge, P::Minor, DeadNotes, times(2), Drums::Full),
                    part(
                        Section::Chorus,
                        P::MinorTwoFiveOne,
                        Funk,
                        times(2),
                        Drums::Full,
                    ),
                    part(Section::Outro, P::Minor, Quarters, 1, Drums::Full),
                ]
            }
            Style::Shuffle => {
                let mut parts = vec![part(Section::Intro, P::Tonic, Shuffle, 1, Drums::Light)];
                parts.extend((0..length.choruses()).map(|_| {
                    let chorus = pick(&[P::Blues, P::QuickBlues]);
                    part(Section::Verse, chorus, Shuffle, 1, Drums::Full)
                }));
                parts.push(part(Section::Outro, P::Tonic, Shuffle, 1, Drums::Full));
                parts
            }
        }
    }

    /// The piece's plan in the key of `root` (pitch class), for export and for its JSON
    /// Schema; `render` below renders the piece directly, without reparsing it.
    pub fn plan(self, root: u8, length: Length, seed: u64) -> PiecePlan {
        PiecePlan {
            title: self.label().to_string(),
            key: format!(
                "{}{}",
                name(i32::from(root), key_fifths(root, self.minor())),
                if self.minor() { "m" } else { "" }
            ),
            bpm: DEFAULT_BPM,
            feel: self.feel(),
            parts: self
                .built(root, length, seed)
                .into_iter()
                .map(|b| b.part)
                .collect(),
        }
    }

    fn feel(self) -> PieceFeel {
        if self == Style::Shuffle {
            PieceFeel::Shuffle
        } else {
            PieceFeel::Straight
        }
    }
}

/// `bpm` of a plan is never read for a built-in piece (the exercise's own tempo rules its
/// MIDI file instead, `render` below); any value in range satisfies `validate`.
const DEFAULT_BPM: u32 = 90;

/// A part together with the data `render` needs and `plan`'s `PlanPart` cannot hold: its
/// actual rhythm and chords, so rendering never has to reparse what this module just built.
struct Built {
    part: PlanPart,
    rhythm: Rhythm,
    chords: Vec<Vec<Chord>>,
}

/// The style's piece as `generate` wants it: the pass, drums and rehearsal marks for `spec`'s
/// key, level, length and seed. Built directly from the style's data (`built`), not through
/// `PiecePlan::check`: a built-in piece cannot fail to check out, so there is nothing to
/// `unwrap`.
pub(super) fn render(spec: &ExerciseSpec, style: Style, level: Level) -> Rendered {
    let built = style.built(spec.key, spec.length, spec.seed);
    let parts: Vec<CheckedPart> = built
        .iter()
        .map(|b| CheckedPart {
            part: &b.part,
            rhythm: b.rhythm,
            bars: b.chords.clone(),
        })
        .collect();
    let checked = Checked {
        key: spec.key,
        minor: style.minor(),
        parts,
    };
    piece_render::render_checked(&checked, style.feel(), style.label(), level, spec.seed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn song(style: Style, level: Level, key: u8) -> (Pass, Vec<(u32, u8)>) {
        let spec = ExerciseSpec {
            kind: ExerciseKind::Song(style, level),
            key,
            bpm: 90.0,
            slow: false,
            repeats: 1,
            ramp_bpm: 0.0,
            judging: Judging::PitchAndTime,
            seed: 0,
            length: Length::Normal,
        };
        let rendered = render(&spec, style, level);
        (rendered.pass, rendered.drums)
    }

    /// `Style::plan` describes the same piece `render` builds: a valid plan whose chords and
    /// form match, for export and for whoever wants to see a built-in piece as a plan.
    #[test]
    fn a_styles_plan_matches_what_it_renders() {
        for style in Style::ALL {
            for length in Length::ALL {
                let plan = style.plan(4, length, 0);
                assert_eq!(plan.validate(), Ok(()));
                let bars: usize = plan
                    .parts
                    .iter()
                    .map(|p| p.bars.len() * p.times as usize)
                    .sum();
                let (pass, _) = {
                    let spec = ExerciseSpec {
                        kind: ExerciseKind::Song(style, Level::Medium),
                        key: 4,
                        bpm: 90.0,
                        slow: false,
                        repeats: 1,
                        ramp_bpm: 0.0,
                        judging: Judging::PitchAndTime,
                        seed: 0,
                        length,
                    };
                    let rendered = render(&spec, style, Level::Medium);
                    (rendered.pass, rendered.drums)
                };
                // One closing bar beyond the plan's own bars.
                assert_eq!(bars as u32 + 1, pass.length / BAR, "{style:?} {length:?}");
            }
        }
    }

    #[test]
    fn every_style_has_a_form_of_two_to_three_minutes() {
        for style in Style::ALL {
            let (pass, _) = song(style, Level::Medium, 4);
            let bars = pass.length / BAR;
            assert!((40..=70).contains(&bars), "{style:?}: {bars} bars");
            assert!(pass.markers.len() as u32 >= bars, "a chord name per bar");
        }
    }

    #[test]
    fn piece_ends_on_the_tonic() {
        let (pass, drums) = song(Style::Funk, Level::Medium, 9);
        let last = pass.tones.last().unwrap();
        assert_eq!(last.key.rem_euclid(12), 9);
        assert_eq!(last.length, BAR);
        assert_eq!(pass.markers.last().unwrap().1, "Am7");
        assert!(drums.iter().all(|(at, _)| *at <= last.start));
    }

    /// QA 27.10. (J10): "Rock in E" read Emaj7 and Amaj7. Rock names its chords as the line
    /// plays them (root, fifth, octave): power chords, minor as minor.
    #[test]
    fn rock_chords_are_power_chords() {
        let (pass, _) = song(Style::Rock, Level::Medium, 4);
        let names: Vec<&str> = pass.markers.iter().map(|m| m.1.as_str()).collect();
        assert!(
            names.iter().all(|n| n.ends_with('5') || n.ends_with('m')),
            "{names:?}"
        );
        assert_eq!(names.first(), Some(&"E5"));
        assert_eq!(names.last(), Some(&"E5"));
        assert!(names.contains(&"C#m"), "{names:?}");
    }

    /// QA 27.10. (J10, round 16): pop and ballad named maj7 and m7 chords; to a beginner the
    /// piece read like jazz. They name triads, the line plays no seventh; funk and shuffle
    /// keep their sevenths.
    #[test]
    fn pop_chords_are_triads() {
        let sevenths = |n: &str| n.ends_with("maj7") || n.ends_with('7') || n.ends_with("7b5");
        // Every level (review #169: the hard line is denser), in C and two other keys.
        let cases = [Style::Pop, Style::Ballad]
            .into_iter()
            .flat_map(|s| [Level::Easy, Level::Medium, Level::Hard].map(|l| (s, l)))
            .flat_map(|(s, l)| [0, 3, 7].map(|key| (s, l, key)));
        for (style, level, key) in cases {
            let (pass, _) = song(style, level, key);
            let names: Vec<&str> = pass.markers.iter().map(|m| m.1.as_str()).collect();
            assert!(
                !names.iter().any(|n| sevenths(n)),
                "{style:?} {level:?}: {names:?}"
            );
            if key == 0 {
                assert_eq!(names.first(), Some(&"C"), "{style:?}");
                assert!(names.contains(&"Am"), "{style:?}: {names:?}");
            }

            // No tone a major seventh (11) or minor seventh (10) above the bar's root.
            let roots: Vec<(u32, i32)> = pass
                .tones
                .iter()
                .filter(|t| t.start % BAR == 0)
                .map(|t| (t.start, t.key))
                .collect();
            for tone in &pass.tones {
                let Some((_, root)) = roots.iter().rev().find(|(at, _)| *at <= tone.start) else {
                    continue;
                };
                let interval = (tone.key - root).rem_euclid(12);
                assert!(
                    interval != 10 && interval != 11,
                    "{style:?} {level:?}: {tone:?}"
                );
            }
        }
        for style in [Style::Funk, Style::Shuffle] {
            let (pass, _) = song(style, Level::Medium, 9);
            assert!(pass.markers.iter().any(|m| sevenths(&m.1)), "{style:?}");
        }
    }

    #[test]
    fn a_fill_leads_into_each_new_part() {
        let (pass, drums) = song(Style::Rock, Level::Medium, 4);
        // Rock: intro 4 bars, then the verse; the intro's last beat is a snare roll.
        let fill_from = 4 * BAR - QUARTER;
        let fill: Vec<_> = drums
            .iter()
            .filter(|(at, _)| (fill_from..4 * BAR).contains(at))
            .collect();
        assert_eq!(fill.len(), 4usize, "{fill:?}");
        assert!(fill.iter().all(|(_, key)| *key == SNARE));
        // No snare before the fill in the intro.
        assert!(!drums
            .iter()
            .any(|(at, key)| *at < fill_from && *key == SNARE));
        assert!(pass.length > 4 * BAR);
    }

    #[test]
    fn levels_thin_or_thicken_the_line_and_keep_the_form() {
        for style in Style::ALL {
            let notes = |level| song(style, level, 4).0.tones.len();
            let (easy, medium, hard) =
                (notes(Level::Easy), notes(Level::Medium), notes(Level::Hard));
            assert!(
                easy < medium && medium < hard,
                "{style:?}: {easy} {medium} {hard}"
            );

            let length = |level| song(style, level, 4).0.length;
            assert_eq!(length(Level::Easy), length(Level::Hard), "{style:?}");
        }
    }

    #[test]
    fn each_level_has_its_own_title_and_the_same_drums() {
        let (easy, easy_drums) = song(Style::Shuffle, Level::Easy, 4);
        let (medium, medium_drums) = song(Style::Shuffle, Level::Medium, 4);
        assert_eq!(easy.label, "Stück Shuffle (leicht)");
        assert_eq!(medium.label, "Stück Shuffle");
        assert_eq!(
            easy_drums, medium_drums,
            "the drums keep the style's groove"
        );
        for level in Level::ALL {
            assert_eq!(Level::from_key(level.key()), Some(level));
        }
    }

    #[test]
    fn seed_zero_is_the_plain_piece_every_other_seed_may_vary() {
        for style in Style::ALL {
            let plain = song(style, Level::Medium, 4);
            for seed in [0, 1] {
                let spec = ExerciseSpec {
                    kind: ExerciseKind::Song(style, Level::Medium),
                    key: 4,
                    bpm: 90.0,
                    slow: false,
                    repeats: 1,
                    ramp_bpm: 0.0,
                    judging: Judging::PitchAndTime,
                    seed,
                    length: Length::Normal,
                };
                let rendered = render(&spec, style, Level::Medium);
                if seed == 0 {
                    assert_eq!(rendered.pass.tones.len(), plain.0.tones.len(), "{style:?}");
                }
            }
        }
    }

    #[test]
    fn lengths_change_the_bar_count() {
        for style in Style::ALL {
            let bars = |length| {
                let spec = ExerciseSpec {
                    kind: ExerciseKind::Song(style, Level::Medium),
                    key: 4,
                    bpm: 90.0,
                    slow: false,
                    repeats: 1,
                    ramp_bpm: 0.0,
                    judging: Judging::PitchAndTime,
                    seed: 0,
                    length,
                };
                render(&spec, style, Level::Medium).pass.length / BAR
            };
            assert!(bars(Length::Short) < bars(Length::Normal), "{style:?}");
            assert!(bars(Length::Normal) < bars(Length::Long), "{style:?}");
        }
    }

    /// Bug (review of PR #316): LH-BIB-15 asks every style to pick between two progressions for
    /// at least one part when seed != 0; only Pop did (`song.rs:271` at the review's head).
    /// Across seeds 1..20, every style must show at least two different chord sequences.
    #[test]
    fn every_style_has_at_least_two_progressions_across_seeds() {
        for style in Style::ALL {
            let sequences: std::collections::HashSet<Vec<String>> = (1..20)
                .map(|seed| {
                    let spec = ExerciseSpec {
                        kind: ExerciseKind::Song(style, Level::Medium),
                        key: 4,
                        bpm: 90.0,
                        slow: false,
                        repeats: 1,
                        ramp_bpm: 0.0,
                        judging: Judging::PitchAndTime,
                        seed,
                        length: Length::Normal,
                    };
                    render(&spec, style, Level::Medium)
                        .pass
                        .markers
                        .iter()
                        .map(|m| m.1.clone())
                        .collect()
                })
                .collect();
            assert!(
                sequences.len() >= 2,
                "{style:?}: only {} distinct progression(s) across seeds 1..20",
                sequences.len()
            );
        }
    }
}
