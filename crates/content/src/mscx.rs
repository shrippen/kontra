//! MuseScore 3 and 4 (`.mscx`; zipped as `.mscz`): the format MuseScore saves natively.
//! Bars and signatures come from the first staff; repeats, voltas, jumps, tempo and notes
//! from every staff. `<fret>`/`<string>` are used when the part has `<StringData>`.
//!
//! ```text
//! <Part> ─ <Staff/>…            one entry per staff of the part, in order
//! <Staff id="n"> ─ <Measure> ─ <voice> ─ <Chord> ─ <Note><pitch>…   (pitch = concert MIDI)
//! ```
//!
//! Durations come from `<durationType>`, `<dots>` and tuplets (MuseScore 4: `<Tuplet>` …
//! `<endTuplet/>`; MuseScore 3: `<Tuplet id>` referenced by the chord). Linked staves (a
//! notation staff with its tab) repeat the same notes; one copy is kept, the one with a fret.

use crate::error::ImportError;
use crate::musicxml::ending_mask;
use crate::score::{self, BarHead, Jump, Part, RawNote, Score, Target, Until, WHOLE_TICKS};
use crate::xml::{self, Element};
use crate::zip::Zip;
use kontra_model::{Pitch, Technique, TimeSignature};
use std::collections::HashMap;

const CONTAINER: &str = "META-INF/container.xml";
const SCORE_SUFFIX: &str = ".mscx";
/// MuseScore 2 and older write measures without voices.
const FIRST_VERSION: u32 = 3;
/// `<Tempo><tempo>` counts quarter notes per second.
const SECONDS_PER_MINUTE: f64 = 60.0;
/// Tempo is rounded to hundredths of a beat per minute.
const BPM_STEPS: f64 = 100.0;
/// Highest MIDI pitch.
const MAX_PITCH: i32 = 127;

/// Zipped MuseScore: the `.mscx` named in the container, else the first one in the archive.
pub(crate) fn parse_mscz(data: &[u8]) -> Result<Score, ImportError> {
    let archive = Zip::open(data)?;
    let named = archive.read(CONTAINER).ok().and_then(|c| rootfile(&c));
    let path = match named {
        Some(path) => path,
        None => archive
            .names()
            .find(|n| n.ends_with(SCORE_SUFFIX))
            .map(str::to_string)
            .ok_or_else(|| ImportError::bad("no score in .mscz archive"))?,
    };
    parse(&archive.read(&path)?)
}

/// The container lists style, thumbnail and score; the score is the `.mscx` one.
fn rootfile(container: &[u8]) -> Option<String> {
    let root = xml::parse(container).ok()?;
    root.every("rootfile")
        .into_iter()
        .filter_map(|f| f.attr("full-path"))
        .find(|p| p.ends_with(SCORE_SUFFIX))
        .map(str::to_string)
}

pub(crate) fn parse(data: &[u8]) -> Result<Score, ImportError> {
    let root = xml::parse(data)?;
    if root.name() != "museScore" {
        return Err(ImportError::UnknownFormat);
    }
    let major: u32 = root
        .attr("version")
        .and_then(|v| v.split('.').next())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    if major < FIRST_VERSION {
        return Err(ImportError::Unsupported(format!("MuseScore {major}")));
    }
    let body = root
        .child("Score")
        .ok_or_else(|| ImportError::bad("no <Score> in MuseScore file"))?;

    let mut score = Score::new();
    score.title = meta(body, "workTitle")
        .or_else(|| frame_text(body, "title"))
        .unwrap_or_default();
    score.artist = meta(body, "composer")
        .or_else(|| frame_text(body, "composer"))
        .unwrap_or_default();

    // Staff n (in file order) belongs to the part that lists it n-th.
    let mut owners: Vec<usize> = Vec::new();
    let mut parts: Vec<(Part, bool)> = Vec::new();
    for (index, element) in body.all("Part").enumerate() {
        owners.extend(std::iter::repeat_n(
            index,
            element.all("Staff").count().max(1),
        ));
        parts.push(part(element));
    }

    let mut reader = StaffReader::new(Role::Leading);
    for (number, staff) in body.all("Staff").enumerate() {
        let Some((part, tab)) = owners.get(number).and_then(|o| parts.get_mut(*o)) else {
            continue;
        };
        if number > 0 {
            reader = StaffReader::new(Role::Following);
        }
        reader.tab = *tab;
        for (index, measure) in staff.all("Measure").enumerate() {
            reader.measure(&mut score, part, index, measure);
        }
    }

    for (mut part, _) in parts {
        for bar in &mut part.bars {
            merge_linked(bar);
        }
        score.parts.push(part);
    }
    if let Some(bpm) = score
        .bars
        .first()
        .and_then(|b| b.tempos.first())
        .filter(|t| t.0 == 0)
    {
        score.bpm = bpm.1;
    }
    Ok(score)
}

