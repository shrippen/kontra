//! Notation for the views (LH-ANZ): notes, bars, staff with rests and ties, hand positions.

use crate::{technique_key, Core};
use kontra_model::Arrangement;
use kontra_session::technique_signs;
use serde_json::json;

impl Core {
    /// Notes for the views: time and duration in seconds, string/fret, beats, midi.
    pub fn notes_json(&self) -> String {
        let Some(a) = self.arrangement() else {
            return "[]".into();
        };
        let positions = hand_positions(&a.notes);
        let techniques = technique_signs(a);
        let notes: Vec<_> = a
            .notes
            .iter()
            .zip(positions)
            .enumerate()
            .map(|(i, (n, position))| {
                let time = a.tempo.seconds_at(n.start_ticks);
                let fret = n.fret.unwrap_or(0);
                json!({
                    "time": time,
                    "duration": a.tempo.seconds_at(n.end_ticks()) - time,
                    "string": n.string.unwrap_or(0),
                    "fret": n.fret.unwrap_or(0),
                    "midi": n.pitch.0,
                    "beats": written_beats(n.duration_ticks),
                    "tied": n.tied,
                    "state": "pending",
                    // Hand position (LH-ANZ-05) and the finger it suggests (0 = open string).
                    "position": position,
                    "finger": if fret == 0 { 0 } else { fret + 1 - position.min(fret) },
                    // Kante's technique sign (LH-BEW-06); a slide shows where it starts.
                    "technique": techniques.get(&i).map_or("", |t| technique_key(*t)),
                    "techniqueResult": "",
                })
            })
            .collect();
        serde_json::Value::Array(notes).to_string()
    }

    pub fn bars_json(&self) -> String {
        let Some(a) = self.arrangement() else {
            return "[]".into();
        };
        let bars: Vec<_> = a
            .bars
            .iter()
            .enumerate()
            .map(|(i, b)| {
                json!({
                    "time": a.tempo.seconds_at(b.start_ticks),
                    "number": b.source_index + 1,
                    "numerator": b.signature.numerator,
                    "denominator": b.signature.denominator,
                    // Written repeat signs and volta numbers (LH-ANZ-02), e.g. "endings": [1, 2].
                    "repeatStart": b.repeat_open,
                    "repeatEnd": b.repeat_close,
                    "endings": (0..32).filter(|i| b.endings & (1 << i) != 0).map(|i| i + 1).collect::<Vec<u32>>(),
                    // Key per bar and chord symbols (Kante 1.27 KanteBassStaff).
                    "keyFifths": b.key_fifths,
                    "chord": a.markers.iter().find(|m| m.tick == b.start_ticks && is_chord(m)).map(|m| m.name.clone()),
                    // Rehearsal marks (Kante 1.50): the parts beginning in this bar.
                    "section": bar_sections(a, i)
                        .map(|names| names.iter().map(|n| self.part_name(n)).collect::<Vec<_>>()),
                })
            })
            .collect();
        serde_json::Value::Array(bars).to_string()
    }

    /// A part name in the UI's language where Kontra's generator wrote it ("Strophe" → "Verse");
    /// any other name as the file has it. Intro, Bridge, Outro and Chorus read alike.
    fn part_name(&self, name: &str) -> String {
        GENERATED_PARTS
            .iter()
            .find(|(written, _)| *written == name)
            .map_or_else(|| name.to_string(), |(_, key)| self.tr(key, &[]))
    }

    /// The notation view (Kante `KanteBassStaff`): notes in the order of `notes_json`, rests,
    /// key signature.
    pub fn staff_json(&self) -> String {
        self.arrangement()
            .map_or_else(empty_staff, staff)
            .to_string()
    }
}

/// Frets one hand covers without shifting: index finger on the position's fret, pinky three up.
const HAND_SPAN: u8 = 3;

/// Hand position (Lage: the index finger's fret, at least 1) for every note, from the tab.
/// The hand stays while notes fit; a note outside moves it to the position, among those
/// holding that note, that also holds the most notes after it. Open strings fit anywhere.
///
/// ```text
/// frets 3 5 7 9 7      positions 3 3 6 6 6    (7 needs a shift; 6–9 holds 7, 9 and 7)
/// ```
pub(crate) fn hand_positions(notes: &[kontra_model::Note]) -> Vec<u8> {
    let frets: Vec<u8> = notes.iter().map(|n| n.fret.unwrap_or(0)).collect();
    let fits =
        |position: u8, fret: u8| fret == 0 || (position..=position + HAND_SPAN).contains(&fret);
    let mut out = Vec::with_capacity(frets.len());
    let mut position = 0u8;
    for (index, &fret) in frets.iter().enumerate() {
        if position == 0 || !fits(position, fret) {
            position = best_position(&frets[index..], position, fits);
        }
        out.push(position.max(1));
    }
    out
}

