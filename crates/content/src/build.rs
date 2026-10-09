//! Turns a `Score` into a linear `Arrangement`: unrolls repeats, places notes and tempo
//! changes on the timeline, resolves ties and hammer-ons, picks the instrument.

use crate::error::ImportError;
use crate::score::{BarHead, Legato, Part, Score};
use crate::tab::assign_tab;
use kontra_model::{
    Arrangement, Bar, BassKind, Instrument, Marker, Note, Pitch, Technique, TempoChange, TempoMap,
    Tuning,
};
use std::collections::HashMap;

/// Upper bound for the unrolled timeline; protects against absurd repeat counts.
pub(crate) const MAX_BARS: usize = 20_000;

/// More plays than this are clamped (Guitar Pro allows up to 99, nobody writes that).
const MAX_PLAYS: u8 = 32;

/// Lowest note below which a 5-string bass is assumed when the file has no usable tuning.
const FIVE_STRING_BELOW: Pitch = Pitch(28);

/// Source bar indices in playing order, repeats and alternate endings unrolled.
///
/// Example: bars `|: A | B(1.) :| C(2.) |` give `A B A C`.
pub(crate) fn unroll(bars: &[BarHead]) -> Result<Vec<usize>, ImportError> {
    let mut order = Vec::new();
    let mut jumps = vec![0u8; bars.len()];
    let mut start = 0;
    let mut pass: u32 = 1;
    let mut i = 0;

    while i < bars.len() {
        let bar = &bars[i];

        // A repeat opened on a fresh arrival (not after jumping back) starts a new section.
        if bar.repeat_open && i != start {
            start = i;
            pass = 1;
        }

        if !ending_plays(bar.endings, pass) {
            i += 1;
            continue;
        }

        order.push(i);
        if order.len() > MAX_BARS {
            return Err(ImportError::TooLong(MAX_BARS));
        }

        let plays = bar.repeat_plays.min(MAX_PLAYS);
        if plays == 0 {
            i += 1;
            continue;
        }

        // Jump back until all plays are done, then continue after the repeat.
        if jumps[i] + 1 < plays {
            jumps[i] += 1;
            pass += 1;
            i = start;
            continue;
        }
        jumps[i] = 0;
        pass = 1;
        i += 1;
        start = i;
    }
    Ok(order)
}

/// True if a bar with the `endings` mask is played on pass `pass` (1-based).
fn ending_plays(endings: u32, pass: u32) -> bool {
    if endings == 0 {
        return true;
    }
    let Some(bit) = 1u32.checked_shl(pass.saturating_sub(1)) else {
        return false;
    };
    endings & bit != 0
}

/// Builds the arrangement for part `index`.
pub(crate) fn arrangement(score: &Score, index: usize) -> Result<Arrangement, ImportError> {
    let part = score.parts.get(index).ok_or(ImportError::NoTrack(index))?;
    let order = unroll(&score.bars)?;
    let (instrument, tab) = instrument(part);

    let mut arrangement = Arrangement::new(&score.title, instrument);
    arrangement.artist = score.artist.clone();

    // Lay out bars, markers, tempo changes and notes on the linear timeline.
    let mut tempos = Vec::new();
    let mut notes = Vec::new();
    let mut tick: u64 = 0;
    for &source in &order {
        let head = &score.bars[source];
        arrangement.bars.push(Bar {
            start_ticks: tick,
            signature: head.signature,
            source_index: u32::try_from(source).unwrap_or(u32::MAX),
        });
        if let Some(name) = &head.marker {
            arrangement.markers.push(Marker {
                tick,
                name: name.clone(),
            });
        }
        for &(offset, bpm) in &head.tempos {
            tempos.push(TempoChange {
                tick: tick.saturating_add(offset),
                bpm,
            });
        }
        for raw in part.bars.get(source).into_iter().flatten() {
            let mut note = Note::new(tick.saturating_add(raw.offset), raw.duration, raw.pitch);
            if tab == TabSource::File {
                note.string = raw.string;
                note.fret = raw.fret;
            }
            note.tied = raw.tied;
            note.techniques = raw.techniques.clone();
            notes.push((note, raw.legato));
        }
        tick = tick.saturating_add(head.signature.bar_ticks());
    }

    // The start tempo goes last so a change written at tick 0 wins (`from_changes` keeps the first).
    tempos.push(TempoChange {
        tick: 0,
        bpm: score.bpm,
    });
    arrangement.tempo = TempoMap::from_changes(tempos);

    notes.sort_by_key(|(n, _)| (n.start_ticks, n.pitch));
    resolve_ties(&mut notes);
    resolve_legato(&mut notes);
    arrangement.notes = notes.into_iter().map(|(n, _)| n).collect();
    arrangement.normalize();
    assign_tab(&mut arrangement);
    Ok(arrangement)
}

