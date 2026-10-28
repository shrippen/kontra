//! MusicXML (`score-partwise`; uncompressed or `.mxl`). Bars, repeats, endings and tempo
//! come from the first part; notes from every part, chords included;
//! `<technical><string>/<fret>` is used when the part declares a staff tuning.
//! Harmonics sound at their partial (`<harmonic>` or a diamond head): a chord of base,
//! touched and sounding pitch becomes one note at the sounding pitch.

use crate::error::ImportError;
use crate::score::{self, BarHead, Jump, Legato, Part, RawNote, Score, Target, Until, WHOLE_TICKS};
use crate::xml::{self, Element};
use crate::zip::Zip;
use kontra_model::{Pitch, Technique, TimeSignature, TICKS_PER_QUARTER};
use std::collections::HashMap;

const CONTAINER: &str = "META-INF/container.xml";
/// MusicXML numbers MIDI programs 1..=128.
const FIRST_PROGRAM: u8 = 1;
/// MusicXML numbers MIDI channels 1..=16; 10 is drums.
const DRUM_CHANNEL: u8 = 10;
const SEMITONES: i32 = 12;
/// One semitone up is seven steps on the circle of fifths (C → G♭ … → C♯ = 7 fifths).
const FIFTHS_PER_SEMITONE: i32 = 7;

/// Compressed MusicXML: the root file named in the container, else the first score file.
pub(crate) fn parse_mxl(data: &[u8]) -> Result<Score, ImportError> {
    let archive = Zip::open(data)?;
    let named = archive.read(CONTAINER).ok().and_then(|c| rootfile(&c));
    let path = match named {
        Some(path) => path,
        None => archive
            .names()
            .find(|n| {
                !n.starts_with("META-INF") && (n.ends_with(".xml") || n.ends_with(".musicxml"))
            })
            .map(str::to_string)
            .ok_or_else(|| ImportError::bad("no score in .mxl archive"))?,
    };
    parse(&archive.read_score(&path)?)
}

fn rootfile(container: &[u8]) -> Option<String> {
    let root = xml::parse(container).ok()?;
    let file = root.find("rootfile")?;
    file.attr("full-path").map(str::to_string)
}