/// The position for `frets[0]` that holds the longest run of the following notes; ties keep
/// the hand nearer to `from`.
fn best_position(frets: &[u8], from: u8, fits: impl Fn(u8, u8) -> bool) -> u8 {
    let Some(&first) = frets.iter().find(|f| **f > 0) else {
        return from;
    };
    // The first position puts the index finger on the first note when nothing else decides.
    let from = if from == 0 { first } else { from };
    let candidates = first.saturating_sub(HAND_SPAN).max(1)..=first;
    let reach = |p: u8| frets.iter().take_while(|f| fits(p, **f)).count();
    candidates
        .map(|p| (reach(p), std::cmp::Reverse(p.abs_diff(from)), p))
        .max()
        .map_or(first, |(_, _, p)| p)
}

/// Written length in quarter notes, rounded to the nearest sixteenth: played (MIDI) notes are
/// often a little shorter than written (staccato), and notation must not turn eighths into
/// sixteenths.
pub(crate) fn written_beats(duration_ticks: u64) -> f64 {
    written_ticks(duration_ticks) as f64 / kontra_model::TICKS_PER_QUARTER as f64
}

/// Written length in ticks, rounded as `written_beats`.
fn written_ticks(duration_ticks: u64) -> u64 {
    let sixteenth = kontra_model::TICKS_PER_QUARTER / 4;
    ((duration_ticks as f64 / sixteenth as f64).round() as u64).max(1) * sixteenth
}

/// Bass is written an octave above its sound (bass clef, as in MuseScore and Guitar Pro):
/// low B (B0, 6 ledger lines) reads as B1 (2 ledger lines). Only the staff is shifted; the
/// importers already turned any written octave (<transpose>) into sounding pitch, and tab,
/// scoring and tuner stay sounding.
const WRITTEN_OCTAVE_UP: u8 = 12;

fn empty_staff() -> serde_json::Value {
    json!({ "notes": [], "rests": [], "keyFifths": 0, "chords": [] })
}

/// `staff_json` for an arrangement. The staff shows one key; it is the first bar's.
pub(crate) fn staff(a: &Arrangement) -> serde_json::Value {
    let techniques = technique_signs(a);
    let notes: Vec<_> = a
        .notes
        .iter()
        .zip(ties_forward(&a.notes))
        .enumerate()
        .map(|(i, (n, tied))| {
            json!({
                "time": a.tempo.seconds_at(n.start_ticks),
                "beats": written_beats(n.duration_ticks),
                "midi": n.pitch.0.saturating_add(WRITTEN_OCTAVE_UP),
                "tied": tied,
                "state": "pending",
                "technique": techniques.get(&i).map_or("", |t| technique_key(*t)),
                "techniqueResult": "",
            })
        })
        .collect();
    let quarter = kontra_model::TICKS_PER_QUARTER as f64;
    let rests: Vec<_> = rests(a)
        .into_iter()
        .map(|(tick, ticks)| {
            json!({ "time": a.tempo.seconds_at(tick), "beats": ticks as f64 / quarter })
        })
        .collect();
    let key = a.bars.first().map_or(0, |b| b.key_fifths);
    // Chord symbols within a bar (MusicXML: B♭m7 on beat 3); those on a bar start come with
    // `bars_json`.
    let chords: Vec<_> = a
        .markers
        .iter()
        .filter(|m| is_chord(m) && !a.bars.iter().any(|b| b.start_ticks == m.tick))
        .map(|m| json!({ "time": a.tempo.seconds_at(m.tick), "text": m.name }))
        .collect();
    json!({ "notes": notes, "rests": rests, "keyFifths": key, "chords": chords })
}

/// Key signature of the bar holding `tick` (0 = C major without bars).
pub(crate) fn key_at(a: &Arrangement, tick: u64) -> i8 {
    a.bar_at(tick)
        .and_then(|b| a.bars.get(b))
        .map_or(0, |b| b.key_fifths)
}

/// Kontra marks the note that continues a tie (`tied`: no new attack); Kante's staff marks
/// the note tied to the next one. The last earlier note of the same pitch gets the tie.
fn ties_forward(notes: &[kontra_model::Note]) -> Vec<bool> {
    let mut out = vec![false; notes.len()];
    for (index, note) in notes.iter().enumerate() {
        if !note.tied {
            continue;
        }
        let before = notes[..index].iter().rposition(|n| n.pitch == note.pitch);
        if let Some(before) = before {
            out[before] = true;
        }
    }
    out
}