/// Whether string/fret from the file fit the chosen instrument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TabSource {
    File,
    Generated,
}

/// Instrument from the part's tuning, or a standard bass when the tuning is unusable.
fn instrument(part: &Part) -> (Instrument, TabSource) {
    let frets = part.frets.clamp(1, 36);
    let tuning = Tuning {
        name: tuning_name(&part.tuning),
        strings: part.tuning.clone(),
    };
    if let Ok(found) = Instrument::new(BassKind::Fretted, tuning, frets) {
        return (found, TabSource::File);
    }

    // No tab information (MIDI, MusicXML) or an unusual string count: pick by range.
    let lowest = part.notes().map(|n| n.pitch).min();
    let five = lowest.is_some_and(|p| p < FIVE_STRING_BELOW);
    if !five {
        return (Instrument::default_bass(), TabSource::Generated);
    }
    let bass5 = Instrument::new(BassKind::Fretted, Tuning::bass5_standard(), 24);
    (
        bass5.unwrap_or_else(|_| Instrument::default_bass()),
        TabSource::Generated,
    )
}

/// Preset name if the strings match a preset, else "custom".
fn tuning_name(strings: &[Pitch]) -> String {
    let presets = [
        Tuning::bass4_standard(),
        Tuning::bass5_standard(),
        Tuning::bass6_standard(),
        Tuning::bass4_drop_d(),
    ];
    presets
        .into_iter()
        .find(|t| t.strings == strings)
        .map_or_else(|| "custom".to_string(), |t| t.name)
}

/// A tied note sounds like its predecessor on the same string: copy pitch and fret.
fn resolve_ties(notes: &mut [(Note, Legato)]) {
    let mut last: HashMap<u8, usize> = HashMap::new();
    for i in 0..notes.len() {
        let Some(string) = notes[i].0.string else {
            continue;
        };
        if notes[i].0.tied {
            if let Some(&prev) = last.get(&string) {
                notes[i].0.pitch = notes[prev].0.pitch;
                notes[i].0.fret = notes[prev].0.fret;
            }
        }
        last.insert(string, i);
    }
}

/// Marks hammer-ons and pull-offs: upward legato is a hammer-on, downward a pull-off.
fn resolve_legato(notes: &mut [(Note, Legato)]) {
    // Key: string, or `None` for notes without tab.
    let mut last: HashMap<Option<u8>, usize> = HashMap::new();
    for i in 0..notes.len() {
        let key = notes[i].0.string;
        let prev = last.insert(key, i);
        let Some(prev) = prev else {
            continue;
        };
        let wanted = notes[prev].1 == Legato::Origin || notes[i].1 == Legato::Target;
        if !wanted {
            continue;
        }
        let (from, to) = (notes[prev].0.pitch, notes[i].0.pitch);
        let technique = match to.cmp(&from) {
            std::cmp::Ordering::Greater => Technique::HammerOn,
            std::cmp::Ordering::Less => Technique::PullOff,
            std::cmp::Ordering::Equal => continue,
        };
        if !notes[i].0.techniques.contains(&technique) {
            notes[i].0.techniques.push(technique);
        }
    }
}

/// Score for "this part is the bass"; higher is more likely. See `is_bass`.
pub(crate) fn bass_score(part: &Part) -> i32 {
    if part.percussion {
        return -10;
    }
    let mut score = 0;
    if bass_name(&part.name) {
        score += 4;
    }
    if part.bass_hint {
        score += 4;
    }
    if part.program.is_some_and(|p| (BASS_PROGRAMS).contains(&p)) {
        score += 3;
    }
    score += tuning_score(&part.tuning);

    // Without notes there is nothing to play; a low register hints at a bass line.
    let pitches: Vec<u32> = part.notes().map(|n| u32::from(n.pitch.0)).collect();
    if pitches.is_empty() {
        return score - 3;
    }
    let average = pitches.iter().sum::<u32>() / pitches.len() as u32;
    if average < LOW_REGISTER {
        score += 1;
    }
    score
}