pub(crate) fn parse(data: &[u8]) -> Result<Score, ImportError> {
    let root = xml::parse(data)?;
    match root.name() {
        "score-partwise" => {}
        "score-timewise" => {
            return Err(ImportError::Unsupported(
                "MusicXML score-timewise".to_string(),
            ))
        }
        _ => return Err(ImportError::UnknownFormat),
    }

    let mut score = Score::new();
    score.title = title(&root);
    score.artist = root
        .child("identification")
        .and_then(|i| {
            i.all("creator")
                .find(|c| c.attr("type") == Some("composer"))
        })
        .map(|c| c.text().to_string())
        .unwrap_or_default();

    let infos = part_list(&root);
    for (number, element) in root.all("part").enumerate() {
        let id = element.attr("id").unwrap_or_default();
        let info = infos.get(id);
        let mut part = Part::new(info.map_or("", |i| i.name.as_str()));
        part.program = info.and_then(|i| i.program);
        part.percussion = info.is_some_and(|i| i.percussion);
        let role = if number == 0 {
            Role::Leading
        } else {
            Role::Following
        };
        let mut reader = PartReader::new(role);
        for (index, measure) in element.all("measure").enumerate() {
            reader.measure(&mut score, &mut part, index, measure)?;
        }
        part.tuning = reader.tuning;
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

fn title(root: &Element) -> String {
    let work = root.path(&["work", "work-title"]).map(Element::text);
    let movement = root.child("movement-title").map(Element::text);
    work.filter(|t| !t.is_empty())
        .or(movement)
        .unwrap_or_default()
        .to_string()
}

#[derive(Debug)]
struct PartInfo {
    name: String,
    program: Option<u8>,
    percussion: bool,
}

fn part_list(root: &Element) -> HashMap<String, PartInfo> {
    let mut infos = HashMap::new();
    let Some(list) = root.child("part-list") else {
        return infos;
    };
    for part in list.all("score-part") {
        let Some(id) = part.attr("id") else {
            continue;
        };
        let midi = part.child("midi-instrument");
        let program: Option<u8> = midi.and_then(|m| m.number("midi-program"));
        let channel: Option<u8> = midi.and_then(|m| m.number("midi-channel"));
        let info = PartInfo {
            name: part.value("part-name").unwrap_or_default().to_string(),
            program: program
                .and_then(|p| p.checked_sub(FIRST_PROGRAM))
                .filter(|p| *p <= 127),
            percussion: channel == Some(DRUM_CHANNEL),
        };
        infos.insert(id.to_string(), info);
    }
    infos
}

/// Whether this part defines bars, repeats and tempo for the score.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Leading,
    Following,
}

/// State carried from measure to measure within one part.
#[derive(Debug)]
struct PartReader {
    role: Role,
    divisions: u64,
    signature: TimeSignature,
    /// Semitones from written to sounding pitch (`<transpose>`).
    transpose: i32,
    /// Open strings, lowest first, from `<staff-tuning>`.
    tuning: Vec<Pitch>,
    /// Alternate ending mask in effect until its closing barline.
    ending: u32,
    /// Segno names in order of appearance: first = segno, others = segno segno.
    segnos: Vec<String>,
    /// Coda names in order of appearance: first = coda, others = double coda.
    codas: Vec<String>,
}

impl PartReader {
    fn new(role: Role) -> PartReader {
        PartReader {
            role,
            divisions: 1,
            signature: TimeSignature::COMMON,
            transpose: 0,
            tuning: Vec::new(),
            ending: 0,
            segnos: Vec::new(),
            codas: Vec::new(),
        }
    }

    fn ticks(&self, element: &Element) -> u64 {
        let duration: u64 = element.number("duration").unwrap_or(0);
        duration.saturating_mul(TICKS_PER_QUARTER) / self.divisions.max(1)
    }

    fn measure(
        &mut self,
        score: &mut Score,
        part: &mut Part,
        index: usize,
        measure: &Element,
    ) -> Result<(), ImportError> {
        if self.role == Role::Leading {
            score.bars.push(BarHead::new(self.signature));
        }
        let mut head = BarHead::new(self.signature);
        let mut parsed: Vec<Parsed> = Vec::new();
        let mut chords = 0usize;
        let mut position = 0u64;
        let mut furthest = 0u64;
        let mut last_start = 0u64;

        for child in measure.children() {
            match child.name() {
                "attributes" => self.attributes(child, &mut head),
                "note" => {
                    let chord = child.child("chord").is_some();
                    let start = if chord { last_start } else { position };
                    let ticks = self.ticks(child);
                    if !chord {
                        chords += 1;
                    }
                    if child.child("grace").is_some() || child.child("cue").is_some() {
                        continue;
                    }
                    if let Some(mut note) = self.note(child, start, ticks) {
                        let harmonic = self.harmonic(child, &note);
                        if harmonic.is_some() {
                            note.add(Technique::Harmonic);
                        }
                        parsed.push(Parsed {
                            note,
                            chord: chords,
                            harmonic,
                        });
                    }
                    if !chord {
                        last_start = position;
                        position = position.saturating_add(ticks);
                    }
                }
                "backup" => position = position.saturating_sub(self.ticks(child)),
                "forward" => position = position.saturating_add(self.ticks(child)),
                "direction" => direction(child, position, &mut head),
                "harmony" => {
                    if let Some(name) = chord_symbol(child) {
                        let offset = child.number::<u64>("offset").unwrap_or(0);
                        let offset =
                            offset.saturating_mul(TICKS_PER_QUARTER) / self.divisions.max(1);
                        head.chords.push((position.saturating_add(offset), name));
                    }
                }
                "sound" => sound(child, position, &mut head),
                "barline" => self.barline(child, &mut head),
                _ => {}
            }
            furthest = furthest.max(position);
        }
        if self.ending != 0 {
            head.endings |= self.ending;
        }
        self.close_ending(measure);
        self.navigation(measure, &mut head);

        // Pickup bar: shorter than its signature.
        if measure.attr("implicit") == Some("yes") && index == 0 {
            if let Some(short) = pickup(head.signature, furthest) {
                head.signature = short;
            }
        }
        let mut notes = harmonics(&parsed);
        notes.sort_by_key(|n| (n.offset, n.pitch));
        part.bar_mut(index).extend(notes);

        let Some(target) = score.bars.get_mut(index) else {
            return Ok(());
        };
        for (offset, bpm) in &head.tempos {
            target.add_tempo(*offset, *bpm);
        }
        // Every part writes the same concert key; the first part that has one wins.
        if target.key.is_none() {
            target.key = head.key;
        }
        // Chord symbols often sit on the melody's part: the first part that has some wins.
        if target.chords.is_empty() {
            target.chords = head.chords;
        }
        if self.role == Role::Leading {
            target.signature = head.signature;
            target.repeat_open = head.repeat_open;
            target.repeat_plays = head.repeat_plays;
            target.endings = head.endings;
            target.marker = head.marker;
            target.targets = head.targets;
            target.jumps = head.jumps;
        }
        Ok(())
    }

    fn attributes(&mut self, attributes: &Element, head: &mut BarHead) {
        if let Some(divisions) = attributes.number::<u64>("divisions").filter(|d| *d > 0) {
            self.divisions = divisions;
        }
        if let Some(time) = attributes.child("time") {
            let numerator: Option<u8> = time.number("beats");
            let denominator: Option<u8> = time.number("beat-type");
            if let (Some(numerator), Some(denominator)) = (numerator, denominator) {
                if numerator > 0 && denominator > 0 {
                    self.signature = TimeSignature {
                        numerator,
                        denominator,
                    };
                    head.signature = self.signature;
                }
            }
        }
        if let Some(transpose) = attributes.child("transpose") {
            let chromatic: i32 = transpose.number("chromatic").unwrap_or(0);
            let octave: i32 = transpose.number("octave-change").unwrap_or(0);
            self.transpose = chromatic + octave * SEMITONES;
        }

        // The written key, moved to concert pitch like the notes (after <transpose>, which
        // MusicXML puts behind <key>).
        let fifths = attributes
            .child("key")
            .and_then(|k| k.number::<i32>("fifths"));
        if let Some(fifths) = fifths {
            head.key = score::key(concert_fifths(fifths, self.transpose));
        }
        if let Some(details) = attributes.child("staff-details") {
            let tuning = staff_tuning(details);
            if !tuning.is_empty() {
                self.tuning = tuning;
            }
        }
    }

    fn note(&self, element: &Element, start: u64, ticks: u64) -> Option<RawNote> {
        let pitch = element.child("pitch")?;
        let step = match pitch.value("step")? {
            "C" => 0,
            "D" => 2,
            "E" => 4,
            "F" => 5,
            "G" => 7,
            "A" => 9,
            "B" => 11,
            _ => return None,
        };
        let alter: f64 = pitch.number("alter").unwrap_or(0.0);
        let octave: i32 = pitch.number("octave")?;
        let midi = (octave + 1) * SEMITONES + step + alter.round() as i32 + self.transpose;
        let pitch = Pitch(u8::try_from(midi).ok().filter(|m| *m <= 127)?);

        let mut note = RawNote::new(start, ticks, pitch);
        note.tied = element.all("tie").any(|t| t.attr("type") == Some("stop"));
        if element.value("notehead") == Some("x") {
            note.add(Technique::Muted);
        }
        // A note head in parentheses: a ghost note (MuseScore, Guitar Pro export).
        if element
            .child("notehead")
            .is_some_and(|h| h.attr("parentheses") == Some("yes"))
        {
            note.add(Technique::Ghost);
        }
        if let Some(notations) = element.child("notations") {
            self.notations(notations, &mut note);
        }
        Some(note)
    }

    /// Harmonic marks of a note: `<technical><harmonic>`, or a diamond head alone (some
    /// exporters). A natural harmonic with string and fret sounds at that string's partial.
    fn harmonic(&self, element: &Element, note: &RawNote) -> Option<Harmonic> {
        let mark = element.path(&["notations", "technical", "harmonic"]);
        let diamond = element.value("notehead") == Some("diamond");
        if mark.is_none() && !diamond {
            return None;
        }
        let written = match mark {
            Some(m) if m.child("sounding-pitch").is_some() => Written::Sounding,
            Some(m) if m.child("base-pitch").is_some() => Written::Base,
            Some(m) if m.child("touching-pitch").is_some() => Written::Touching,
            _ => Written::Unspecified,
        };

        // An artificial harmonic's fret is the stopped one: where it is touched is unknown.
        let natural = mark.is_none_or(|m| m.child("artificial").is_none());
        let from_tab = match (natural, note.string, note.fret) {
            (true, Some(string), Some(fret)) => self
                .tuning
                .get(usize::from(string))
                .and_then(|open| crate::harmonic::natural_harmonic(*open, f64::from(fret))),
            _ => None,
        };
        Some(Harmonic { written, from_tab })
    }

    fn notations(&self, notations: &Element, note: &mut RawNote) {
        // A slide is marked where it starts, as in Guitar Pro; its stop is the next note.
        let slide_starts = ["slide", "glissando"]
            .iter()
            .filter_map(|name| notations.child(name))
            .any(|s| s.attr("type") != Some("stop"));
        if slide_starts {
            note.add(Technique::Slide);
        }
        if let Some(articulations) = notations.child("articulations") {
            if articulations.child("staccato").is_some() {
                note.add(Technique::Staccato);
            }
            if articulations.child("accent").is_some()
                || articulations.child("strong-accent").is_some()
            {
                note.add(Technique::Accent);
            }
        }
        let Some(technical) = notations.child("technical") else {
            return;
        };
        let legato = ["hammer-on", "pull-off"]
            .iter()
            .filter_map(|name| technical.child(name))
            .any(|t| t.attr("type") == Some("stop"));
        if legato {
            note.legato = Legato::Target;
        }
        if technical.child("bend").is_some() {
            note.add(Technique::Bend);
        }

        // MusicXML string 1 is the highest; the model counts from the lowest.
        let string: Option<usize> = technical.number("string");
        let fret: Option<u8> = technical.number("fret");
        let (Some(string), Some(fret)) = (string, fret) else {
            return;
        };
        let count = self.tuning.len();
        if string == 0 || string > count {
            return;
        }
        note.string = u8::try_from(count - string).ok();
        note.fret = Some(fret);
    }

    fn barline(&mut self, barline: &Element, head: &mut BarHead) {
        if let Some(repeat) = barline.child("repeat") {
            match repeat.attr("direction") {
                Some("forward") => head.repeat_open = true,
                Some("backward") => {
                    let times: u8 = repeat
                        .attr("times")
                        .and_then(|t| t.parse().ok())
                        .unwrap_or(2);
                    head.repeat_plays = times.max(2);
                }
                _ => {}
            }
        }
        let Some(ending) = barline.child("ending") else {
            return;
        };
        if ending.attr("type") == Some("start") {
            self.ending = ending_mask(ending.attr("number").unwrap_or_default());
        }
    }

    /// An ending stops at a `stop` or `discontinue` ending in this measure.
    /// Playback directions from `<sound>`: segno, coda, fine, dacapo, dalsegno, tocoda.
    fn navigation(&mut self, measure: &Element, head: &mut BarHead) {
        for sound in measure.every("sound") {
            if let Some(name) = sound.attr("segno") {
                let target = pick(&mut self.segnos, name, Target::Segno, Target::SegnoSegno);
                head.targets.push(target);
            }
            if let Some(name) = sound.attr("coda") {
                let target = pick(&mut self.codas, name, Target::Coda, Target::DoubleCoda);
                head.targets.push(target);
            }
            if sound.attr("fine").is_some() {
                head.targets.push(Target::Fine);
            }
            if sound.attr("dacapo") == Some("yes") {
                head.jumps.push(Jump::DaCapo(Until::Any));
            }
            if let Some(name) = sound.attr("dalsegno") {
                let (single, double) =
                    (Jump::DalSegno(Until::Any), Jump::DalSegnoSegno(Until::Any));
                let jump = pick(&mut self.segnos, name, single, double);
                head.jumps.push(jump);
            }
            if let Some(name) = sound.attr("tocoda") {
                let jump = pick(&mut self.codas, name, Jump::ToCoda, Jump::ToDoubleCoda);
                head.jumps.push(jump);
            }
        }
    }

    fn close_ending(&mut self, measure: &Element) {
        let closes = measure
            .all("barline")
            .filter_map(|b| b.child("ending"))
            .any(|e| matches!(e.attr("type"), Some("stop" | "discontinue")));
        if closes {
            self.ending = 0;
        }
    }
}

/// A note as read, with its chord and harmonic marks.
#[derive(Debug)]
struct Parsed {
    note: RawNote,
    /// Notes joined by `<chord/>` share this number.
    chord: usize,
    harmonic: Option<Harmonic>,
}

#[derive(Debug, Clone, Copy)]
struct Harmonic {
    written: Written,
    /// Sounding pitch of a natural harmonic from string and fret, if the tab gives them.
    from_tab: Option<Pitch>,
}

/// What the `<pitch>` of a harmonic note stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Written {
    Sounding,
    /// The pitch played before touching: the open string or the stopped note.
    Base,
    /// The touched pitch, often a diamond head.
    Touching,
    Unspecified,
}

