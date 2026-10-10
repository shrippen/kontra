//! Standard MIDI files (type 0 and 1). Every (track, channel) pair with notes becomes a part;
//! ticks are rescaled to 960 per quarter, bars come from the time signature events.

use crate::error::ImportError;
use crate::score::{self, BarHead, Part, RawNote, Score, DEFAULT_BPM};
use crate::unroll::MAX_BARS;
use kontra_model::{Pitch, TimeSignature, TICKS_PER_QUARTER};
use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};
use std::collections::{BTreeMap, HashMap, VecDeque};

/// General MIDI drum channel (10, 0-based 9).
const DRUM_CHANNEL: u8 = 9;
/// Header byte holding the high bits of the time division ("MThd", length, format, tracks).
const DIVISION_AT: usize = 12;
/// Set in the division: SMPTE frames instead of ticks per quarter.
const SMPTE_BIT: u8 = 0x80;
const MICROS_PER_MINUTE: f64 = 60_000_000.0;
/// Largest time signature denominator exponent accepted (2^6 = 64th).
const MAX_DENOMINATOR_POW: u8 = 6;

/// Notes, names and programs collected from one track.
#[derive(Debug, Default)]
struct TrackData {
    name: String,
    /// Notes per channel: (start, end, key).
    notes: BTreeMap<u8, Vec<(u64, u64, u8)>>,
    programs: HashMap<u8, u8>,
}

pub(crate) fn parse(data: &[u8]) -> Result<Score, ImportError> {
    // SMPTE timing is refused before midly sees it: its frame-rate negation overflows on 0x80.
    if data.get(DIVISION_AT).is_some_and(|b| b & SMPTE_BIT != 0) {
        return Err(ImportError::Unsupported(
            "MIDI with SMPTE timing".to_string(),
        ));
    }
    let smf = Smf::parse(data).map_err(|e| ImportError::Malformed(format!("MIDI: {e}")))?;
    let ppq = match smf.header.timing {
        Timing::Metrical(ppq) => u64::from(ppq.as_int()),
        Timing::Timecode(..) => {
            return Err(ImportError::Unsupported(
                "MIDI with SMPTE timing".to_string(),
            ))
        }
    };
    if ppq == 0 {
        return Err(ImportError::bad("MIDI resolution 0"));
    }
    let scale = |tick: u64| -> u64 {
        let scaled = u128::from(tick) * u128::from(TICKS_PER_QUARTER) / u128::from(ppq);
        u64::try_from(scaled).unwrap_or(u64::MAX)
    };

    let mut tempos: Vec<(u64, f64)> = Vec::new();
    let mut signatures: Vec<(u64, TimeSignature)> = Vec::new();
    let mut keys: Vec<(u64, i8)> = Vec::new();
    let mut markers: Vec<(u64, String)> = Vec::new();
    let mut tracks = Vec::new();
    for events in &smf.tracks {
        let mut track = TrackData::default();
        let mut open: HashMap<(u8, u8), VecDeque<u64>> = HashMap::new();
        let mut tick = 0u64;
        for event in events {
            tick = tick.saturating_add(u64::from(event.delta.as_int()));
            let at = scale(tick);
            match event.kind {
                TrackEventKind::Midi { channel, message } => {
                    let channel = channel.as_int();
                    midi_event(&mut track, &mut open, channel, message, at);
                }
                TrackEventKind::Meta(meta) => {
                    if let MetaMessage::KeySignature(fifths, _) = meta {
                        keys.extend(score::key(i32::from(fifths)).map(|k| (at, k)));
                    }
                    if let MetaMessage::Marker(text) = meta {
                        let text = String::from_utf8_lossy(text).trim().to_string();
                        if !text.is_empty() {
                            markers.push((at, text));
                        }
                    }
                    meta_event(&mut track, &mut tempos, &mut signatures, meta, at);
                }
                _ => {}
            }
        }

        // Notes still sounding at the end of the track stop there.
        let end = scale(tick);
        for ((channel, key), starts) in open {
            for start in starts {
                track.notes.entry(channel).or_default().push((
                    start,
                    end.max(start.saturating_add(1)),
                    key,
                ));
            }
        }
        tracks.push(track);
    }

    let mut score = Score::new();
    // Type 1 files name the song in the first track, which holds only tempo and meta events.
    if let Some(title) = tracks
        .first()
        .filter(|t| t.notes.is_empty())
        .and_then(|t| song_title(&t.name))
    {
        score.title = title;
    }
    let end = tracks
        .iter()
        .flat_map(|t| t.notes.values().flatten())
        .map(|n| n.1)
        .max()
        .unwrap_or(0);
    let starts = bars(&mut score, &signatures, end)?;
    place_tempos(&mut score, &starts, &tempos);
    place_keys(&mut score, &starts, &keys);
    place_markers(&mut score, &starts, &markers);
    parts(&mut score, &starts, &tracks);
    Ok(score)
}

