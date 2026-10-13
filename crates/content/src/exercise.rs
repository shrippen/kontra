//! Generated exercises (LH-LRN-04, LH-LRN-01, LH-BIB-08): scales, arpeggios, walking bass and
//! root-note grooves in any key and tempo, written as standard MIDI files so they go through the
//! normal import (tab assignment, library, progress) like any other piece.
//!
//! Chord names travel as MIDI marker events at the bar they belong to; the importer turns them
//! into bar markers.

use midly::num::{u15, u24, u28, u4, u7};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};

/// Resolution of the written files.
const PPQ: u32 = 480;
const QUARTER: u32 = PPQ;
const EIGHTH: u32 = PPQ / 2;
const SIXTEENTH: u32 = PPQ / 4;
const BAR: u32 = 4 * PPQ;
/// Lowest note of a four-string bass (E1) and the highest a beginner reaches comfortably (G3).
const LOWEST: i32 = 28;
const HIGHEST: i32 = 55;
/// General MIDI "Electric Bass (finger)".
const BASS_PROGRAM: u8 = 33;
const VELOCITY: u8 = 96;
/// Ghost notes are muted, so they sound quieter than the line around them.
const GHOST_VELOCITY: u8 = 40;
/// General MIDI drum channel (10) and keys of the accompaniment (LH-LRN-01).
const DRUM_CHANNEL: u8 = 9;
const KICK: u8 = 36;
const SNARE: u8 = 38;
const HIHAT: u8 = 42;
const DRUM_VELOCITY: u8 = 90;

/// What to practise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExerciseKind {
    /// A scale up and down; `steps` are quarters (level 1) or eighths.
    Scale(ScaleKind),
    /// Chord tones per bar in quarters over a progression.
    Arpeggio(Progression),
    /// A walking line over a progression: root, chord tones, chromatic approach.
    WalkingBass(Progression),
    /// Roots of a progression in a fixed rhythm (groove before notes, LH-LRN-01).
    Groove(Progression, Rhythm),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaleKind {
    Major,
    Minor,
    MajorPentatonic,
    MinorPentatonic,
    Blues,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Progression {
    /// One chord (the key's tonic), four bars.
    Tonic,
    /// ii – V – I – I.
    TwoFiveOne,
    /// Twelve-bar blues with dominant sevenths.
    Blues,
    /// i – iv – i – v in minor.
    Minor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rhythm {
    Quarters,
    Eighths,
    /// Eighth plus two sixteenths per beat.
    Sixteenths,
    /// Beats one and three, the "and" of two: space as part of the groove.
    WithRests,
    /// Swung eighths (2:1).
    Shuffle,
    /// Root and octave alternating in eighths.
    Octaves,
    /// Sixteenths on the root, the "e" and "a" of each beat as ghost notes (funk).
    Ghost,
}

/// One exercise to generate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExerciseSpec {
    pub kind: ExerciseKind,
    /// Key root as pitch class, 0 = C .. 11 = B.
    pub key: u8,
    pub bpm: f64,
    /// Scale exercises: quarters instead of eighths.
    pub slow: bool,
    /// How often the pattern is played (each pass is 2–12 bars).
    pub repeats: u8,
}

/// A written exercise: a title and a standard MIDI file.
#[derive(Debug, Clone, PartialEq)]
pub struct Exercise {
    pub title: String,
    pub midi: Vec<u8>,
}

/// What an exercise is, as text keys, so the UI can name it in its language; the file keeps
/// its German title (progress is keyed by it). Example: `Groove` over `blues` in `shuffle`,
/// key `E`, 90 BPM → "Groove Shuffle 12-Takt-Blues in E (90 BPM)" / "Groove shuffle 12-bar
/// blues in E (90 BPM)".
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Description {
    /// "scale", "arpeggio", "walking" or "groove".
    pub kind: &'static str,
    /// Scale ("major", "minorpenta" …) or progression ("tonic", "251", "blues", "minor").
    pub variant: &'static str,
    /// Grooves only: "quarters", "eighths", "sixteenths", "rests", "shuffle", "octaves",
    /// "ghost".
    pub rhythm: Option<&'static str>,
    /// Key root spelled for its signature ("Bb", "F#") with "m" for minor.
    pub key: &'static str,
    pub minor: bool,
    pub bpm: f64,
}

/// One note of the generated line, in file ticks.
#[derive(Debug, Clone, Copy)]
struct Tone {
    start: u32,
    length: u32,
    key: i32,
    stroke: Stroke,
}

/// How a tone is struck.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stroke {
    Plain,
    /// Muted with the fretting hand: a percussive click without pitch (LH-BEW-06).
    Ghost,
}

/// Text event that marks the notes starting at its tick as ghost notes (see `midi.rs`).
pub(crate) const GHOST_TEXT: &str = "ghost";