/// One note per harmonic. MusicXML may write it as a chord:
///
/// ```text
/// base A1 (fret 5) + touching D2 (◇) + sounding A3 (hidden) → A3 on fret 5
/// base A1 (fret 5) + touching D2 (◇)                       → A3 (touched a fourth up)
/// touching A1 (◇, fret 5 of the E string, natural)         → E3 (partial of the string)
/// ```
///
/// What cannot be told (e.g. an artificial harmonic with its base only) keeps its pitch and is
/// judged in any octave.
fn harmonics(parsed: &[Parsed]) -> Vec<RawNote> {
    let mut notes = Vec::new();
    for chord in parsed.chunk_by(|a, b| a.chord == b.chord) {
        let find = |written: Written| {
            chord
                .iter()
                .find(|p| p.harmonic.is_some_and(|h| h.written == written))
        };
        let (base, touching) = (find(Written::Base), find(Written::Touching));
        let sounding = find(Written::Sounding).is_some();
        let tab = base
            .or(touching)
            .filter(|p| p.note.fret.is_some())
            .map(|p| (p.note.string, p.note.fret));

        for p in chord {
            let mut note = p.note.clone();
            let Some(harmonic) = p.harmonic else {
                notes.push(note);
                continue;
            };
            match harmonic.written {
                Written::Sounding => {
                    if let (None, Some((string, fret))) = (note.fret, tab) {
                        note.string = string;
                        note.fret = fret;
                    }
                }
                Written::Base | Written::Touching if sounding => continue,
                Written::Touching if base.is_some() => continue,
                Written::Base => {
                    let touched = touching
                        .map(|t| i32::from(t.note.pitch.0) - i32::from(note.pitch.0))
                        .and_then(|frets| {
                            crate::harmonic::natural_harmonic(note.pitch, f64::from(frets))
                        });
                    note.pitch = touched.or(harmonic.from_tab).unwrap_or(note.pitch);
                }
                Written::Touching | Written::Unspecified => {
                    note.pitch = harmonic.from_tab.unwrap_or(note.pitch);
                }
            }
            notes.push(note);
        }
    }
    notes
}