/// Non-empty `<metaTag name>` text.
fn meta(body: &Element, name: &str) -> Option<String> {
    body.all("metaTag")
        .find(|m| m.attr("name") == Some(name))
        .map(|m| m.text().to_string())
        .filter(|t| !t.is_empty())
}

/// Text of the title frame's line in `style` (title, composer).
fn frame_text(body: &Element, style: &str) -> Option<String> {
    body.every("Text")
        .into_iter()
        .find(|t| t.value("style") == Some(style))
        .and_then(|t| t.value("text"))
        .map(str::to_string)
        .filter(|t| !t.is_empty())
}

/// A part with its name, instrument and tuning; the flag says whether frets can be used.
fn part(element: &Element) -> (Part, bool) {
    let instrument = element.child("Instrument");
    let name = element
        .value("trackName")
        .filter(|n| !n.is_empty())
        .or_else(|| instrument.and_then(|i| i.value("longName")))
        .unwrap_or_default();
    let mut part = Part::new(name);

    let id = instrument
        .and_then(|i| i.value("instrumentId"))
        .unwrap_or_default();
    part.bass_hint = id.starts_with("pluck.bass") || id == "strings.contrabass";
    part.program = instrument
        .and_then(|i| i.child("Channel"))
        .and_then(|c| c.child("program"))
        .and_then(|p| p.attr("value"))
        .and_then(|v| v.parse().ok())
        .filter(|p: &u8| *p <= 127);
    let drums = instrument.and_then(|i| i.value("useDrumset")) == Some("1");
    let percussion_staff = element
        .every("StaffType")
        .iter()
        .any(|t| t.attr("group") == Some("percussion"));
    part.percussion = drums || percussion_staff;

    // <string> lists the open strings lowest first.
    let Some(strings) = instrument.and_then(|i| i.child("StringData")) else {
        return (part, false);
    };
    part.tuning = strings
        .all("string")
        .filter_map(|s| s.text().parse::<u8>().ok())
        .filter(|p| *p <= 127)
        .map(Pitch)
        .collect();
    if let Some(frets) = strings.number("frets") {
        part.frets = frets;
    }
    let tab = !part.tuning.is_empty();
    (part, tab)
}

/// Whether this staff defines the bars and their signatures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Leading,
    Following,
}

/// State carried from measure to measure within one staff.
#[derive(Debug)]
struct StaffReader {
    role: Role,
    signature: TimeSignature,
    /// Frets and strings in the notes match the part's tuning.
    tab: bool,
    /// Volta in effect: ending mask and bars it still covers, this one included.
    volta: Option<(u32, u32)>,
}

impl StaffReader {
    fn new(role: Role) -> StaffReader {
        StaffReader {
            role,
            signature: TimeSignature::COMMON,
            tab: false,
            volta: None,
        }
    }

