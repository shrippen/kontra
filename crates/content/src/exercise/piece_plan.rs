//! The plan of a generated piece (Harmonicon: "piece plan"): what a piece is made of, as data
//! that a person, a test or a language model writes, and that `render_piece` turns into a MIDI
//! file. The five built-in styles are plans too (`Style::plan`).
//!
//! ```text
//! JSON ─▶ PiecePlan ─validate─▶ chords, grooves checked, every problem with its place
//!                   └render───▶ Exercise (title, MIDI) ─▶ the normal import
//! ```
//!
//! The doc comments of these types go into the JSON Schema (`kontra-cli schemas`); write them
//! for the reader of the schema.

use super::chord::{note, Chord};
use super::{Meter, Rhythm};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const MIN_BPM: u32 = 40;
pub const MAX_BPM: u32 = 200;
pub const MAX_PARTS: usize = 16;
pub const MAX_BARS_PER_PART: usize = 16;
pub const MAX_TIMES: u32 = 8;
pub const MAX_TOTAL_BARS: usize = 160;
pub const MAX_TITLE_CHARS: usize = 80;
/// A bar holds one chord, or two that split it in halves.
const MAX_CHORDS_PER_BAR: usize = 2;

/// A piece: a key, a tempo and parts in order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct PiecePlan {
    /// Name of the piece, 1 to 80 characters. Progress is kept per title.
    pub title: String,
    /// Key of the piece: a note name, "m" after it for minor. Examples: "G", "F#m", "Bb".
    /// The piece ends on this key's tonic chord.
    pub key: String,
    /// Tempo in beats per minute, 40 to 200.
    pub bpm: u32,
    /// How eighth notes are played. Use the groove "shuffle" or "triplets" with a shuffle feel.
    pub feel: PieceFeel,
    /// The parts in playing order, 1 to 16 parts and at most 160 bars in all (every part
    /// counted `times` times).
    pub parts: Vec<PlanPart>,
}

/// How the eighth notes are played.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum PieceFeel {
    /// Even eighths.
    Straight,
    /// Swung eighths (2:1); the metronome clicks the triplet.
    Shuffle,
}

/// One part of the form, e.g. a verse, played `times` in a row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct PlanPart {
    /// What the part is; it names the rehearsal mark over its first bar.
    pub section: Section,
    /// One entry per bar (1 to 16 bars), the chord symbols of that bar separated by a space.
    /// One chord fills the bar, two chords split it in halves: "C G". Symbols are a note name
    /// ("C", "F#", "Bb") plus an optional quality: "" (major), m, 5, 6, m6, 7, maj7, m7, m7b5
    /// (or ø), dim, dim7, sus2, sus4, 7sus4, add9, aug; "/" and a note names the bass note:
    /// "C/E". Example: ["G", "D", "Em", "C"]; with a split bar ["Am", "Dm G", "C", "C"].
    pub bars: Vec<String>,
    /// How often the bars are played in a row, 1 to 8.
    pub times: u32,
    /// Rhythm of the bass, one of: quarters, eighths, sixteenths, rests, shuffle, octaves,
    /// ghost, rootfifth, rootfifthoctave, dead, offbeat, tresillo, triplets, funk, slap,
    /// staccato (all in 4/4; "rests" leaves the beat out, "rootfifth" is two half notes).
    pub groove: String,
    /// What the bass plays over the groove.
    pub line: PlanLine,
    /// The last beat of the part becomes a short run to the first note of the next part.
    pub fill: bool,
    /// The drums under the part.
    pub drums: Drums,
}

/// Where a part stands in the form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Section {
    Intro,
    Verse,
    PreChorus,
    Chorus,
    Bridge,
    Solo,
    Outro,
}

/// What the bass plays; the groove gives the rhythm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum PlanLine {
    /// The groove on the chord's root (the groove's own octaves and fifths stay).
    Roots,
    /// As roots, a fifth on the third beat of the bar.
    RootFifth,
    /// The root on beat one, then the third, fifth and seventh (or octave) of the chord on
    /// the following beats.
    ChordTones,
    /// A walking line in quarter notes (the groove's rhythm is ignored, the drums follow it).
    Walking,
    /// As roots; the last note before a chord change leads to the next root by a half or
    /// whole step.
    Approach,
}