const MAJOR: [i32; 8] = [0, 2, 4, 5, 7, 9, 11, 12];
const MINOR: [i32; 8] = [0, 2, 3, 5, 7, 8, 10, 12];
const MAJOR_PENTA: [i32; 6] = [0, 2, 4, 7, 9, 12];
const MINOR_PENTA: [i32; 6] = [0, 3, 5, 7, 10, 12];
const BLUES: [i32; 7] = [0, 3, 5, 6, 7, 10, 12];

/// Chord quality of a progression step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Quality {
    Major7,
    Minor7,
    Dominant7,
}

impl Quality {
    fn tones(self) -> [i32; 4] {
        match self {
            Quality::Major7 => [0, 4, 7, 11],
            Quality::Minor7 => [0, 3, 7, 10],
            Quality::Dominant7 => [0, 4, 7, 10],
        }
    }

    fn suffix(self) -> &'static str {
        match self {
            Quality::Major7 => "maj7",
            Quality::Minor7 => "m7",
            Quality::Dominant7 => "7",
        }
    }
}

impl Progression {
    /// (semitones above the key root, quality) per bar.
    fn bars(self) -> Vec<(i32, Quality)> {
        use Quality::*;
        match self {
            Progression::Tonic => vec![(0, Dominant7); 4],
            Progression::TwoFiveOne => vec![(2, Minor7), (7, Dominant7), (0, Major7), (0, Major7)],
            Progression::Blues => [0, 5, 0, 0, 5, 5, 0, 0, 7, 5, 0, 7]
                .iter()
                .map(|r| (*r, Dominant7))
                .collect(),
            Progression::Minor => vec![(0, Minor7), (5, Minor7), (0, Minor7), (7, Minor7)],
        }
    }

    fn minor(self) -> bool {
        matches!(self, Progression::Minor)
    }
}

impl ScaleKind {
    fn steps(self) -> &'static [i32] {
        match self {
            ScaleKind::Major => &MAJOR,
            ScaleKind::Minor => &MINOR,
            ScaleKind::MajorPentatonic => &MAJOR_PENTA,
            ScaleKind::MinorPentatonic => &MINOR_PENTA,
            ScaleKind::Blues => &BLUES,
        }
    }

    fn minor(self) -> bool {
        matches!(
            self,
            ScaleKind::Minor | ScaleKind::MinorPentatonic | ScaleKind::Blues
        )
    }
}

const SHARP_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];
const FLAT_NAMES: [&str; 12] = [
    "C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B",
];

/// Key signature in fifths for a major or minor key on `root`.
fn key_fifths(root: u8, minor: bool) -> i8 {
    // Fifths of each major key root: C G D A E B F# | F Bb Eb Ab Db.
    const MAJOR_FIFTHS: [i8; 12] = [0, -5, 2, -3, 4, -1, 6, 1, -4, 3, -2, 5];
    let major_root = if minor { (root + 3) % 12 } else { root % 12 };
    MAJOR_FIFTHS[major_root as usize]
}

/// Note name of pitch class `pc` spelled for a key with `fifths`.
fn name(pc: i32, fifths: i8) -> &'static str {
    let pc = pc.rem_euclid(12) as usize;
    if fifths < 0 {
        FLAT_NAMES[pc]
    } else {
        SHARP_NAMES[pc]
    }
}

/// The key root placed as low as possible on the bass (E1..D#2).
fn low_root(key: u8) -> i32 {
    let pc = i32::from(key % 12);
    LOWEST + (pc - LOWEST).rem_euclid(12)
}

/// `pc` in the octave nearest to `near`, kept inside the bass range.
fn nearest(pc: i32, near: i32) -> i32 {
    let mut key = near + (pc - near).rem_euclid(12);
    if key - near > 6 {
        key -= 12;
    }
    while key < LOWEST {
        key += 12;
    }
    while key > HIGHEST {
        key -= 12;
    }
    key
}