    fn measure(&mut self, score: &mut Score, part: &mut Part, index: usize, measure: &Element) {
        let mut head = BarHead::new(self.signature);
        let mut notes = Vec::new();
        for child in measure.children() {
            match child.name() {
                "voice" => self.voice(child, part, &mut head, &mut notes),
                "startRepeat" => head.repeat_open = true,
                "endRepeat" => head.repeat_plays = child.text().parse().unwrap_or(2).max(2),
                // MuseScore 3 keeps jumps and markers at the measure.
                "Marker" => marker(child, &mut head),
                "Jump" => jump(child, &mut head),
                _ => {}
            }
        }

        // A bar shorter or longer than its signature (pickup, odd bar) carries its length.
        if let Some(length) = measure.attr("len").and_then(fraction_signature) {
            head.signature = length;
        }
        if let Some((mask, left)) = self.volta {
            head.endings |= mask;
            self.volta = (left > 1).then_some((mask, left - 1));
        }
        part.bar_mut(index).extend(notes);

        if self.role == Role::Leading {
            score.bars.push(BarHead::new(head.signature));
        }
        let Some(target) = score.bars.get_mut(index) else {
            return;
        };
        for (offset, bpm) in &head.tempos {
            target.add_tempo(*offset, *bpm);
        }
        if target.key.is_none() {
            target.key = head.key;
        }

        // Bar lines and jumps belong to the whole system; any staff may carry them.
        target.repeat_open |= head.repeat_open;
        target.repeat_plays = target.repeat_plays.max(head.repeat_plays);
        target.endings |= head.endings;
        if target.targets.is_empty() && target.jumps.is_empty() {
            target.targets = head.targets;
            target.jumps = head.jumps;
        }
    }

    fn voice(
        &mut self,
        voice: &Element,
        part: &Part,
        head: &mut BarHead,
        notes: &mut Vec<RawNote>,
    ) {
        let mut position = 0u64;
        // MuseScore 4 nests tuplets in order; MuseScore 3 names them by id.
        let mut open: Vec<(u64, u64)> = Vec::new();
        let mut named: HashMap<String, (u64, u64)> = HashMap::new();

        for child in voice.children() {
            match child.name() {
                "TimeSig" => {
                    let numerator: Option<u8> = child.number("sigN");
                    let denominator: Option<u8> = child.number("sigD");
                    if let (Some(n), Some(d)) = (numerator, denominator) {
                        if n > 0 && d > 0 {
                            self.signature = TimeSignature {
                                numerator: n,
                                denominator: d,
                            };
                            head.signature = self.signature;
                        }
                    }
                }
                "KeySig" => {
                    let fifths = child
                        .number::<i32>("concertKey")
                        .or_else(|| child.number("accidental"));
                    if let Some(fifths) = fifths {
                        head.key = score::key(fifths);
                    }
                }
                "Tempo" => {
                    if let Some(per_second) = child.number::<f64>("tempo") {
                        // Six digits per second: 1.666667 → 100.00002 → 100.
                        let bpm = per_second * SECONDS_PER_MINUTE;
                        head.add_tempo(position, (bpm * BPM_STEPS).round() / BPM_STEPS);
                    }
                }
                "Tuplet" => {
                    let ratio = (
                        child.number("normalNotes").unwrap_or(1),
                        child.number("actualNotes").unwrap_or(1),
                    );
                    match child.attr("id") {
                        Some(id) => {
                            named.insert(id.to_string(), ratio);
                        }
                        None => open.push(ratio),
                    }
                }
                "endTuplet" => {
                    open.pop();
                }
                "location" => {
                    let shift = child.value("fractions").map_or(0, fraction_ticks);
                    position = position.saturating_add_signed(shift);
                }
                "Spanner" if child.attr("type") == Some("Volta") => self.volta_start(child),
                "Marker" => marker(child, head),
                "Jump" => jump(child, head),
                "Rest" => {
                    let ticks = self.duration(child, &open, &named);
                    position = position.saturating_add(ticks);
                }
                "Chord" => {
                    if is_grace(child) {
                        continue;
                    }
                    let ticks = self.duration(child, &open, &named);
                    for element in child.all("Note") {
                        if let Some(note) = self.note(child, element, part, position, ticks) {
                            notes.push(note);
                        }
                    }
                    position = position.saturating_add(ticks);
                }
                _ => {}
            }
        }
    }