/// Key of a part written `transpose` semitones away from concert pitch, e.g. a B♭ clarinet
/// (-2) written in C major (0) sounds in B♭ major (-2); octaves change nothing.
fn concert_fifths(written: i32, transpose: i32) -> i32 {
    let mut shift = (transpose * FIFTHS_PER_SEMITONE).rem_euclid(SEMITONES);
    if shift > SEMITONES / 2 {
        shift -= SEMITONES;
    }
    let concert = written + shift;
    let max = i32::from(score::MAX_FIFTHS);
    if concert > max {
        return concert - SEMITONES;
    }
    if concert < -max {
        return concert + SEMITONES;
    }
    concert
}

/// `first` for the first name seen in `names`, `second` for any other name.
fn pick<T>(names: &mut Vec<String>, name: &str, first: T, second: T) -> T {
    if !names.iter().any(|n| n == name) {
        names.push(name.to_string());
    }
    if names.first().is_some_and(|n| n == name) {
        first
    } else {
        second
    }
}

/// "1, 2" → bits 0 and 1.
pub(crate) fn ending_mask(numbers: &str) -> u32 {
    numbers
        .split(|c: char| !c.is_ascii_digit())
        .filter_map(|n| n.parse::<u32>().ok())
        .filter_map(|n| n.checked_sub(1).and_then(|n| 1u32.checked_shl(n)))
        .fold(0, |mask, bit| mask | bit)
}

