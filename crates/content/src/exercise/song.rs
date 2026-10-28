//! Generated pieces to play through (owner, 24.10.: "Etüden wie Lieder"): a form of intro,
//! verses, choruses, a bridge and an outro, each part with its own chords and groove, drums with
//! a fill before each new part, the last bar on the tonic. Like the other exercises a MIDI file
//! through the normal import, so the workshop can loop a chorus like any bars.
//!
//! ```text
//! Style ──form──▶ Part, Part, …  (section, progression, rhythm, times)
//!                  │ Level: the bass rhythm of each part one step simpler or denser
//!                  │ chords_line + groove_bar ──▶ tones, chord names ─┐
//!                  └ drum_bar, fill on the last beat ──▶ drums ───────┴──▶ Pass + drums
//! ```

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
    fn label(self) -> &'static str {
        match self {
            Level::Easy => " (leicht)",
            Level::Medium => "",
            Level::Hard => " (schwer)",
        }
    }

    /// `rhythm` at this level. Easy drops subdivisions and syncopation, hard adds them; a
    /// rhythm keeps its meter (all are 4/4 here), so the drums and the form fit.
    fn rhythm(self, rhythm: Rhythm) -> Rhythm {
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

/// Where a part stands in the form; the intro plays lighter drums.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    Intro,
    Verse,
    Chorus,
    Bridge,
    Outro,
}

impl Section {
    /// The rehearsal mark over the part's first bar, as German bands say it (LH-BIB-12); the
    /// shuffle's twelve bars of blues are a chorus. The app shows Strophe and Refrain in its
    /// language (`GENERATED_PARTS` in appcore).
    fn mark(self, style: Style) -> &'static str {
        match (self, style) {
            (Section::Intro, _) => "Intro",
            (Section::Verse, Style::Shuffle) => "Chorus",
            (Section::Verse, _) => "Strophe",
            (Section::Chorus, _) => "Refrain",
            (Section::Bridge, _) => "Bridge",
            (Section::Outro, _) => "Outro",
        }
    }
}

/// One part of the form: its chords played `times` in a row.
#[derive(Debug, Clone, Copy)]
struct Part {
    section: Section,
    progression: Progression,
    rhythm: Rhythm,
    times: u32,
}

const fn part(section: Section, progression: Progression, rhythm: Rhythm, times: u32) -> Part {
    Part {
        section,
        progression,
        rhythm,
        times,
    }
}

/// General MIDI snare roll of a fill: four sixteenths on the last beat.
const FILL_STROKES: u32 = 4;

impl Style {
    pub const ALL: [Style; 5] = [
        Style::Ballad,
        Style::Pop,
        Style::Rock,
        Style::Funk,
        Style::Shuffle,
    ];

    /// The parts in order: intro, verse and chorus twice, outro; pop, rock and funk add a bridge
    /// and a last chorus. The shuffle plays four twelve-bar choruses of the blues. 40 to 60 bars.
    fn form(self) -> Vec<Part> {
        use Progression as P;
        use Rhythm::*;
        use Section::*;
        match self {
            Style::Ballad => vec![
                part(Intro, P::OneFourFive, Quarters, 1),
                part(Verse, P::Pop, RootFifth, 2),
                part(Chorus, P::OneFourFive, Quarters, 2),
                part(Verse, P::Pop, RootFifth, 2),
                part(Chorus, P::OneFourFive, Quarters, 2),
                part(Outro, P::OneFourFive, RootFifth, 1),
            ],
            Style::Pop => vec![
                part(Intro, P::Pop, Quarters, 1),
                part(Verse, P::Pop, Eighths, 2),
                part(Chorus, P::OneFourFive, Octaves, 2),
                part(Verse, P::Pop, Eighths, 2),
                part(Chorus, P::OneFourFive, Octaves, 2),
                part(Bridge, P::Turnaround, Tresillo, 2),
                part(Chorus, P::OneFourFive, Octaves, 2),
                part(Outro, P::Pop, Quarters, 1),
            ],
            Style::Rock => vec![
                part(Intro, P::OneFourFive, Eighths, 1),
                part(Verse, P::OneFourFive, Eighths, 2),
                part(Chorus, P::Pop, Octaves, 2),
                part(Verse, P::OneFourFive, Eighths, 2),
                part(Chorus, P::Pop, Octaves, 2),
                part(Bridge, P::Turnaround, RootFifthOctave, 2),
                part(Chorus, P::Pop, Octaves, 2),
                part(Outro, P::OneFourFive, Quarters, 1),
            ],
            Style::Funk => vec![
                part(Intro, P::Minor, Ghost, 1),
                part(Verse, P::Minor, Ghost, 2),
                part(Chorus, P::MinorTwoFiveOne, Funk, 2),
                part(Verse, P::Minor, Ghost, 2),
                part(Chorus, P::MinorTwoFiveOne, Funk, 2),
                part(Bridge, P::Minor, DeadNotes, 2),
                part(Chorus, P::MinorTwoFiveOne, Funk, 2),
                part(Outro, P::Minor, Quarters, 1),
            ],
            Style::Shuffle => vec![
                part(Intro, P::Tonic, Shuffle, 1),
                part(Verse, P::Blues, Shuffle, 1),
                part(Verse, P::Blues, Shuffle, 1),
                part(Verse, P::Blues, Shuffle, 1),
                part(Verse, P::Blues, Shuffle, 1),
                part(Outro, P::Tonic, Shuffle, 1),
            ],
        }
    }

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
}