fn midi_event(
    track: &mut TrackData,
    open: &mut HashMap<(u8, u8), VecDeque<u64>>,
    channel: u8,
    message: MidiMessage,
    at: u64,
) {
    match message {
        MidiMessage::NoteOn { key, vel } if vel.as_int() > 0 => {
            open.entry((channel, key.as_int()))
                .or_default()
                .push_back(at);
        }
        MidiMessage::NoteOn { key, .. } | MidiMessage::NoteOff { key, .. } => {
            let key = key.as_int();
            let Some(start) = open.get_mut(&(channel, key)).and_then(VecDeque::pop_front) else {
                return;
            };
            let end = at.max(start.saturating_add(1));
            track
                .notes
                .entry(channel)
                .or_default()
                .push((start, end, key));
        }
        MidiMessage::ProgramChange { program } => {
            track.programs.entry(channel).or_insert(program.as_int());
        }
        _ => {}
    }
}

fn meta_event(
    track: &mut TrackData,
    tempos: &mut Vec<(u64, f64)>,
    signatures: &mut Vec<(u64, TimeSignature)>,
    meta: MetaMessage,
    at: u64,
) {
    match meta {
        MetaMessage::Tempo(micros) if micros.as_int() > 0 => {
            tempos.push((at, MICROS_PER_MINUTE / f64::from(micros.as_int())));
        }
        MetaMessage::TimeSignature(numerator, pow, _, _) => {
            if numerator == 0 || pow > MAX_DENOMINATOR_POW {
                return;
            }
            let signature = TimeSignature {
                numerator,
                denominator: 1 << pow,
            };
            signatures.push((at, signature));
        }
        MetaMessage::TrackName(name) if track.name.is_empty() => {
            track.name = String::from_utf8_lossy(name).trim().to_string();
        }
        MetaMessage::InstrumentName(name) if track.name.is_empty() => {
            track.name = String::from_utf8_lossy(name).trim().to_string();
        }
        _ => {}
    }
}

/// Bars up to `end`; a signature change takes effect at the next bar line.
/// Returns the start tick of every bar.
fn bars(
    score: &mut Score,
    signatures: &[(u64, TimeSignature)],
    end: u64,
) -> Result<Vec<u64>, ImportError> {
    let mut signatures = signatures.to_vec();
    signatures.sort_by_key(|s| s.0);
    let mut starts = Vec::new();
    let mut signature = TimeSignature::COMMON;
    let mut next = 0;
    let mut tick = 0u64;
    loop {
        while let Some(&(_, changed)) = signatures.get(next).filter(|s| s.0 <= tick) {
            signature = changed;
            next += 1;
        }
        starts.push(tick);
        score.bars.push(BarHead::new(signature));
        if starts.len() > MAX_BARS {
            return Err(ImportError::TooLong(MAX_BARS));
        }
        tick = tick.saturating_add(signature.bar_ticks());
        if tick >= end {
            return Ok(starts);
        }
    }
}