    /// A volta starts here and spans the bars up to its end location.
    fn volta_start(&mut self, spanner: &Element) {
        let Some(volta) = spanner.child("Volta") else {
            return;
        };
        let mask = ending_mask(volta.value("endings").unwrap_or_default());
        let location = spanner.path(&["next", "location"]);
        let bars: u32 = location.and_then(|l| l.number("measures")).unwrap_or(0);
        let partial = location
            .and_then(|l| l.value("fractions"))
            .is_some_and(|f| fraction_ticks(f) > 0);
        let span = (bars + u32::from(partial)).max(1);
        if mask != 0 {
            self.volta = Some((mask, span));
        }
    }

    /// Ticks of a chord or rest: its type, dots and the tuplets it sits in.
    fn duration(
        &self,
        element: &Element,
        open: &[(u64, u64)],
        named: &HashMap<String, (u64, u64)>,
    ) -> u64 {
        let base = match element.value("durationType").unwrap_or("quarter") {
            "long" => WHOLE_TICKS * 4,
            "breve" => WHOLE_TICKS * 2,
            "whole" => WHOLE_TICKS,
            "half" => WHOLE_TICKS / 2,
            "eighth" => WHOLE_TICKS / 8,
            "16th" => WHOLE_TICKS / 16,
            "32nd" => WHOLE_TICKS / 32,
            "64th" => WHOLE_TICKS / 64,
            "128th" => WHOLE_TICKS / 128,
            // A whole-bar rest gives its length as a fraction.
            "measure" => element
                .value("duration")
                .map(fraction_ticks)
                .and_then(|t| u64::try_from(t).ok())
                .unwrap_or_else(|| self.signature.bar_ticks()),
            _ => WHOLE_TICKS / 4,
        };
        let dots: u32 = element.number("dots").unwrap_or(0);
        let mut ticks = base;
        let mut add = base;
        for _ in 0..dots.min(4) {
            add /= 2;
            ticks += add;
        }
        let referenced = element.value("Tuplet").and_then(|id| named.get(id));
        for (normal, actual) in open.iter().chain(referenced) {
            ticks = ticks * normal / (*actual).max(1);
        }
        ticks
    }

    fn note(
        &self,
        chord: &Element,
        element: &Element,
        part: &Part,
        start: u64,
        ticks: u64,
    ) -> Option<RawNote> {
        let midi: i32 = element.number("pitch")?;
        let pitch = Pitch(
            u8::try_from(midi)
                .ok()
                .filter(|m| i32::from(*m) <= MAX_PITCH)?,
        );
        let mut note = RawNote::new(start, ticks, pitch);

        let spanners = |kind: &'static str| {
            element
                .all("Spanner")
                .filter(move |s| s.attr("type") == Some(kind))
        };
        note.tied = spanners("Tie").any(|s| s.child("prev").is_some());
        if spanners("Glissando").any(|s| s.child("next").is_some()) {
            note.add(Technique::Slide);
        }
        if element.value("ghost") == Some("1") {
            note.add(Technique::Ghost);
        }
        if element.value("headGroup") == Some("cross") {
            note.add(Technique::Muted);
        }
        for articulation in chord.all("Articulation") {
            let subtype = articulation.value("subtype").unwrap_or_default();
            if subtype.starts_with("articStaccato") {
                note.add(Technique::Staccato);
            }
            if subtype.starts_with("articAccent") || subtype.starts_with("articMarcato") {
                note.add(Technique::Accent);
            }
        }

        // MuseScore string 0 is the highest; the model counts from the lowest.
        let string: Option<usize> = element.number("string");
        let fret: Option<u8> = element.number("fret");
        if let (true, Some(string), Some(fret)) = (self.tab, string, fret) {
            let count = part.tuning.len();
            if string < count {
                note.string = u8::try_from(count - 1 - string).ok();
                note.fret = Some(fret);
            }
        }
        Some(note)
    }
}