/// Generates `spec` as a titled MIDI file.
pub fn generate(spec: &ExerciseSpec) -> Exercise {
    let repeats = u32::from(spec.repeats.max(1));
    let (tones, markers, minor, label) = match spec.kind {
        ExerciseKind::Scale(scale) => {
            let (tones, length) = scale_line(scale, spec.key, spec.slow);
            (
                repeat(&tones, &[], length, repeats).0,
                Vec::new(),
                scale.minor(),
                scale_label(scale),
            )
        }
        ExerciseKind::Arpeggio(progression) => {
            let (tones, markers) = chords_line(progression, spec.key, arpeggio_bar);
            let (t, m) = repeat(&tones, &markers, bars_len(progression), repeats);
            (
                t,
                m,
                progression.minor(),
                format!("Arpeggios {}", progression_label(progression)),
            )
        }
        ExerciseKind::WalkingBass(progression) => {
            let (tones, markers) = walking_line(progression, spec.key);
            let (t, m) = repeat(&tones, &markers, bars_len(progression), repeats);
            (
                t,
                m,
                progression.minor(),
                format!("Walking Bass {}", progression_label(progression)),
            )
        }
        ExerciseKind::Groove(progression, rhythm) => {
            let (tones, markers) = chords_line(progression, spec.key, |start, root, _| {
                groove_bar(start, root, rhythm)
            });
            let (t, m) = repeat(&tones, &markers, bars_len(progression), repeats);
            (
                t,
                m,
                progression.minor(),
                format!(
                    "Groove {} {}",
                    rhythm_label(rhythm),
                    progression_label(progression)
                ),
            )
        }
    };
    let fifths = key_fifths(spec.key, minor);
    let key_name = format!(
        "{}{}",
        name(i32::from(spec.key), fifths),
        if minor { "m" } else { "" }
    );
    let title = format!("{label} in {key_name} ({:.0} BPM)", spec.bpm);
    let drums = match spec.kind {
        ExerciseKind::Groove(progression, rhythm) => {
            let bars = progression.bars().len() as u32 * repeats;
            (0..bars).flat_map(|b| drum_bar(b * BAR, rhythm)).collect()
        }
        _ => Vec::new(),
    };
    let line = Line {
        tones: &tones,
        markers: &markers,
        drums: &drums,
    };
    let midi = write(&title, spec.bpm, fifths, minor, &line);
    Exercise { title, midi }
}

/// What `write` puts into the file besides the header data.
struct Line<'a> {
    tones: &'a [Tone],
    markers: &'a [(u32, String)],
    /// (tick, General MIDI drum key).
    drums: &'a [(u32, u8)],
}

/// One bar of drums under a groove: kick on one and three, snare on two and four, hi-hat in
/// eighths (rock beat), swung for the shuffle.
fn drum_bar(start: u32, rhythm: Rhythm) -> Vec<(u32, u8)> {
    let offbeat = if rhythm == Rhythm::Shuffle {
        QUARTER * 2 / 3
    } else {
        EIGHTH
    };
    let mut hits = Vec::new();
    for beat in 0..4 {
        let at = start + beat * QUARTER;
        hits.push((at, if beat % 2 == 0 { KICK } else { SNARE }));
        hits.push((at, HIHAT));
        hits.push((at + offbeat, HIHAT));
    }
    // Rock beat: a second kick on the "and" of three drives the straight grooves.
    if rhythm != Rhythm::Shuffle {
        hits.push((start + 2 * QUARTER + EIGHTH, KICK));
    }
    hits
}

fn bars_len(progression: Progression) -> u32 {
    progression.bars().len() as u32 * BAR
}

fn scale_label(scale: ScaleKind) -> String {
    match scale {
        ScaleKind::Major => "Dur-Tonleiter",
        ScaleKind::Minor => "Moll-Tonleiter",
        ScaleKind::MajorPentatonic => "Dur-Pentatonik",
        ScaleKind::MinorPentatonic => "Moll-Pentatonik",
        ScaleKind::Blues => "Blues-Tonleiter",
    }
    .to_string()
}

fn progression_label(progression: Progression) -> &'static str {
    match progression {
        Progression::Tonic => "auf einem Akkord",
        Progression::TwoFiveOne => "II-V-I",
        Progression::Blues => "12-Takt-Blues",
        Progression::Minor => "Moll-Kadenz",
    }
}

fn rhythm_label(rhythm: Rhythm) -> &'static str {
    match rhythm {
        Rhythm::Quarters => "Viertel",
        Rhythm::Eighths => "Achtel",
        Rhythm::Sixteenths => "Sechzehntel",
        Rhythm::WithRests => "mit Pausen",
        Rhythm::Shuffle => "Shuffle",
        Rhythm::Octaves => "Oktaven",
        Rhythm::Ghost => "Ghost Notes",
    }
}

/// Text keys for `spec` (see `Description`).
pub fn describe(spec: &ExerciseSpec) -> Description {
    let scale_key = |scale| match scale {
        ScaleKind::Major => "major",
        ScaleKind::Minor => "minor",
        ScaleKind::MajorPentatonic => "majorpenta",
        ScaleKind::MinorPentatonic => "minorpenta",
        ScaleKind::Blues => "blues",
    };
    let progression_key = |progression| match progression {
        Progression::Tonic => "tonic",
        Progression::TwoFiveOne => "251",
        Progression::Blues => "blues",
        Progression::Minor => "minor",
    };
    let rhythm_key = |rhythm| match rhythm {
        Rhythm::Quarters => "quarters",
        Rhythm::Eighths => "eighths",
        Rhythm::Sixteenths => "sixteenths",
        Rhythm::WithRests => "rests",
        Rhythm::Shuffle => "shuffle",
        Rhythm::Octaves => "octaves",
        Rhythm::Ghost => "ghost",
    };
    let (kind, variant, rhythm, minor) = match spec.kind {
        ExerciseKind::Scale(s) => ("scale", scale_key(s), None, s.minor()),
        ExerciseKind::Arpeggio(p) => ("arpeggio", progression_key(p), None, p.minor()),
        ExerciseKind::WalkingBass(p) => ("walking", progression_key(p), None, p.minor()),
        ExerciseKind::Groove(p, r) => {
            ("groove", progression_key(p), Some(rhythm_key(r)), p.minor())
        }
    };
    Description {
        kind,
        variant,
        rhythm,
        key: name(i32::from(spec.key), key_fifths(spec.key, minor)),
        minor,
        bpm: spec.bpm,
    }
}