/// Rests as (start, length) in ticks: the gaps between written notes in each bar, on the
/// sixteenth grid. A gap fills the bar → one bar rest; else it is split into values that
/// start on their own beat, e.g. a gap from beat 2 to the end of 4/4 → quarter, half.
fn rests(a: &Arrangement) -> Vec<(u64, u64)> {
    let mut out = Vec::new();
    let mut covered = 0u64;
    let mut notes = a.notes.iter().peekable();
    for bar in &a.bars {
        let (start, end) = (bar.start_ticks, bar.end_ticks());

        // Gaps between the notes starting in this bar; a note may reach into the next bar.
        let mut gaps = Vec::new();
        while let Some(note) = notes.next_if(|n| n.start_ticks < end) {
            let from = covered.max(start);
            if note.start_ticks > from {
                gaps.push((from, note.start_ticks));
            }
            let written = note.start_ticks + written_ticks(note.duration_ticks);
            covered = covered.max(note.end_ticks()).max(written);
        }
        if covered < end {
            gaps.push((covered.max(start), end));
        }

        for (from, to) in gaps {
            let (from, to) = (snap(from, start).min(end), snap(to, start).min(end));
            if from == start && to == end {
                out.push((start, end - start));
                continue;
            }
            split_rest(start, from, to, &mut out);
        }
    }
    out
}

/// `tick` on the sixteenth grid of the bar starting at `bar`.
fn snap(tick: u64, bar: u64) -> u64 {
    let sixteenth = kontra_model::TICKS_PER_QUARTER / 4;
    let offset = tick.saturating_sub(bar);
    bar + (offset + sixteenth / 2) / sixteenth * sixteenth
}

/// Rest values from whole to sixteenth, in ticks.
const REST_TICKS: [u64; 5] = [
    kontra_model::TICKS_PER_QUARTER * 4,
    kontra_model::TICKS_PER_QUARTER * 2,
    kontra_model::TICKS_PER_QUARTER,
    kontra_model::TICKS_PER_QUARTER / 2,
    kontra_model::TICKS_PER_QUARTER / 4,
];

/// Splits the rest `from..to` into the largest values that fit and start on a multiple
/// of their own length within the bar.
fn split_rest(bar: u64, from: u64, to: u64, out: &mut Vec<(u64, u64)>) {
    let mut at = from;
    while at < to {
        let offset = at - bar;
        let value = REST_TICKS
            .into_iter()
            .find(|v| *v <= to - at && offset.is_multiple_of(*v))
            .unwrap_or(to - at);
        out.push((at, value));
        at += value;
    }
}

/// Part names the exercise generator writes in German (`exercise/song.rs`), with their keys.
const GENERATED_PARTS: [(&str, &str); 2] = [("Strophe", "part.verse"), ("Refrain", "part.chorus")];

/// Names of the parts that begin in bar `index`, e.g. `["Chorus", "B"]`; `None` without one.
fn bar_sections(a: &Arrangement, index: usize) -> Option<Vec<String>> {
    let start = a.bars.get(index)?.start_ticks;
    let end = a.bars.get(index + 1).map_or(u64::MAX, |b| b.start_ticks);
    let names: Vec<String> = a
        .markers
        .iter()
        .filter(|m| (start..end).contains(&m.tick) && !is_chord(m))
        .map(|m| m.name.clone())
        .collect();
    (!names.is_empty()).then_some(names)
}

/// A marker that names a chord ("Am7", "F#m7b5", "Bb/D") rather than a section ("Chorus").
/// A bare letter ("A", "Bb") counts as a rehearsal mark: Guitar Pro files use those.
/// A chord symbol: marked as one by the file, or a marker named like one (MIDI).
fn is_chord(marker: &kontra_model::Marker) -> bool {
    marker.kind == kontra_model::MarkerKind::Chord || is_chord_name(&marker.name)
}

pub(crate) fn is_chord_name(name: &str) -> bool {
    let mut chars = name.chars();
    if !matches!(chars.next(), Some('A'..='G')) {
        return false;
    }
    let rest: String = chars.collect();
    let rest = rest.strip_prefix(['#', 'b']).unwrap_or(&rest);
    if rest.is_empty() {
        return false;
    }
    const WORDS: [&str; 7] = ["maj", "min", "dim", "aug", "sus", "add", "m"];
    let mut s = rest;
    while !s.is_empty() {
        if let Some(w) = WORDS.iter().find(|w| s.starts_with(**w)) {
            s = &s[w.len()..];
        } else if s.starts_with(|c: char| {
            c.is_ascii_digit() || "#b/+-()°ø".contains(c) || ('A'..='G').contains(&c)
        }) {
            s = &s[s.chars().next().map_or(1, char::len_utf8)..];
        } else {
            return false;
        }
    }
    true
}