/// Key signature events (FF 59) apply to the bar they fall in.
fn place_keys(score: &mut Score, starts: &[u64], keys: &[(u64, i8)]) {
    let mut keys = keys.to_vec();
    keys.sort_by_key(|k| k.0);
    for (tick, fifths) in keys {
        let (bar, _) = locate(starts, tick);
        if let Some(head) = score.bars.get_mut(bar) {
            head.key = Some(fifths);
        }
    }
}

/// Marker events name the bar they fall in (rehearsal marks, chord names of exercises).
fn place_markers(score: &mut Score, starts: &[u64], markers: &[(u64, String)]) {
    for (tick, text) in markers {
        let (bar, _) = locate(starts, *tick);
        if let Some(head) = score.bars.get_mut(bar) {
            head.marker.get_or_insert_with(|| text.clone());
        }
    }
}

/// Bar index and offset of `tick`.
fn locate(starts: &[u64], tick: u64) -> (usize, u64) {
    let bar = starts.partition_point(|s| *s <= tick).saturating_sub(1);
    (bar, tick - starts[bar].min(tick))
}

fn place_tempos(score: &mut Score, starts: &[u64], tempos: &[(u64, f64)]) {
    let mut tempos = tempos.to_vec();
    tempos.sort_by_key(|t| t.0);
    score.bpm = tempos
        .first()
        .filter(|t| t.0 == 0)
        .map_or(DEFAULT_BPM, |t| t.1);
    for (tick, bpm) in tempos {
        let (bar, offset) = locate(starts, tick);
        if let Some(head) = score.bars.get_mut(bar) {
            head.add_tempo(offset, bpm);
        }
    }
}

fn parts(score: &mut Score, starts: &[u64], tracks: &[TrackData]) {
    // Program changes may sit in another track of a type 1 file; fall back to the first seen.
    let mut global: HashMap<u8, u8> = HashMap::new();
    for track in tracks {
        for (&channel, &program) in &track.programs {
            global.entry(channel).or_insert(program);
        }
    }

    for (number, track) in tracks.iter().enumerate() {
        for (&channel, notes) in &track.notes {
            let mut name = if track.name.is_empty() {
                format!("Track {}", number + 1)
            } else {
                track.name.clone()
            };
            if track.notes.len() > 1 {
                name = format!("{name} ({})", channel + 1);
            }
            let mut part = Part::new(&name);
            part.program = track
                .programs
                .get(&channel)
                .or_else(|| global.get(&channel))
                .copied();
            part.percussion = channel == DRUM_CHANNEL;
            for &(start, end, key) in notes {
                let (bar, offset) = locate(starts, start);
                let note = RawNote::new(offset, end - start, Pitch(key));
                part.bar_mut(bar).push(note);
            }
            score.parts.push(part);
        }
    }
}

/// The first track's name as song title, unless it is a sequencer's generic name for the tempo
/// track (QA P2: a song was called "Tempo"); then the app falls back to the file name.
fn song_title(name: &str) -> Option<String> {
    const GENERIC: [&str; 6] = [
        "tempo",
        "tempo track",
        "conductor",
        "untitled",
        "track 1",
        "meta",
    ];
    let name = name.trim();
    let generic = GENERIC.iter().any(|g| name.eq_ignore_ascii_case(g));
    (!name.is_empty() && !generic).then(|| name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_tempo_track_names_are_no_title() {
        assert_eq!(song_title("Tempo"), None);
        assert_eq!(song_title(" Conductor "), None);
        assert_eq!(song_title(""), None);
        assert_eq!(
            song_title("Seven Nation Army"),
            Some("Seven Nation Army".into())
        );
    }

    #[test]
    fn locates_ticks() {
        let starts = [0, 3840, 6720];
        assert_eq!(locate(&starts, 0), (0, 0));
        assert_eq!(locate(&starts, 4000), (1, 160));
        assert_eq!(locate(&starts, 9000), (2, 2280));
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse(b"MThd garbage").is_err());
    }
}