/// Grace notes take no time of their own; they are left out like in MusicXML.
fn is_grace(chord: &Element) -> bool {
    const GRACE: [&str; 8] = [
        "acciaccatura",
        "appoggiatura",
        "grace4",
        "grace16",
        "grace32",
        "grace8after",
        "grace16after",
        "grace32after",
    ];
    GRACE.iter().any(|g| chord.child(g).is_some())
}

/// Markers are named by their label: segno, varsegno, codab (coda sign), varcoda, fine,
/// coda (the "To Coda" text).
fn marker(element: &Element, head: &mut BarHead) {
    let label = element
        .value("label")
        .or_else(|| element.value("subtype"))
        .unwrap_or_default();
    match label {
        "segno" => head.targets.push(Target::Segno),
        "varsegno" => head.targets.push(Target::SegnoSegno),
        "codab" | "codetta" => head.targets.push(Target::Coda),
        "varcoda" => head.targets.push(Target::DoubleCoda),
        "fine" => head.targets.push(Target::Fine),
        "coda" | "tocodasym" => head.jumps.push(Jump::ToCoda),
        _ => {}
    }
}

/// `<jumpTo>` start, segno or varsegno; `<playUntil>` end, fine or a coda label.
fn jump(element: &Element, head: &mut BarHead) {
    let until = match element.value("playUntil").unwrap_or_default() {
        "fine" => Until::Fine,
        "coda" => Until::Coda,
        "varcoda" => Until::DoubleCoda,
        _ => Until::End,
    };
    match element.value("jumpTo").unwrap_or_default() {
        "start" => head.jumps.push(Jump::DaCapo(until)),
        "segno" => head.jumps.push(Jump::DalSegno(until)),
        "varsegno" => head.jumps.push(Jump::DalSegnoSegno(until)),
        _ => {}
    }
}

/// "3/8" → ticks of three eighths; "-1/4" → minus a quarter; anything else → 0.
fn fraction_ticks(text: &str) -> i64 {
    let Some((numerator, denominator)) = text.trim().split_once('/') else {
        return 0;
    };
    let (Ok(n), Ok(d)) = (numerator.parse::<i64>(), denominator.parse::<i64>()) else {
        return 0;
    };
    let whole = i64::try_from(WHOLE_TICKS).unwrap_or(0);
    if d <= 0 {
        return 0;
    }
    n.saturating_mul(whole) / d
}

/// A measure length "3/4" as a signature.
fn fraction_signature(text: &str) -> Option<TimeSignature> {
    let (numerator, denominator) = text.trim().split_once('/')?;
    let numerator: u8 = numerator.parse().ok().filter(|n| *n > 0)?;
    let denominator: u8 = denominator.parse().ok().filter(|d| *d > 0)?;
    Some(TimeSignature {
        numerator,
        denominator,
    })
}

/// Linked staves repeat each note: one per start and pitch, the copy with a fret first.
fn merge_linked(bar: &mut Vec<RawNote>) {
    bar.sort_by_key(|n| (n.offset, n.pitch, n.fret.is_none()));
    bar.dedup_by(|later, kept| later.offset == kept.offset && later.pitch == kept.pitch);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fractions() {
        assert_eq!(fraction_ticks("1/4"), 960);
        assert_eq!(fraction_ticks("-3/8"), -1440);
        assert_eq!(fraction_ticks("x"), 0);
        assert_eq!(fraction_ticks("1/0"), 0);
        let short = fraction_signature("1/4");
        assert_eq!(
            short,
            Some(TimeSignature {
                numerator: 1,
                denominator: 4
            })
        );
    }

    #[test]
    fn rejects_old_versions() {
        let old = br#"<museScore version="2.06"><Score/></museScore>"#;
        assert!(matches!(parse(old), Err(ImportError::Unsupported(_))));
    }
}
