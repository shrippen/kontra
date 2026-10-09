//! GPIF, the XML score inside Guitar Pro 6 (`.gpx`) and 7/8 (`.gp`) files.
//!
//! ```text
//! MasterBars/MasterBar ─Bars (one id per track staff)─→ Bars/Bar ─Voices─→ Voices/Voice
//!   ─Beats─→ Beats/Beat ─Rhythm ref─→ Rhythms/Rhythm (note value, dots, tuplet)
//!                       ─Notes─→ Notes/Note (string, fret, MIDI number, effects)
//! ```
//! Elements refer to each other by `id`; lists are space-separated ids, `-1` = none.

use crate::error::ImportError;
use crate::gp::binary::tuplet_ticks;
use crate::score::{self, BarHead, Jump, Legato, Part, RawNote, Score, Target, Until, WHOLE_TICKS};
use crate::xml::{self, Element};
use kontra_model::{Pitch, Technique, TimeSignature};
use std::collections::HashMap;

/// Lookup tables from id to element.
#[derive(Debug, Default)]
struct Index<'a> {
    bars: HashMap<&'a str, &'a Element>,
    voices: HashMap<&'a str, &'a Element>,
    beats: HashMap<&'a str, &'a Element>,
    notes: HashMap<&'a str, &'a Element>,
    rhythms: HashMap<&'a str, u64>,
}

/// GP accent flags.
const ACCENT_STACCATO: u32 = 0x01;
const ACCENT_HEAVY: u32 = 0x04;
const ACCENT_NORMAL: u32 = 0x08;
/// Space-separated id meaning "none".
const NO_ID: &str = "-1";

pub(crate) fn parse(data: &[u8]) -> Result<Score, ImportError> {
    let root = xml::parse(data)?;
    if root.name() != "GPIF" {
        return Err(ImportError::bad("score.gpif without GPIF root"));
    }

    let mut score = Score::new();
    if let Some(info) = root.child("Score") {
        score.title = info.value("Title").unwrap_or_default().to_string();
        score.artist = info.value("Artist").unwrap_or_default().to_string();
    }

    let index = index(&root);
    let tracks: Vec<&Element> = root
        .child("Tracks")
        .map(|t| t.all("Track").collect())
        .unwrap_or_default();
    // Each track owns one bar id per staff in every master bar; remember where its first staff is.
    let mut first_staff = Vec::with_capacity(tracks.len());
    let mut staves = 0;
    for track in &tracks {
        first_staff.push(staves);
        staves += staff_count(track);
        score.parts.push(part(track));
    }

    let masters: Vec<&Element> = root
        .child("MasterBars")
        .map(|m| m.all("MasterBar").collect())
        .unwrap_or_default();
    for (bar, master) in masters.iter().enumerate() {
        score.bars.push(head(master)?);
        let ids: Vec<&str> = ids(master.value("Bars").unwrap_or_default());
        for (part, &staff) in first_staff.iter().enumerate() {
            let Some(bar_el) = ids.get(staff).and_then(|id| index.bars.get(id)) else {
                continue;
            };
            let notes = bar_notes(bar_el, &index, &score.parts[part].tuning);
            score.parts[part].bar_mut(bar).extend(notes);
        }
    }

    tempos(&root, &mut score);
    Ok(score)
}

fn index(root: &Element) -> Index<'_> {
    let mut index = Index::default();
    let table = |list: &'static str, item: &'static str| -> HashMap<&str, &Element> {
        root.child(list)
            .map(|l| {
                l.all(item)
                    .filter_map(|e| Some((e.attr("id")?, e)))
                    .collect()
            })
            .unwrap_or_default()
    };
    index.bars = table("Bars", "Bar");
    index.voices = table("Voices", "Voice");
    index.beats = table("Beats", "Beat");
    index.notes = table("Notes", "Note");
    for (id, rhythm) in table("Rhythms", "Rhythm") {
        index.rhythms.insert(id, rhythm_ticks(rhythm));
    }
    index
}

fn ids(text: &str) -> Vec<&str> {
    text.split_whitespace().collect()
}

fn staff_count(track: &Element) -> usize {
    let staves = track.child("Staves").map_or(0, |s| s.all("Staff").count());
    staves.max(1)
}

/// Value of the `<Property name="...">` found anywhere below `element`.
fn property<'a>(element: &'a Element, name: &str) -> Option<&'a Element> {
    element
        .every("Property")
        .into_iter()
        .find(|p| p.attr("name") == Some(name))
}