/// Repeats a line of `length` ticks `times`, shifting notes and markers.
fn repeat(
    tones: &[Tone],
    markers: &[(u32, String)],
    length: u32,
    times: u32,
) -> (Vec<Tone>, Vec<(u32, String)>) {
    let mut all = Vec::new();
    let mut all_markers = Vec::new();
    for pass in 0..times {
        let shift = pass * length;
        all.extend(tones.iter().map(|t| Tone {
            start: t.start + shift,
            ..*t
        }));
        all_markers.extend(markers.iter().map(|(at, text)| (at + shift, text.clone())));
    }
    (all, all_markers)
}

/// Up and down the scale (one octave, two from a low root when it fits), padded to whole bars.
fn scale_line(scale: ScaleKind, key: u8, slow: bool) -> (Vec<Tone>, u32) {
    let root = low_root(key);
    let step = if slow { QUARTER } else { EIGHTH };
    let mut degrees: Vec<i32> = scale.steps().to_vec();
    if !slow && root + 24 <= HIGHEST {
        let top: Vec<i32> = scale.steps()[1..].iter().map(|d| d + 12).collect();
        degrees.extend(top);
    }
    let down: Vec<i32> = degrees.iter().rev().skip(1).copied().collect();
    degrees.extend(down);
    let tones: Vec<Tone> = degrees
        .iter()
        .enumerate()
        .map(|(i, d)| Tone {
            start: i as u32 * step,
            length: step,
            key: root + d,
            stroke: Stroke::Plain,
        })
        .collect();
    let end = tones.len() as u32 * step;
    // The last root rings to the bar line.
    let length = end.div_ceil(BAR) * BAR;
    let mut tones = tones;
    if let Some(last) = tones.last_mut() {
        last.length = length - last.start;
    }
    (tones, length)
}

/// One bar per chord, filled by `bar(start, root, quality)`; markers carry the chord names.
fn chords_line(
    progression: Progression,
    key: u8,
    bar: impl Fn(u32, i32, Quality) -> Vec<Tone>,
) -> (Vec<Tone>, Vec<(u32, String)>) {
    let fifths = key_fifths(key, progression.minor());
    let base = low_root(key);
    let mut tones = Vec::new();
    let mut markers = Vec::new();
    for (i, (offset, quality)) in progression.bars().into_iter().enumerate() {
        let start = i as u32 * BAR;
        let root = nearest(i32::from(key) + offset, base);
        let chord = format!("{}{}", name(root, fifths), quality.suffix());
        markers.push((start, chord));
        tones.extend(bar(start, root, quality));
    }
    (tones, markers)
}

fn arpeggio_bar(start: u32, root: i32, quality: Quality) -> Vec<Tone> {
    quality
        .tones()
        .iter()
        .enumerate()
        .map(|(i, t)| Tone {
            start: start + i as u32 * QUARTER,
            length: QUARTER,
            key: fit(root + t),
            stroke: Stroke::Plain,
        })
        .collect()
}

/// Keeps a chord tone playable: an octave down when it is above the range.
fn fit(key: i32) -> i32 {
    if key > HIGHEST {
        key - 12
    } else {
        key
    }
}

fn groove_bar(start: u32, root: i32, rhythm: Rhythm) -> Vec<Tone> {
    let note = |at: u32, length: u32, key: i32| Tone {
        start: start + at,
        length,
        key,
        stroke: Stroke::Plain,
    };
    match rhythm {
        Rhythm::Quarters => (0..4).map(|b| note(b * QUARTER, QUARTER, root)).collect(),
        Rhythm::Eighths => (0..8).map(|e| note(e * EIGHTH, EIGHTH, root)).collect(),
        Rhythm::Sixteenths => (0..4)
            .flat_map(|b| {
                let beat = b * QUARTER;
                [
                    note(beat, EIGHTH, root),
                    note(beat + EIGHTH, SIXTEENTH, root),
                    note(beat + EIGHTH + SIXTEENTH, SIXTEENTH, root),
                ]
            })
            .collect(),
        Rhythm::WithRests => vec![
            note(0, QUARTER, root),
            note(QUARTER + EIGHTH, EIGHTH, root),
            note(2 * QUARTER, QUARTER, root),
        ],
        Rhythm::Shuffle => (0..4)
            .flat_map(|b| {
                let beat = b * QUARTER;
                let long = QUARTER * 2 / 3;
                [
                    note(beat, long, root),
                    note(beat + long, QUARTER - long, root),
                ]
            })
            .collect(),
        Rhythm::Octaves => (0..8)
            .map(|e| {
                let key = if e % 2 == 0 { root } else { fit(root + 12) };
                note(e * EIGHTH, EIGHTH, key)
            })
            .collect(),
        // 1 e & a: root on the beat and the "and", muted on the sixteenths between.
        Rhythm::Ghost => (0..16)
            .map(|s| {
                let tone = note(s * SIXTEENTH, SIXTEENTH, root);
                if s % 2 == 0 {
                    return tone;
                }
                Tone {
                    stroke: Stroke::Ghost,
                    ..tone
                }
            })
            .collect(),
    }
}