/// Minimum `bass_score` to call a part a bass.
pub(crate) const BASS_THRESHOLD: i32 = 3;

/// General MIDI programs 33-40 (0-based 32..=39): acoustic to synth bass.
const BASS_PROGRAMS: std::ops::RangeInclusive<u8> = 32..=39;

/// Average MIDI note below C3: typical bass register.
const LOW_REGISTER: u32 = 48;

/// Highest open string a bass has (6-string bass: C3 = 48).
const BASS_TOP_STRING: Pitch = Pitch(50);

/// Lowest open string a bass has at most (4-string in E: E1 = 28; drop tunings go lower).
const BASS_LOW_STRING: Pitch = Pitch(35);

fn tuning_score(tuning: &[Pitch]) -> i32 {
    let (Some(low), Some(high)) = (tuning.iter().min(), tuning.iter().max()) else {
        return 0;
    };
    let bass_like =
        (4..=6).contains(&tuning.len()) && *high <= BASS_TOP_STRING && *low <= BASS_LOW_STRING;
    if bass_like {
        3
    } else {
        -2
    }
}

/// Track names that mean bass in common languages; excludes bassoon and bass drum.
fn bass_name(name: &str) -> bool {
    let name = name.to_lowercase();
    let excluded = ["bassoon", "drum", "kick", "bassdrum"];
    if excluded.iter().any(|e| name.contains(e)) {
        return false;
    }
    let words = ["bass", "bajo", "baixo", "basso", "bas "];
    words.iter().any(|w| name.contains(w)) || name.trim_end().ends_with(" bas")
}

#[cfg(test)]
mod tests {
    use super::*;
    use kontra_model::TimeSignature;

    fn bars(n: usize) -> Vec<BarHead> {
        vec![BarHead::new(TimeSignature::COMMON); n]
    }

    #[test]
    fn plain_order() {
        assert_eq!(unroll(&bars(3)).unwrap(), vec![0, 1, 2]);
    }

    #[test]
    fn simple_repeat() {
        let mut b = bars(3);
        b[0].repeat_open = true;
        b[1].repeat_plays = 3;
        assert_eq!(unroll(&b).unwrap(), vec![0, 1, 0, 1, 0, 1, 2]);
    }

    #[test]
    fn repeat_without_open_starts_at_beginning() {
        let mut b = bars(4);
        b[1].repeat_plays = 2;
        b[3].repeat_plays = 2;
        assert_eq!(unroll(&b).unwrap(), vec![0, 1, 0, 1, 2, 3, 2, 3]);
    }

    #[test]
    fn alternate_endings() {
        // |: 0 | 1 (1.) :| 2 (2.) | 3
        let mut b = bars(4);
        b[0].repeat_open = true;
        b[1].endings = 0b01;
        b[1].repeat_plays = 2;
        b[2].endings = 0b10;
        assert_eq!(unroll(&b).unwrap(), vec![0, 1, 0, 2, 3]);
    }

    #[test]
    fn endings_before_close() {
        // GP7 style: |: 0 | 1 (1.,3.) | 2 (2.) | 3 x4 :| 4
        let mut b = bars(5);
        b[0].repeat_open = true;
        b[1].endings = 0b101;
        b[2].endings = 0b010;
        b[3].repeat_plays = 4;
        let order = unroll(&b).unwrap();
        assert_eq!(order, vec![0, 1, 3, 0, 2, 3, 0, 1, 3, 0, 3, 4]);
    }

    #[test]
    fn too_long_is_an_error() {
        let mut b = bars(MAX_BARS);
        b[MAX_BARS - 1].repeat_plays = 2;
        assert!(matches!(unroll(&b), Err(ImportError::TooLong(_))));
    }

    #[test]
    fn bass_names() {
        assert!(bass_name("Electric Bass"));
        assert!(bass_name("E-Bass"));
        assert!(bass_name("Bajo"));
        assert!(!bass_name("Bassoon"));
        assert!(!bass_name("Bass Drum"));
        assert!(!bass_name("Guitar"));
    }
}