/// How much the drums play under a part.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Drums {
    /// Kick, snare and hi-hat; a snare fill on the last beat before the next part.
    Full,
    /// Kick and hi-hat only (an intro); still gets the snare fill into the next part.
    Light,
    /// No drums.
    None,
}

/// What is wrong with a plan, and where: the place reads like `parts[2].bars[3]`. The text is
/// for a retry by whoever wrote the plan; the UI shows its own.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PlanError {
    #[error("not a piece plan: {message}")]
    Json { message: String },
    #[error("title: {length} characters, must be 1 to {MAX_TITLE_CHARS}")]
    Title { length: usize },
    #[error(
        "key: unknown key '{key}', write a note name and 'm' for minor: \"G\", \"F#m\", \"Bb\""
    )]
    Key { key: String },
    #[error("bpm: {bpm} is outside {MIN_BPM} to {MAX_BPM}")]
    Bpm { bpm: u32 },
    #[error("parts: {count} parts, must be 1 to {MAX_PARTS}")]
    PartCount { count: usize },
    #[error("{at}.bars: {count} bars, must be 1 to {MAX_BARS_PER_PART}")]
    BarCount { at: String, count: usize },
    #[error("{at}: a bar needs one chord, or two to split it")]
    EmptyBar { at: String },
    #[error("{at}: {count} chords in one bar, at most {MAX_CHORDS_PER_BAR}")]
    ChordsInBar { at: String, count: usize },
    #[error("{at}: unknown chord '{chord}'")]
    UnknownChord { at: String, chord: String },
    #[error("{at}.times: {times} is outside 1 to {MAX_TIMES}")]
    Times { at: String, times: u32 },
    #[error("parts: {total} bars in all, at most {MAX_TOTAL_BARS}")]
    TotalBars { total: usize },
    #[error("{at}.groove: unknown groove '{groove}'")]
    UnknownGroove { at: String, groove: String },
    #[error("{at}.groove: '{groove}' is not in 4/4")]
    NotFourFour { at: String, groove: String },
}

/// A plan after `validate`: chords parsed, grooves found.
pub(super) struct Checked<'a> {
    /// Pitch class of the key's tonic.
    pub(super) key: u8,
    pub(super) minor: bool,
    pub(super) parts: Vec<CheckedPart<'a>>,
}

pub(super) struct CheckedPart<'a> {
    pub(super) part: &'a PlanPart,
    pub(super) rhythm: Rhythm,
    /// The chords of each bar, one or two.
    pub(super) bars: Vec<Vec<Chord>>,
}

/// Pitch class and mode of a key text ("F#m" → 6, minor).
pub(super) fn parse_key(text: &str) -> Option<(u8, bool)> {
    let (pc, _, rest) = note(text.trim())?;
    match rest {
        "" => Some((pc, false)),
        "m" => Some((pc, true)),
        _ => None,
    }
}

impl PiecePlan {
    /// Reads a plan from JSON; every problem of the structure is one `Json` error, those of
    /// the content come from `validate`.
    pub fn from_json(text: &str) -> Result<PiecePlan, PlanError> {
        serde_json::from_str(text).map_err(|e| PlanError::Json {
            message: e.to_string(),
        })
    }

    /// Every problem of the plan with its place; `Ok` when it can be rendered.
    pub fn validate(&self) -> Result<(), Vec<PlanError>> {
        self.check().map(|_| ())
    }

    pub(super) fn check(&self) -> Result<Checked<'_>, Vec<PlanError>> {
        let mut errors = Vec::new();
        let length = self.title.trim().chars().count();
        if length == 0 || length > MAX_TITLE_CHARS {
            errors.push(PlanError::Title { length });
        }
        let key = parse_key(&self.key);
        if key.is_none() {
            errors.push(PlanError::Key {
                key: self.key.clone(),
            });
        }
        if !(MIN_BPM..=MAX_BPM).contains(&self.bpm) {
            errors.push(PlanError::Bpm { bpm: self.bpm });
        }
        if self.parts.is_empty() || self.parts.len() > MAX_PARTS {
            errors.push(PlanError::PartCount {
                count: self.parts.len(),
            });
        }