/// Root on one, chord tones on two and three, a chromatic approach to the next root on four.
fn walking_line(progression: Progression, key: u8) -> (Vec<Tone>, Vec<(u32, String)>) {
    let bars = progression.bars();
    let base = low_root(key);
    let roots: Vec<i32> = bars
        .iter()
        .map(|(offset, _)| nearest(i32::from(key) + offset, base))
        .collect();
    let fifths = key_fifths(key, progression.minor());
    let mut tones = Vec::new();
    let mut markers = Vec::new();
    for (i, (_, quality)) in bars.iter().enumerate() {
        let start = i as u32 * BAR;
        let root = roots[i];
        let next = roots[(i + 1) % roots.len()];
        markers.push((start, format!("{}{}", name(root, fifths), quality.suffix())));
        let chord = quality.tones();
        // Alternate the shape so the line does not repeat itself bar after bar.
        let (second, third) = if i % 2 == 0 {
            (chord[1], chord[2])
        } else {
            (chord[2], chord[3])
        };
        // Approach from below when the next root lies higher, else from above.
        let approach = if next >= root { next - 1 } else { next + 1 };
        let line = [root, fit(root + second), fit(root + third), approach];
        for (beat, key) in line.iter().enumerate() {
            tones.push(Tone {
                start: start + beat as u32 * QUARTER,
                length: QUARTER,
                key: (*key).clamp(LOWEST, HIGHEST),
                stroke: Stroke::Plain,
            });
        }
    }
    (tones, markers)
}

/// A type 1 file: a conductor track (title, tempo, metre, key, chord markers), a bass track and,
/// for grooves, a drum track on channel 10.
fn write(title: &str, bpm: f64, fifths: i8, minor: bool, line: &Line) -> Vec<u8> {
    let (tones, markers) = (line.tones, line.markers);
    let micros = (60_000_000.0 / bpm.clamp(20.0, 300.0)).round() as u32;
    let mut conductor: Vec<(u32, TrackEventKind)> = vec![
        (
            0,
            TrackEventKind::Meta(MetaMessage::TrackName(title.as_bytes())),
        ),
        (
            0,
            TrackEventKind::Meta(MetaMessage::Tempo(u24::new(micros))),
        ),
        (
            0,
            TrackEventKind::Meta(MetaMessage::TimeSignature(4, 2, 24, 8)),
        ),
        (
            0,
            TrackEventKind::Meta(MetaMessage::KeySignature(fifths, minor)),
        ),
    ];
    conductor.extend(markers.iter().map(|(at, text)| {
        (
            *at,
            TrackEventKind::Meta(MetaMessage::Marker(text.as_bytes())),
        )
    }));
    let end = tones.iter().map(|t| t.start + t.length).max().unwrap_or(0);
    let channel = u4::new(0);
    let mut bass: Vec<(u32, TrackEventKind)> = vec![
        (0, TrackEventKind::Meta(MetaMessage::TrackName(b"Bass"))),
        (
            0,
            TrackEventKind::Midi {
                channel,
                message: MidiMessage::ProgramChange {
                    program: u7::new(BASS_PROGRAM),
                },
            },
        ),
    ];
    for t in tones {
        let key = u7::new(t.key.clamp(0, 127) as u8);
        let velocity = match t.stroke {
            Stroke::Plain => VELOCITY,
            Stroke::Ghost => GHOST_VELOCITY,
        };
        // MIDI has no ghost note: a text event at the note's tick tells the importer.
        if t.stroke == Stroke::Ghost {
            bass.push((
                t.start,
                TrackEventKind::Meta(MetaMessage::Text(GHOST_TEXT.as_bytes())),
            ));
        }
        bass.push((
            t.start,
            TrackEventKind::Midi {
                channel,
                message: MidiMessage::NoteOn {
                    key,
                    vel: u7::new(velocity),
                },
            },
        ));
        // Release slightly early so repeated notes re-attack cleanly.
        let off = t.start + t.length.saturating_sub(t.length / 16).max(1);
        bass.push((
            off,
            TrackEventKind::Midi {
                channel,
                message: MidiMessage::NoteOff {
                    key,
                    vel: u7::new(0),
                },
            },
        ));
    }
    let mut tracks = vec![track(conductor, end), track(bass, end)];
    if !line.drums.is_empty() {
        tracks.push(track(drum_track(line.drums), end));
    }
    let smf = Smf {
        header: Header::new(Format::Parallel, Timing::Metrical(u15::new(PPQ as u16))),
        tracks,
    };
    let mut out = Vec::new();
    // Writing into memory only fails on invalid events, which `track` never builds.
    if smf.write_std(&mut out).is_err() {
        out.clear();
    }
    out
}