/// Open strings from `<staff-tuning line="1">` (line 1 = lowest string).
fn staff_tuning(details: &Element) -> Vec<Pitch> {
    let mut lines: Vec<(u32, Pitch)> = Vec::new();
    for tuning in details.all("staff-tuning") {
        let line: u32 = tuning
            .attr("line")
            .and_then(|l| l.parse().ok())
            .unwrap_or(0);
        let step = match tuning.value("tuning-step").unwrap_or_default() {
            "C" => 0,
            "D" => 2,
            "E" => 4,
            "F" => 5,
            "G" => 7,
            "A" => 9,
            "B" => 11,
            _ => continue,
        };
        let alter: i32 = tuning.number("tuning-alter").unwrap_or(0);
        let Some(octave) = tuning.number::<i32>("tuning-octave") else {
            continue;
        };
        let midi = (octave + 1) * SEMITONES + step + alter;
        if let Some(pitch) = u8::try_from(midi).ok().filter(|m| *m <= 127) {
            lines.push((line, Pitch(pitch)));
        }
    }
    lines.sort_by_key(|l| l.0);
    lines.into_iter().map(|l| l.1).collect()
}

/// Chord symbol of a `<harmony>` as the generator writes them: "C", "Bbm7", "F#/A#";
/// `None` for "N.C." (kind `none`) or without a root.
fn chord_symbol(harmony: &Element) -> Option<String> {
    let root = harmony.child("root")?;
    let mut name = spelled(root.value("root-step")?, root.number("root-alter"))?;

    let kind = harmony.child("kind");
    let suffix = match kind.map_or("major", Element::text) {
        "none" => return None,
        // A kind MusicXML has no name for: as the file writes it.
        "other" => kind.and_then(|k| k.attr("text")).unwrap_or(""),
        value => chord_suffix(value),
    };
    name.push_str(suffix);

    if let Some(bass) = harmony.child("bass") {
        let step = bass.value("bass-step").unwrap_or("");
        if let Some(bass) = spelled(step, bass.number("bass-alter")) {
            name = format!("{name}/{bass}");
        }
    }
    Some(name)
}