/// The rehearsal marks of the piece: (tick of the part's first bar, name).
pub(super) fn parts(style: Style) -> Vec<(u32, String)> {
    let mut start = 0;
    let mut marks = Vec::new();
    for p in style.form() {
        marks.push((start, p.section.mark(style).to_string()));
        start += p.times * bars_len(p.progression, p.rhythm.meter());
    }
    marks
}

/// The whole piece as one pass, closing on a whole-bar tonic; and its drums.
pub(super) fn song(style: Style, level: Level, key: u8) -> (Pass, Vec<(u32, u8)>) {
    let form = style.form();
    let mut tones = Vec::new();
    let mut markers = Vec::new();
    let mut drums = Vec::new();
    let mut start = 0;
    for (i, p) in form.iter().enumerate() {
        let meter = p.rhythm.meter();
        let (line, chords) = chords_line(
            p.progression,
            key,
            style.voicing(),
            meter,
            // The level changes the bass line; the drums keep the style's groove (a shuffle).
            |at, root, q| groove_bar(at, root, q, level.rhythm(p.rhythm)),
        );
        let length = bars_len(p.progression, meter);
        for time in 0..p.times {
            let shift = start + time * length;
            tones.extend(line.iter().map(|t| Tone {
                start: t.start + shift,
                ..*t
            }));
            markers.extend(chords.iter().map(|(at, c)| (at + shift, c.clone())));
        }
        let bars = p.times * length / meter.bar();
        let last = i + 1 == form.len();
        drums.extend(part_drums(p, start, bars, last));
        start += p.times * length;
    }

    // The end: the tonic for a whole bar, a last kick under it.
    let (offset, quality) = form[0].progression.bars()[0];
    let root = nearest(i32::from(key) + offset, low_root(key));
    let fifths = key_fifths(key, style.minor());
    markers.push((
        start,
        format!(
            "{}{}",
            name(root, fifths),
            quality.suffix_for(style.voicing())
        ),
    ));
    tones.push(Tone {
        start,
        length: BAR,
        key: root,
        stroke: Stroke::Plain,
        position: None,
    });
    drums.extend([(start, KICK), (start, HIHAT)]);

    let pass = Pass {
        tones,
        markers,
        length: start + BAR,
        minor: style.minor(),
        to_major: if style.minor() { RELATIVE_MAJOR } else { 0 },
        label: format!("{}{}", style.label(), level.label()),
    };
    (pass, drums)
}

/// Drums of a part from `start`: the groove's beat (the intro only kick and hi-hat), and a snare
/// fill on the last beat before the next part.
fn part_drums(p: &Part, start: u32, bars: u32, last: bool) -> Vec<(u32, u8)> {
    let bar = p.rhythm.meter().bar();
    let mut hits: Vec<(u32, u8)> = (0..bars)
        .flat_map(|b| drum_bar(start + b * bar, p.rhythm))
        .filter(|(_, key)| p.section != Section::Intro || *key != SNARE)
        .collect();
    if last {
        return hits;
    }
    let fill_from = start + bars * bar - QUARTER;
    hits.retain(|(at, _)| *at < fill_from);
    hits.extend((0..FILL_STROKES).map(|s| (fill_from + s * SIXTEENTH, SNARE)));
    hits
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(fill.len(), FILL_STROKES as usize, "{fill:?}");
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
            for p in style.form() {
                for level in Level::ALL {
                    assert_eq!(level.rhythm(p.rhythm).meter(), p.rhythm.meter());
                }
            }
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
}