/// Drum strokes as short notes on channel 10.
fn drum_track(drums: &[(u32, u8)]) -> Vec<(u32, TrackEventKind<'static>)> {
    let channel = u4::new(DRUM_CHANNEL);
    let mut events = vec![(0, TrackEventKind::Meta(MetaMessage::TrackName(b"Drums")))];
    for &(at, key) in drums {
        let key = u7::new(key);
        let on = MidiMessage::NoteOn {
            key,
            vel: u7::new(DRUM_VELOCITY),
        };
        let off = MidiMessage::NoteOff {
            key,
            vel: u7::new(0),
        };
        events.push((
            at,
            TrackEventKind::Midi {
                channel,
                message: on,
            },
        ));
        events.push((
            at + SIXTEENTH / 2,
            TrackEventKind::Midi {
                channel,
                message: off,
            },
        ));
    }
    events
}

/// Sorts absolute events (note-offs before note-ons at the same tick) into delta time.
fn track(mut events: Vec<(u32, TrackEventKind)>, end: u32) -> Vec<TrackEvent> {
    let order = |kind: &TrackEventKind| match kind {
        TrackEventKind::Meta(_) => 0,
        TrackEventKind::Midi {
            message: MidiMessage::NoteOff { .. },
            ..
        } => 1,
        _ => 2,
    };
    events.sort_by_key(|(at, kind)| (*at, order(kind)));
    let mut out = Vec::with_capacity(events.len() + 1);
    let mut last = 0;
    for (at, kind) in events {
        out.push(TrackEvent {
            delta: u28::new(at - last),
            kind,
        });
        last = at;
    }
    out.push(TrackEvent {
        delta: u28::new(end.saturating_sub(last)),
        kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
    });
    out
}

/// The bundled set (LH-BIB-08): (level folder, file name, exercise), easiest first.
pub fn bundled() -> Vec<(&'static str, String, Exercise)> {
    bundled_specs()
        .into_iter()
        .enumerate()
        .map(|(i, (level, spec))| {
            let exercise = generate(&spec);
            let file = format!("{:02} {}.mid", i + 1, file_name(&exercise.title));
            (level, file, exercise)
        })
        .collect()
}