fn part(track: &Element) -> Part {
    let mut part = Part::new(track.value("Name").unwrap_or_default());

    // GP6 keeps the tuning in Track/Properties, GP7 in Track/Staves/Staff/Properties.
    if let Some(pitches) = property(track, "Tuning").and_then(|p| p.value("Pitches")) {
        part.tuning = pitches
            .split_whitespace()
            .filter_map(|p| p.parse::<u8>().ok())
            .filter(|p| *p <= 127)
            .map(Pitch)
            .collect();
    }

    // GP6: GeneralMidi/Program; GP7: Sounds/Sound/MIDI/Program.
    let program = track
        .path(&["GeneralMidi", "Program"])
        .or_else(|| track.path(&["Sounds", "Sound", "MIDI", "Program"]));
    part.program = program
        .and_then(|p| p.text().parse().ok())
        .filter(|p| *p <= 127);

    let channel = track
        .path(&["GeneralMidi", "PrimaryChannel"])
        .or_else(|| track.path(&["MidiConnection", "PrimaryChannel"]));
    let drum_channel = channel.is_some_and(|c| c.text() == "9");
    let kind = track
        .path(&["InstrumentSet", "Type"])
        .map(|t| t.text().to_lowercase())
        .or_else(|| {
            track
                .child("Instrument")
                .and_then(|i| i.attr("ref"))
                .map(str::to_lowercase)
        })
        .unwrap_or_default();
    part.percussion = drum_channel || kind.contains("drum");
    part.bass_hint = kind.contains("bass");
    part
}

fn head(master: &Element) -> Result<BarHead, ImportError> {
    let signature = master
        .value("Time")
        .and_then(signature)
        .ok_or_else(|| ImportError::bad("master bar without valid time signature"))?;
    let mut head = BarHead::new(signature);

    if let Some(repeat) = master.child("Repeat") {
        head.repeat_open = repeat.attr("start") == Some("true");
        if repeat.attr("end") == Some("true") {
            let plays: u32 = repeat
                .attr("count")
                .and_then(|c| c.parse().ok())
                .unwrap_or(2);
            head.repeat_plays = u8::try_from(plays).unwrap_or(u8::MAX);
        }
    }

    // "1 3" = first and third ending.
    for number in ids(master.value("AlternateEndings").unwrap_or_default()) {
        let Ok(number) = number.parse::<u32>() else {
            continue;
        };
        if let Some(bit) = number.checked_sub(1).and_then(|n| 1u32.checked_shl(n)) {
            head.endings |= bit;
        }
    }

    if let Some(directions) = master.child("Directions") {
        directions_of(directions, &mut head);
    }

    // <Key><AccidentalCount>-2</AccidentalCount><Mode>Major</Mode></Key>
    let fifths = master.path(&["Key", "AccidentalCount"]);
    head.key = fifths
        .and_then(|f| f.text().trim().parse::<i32>().ok())
        .and_then(score::key);

    if let Some(section) = master.child("Section") {
        let text = section.value("Text").filter(|t| !t.is_empty());
        let name = text.or_else(|| section.value("Letter").filter(|t| !t.is_empty()));
        head.marker = name.map(str::to_string);
    }
    Ok(head)
}

/// `<Directions><Target>Segno</Target><Jump>DaCapoAlFine</Jump></Directions>`.
fn directions_of(directions: &Element, head: &mut BarHead) {
    for child in directions.children() {
        match child.name() {
            "Target" => head.targets.extend(target(child.text())),
            "Jump" => head.jumps.extend(jump(child.text())),
            _ => {}
        }
    }
}

fn target(name: &str) -> Option<Target> {
    match name {
        "Segno" => Some(Target::Segno),
        "SegnoSegno" => Some(Target::SegnoSegno),
        "Coda" => Some(Target::Coda),
        "DoubleCoda" => Some(Target::DoubleCoda),
        "Fine" => Some(Target::Fine),
        _ => None,
    }
}

/// GPIF jump names: `DaCapo`, `DaSegnoSegnoAlFine`, `DaCoda`, ...
fn jump(name: &str) -> Option<Jump> {
    match name {
        "DaCoda" => return Some(Jump::ToCoda),
        "DaDoubleCoda" => return Some(Jump::ToDoubleCoda),
        _ => {}
    }
    let (from, until) = match name.split_once("Al") {
        Some((from, until)) => (from, until),
        None => (name, ""),
    };
    let until = match until {
        "" => Until::End,
        "Fine" => Until::Fine,
        "Coda" => Until::Coda,
        "DoubleCoda" => Until::DoubleCoda,
        _ => return None,
    };
    match from {
        "DaCapo" => Some(Jump::DaCapo(until)),
        "DaSegno" => Some(Jump::DalSegno(until)),
        "DaSegnoSegno" => Some(Jump::DalSegnoSegno(until)),
        _ => None,
    }
}