/// A step with its alteration: ("B", -1) → "Bb".
fn spelled(step: &str, alter: Option<f64>) -> Option<String> {
    let step = step.trim();
    if !matches!(step, "A" | "B" | "C" | "D" | "E" | "F" | "G") {
        return None;
    }
    let sign = match alter.map_or(0, |a| a.round() as i32) {
        1 => "#",
        2 => "##",
        -1 => "b",
        -2 => "bb",
        _ => "",
    };
    Some(format!("{step}{sign}"))
}

/// Symbol suffix of a MusicXML chord kind, e.g. "minor-seventh" → "m7".
fn chord_suffix(kind: &str) -> &'static str {
    match kind.trim() {
        "minor" => "m",
        "augmented" => "aug",
        "diminished" => "dim",
        "dominant" => "7",
        "major-seventh" => "maj7",
        "minor-seventh" => "m7",
        "diminished-seventh" => "dim7",
        "augmented-seventh" => "aug7",
        "half-diminished" => "m7b5",
        "major-minor" => "m(maj7)",
        "major-sixth" => "6",
        "minor-sixth" => "m6",
        "dominant-ninth" => "9",
        "major-ninth" => "maj9",
        "minor-ninth" => "m9",
        "dominant-11th" => "11",
        "major-11th" => "maj11",
        "minor-11th" => "m11",
        "dominant-13th" => "13",
        "major-13th" => "maj13",
        "minor-13th" => "m13",
        "suspended-second" => "sus2",
        "suspended-fourth" => "sus4",
        "power" => "5",
        _ => "",
    }
}