/// The plan of the bundled set: (level folder, spec); file "NN …" is entry NN − 1.
pub fn bundled_specs() -> Vec<(&'static str, ExerciseSpec)> {
    use ExerciseKind::*;
    use Progression as P;
    use Rhythm::*;
    use ScaleKind as S;
    // (level, kind, key, bpm, slow, repeats)
    let plan: Vec<(&str, ExerciseKind, u8, f64, bool, u8)> = vec![
        ("1 Einstieg", Groove(P::Tonic, Quarters), 4, 70.0, false, 2),
        ("1 Einstieg", Groove(P::Tonic, Quarters), 9, 70.0, false, 2),
        ("1 Einstieg", Groove(P::Tonic, Quarters), 2, 70.0, false, 2),
        ("1 Einstieg", Groove(P::Minor, Quarters), 9, 70.0, false, 2),
        ("1 Einstieg", Scale(S::MinorPentatonic), 4, 60.0, true, 2),
        ("1 Einstieg", Scale(S::MinorPentatonic), 9, 60.0, true, 2),
        ("1 Einstieg", Scale(S::Major), 7, 60.0, true, 2),
        ("1 Einstieg", Scale(S::Major), 0, 60.0, true, 2),
        ("2 Grundlagen", Groove(P::Tonic, Eighths), 4, 85.0, false, 2),
        (
            "2 Grundlagen",
            Groove(P::Blues, Quarters),
            4,
            85.0,
            false,
            1,
        ),
        (
            "2 Grundlagen",
            Groove(P::Blues, Quarters),
            9,
            85.0,
            false,
            1,
        ),
        (
            "2 Grundlagen",
            Groove(P::Minor, WithRests),
            4,
            80.0,
            false,
            2,
        ),
        ("2 Grundlagen", Scale(S::Major), 0, 80.0, false, 2),
        ("2 Grundlagen", Scale(S::Major), 5, 80.0, false, 2),
        ("2 Grundlagen", Scale(S::Major), 7, 80.0, false, 2),
        ("2 Grundlagen", Scale(S::MinorPentatonic), 4, 80.0, false, 2),
        ("2 Grundlagen", Scale(S::Minor), 9, 80.0, false, 2),
        (
            "3 Fortgeschritten",
            Groove(P::Blues, Shuffle),
            4,
            90.0,
            false,
            1,
        ),
        (
            "3 Fortgeschritten",
            Groove(P::Blues, Shuffle),
            9,
            90.0,
            false,
            1,
        ),
        (
            "3 Fortgeschritten",
            Groove(P::Minor, Sixteenths),
            4,
            85.0,
            false,
            2,
        ),
        (
            "3 Fortgeschritten",
            Groove(P::Tonic, Octaves),
            5,
            95.0,
            false,
            2,
        ),
        ("3 Fortgeschritten", Scale(S::Blues), 4, 90.0, false, 2),
        ("3 Fortgeschritten", Scale(S::Blues), 9, 90.0, false, 2),
        (
            "3 Fortgeschritten",
            Arpeggio(P::TwoFiveOne),
            0,
            90.0,
            false,
            2,
        ),
        (
            "3 Fortgeschritten",
            Arpeggio(P::TwoFiveOne),
            5,
            90.0,
            false,
            2,
        ),
        ("3 Fortgeschritten", Arpeggio(P::Blues), 9, 90.0, false, 1),
        (
            "4 Walking Bass",
            WalkingBass(P::TwoFiveOne),
            0,
            100.0,
            false,
            2,
        ),
        (
            "4 Walking Bass",
            WalkingBass(P::TwoFiveOne),
            5,
            100.0,
            false,
            2,
        ),
        (
            "4 Walking Bass",
            WalkingBass(P::TwoFiveOne),
            10,
            100.0,
            false,
            2,
        ),
        (
            "4 Walking Bass",
            WalkingBass(P::TwoFiveOne),
            3,
            110.0,
            false,
            2,
        ),
        ("4 Walking Bass", WalkingBass(P::Blues), 5, 110.0, false, 1),
        ("4 Walking Bass", WalkingBass(P::Blues), 10, 110.0, false, 1),
        ("4 Walking Bass", WalkingBass(P::Blues), 7, 120.0, false, 1),
        ("4 Walking Bass", WalkingBass(P::Minor), 2, 110.0, false, 2),
        // Groove level (LH-LRN-01): with the two levels above, twenty grooves on drums.
        ("5 Groove", Groove(P::Minor, Eighths), 2, 90.0, false, 2),
        ("5 Groove", Groove(P::Blues, Eighths), 7, 95.0, false, 1),
        ("5 Groove", Groove(P::Tonic, WithRests), 9, 90.0, false, 2),
        ("5 Groove", Groove(P::Blues, WithRests), 4, 95.0, false, 1),
        ("5 Groove", Groove(P::Blues, Shuffle), 7, 100.0, false, 1),
        ("5 Groove", Groove(P::Minor, Shuffle), 9, 100.0, false, 2),
        ("5 Groove", Groove(P::Tonic, Sixteenths), 4, 90.0, false, 2),
        ("5 Groove", Groove(P::Minor, Octaves), 2, 100.0, false, 2),
        (
            "5 Groove",
            Groove(P::TwoFiveOne, Sixteenths),
            5,
            95.0,
            false,
            2,
        ),
        ("5 Groove", Groove(P::Tonic, Ghost), 4, 80.0, false, 2),
        ("5 Groove", Groove(P::Minor, Ghost), 9, 85.0, false, 2),
    ];
    plan.into_iter()
        .map(|(level, kind, key, bpm, slow, repeats)| {
            let spec = ExerciseSpec {
                kind,
                key,
                bpm,
                slow,
                repeats,
            };
            (level, spec)
        })
        .collect()
}

