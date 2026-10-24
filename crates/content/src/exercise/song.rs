//! Generated pieces to play through (owner, 24.10.: "Etüden wie Lieder"): a form of intro,
//! verses, choruses, a bridge and an outro, each part with its own chords and groove, drums with
//! a fill before each new part, the last bar on the tonic. Like the other exercises a MIDI file
//! through the normal import, so the workshop can loop a chorus like any bars.
//!
//! ```text
//! Style ──form──▶ Part, Part, …  (section, progression, rhythm, times)
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

/// Where a part stands in the form; the intro plays lighter drums.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    Intro,
    Verse,
    Chorus,
    Bridge,
    Outro,
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

    /// The parts in order. Verse and chorus come twice, then a bridge and a last chorus:
    /// about 50 to 60 bars, two to three minutes.
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
                part(Chorus, P::Blues, Shuffle, 1),
                part(Verse, P::Blues, Shuffle, 1),
                part(Chorus, P::Blues, Shuffle, 1),
                part(Outro, P::Tonic, Shuffle, 1),
            ],
        }
    }

    pub fn minor(self) -> bool {
        self == Style::Funk
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

/// The whole piece as one pass, closing on a whole-bar tonic; and its drums.
pub(super) fn song(style: Style, key: u8) -> (Pass, Vec<(u32, u8)>) {
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
            Voicing::Seventh,
            meter,
            |at, root, q| groove_bar(at, root, q, p.rhythm),
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
    markers.push((start, format!("{}{}", name(root, fifths), quality.suffix())));
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
        label: style.label().to_string(),
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
            let (pass, _) = song(style, 4);
            let bars = pass.length / BAR;
            assert!((40..=70).contains(&bars), "{style:?}: {bars} bars");
            assert!(pass.markers.len() as u32 >= bars, "a chord name per bar");
        }
    }

    #[test]
    fn piece_ends_on_the_tonic() {
        let (pass, drums) = song(Style::Funk, 9);
        let last = pass.tones.last().unwrap();
        assert_eq!(last.key.rem_euclid(12), 9);
        assert_eq!(last.length, BAR);
        assert_eq!(pass.markers.last().unwrap().1, "Am7");
        assert!(drums.iter().all(|(at, _)| *at <= last.start));
    }

    #[test]
    fn a_fill_leads_into_each_new_part() {
        let (pass, drums) = song(Style::Rock, 4);
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
}