/// Parses "3/4".
fn signature(text: &str) -> Option<TimeSignature> {
    let (numerator, denominator) = text.split_once('/')?;
    let numerator: u8 = numerator.trim().parse().ok().filter(|n| *n > 0)?;
    let denominator: u8 = denominator.trim().parse().ok().filter(|d| *d > 0)?;
    Some(TimeSignature {
        numerator,
        denominator,
    })
}

fn rhythm_ticks(rhythm: &Element) -> u64 {
    let base = match rhythm.value("NoteValue").unwrap_or_default() {
        "DoubleWhole" => WHOLE_TICKS * 2,
        "Whole" => WHOLE_TICKS,
        "Half" => WHOLE_TICKS / 2,
        "Eighth" => WHOLE_TICKS / 8,
        "16th" => WHOLE_TICKS / 16,
        "32nd" => WHOLE_TICKS / 32,
        "64th" => WHOLE_TICKS / 64,
        "128th" => WHOLE_TICKS / 128,
        "256th" => WHOLE_TICKS / 256,
        _ => WHOLE_TICKS / 4,
    };
    let dots: u32 = rhythm
        .child("AugmentationDot")
        .and_then(|d| d.attr("count"))
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);

    // Each dot adds half of the previous addition: 1 dot ×1.5, 2 dots ×1.75.
    let mut ticks = base;
    let mut extra = base;
    for _ in 0..dots.min(4) {
        extra /= 2;
        ticks += extra;
    }

    let Some(tuplet) = rhythm.child("PrimaryTuplet") else {
        return ticks;
    };
    let num: u64 = tuplet.attr("num").and_then(|n| n.parse().ok()).unwrap_or(1);
    let den: u64 = tuplet.attr("den").and_then(|d| d.parse().ok()).unwrap_or(1);
    if num == 0 || den == 0 {
        return ticks;
    }
    if den == 1 {
        return tuplet_ticks(ticks, i32::try_from(num).unwrap_or(1));
    }
    ticks * den / num
}

fn bar_notes(bar: &Element, index: &Index, tuning: &[Pitch]) -> Vec<RawNote> {
    let mut notes = Vec::new();
    for voice_id in ids(bar.value("Voices").unwrap_or_default()) {
        if voice_id == NO_ID {
            continue;
        }
        let Some(voice) = index.voices.get(voice_id) else {
            continue;
        };
        let mut offset = 0u64;
        for beat_id in ids(voice.value("Beats").unwrap_or_default()) {
            let Some(beat) = index.beats.get(beat_id) else {
                continue;
            };
            // Grace notes sit before the beat and take no time of their own.
            if beat.child("GraceNotes").is_some() {
                continue;
            }
            let ticks = beat
                .child("Rhythm")
                .and_then(|r| r.attr("ref"))
                .and_then(|r| index.rhythms.get(r))
                .copied()
                .unwrap_or(WHOLE_TICKS / 4);
            let beat_fx = beat_techniques(beat);
            for note_id in ids(beat.value("Notes").unwrap_or_default()) {
                let Some(element) = index.notes.get(note_id) else {
                    continue;
                };
                if let Some(mut note) = note(element, tuning, offset, ticks) {
                    for t in &beat_fx {
                        note.add(*t);
                    }
                    notes.push(note);
                }
            }
            offset = offset.saturating_add(ticks);
        }
    }
    notes
}

fn beat_techniques(beat: &Element) -> Vec<Technique> {
    let mut found = Vec::new();
    if property(beat, "Slapped").is_some() {
        found.push(Technique::Slap);
    }
    if property(beat, "Popped").is_some() {
        found.push(Technique::Pop);
    }
    found
}

fn note(element: &Element, tuning: &[Pitch], offset: u64, duration: u64) -> Option<RawNote> {
    let string: Option<usize> = property(element, "String").and_then(|p| p.number("String"));
    let fret: Option<u8> = property(element, "Fret").and_then(|p| p.number("Fret"));
    let midi: Option<u8> = property(element, "Midi").and_then(|p| p.number("Number"));

    // Prefer tab position (string 0 = lowest); fall back to the MIDI number.
    let tab = match (string, fret) {
        (Some(s), Some(f)) if s < tuning.len() => Some((s, f)),
        _ => None,
    };
    let pitch = match tab {
        Some((s, f)) => tuning[s].transposed(i32::from(f)),
        None => midi.filter(|m| *m <= 127).map(Pitch),
    }?;

    let mut note = RawNote::new(offset, duration, pitch);
    if let Some((s, f)) = tab {
        note.string = u8::try_from(s).ok();
        note.fret = Some(f);
    }
    note.tied = element
        .child("Tie")
        .is_some_and(|t| t.attr("destination") == Some("true"));
    techniques(element, &mut note);
    Some(note)
}