/// Tempo from `<sound tempo>` or, failing that, a quarter-based `<metronome>`.
fn direction(direction: &Element, position: u64, head: &mut BarHead) {
    if let Some(sound) = direction.child("sound") {
        if sound.attr("tempo").is_some() {
            return self::sound(sound, position, head);
        }
    }
    if let Some(text) = direction.find("rehearsal").map(Element::text) {
        if !text.is_empty() && head.marker.is_none() {
            head.marker = Some(text.to_string());
        }
    }
    let Some(metronome) = direction.find("metronome") else {
        return;
    };
    let Some(per_minute) = metronome.number::<f64>("per-minute") else {
        return;
    };
    let mut factor = match metronome.value("beat-unit").unwrap_or("quarter") {
        "eighth" => 0.5,
        "half" => 2.0,
        "whole" => 4.0,
        _ => 1.0,
    };
    if metronome.child("beat-unit-dot").is_some() {
        factor *= 1.5;
    }
    head.add_tempo(position, per_minute * factor);
}

fn sound(sound: &Element, position: u64, head: &mut BarHead) {
    let Some(bpm) = sound.attr("tempo").and_then(|t| t.parse::<f64>().ok()) else {
        return;
    };
    head.add_tempo(position, bpm);
}

/// Signature matching a shorter first bar, e.g. a one-beat pickup in 4/4 → 1/4.
fn pickup(signature: TimeSignature, length: u64) -> Option<TimeSignature> {
    let beat = WHOLE_TICKS / u64::from(signature.denominator.max(1));
    if length == 0 || length >= signature.bar_ticks() || !length.is_multiple_of(beat) {
        return None;
    }
    Some(TimeSignature {
        numerator: u8::try_from(length / beat).ok()?,
        denominator: signature.denominator,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ending_numbers() {
        assert_eq!(ending_mask("1"), 0b1);
        assert_eq!(ending_mask("1, 2"), 0b11);
        assert_eq!(ending_mask("3"), 0b100);
    }

    #[test]
    fn pickup_bar() {
        let short = pickup(TimeSignature::COMMON, 960).unwrap();
        assert_eq!((short.numerator, short.denominator), (1, 4));
        assert!(pickup(TimeSignature::COMMON, 3840).is_none());
    }
}