/// A title as a portable file name (no slashes, sharps spelled out).
fn file_name(title: &str) -> String {
    title
        .replace('#', "is")
        .chars()
        .map(|c| if c == '/' || c == ':' { '-' } else { c })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{import_bytes, FileKind};

    fn spec(kind: ExerciseKind, key: u8) -> ExerciseSpec {
        ExerciseSpec {
            kind,
            key,
            bpm: 90.0,
            slow: false,
            repeats: 1,
        }
    }

    fn notes(exercise: &Exercise) -> Vec<u8> {
        let imported = import_bytes(&exercise.midi, FileKind::Midi).expect("imports");
        let track = imported.suggested().expect("bass track");
        let arrangement = imported.arrangement(track).expect("arrangement");
        arrangement.notes.iter().map(|n| n.pitch.0).collect()
    }

    #[test]
    fn g_major_scale_goes_up_and_down() {
        let e = generate(&ExerciseSpec {
            slow: true,
            ..spec(ExerciseKind::Scale(ScaleKind::Major), 7)
        });
        assert_eq!(
            notes(&e),
            [31, 33, 35, 36, 38, 40, 42, 43, 42, 40, 38, 36, 35, 33, 31]
        );
        assert!(e.title.contains("Dur-Tonleiter in G"), "{}", e.title);
    }

    #[test]
    fn every_note_is_playable_on_four_strings() {
        for (_, _, e) in bundled() {
            for key in notes(&e) {
                assert!((28..=55).contains(&key), "{}: {key}", e.title);
            }
        }
    }

    #[test]
    fn walking_bass_approaches_the_next_root() {
        let e = generate(&spec(ExerciseKind::WalkingBass(Progression::TwoFiveOne), 0));
        let line = notes(&e);
        assert_eq!(line.len(), 16);
        // Beat four of each bar is a semitone from the next bar's root.
        for bar in 0..3 {
            let approach = i32::from(line[bar * 4 + 3]);
            let next = i32::from(line[(bar + 1) * 4]);
            assert_eq!((approach - next).abs(), 1, "bar {bar}: {line:?}");
        }
    }

    #[test]
    fn chord_names_become_bar_markers() {
        let e = generate(&spec(ExerciseKind::Arpeggio(Progression::TwoFiveOne), 5));
        let imported = import_bytes(&e.midi, FileKind::Midi).unwrap();
        let a = imported.arrangement(imported.suggested().unwrap()).unwrap();
        let names: Vec<&str> = a.markers.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, ["Gm7", "C7", "Fmaj7", "Fmaj7"]);
        assert_eq!(a.bars[0].key_fifths, -1);
    }

    #[test]
    fn bundled_set_has_thirty_exercises_by_level() {
        let set = bundled();
        assert!(set.len() >= 30);
        let mut levels: Vec<&str> = set.iter().map(|s| s.0).collect();
        levels.dedup();
        assert_eq!(levels.len(), 5, "sorted by level: {levels:?}");
        // LH-LRN-01 asks for twenty groove exercises.
        let grooves = set
            .iter()
            .filter(|s| s.2.title.starts_with("Groove"))
            .count();
        assert!(grooves >= 20, "{grooves}");
    }

    #[test]
    fn shuffle_swings_the_eighths() {
        let e = generate(&spec(
            ExerciseKind::Groove(Progression::Tonic, Rhythm::Shuffle),
            4,
        ));
        let imported = import_bytes(&e.midi, FileKind::Midi).unwrap();
        let a = imported.arrangement(imported.suggested().unwrap()).unwrap();
        // 960 ticks per quarter in the model: the off-beat lands on the triplet (640).
        assert_eq!(a.notes[1].start_ticks, 640);
    }

    #[test]
    fn grooves_bring_a_drum_part() {
        use kontra_model::DrumKind;
        let groove = |rhythm| {
            let e = generate(&spec(ExerciseKind::Groove(Progression::Tonic, rhythm), 4));
            let imported = import_bytes(&e.midi, FileKind::Midi).unwrap();
            imported.arrangement(imported.suggested().unwrap()).unwrap()
        };
        let rock = groove(Rhythm::Eighths);
        // The drum track is not offered as the bass and does not add notes to play.
        assert_eq!(rock.notes.len(), 32);
        let kind_at = |a: &kontra_model::Arrangement, tick, kind| {
            a.drums.iter().any(|d| d.tick == tick && d.kind == kind)
        };
        assert!(kind_at(&rock, 0, DrumKind::Kick));
        assert!(kind_at(&rock, 960, DrumKind::Snare));
        assert!(kind_at(&rock, 480, DrumKind::HiHat));
        // Shuffle: the hi-hat swings onto the triplet.
        let shuffle = groove(Rhythm::Shuffle);
        assert!(kind_at(&shuffle, 640, DrumKind::HiHat));
        assert!(!kind_at(&shuffle, 480, DrumKind::HiHat));
        // Scales stay without drums.
        let scale = generate(&spec(ExerciseKind::Scale(ScaleKind::Major), 0));
        let imported = import_bytes(&scale.midi, FileKind::Midi).unwrap();
        let a = imported.arrangement(imported.suggested().unwrap()).unwrap();
        assert!(a.drums.is_empty());
    }

    #[test]
    fn ghost_groove_mutes_the_offbeat_sixteenths() {
        use kontra_model::Technique;
        let e = generate(&spec(
            ExerciseKind::Groove(Progression::Tonic, Rhythm::Ghost),
            4,
        ));
        let imported = import_bytes(&e.midi, FileKind::Midi).unwrap();
        let a = imported.arrangement(imported.suggested().unwrap()).unwrap();
        assert_eq!(a.notes.len(), 64);
        // 240 model ticks per sixteenth: "e" and "a" are odd sixteenths.
        for n in &a.notes {
            let offbeat = (n.start_ticks / 240) % 2 == 1;
            let ghost = n.techniques.contains(&Technique::Ghost);
            assert_eq!(ghost, offbeat, "tick {}", n.start_ticks);
            assert_eq!(n.pitch.0, 28, "root E1 at tick {}", n.start_ticks);
        }
        assert!(!a.drums.is_empty());
        // Other grooves stay without ghost notes.
        let plain = generate(&spec(
            ExerciseKind::Groove(Progression::Tonic, Rhythm::Sixteenths),
            4,
        ));
        let imported = import_bytes(&plain.midi, FileKind::Midi).unwrap();
        let a = imported.arrangement(imported.suggested().unwrap()).unwrap();
        assert!(a.notes.iter().all(|n| n.techniques.is_empty()));
    }
}