        let mut parts = Vec::new();
        let mut total = 0;
        for (i, part) in self.parts.iter().enumerate() {
            let at = format!("parts[{i}]");
            let before = errors.len();
            let checked = check_part(part, &at, &mut errors);
            total += part.bars.len() * part.times as usize;
            if errors.len() == before {
                parts.extend(checked);
            }
        }
        if total > MAX_TOTAL_BARS {
            errors.push(PlanError::TotalBars { total });
        }

        match (key, errors.is_empty()) {
            (Some((key, minor)), true) => Ok(Checked { key, minor, parts }),
            _ => Err(errors),
        }
    }
}

/// The checked form of `part` at `at`; problems go to `errors`.
fn check_part<'a>(
    part: &'a PlanPart,
    at: &str,
    errors: &mut Vec<PlanError>,
) -> Option<CheckedPart<'a>> {
    if !(1..=MAX_TIMES).contains(&part.times) {
        errors.push(PlanError::Times {
            at: at.to_string(),
            times: part.times,
        });
    }
    if part.bars.is_empty() || part.bars.len() > MAX_BARS_PER_PART {
        errors.push(PlanError::BarCount {
            at: at.to_string(),
            count: part.bars.len(),
        });
    }
    let rhythm = match Rhythm::from_key(&part.groove) {
        Some(r) if r.meter() == Meter::Common => Some(r),
        Some(_) => {
            errors.push(PlanError::NotFourFour {
                at: at.to_string(),
                groove: part.groove.clone(),
            });
            None
        }
        None => {
            errors.push(PlanError::UnknownGroove {
                at: at.to_string(),
                groove: part.groove.clone(),
            });
            None
        }
    };

    let mut bars = Vec::new();
    for (b, text) in part.bars.iter().enumerate() {
        let place = format!("{at}.bars[{b}]");
        let symbols: Vec<&str> = text.split_whitespace().collect();
        if symbols.is_empty() {
            errors.push(PlanError::EmptyBar { at: place });
            continue;
        }
        if symbols.len() > MAX_CHORDS_PER_BAR {
            errors.push(PlanError::ChordsInBar {
                at: place,
                count: symbols.len(),
            });
            continue;
        }
        let chords: Vec<Chord> = symbols.iter().filter_map(|s| Chord::parse(s)).collect();
        if chords.len() == symbols.len() {
            bars.push(chords);
            continue;
        }
        for symbol in symbols.iter().filter(|s| Chord::parse(s).is_none()) {
            errors.push(PlanError::UnknownChord {
                at: place.clone(),
                chord: (*symbol).to_string(),
            });
        }
    }
    Some(CheckedPart {
        part,
        rhythm: rhythm?,
        bars,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn part(bars: &[&str]) -> PlanPart {
        PlanPart {
            section: Section::Verse,
            bars: bars.iter().map(|b| (*b).to_string()).collect(),
            times: 2,
            groove: "eighths".to_string(),
            line: PlanLine::Roots,
            fill: false,
            drums: Drums::Full,
        }
    }

    pub(crate) fn plan() -> PiecePlan {
        PiecePlan {
            title: "Test".to_string(),
            key: "G".to_string(),
            bpm: 90,
            feel: PieceFeel::Straight,
            parts: vec![part(&["G", "D", "Em", "C"])],
        }
    }

    #[test]
    fn plan_survives_json() {
        let mut original = plan();
        original.parts[0].line = PlanLine::ChordTones;
        original.parts.push(PlanPart {
            section: Section::PreChorus,
            ..part(&["Am", "Dm G"])
        });
        let text = serde_json::to_string_pretty(&original).expect("json");
        assert!(text.contains("\"pre_chorus\""), "{text}");
        assert!(text.contains("\"chord_tones\""), "{text}");
        assert_eq!(PiecePlan::from_json(&text), Ok(original));
    }

    #[test]
    fn json_problems_are_one_error() {
        let Err(PlanError::Json { message }) = PiecePlan::from_json(r#"{"title": "x"}"#) else {
            panic!("missing fields must fail");
        };
        assert!(message.contains("missing field"), "{message}");
        let text = serde_json::to_string(&plan())
            .expect("json")
            .replace("\"bpm\"", "\"tempo\"");
        assert!(PiecePlan::from_json(&text).is_err(), "unknown field");
    }

    #[test]
    fn a_good_plan_is_valid() {
        assert_eq!(plan().validate(), Ok(()));
    }

    /// Every kind of problem is named with its place; all are reported together.
    #[test]
    fn validate_names_each_problem_where_it_is() {
        let mut bad = plan();
        bad.title = String::new();
        bad.key = "H".to_string();
        bad.bpm = 300;
        bad.parts = vec![
            part(&["G", "Hm", "C Am G", ""]),
            PlanPart {
                times: 9,
                groove: "34".to_string(),
                ..part(&["C"])
            },
            PlanPart {
                bars: Vec::new(),
                groove: "polka".to_string(),
                ..part(&["C"])
            },
        ];
        let errors = bad.validate().expect_err("invalid");
        let text: Vec<String> = errors.iter().map(ToString::to_string).collect();
        for expected in [
            "title: 0 characters",
            "key: unknown key 'H'",
            "bpm: 300 is outside 40 to 200",
            "parts[0].bars[1]: unknown chord 'Hm'",
            "parts[0].bars[2]: 3 chords in one bar",
            "parts[0].bars[3]: a bar needs one chord",
            "parts[1].times: 9 is outside 1 to 8",
            "parts[1].groove: '34' is not in 4/4",
            "parts[2].bars: 0 bars",
            "parts[2].groove: unknown groove 'polka'",
        ] {
            assert!(
                text.iter().any(|t| t.contains(expected)),
                "{expected}: {text:#?}"
            );
        }
    }

    #[test]
    fn limits_count_parts_bars_and_total() {
        let mut many = plan();
        many.parts = vec![part(&["C"]); MAX_PARTS + 1];
        assert!(matches!(
            many.validate().expect_err("parts")[0],
            PlanError::PartCount { count: 17 }
        ));

        let mut long = plan();
        long.parts = vec![part(&["C"; MAX_BARS_PER_PART + 1])];
        assert!(long
            .validate()
            .expect_err("bars")
            .iter()
            .any(|e| matches!(e, PlanError::BarCount { count: 17, .. })));

        // Ten parts of 16 bars, twice: 320 bars.
        let mut total = plan();
        total.parts = vec![part(&["C"; MAX_BARS_PER_PART]); 10];
        let errors = total.validate().expect_err("total");
        assert!(
            errors.contains(&PlanError::TotalBars { total: 320 }),
            "{errors:?}"
        );

        let mut title = plan();
        title.title = "x".repeat(MAX_TITLE_CHARS + 1);
        assert!(matches!(
            title.validate().expect_err("title")[0],
            PlanError::Title { length: 81 }
        ));
    }

    #[test]
    fn keys_parse() {
        assert_eq!(parse_key("G"), Some((7, false)));
        assert_eq!(parse_key("F#m"), Some((6, true)));
        assert_eq!(parse_key("Bb"), Some((10, false)));
        assert_eq!(parse_key("Ebm"), Some((3, true)));
        assert_eq!(parse_key("Gmaj"), None);
        assert_eq!(parse_key("x"), None);
    }

    /// The schema tells the writer which grooves exist: every 4/4 groove is in the doc comment.
    #[test]
    fn the_doc_lists_every_four_four_groove() {
        let source = include_str!("piece_plan.rs");
        let doc = source
            .split("pub struct PlanPart")
            .nth(1)
            .and_then(|s| s.split("pub enum Section").next())
            .expect("doc");
        for rhythm in Rhythm::ALL
            .into_iter()
            .filter(|r| r.meter() == Meter::Common)
        {
            assert!(
                doc.contains(&format!(" {}", rhythm.key())),
                "{}",
                rhythm.key()
            );
        }
    }
}