fn techniques(element: &Element, note: &mut RawNote) {
    let flags = [
        ("Muted", Technique::Muted),
        ("PalmMuted", Technique::Muted),
        ("Slide", Technique::Slide),
        ("HarmonicType", Technique::Harmonic),
        ("Bended", Technique::Bend),
    ];
    for (name, technique) in flags {
        if property(element, name).is_some() {
            note.add(technique);
        }
    }
    if property(element, "HopoDestination").is_some() {
        note.legato = Legato::Target;
    } else if property(element, "HopoOrigin").is_some() {
        note.legato = Legato::Origin;
    }
    if element.child("AntiAccent").is_some() {
        note.add(Technique::Ghost);
    }
    if element.child("Vibrato").is_some() {
        note.add(Technique::Vibrato);
    }
    let accent: u32 = element.number("Accent").unwrap_or(0);
    if accent & ACCENT_STACCATO != 0 {
        note.add(Technique::Staccato);
    }
    if accent & (ACCENT_HEAVY | ACCENT_NORMAL) != 0 {
        note.add(Technique::Accent);
    }
}

/// Tempo automations of the master track: "bpm unit" at a bar and a position ratio.
fn tempos(root: &Element, score: &mut Score) {
    let Some(list) = root.path(&["MasterTrack", "Automations"]) else {
        return;
    };
    for automation in list.all("Automation") {
        if automation.value("Type") != Some("Tempo") {
            continue;
        }
        let Some(bpm) = automation.value("Value").and_then(tempo_value) else {
            continue;
        };
        let bar: usize = automation.number("Bar").unwrap_or(0);
        let ratio: f64 = automation.number("Position").unwrap_or(0.0);
        let Some(head) = score.bars.get_mut(bar) else {
            continue;
        };
        let offset = (ratio.clamp(0.0, 1.0) * head.signature.bar_ticks() as f64) as u64;
        head.add_tempo(offset, bpm);
        if bar == 0 && offset == 0 {
            score.bpm = bpm;
        }
    }
}

/// "120 2" → 120 quarter notes per minute. Unit: 1 eighth, 2 quarter, 3 dotted quarter,
/// 4 half, 5 dotted half.
fn tempo_value(text: &str) -> Option<f64> {
    let mut parts = text.split_whitespace();
    let bpm: f64 = parts.next()?.parse().ok()?;
    let factor = match parts.next() {
        Some("1") => 0.5,
        Some("3") => 1.5,
        Some("4") => 2.0,
        Some("5") => 3.0,
        _ => 1.0,
    };
    let bpm = bpm * factor;
    (bpm.is_finite() && bpm > 0.0).then_some(bpm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jump_names() {
        assert_eq!(jump("DaCapo"), Some(Jump::DaCapo(Until::End)));
        assert_eq!(jump("DaSegnoAlCoda"), Some(Jump::DalSegno(Until::Coda)));
        assert_eq!(
            jump("DaSegnoSegnoAlDoubleCoda"),
            Some(Jump::DalSegnoSegno(Until::DoubleCoda))
        );
        assert_eq!(jump("DaCoda"), Some(Jump::ToCoda));
        assert_eq!(jump("Nonsense"), None);
    }

    #[test]
    fn tempo_units() {
        assert_eq!(tempo_value("120 2"), Some(120.0));
        assert_eq!(tempo_value("60 4"), Some(120.0));
        assert_eq!(tempo_value("90"), Some(90.0));
        assert_eq!(tempo_value("0 2"), None);
    }

    #[test]
    fn rhythms() {
        let doc = br#"<R><Rhythm><NoteValue>Eighth</NoteValue><AugmentationDot count="1"/></Rhythm>
            <Rhythm><NoteValue>Quarter</NoteValue><PrimaryTuplet num="3" den="2"/></Rhythm></R>"#;
        let root = xml::parse(doc).unwrap();
        let ticks: Vec<u64> = root.all("Rhythm").map(rhythm_ticks).collect();
        assert_eq!(ticks, vec![720, 640]);
    }
}
