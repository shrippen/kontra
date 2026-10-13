//! Guitar Pro 3, 4 and 5 (`.gp3`, `.gp4`, `.gp5`): little-endian binary files.
//!
//! ```text
//! version string │ song info │ (lyrics, RSE, page setup) │ tempo, key │ 64 MIDI channels
//! │ measure headers │ tracks │ measures: for each header, for each track: beats → notes
//! ```
//!
//! Layout knowledge comes from public format descriptions; no code was taken from other
//! importers. Data the trainer does not need (chord diagrams, bends, RSE) is skipped.

use crate::bytes::Reader;
use crate::error::ImportError;
use crate::score::{
    self, BarHead, Jump, Legato, Part, RawNote, Score, Target, Until, DEFAULT_BPM, WHOLE_TICKS,
};
use kontra_model::{Pitch, Technique, TimeSignature};

// Harmonic kinds of GP4 and GP5. Pinch and semi harmonics sound where the pick touches the
// string: not stored, so they keep the fretted pitch (judged in any octave).
const HARMONIC_NATURAL: u8 = 1;
const GP5_ARTIFICIAL: u8 = 2;
const GP5_TAPPED: u8 = 3;
/// GP4 artificial harmonics, touched 5, 7 or 12 frets above the fretted note ("AH5" …).
const GP4_ARTIFICIAL: [(u8, f64); 3] = [(15, 5.0), (17, 7.0), (22, 12.0)];

/// Slide types that go to the next note: GP4 stores a number, GP5 bit flags.
const GP4_SLIDE_SHIFT: u8 = 1;
const GP4_SLIDE_LEGATO: u8 = 2;
const GP5_SLIDE_SHIFT: u8 = 0x01;
const GP5_SLIDE_LEGATO: u8 = 0x02;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Version {
    V3,
    V4,
    V500,
    V510,
}

/// MIDI channel table: 4 ports of 16 channels.
const CHANNELS: usize = 64;
/// Channel 10 (index 9) carries drums in General MIDI.
const DRUM_CHANNEL: i32 = 10;
/// Guitar Pro stores up to 7 strings per track.
const MAX_STRINGS: usize = 7;
/// More tracks than this means the data is garbage.
const MAX_TRACKS: usize = 256;

// Beat flags.
const BEAT_DOTTED: u8 = 0x01;
const BEAT_CHORD: u8 = 0x02;
const BEAT_TEXT: u8 = 0x04;
const BEAT_EFFECTS: u8 = 0x08;
const BEAT_MIX: u8 = 0x10;
const BEAT_TUPLET: u8 = 0x20;
const BEAT_STATUS: u8 = 0x40;
/// Beat status "empty": the beat takes no time.
const STATUS_EMPTY: u8 = 0;

// Note types.
const NOTE_TIE: u8 = 2;
const NOTE_DEAD: u8 = 3;

pub(crate) fn parse(data: &[u8]) -> Result<Score, ImportError> {
    let mut reader = Reader::new(data);
    let version = version(&mut reader)?;
    let mut parser = Parser { r: reader, version };
    parser.song()
}

/// Reads the 30-byte version field, e.g. `FICHIER GUITAR PRO v5.10`.
fn version(r: &mut Reader) -> Result<Version, ImportError> {
    let text = r.byte_string(30)?;
    if !text.starts_with("FICHIER GUITAR PRO ") {
        return Err(ImportError::UnknownFormat);
    }
    let number = text.rsplit(' ').next().unwrap_or_default();
    let digits: String = number.chars().filter(char::is_ascii_digit).collect();
    match digits.as_str() {
        "300" => Ok(Version::V3),
        "400" | "406" => Ok(Version::V4),
        "500" => Ok(Version::V500),
        "510" => Ok(Version::V510),
        _ => Err(ImportError::Unsupported(text)),
    }
}

/// Effects that apply to every note of a beat.
#[derive(Debug, Default)]
struct BeatFx {
    techniques: Vec<Technique>,
}

/// Effects of one note.
#[derive(Debug, Default)]
struct NoteFx {
    techniques: Vec<Technique>,
    hammer: bool,
    /// A harmonic: it sounds at a partial of the string, not at the fret.
    harmonic: Option<Harmonic>,
}

/// Where a harmonic sounds, as far as the file tells.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Harmonic {
    /// Touched at the note's fret: a partial of the open string.
    Natural,
    /// Touched this many frets above the fretted note (artificial, tapped).
    Above(f64),
    /// Pitch class (C = 0) and octaves above the fretted note (GP5 artificial).
    Class { class: i32, octaves: i32 },
    /// Pinch, semi or a GP4 tapped harmonic: unknown, keeps the fretted pitch.
    Unknown,
}

#[derive(Debug)]
struct Parser<'a> {
    r: Reader<'a>,
    version: Version,
}

impl Parser<'_> {
    fn song(&mut self) -> Result<Score, ImportError> {
        let mut score = Score::new();
        self.info(&mut score)?;
        if self.version < Version::V500 {
            self.r.skip(1)?; // global triplet feel
        }
        if self.version >= Version::V4 {
            self.lyrics()?;
        }
        if self.version >= Version::V510 {
            self.r.skip(19)?; // RSE master effect
        }
        if self.version >= Version::V500 {
            self.page_setup()?;
            self.r.int_byte_string()?; // tempo name
        }
        let bpm = self.r.i32()?;
        score.bpm = if bpm > 0 { f64::from(bpm) } else { DEFAULT_BPM };
        if self.version >= Version::V510 {
            self.r.skip(1)?; // hide tempo
        }
        let song_key = score::key(i32::from(self.r.i8()?));
        self.r.skip(3)?; // key padding
        if self.version >= Version::V4 {
            self.r.skip(1)?; // octave
        }
        let programs = self.channels()?;
        let mut signs = Vec::new();
        if self.version >= Version::V500 {
            signs = self.directions()?;
            self.r.skip(4)?; // master reverb
        }

        let bar_count = self.r.count()?;
        let track_count = self.r.count()?;
        if track_count > MAX_TRACKS {
            return Err(ImportError::Malformed(format!("{track_count} tracks")));
        }
        for index in 0..bar_count {
            let head = self.bar_head(index, &score.bars)?;
            score.bars.push(head);
        }
        // The song key holds until a bar changes it.
        if let Some(first) = score.bars.first_mut() {
            first.key = first.key.or(song_key);
        }
        place_signs(&mut score.bars, &signs);
        for index in 0..track_count {
            let part = self.part(index, &programs)?;
            score.parts.push(part);
        }
        if self.version >= Version::V500 {
            let gap = if self.version == Version::V500 { 2 } else { 1 };
            self.r.skip(gap)?;
        }
        for bar in 0..bar_count {
            for part in 0..track_count {
                self.measure(&mut score, bar, part)?;
            }
        }
        Ok(score)
    }

    /// GP5 directions: 19 bar numbers (1-based, -1 = unused) in `SIGNS` order.
    fn directions(&mut self) -> Result<Vec<(Sign, i16)>, ImportError> {
        let mut found = Vec::with_capacity(SIGNS.len());
        for sign in SIGNS {
            found.push((sign, self.r.i16()?));
        }
        Ok(found)
    }

    fn info(&mut self, score: &mut Score) -> Result<(), ImportError> {
        score.title = self.r.int_byte_string()?;
        self.r.int_byte_string()?; // subtitle
        score.artist = self.r.int_byte_string()?;
        self.r.int_byte_string()?; // album
        self.r.int_byte_string()?; // words
        if self.version >= Version::V500 {
            self.r.int_byte_string()?; // music
        }
        self.r.int_byte_string()?; // copyright
        self.r.int_byte_string()?; // tab author
        self.r.int_byte_string()?; // instructions
        let notices = self.r.count()?;
        for _ in 0..notices {
            self.r.int_byte_string()?;
        }
        Ok(())
    }

    fn lyrics(&mut self) -> Result<(), ImportError> {
        self.r.skip(4)?; // lyrics track
        for _ in 0..5 {
            self.r.skip(4)?; // starting bar
            self.r.int_string()?;
        }
        Ok(())
    }

    fn page_setup(&mut self) -> Result<(), ImportError> {
        self.r.skip(30)?; // size, margins, proportion, header/footer flags
        for _ in 0..10 {
            self.r.int_byte_string()?;
        }
        Ok(())
    }

    /// MIDI program per channel slot.
    fn channels(&mut self) -> Result<Vec<i32>, ImportError> {
        let mut programs = Vec::with_capacity(CHANNELS);
        for _ in 0..CHANNELS {
            programs.push(self.r.i32()?);
            self.r.skip(6 + 2)?; // volume, balance, chorus, reverb, phaser, tremolo, padding
        }
        Ok(programs)
    }

    fn bar_head(&mut self, index: usize, previous: &[BarHead]) -> Result<BarHead, ImportError> {
        if self.version >= Version::V500 && index > 0 {
            self.r.skip(1)?;
        }
        let flags = self.r.u8()?;
        let last = previous
            .last()
            .map_or(TimeSignature::COMMON, |h| h.signature);
        let numerator = if flags & 0x01 != 0 {
            positive(self.r.i8()?)?
        } else {
            last.numerator
        };
        let denominator = if flags & 0x02 != 0 {
            positive(self.r.i8()?)?
        } else {
            last.denominator
        };
        let mut head = BarHead::new(TimeSignature {
            numerator,
            denominator,
        });
        head.repeat_open = flags & 0x04 != 0;
        if flags & 0x08 != 0 {
            let value = i32::from(self.r.i8()?);
            // GP5 stores the number of plays, GP3/4 the number of repeats.
            let plays = if self.version >= Version::V500 {
                value
            } else {
                value + 1
            };
            head.repeat_plays = u8::try_from(plays.max(0)).unwrap_or(0);
        }

        if self.version < Version::V500 {
            if flags & 0x10 != 0 {
                let number = self.r.u8()?;
                head.endings = old_endings(number, previous);
            }
            if flags & 0x20 != 0 {
                head.marker = Some(self.marker()?);
            }
            if flags & 0x40 != 0 {
                head.key = self.key()?;
            }
            return Ok(head);
        }

        if flags & 0x20 != 0 {
            head.marker = Some(self.marker()?);
        }
        if flags & 0x40 != 0 {
            head.key = self.key()?;
        }
        if flags & 0x10 != 0 {
            head.endings = u32::from(self.r.u8()?);
        }
        if flags & 0x03 != 0 {
            self.r.skip(4)?; // beam grouping
        }
        if flags & 0x10 == 0 {
            self.r.skip(1)?;
        }
        self.r.skip(1)?; // triplet feel
        Ok(head)
    }

    /// Bar key change: fifths, then major/minor (not needed for the signature).
    fn key(&mut self) -> Result<Option<i8>, ImportError> {
        let fifths = self.r.i8()?;
        self.r.skip(1)?;
        Ok(score::key(i32::from(fifths)))
    }

    fn marker(&mut self) -> Result<String, ImportError> {
        let name = self.r.int_byte_string()?;
        self.r.skip(4)?; // color
        Ok(name)
    }

    fn part(&mut self, index: usize, programs: &[i32]) -> Result<Part, ImportError> {
        if self.version >= Version::V500 && (index == 0 || self.version == Version::V500) {
            self.r.skip(1)?;
        }
        let flags = self.r.u8()?;
        let name = self.r.byte_string(40)?;
        let strings = self.r.count()?;
        if strings == 0 || strings > MAX_STRINGS {
            return Err(ImportError::Malformed(format!("{strings} strings")));
        }

        // Stored highest string first; the model wants lowest first.
        let mut tuning = Vec::with_capacity(strings);
        for slot in 0..MAX_STRINGS {
            let value = self.r.i32()?;
            if slot >= strings {
                continue;
            }
            let pitch = u8::try_from(value)
                .ok()
                .filter(|v| *v <= 127)
                .ok_or_else(|| ImportError::Malformed(format!("tuning value {value}")))?;
            tuning.push(Pitch(pitch));
        }
        tuning.reverse();

        let port = self.r.i32()?;
        let channel = self.r.i32()?;
        self.r.skip(4)?; // effect channel
        let frets = self.r.i32()?;
        self.r.skip(4 + 4)?; // capo, color
        if self.version >= Version::V500 {
            self.track_rse()?;
        }

        let mut part = Part::new(&name);
        part.tuning = tuning;
        part.frets = u8::try_from(frets)
            .ok()
            .filter(|f| *f > 0)
            .unwrap_or(part.frets);
        part.program = program(programs, port, channel);
        part.percussion = flags & 0x01 != 0 || channel == DRUM_CHANNEL;
        Ok(part)
    }

    /// GP5 track settings and RSE instrument; nothing the trainer needs.
    fn track_rse(&mut self) -> Result<(), ImportError> {
        // flags (2), accentuation, bank, humanize, 3 ints, 12 unknown, instrument, unknown, bank.
        self.r.skip(2 + 1 + 1 + 1 + 12 + 12 + 12)?;
        if self.version == Version::V500 {
            self.r.skip(3)?; // effect number (short) + padding
            return Ok(());
        }
        self.r.skip(4 + 4)?; // effect number, equalizer
        self.r.int_byte_string()?; // effect
        self.r.int_byte_string()?; // effect category
        Ok(())
    }

    fn measure(&mut self, score: &mut Score, bar: usize, part: usize) -> Result<(), ImportError> {
        let voices = if self.version >= Version::V500 { 2 } else { 1 };
        for _ in 0..voices {
            let beats = self.r.count()?;
            let mut offset = 0u64;
            for _ in 0..beats {
                let length = self.beat(score, bar, part, offset)?;
                offset = offset.saturating_add(length);
            }
        }
        // Line break; writers omit it after the very last measure.
        if self.version >= Version::V500 && !self.r.at_end() {
            self.r.skip(1)?;
        }
        Ok(())
    }

    /// Reads one beat and its notes; returns the time it takes.
    fn beat(
        &mut self,
        score: &mut Score,
        bar: usize,
        part: usize,
        offset: u64,
    ) -> Result<u64, ImportError> {
        let flags = self.r.u8()?;
        let mut empty = false;
        if flags & BEAT_STATUS != 0 {
            empty = self.r.u8()? == STATUS_EMPTY;
        }
        let value = self.r.i8()?;
        let tuplet = if flags & BEAT_TUPLET != 0 {
            self.r.i32()?
        } else {
            1
        };
        let duration = duration(value, flags & BEAT_DOTTED != 0, tuplet)?;

        if flags & BEAT_CHORD != 0 {
            self.chord()?;
        }
        if flags & BEAT_TEXT != 0 {
            self.r.int_byte_string()?;
        }
        let fx = if flags & BEAT_EFFECTS != 0 {
            self.beat_effects()?
        } else {
            BeatFx::default()
        };
        if flags & BEAT_MIX != 0 {
            if let Some(bpm) = self.mix_table()? {
                if let Some(head) = score.bars.get_mut(bar) {
                    head.add_tempo(offset, f64::from(bpm));
                }
            }
        }

        // String bits: bit 6 = string 1 (highest) ... bit 0 = string 7.
        let string_bits = self.r.u8()?;
        let tuning = score.parts[part].tuning.clone();
        for number in 1..=MAX_STRINGS {
            if string_bits & (1 << (7 - number)) == 0 || number > tuning.len() {
                continue;
            }
            let string = tuning.len() - number;
            let note = self.note(&tuning, string, offset, duration, &fx)?;
            if let Some(note) = note {
                score.parts[part].bar_mut(bar).push(note);
            }
        }

        if self.version >= Version::V500 {
            let flags2 = self.r.i16()?;
            if flags2 & 0x0800 != 0 {
                self.r.skip(1)?; // secondary beam break
            }
        }
        Ok(if empty { 0 } else { duration })
    }

    fn chord(&mut self) -> Result<(), ImportError> {
        if self.version >= Version::V500 {
            return self.r.skip(107);
        }
        let new_format = self.r.bool()?;
        if new_format {
            let size = if self.version == Version::V3 {
                124
            } else {
                106
            };
            return self.r.skip(size);
        }
        self.r.int_byte_string()?; // name
        let first_fret = self.r.i32()?;
        if first_fret > 0 {
            self.r.skip(6 * 4)?;
        }
        Ok(())
    }

    fn beat_effects(&mut self) -> Result<BeatFx, ImportError> {
        let mut fx = BeatFx::default();
        let flags1 = self.r.u8()?;
        let flags2 = if self.version >= Version::V4 {
            self.r.u8()?
        } else {
            0
        };
        if flags1 & 0x03 != 0 {
            fx.techniques.push(Technique::Vibrato);
        }
        if self.version == Version::V3 && flags1 & 0x0c != 0 {
            fx.techniques.push(Technique::Harmonic);
        }
        if flags1 & 0x20 != 0 {
            match self.r.i8()? {
                2 => fx.techniques.push(Technique::Slap),
                3 => fx.techniques.push(Technique::Pop),
                _ => {}
            }
            if self.version == Version::V3 {
                self.r.skip(4)?; // tremolo bar value
            }
        }
        if flags2 & 0x04 != 0 {
            self.bend()?; // tremolo bar
        }
        if flags1 & 0x40 != 0 {
            self.r.skip(2)?; // stroke
        }
        if flags2 & 0x02 != 0 {
            self.r.skip(1)?; // pick stroke
        }
        Ok(fx)
    }

    /// Mix table change; returns the new tempo if it sets one.
    fn mix_table(&mut self) -> Result<Option<i32>, ImportError> {
        self.r.skip(1)?; // instrument
        if self.version >= Version::V500 {
            self.r.skip(16)?; // RSE instrument
        }
        let mut values = [0i8; 6]; // volume, balance, chorus, reverb, phaser, tremolo
        for value in &mut values {
            *value = self.r.i8()?;
        }
        if self.version >= Version::V500 {
            self.r.int_byte_string()?; // tempo name
        }
        let tempo = self.r.i32()?;

        // Transition durations exist only for values that change.
        for value in values {
            if value >= 0 {
                self.r.skip(1)?;
            }
        }
        if tempo >= 0 {
            self.r.skip(1)?;
            if self.version >= Version::V510 {
                self.r.skip(1)?; // hide tempo
            }
        }
        if self.version >= Version::V4 {
            self.r.skip(1)?; // apply-to-all-tracks flags
        }
        if self.version >= Version::V500 {
            self.r.skip(1)?; // wah
        }
        if self.version >= Version::V510 {
            self.r.int_byte_string()?;
            self.r.int_byte_string()?;
        }
        Ok((tempo > 0).then_some(tempo))
    }

    fn note(
        &mut self,
        tuning: &[Pitch],
        string: usize,
        offset: u64,
        duration: u64,
        beat: &BeatFx,
    ) -> Result<Option<RawNote>, ImportError> {
        let flags = self.r.u8()?;
        let kind = if flags & 0x20 != 0 { self.r.u8()? } else { 1 };
        if flags & 0x01 != 0 && self.version < Version::V500 {
            self.r.skip(2)?; // own duration and tuplet
        }
        if flags & 0x10 != 0 {
            self.r.skip(1)?; // dynamics
        }
        let fret = if flags & 0x20 != 0 { self.r.i8()? } else { 0 };
        if flags & 0x80 != 0 {
            self.r.skip(2)?; // fingering
        }
        if self.version >= Version::V500 {
            if flags & 0x01 != 0 {
                self.r.skip(8)?; // duration percent
            }
            self.r.skip(1)?; // flags2
        }
        let fx = if flags & 0x08 != 0 {
            self.note_effects()?
        } else {
            NoteFx::default()
        };

        let Ok(fret) = u8::try_from(fret) else {
            return Ok(None);
        };
        let fretted = tuning[string].transposed(i32::from(fret));
        let sounding = fx
            .harmonic
            .and_then(|h| harmonic_pitch(h, tuning[string], fret));
        let Some(pitch) = sounding.or(fretted) else {
            return Ok(None);
        };
        let mut note = RawNote::new(offset, duration, pitch);
        note.string = u8::try_from(string).ok();
        note.fret = Some(fret);
        note.tied = kind == NOTE_TIE;
        if kind == NOTE_DEAD {
            note.add(Technique::Muted);
        }
        if flags & 0x04 != 0 {
            note.add(Technique::Ghost);
        }
        if flags & 0x42 != 0 {
            note.add(Technique::Accent);
        }
        for technique in fx.techniques.iter().chain(&beat.techniques) {
            note.add(*technique);
        }
        if fx.hammer {
            note.legato = Legato::Origin;
        }
        Ok(Some(note))
    }

    fn note_effects(&mut self) -> Result<NoteFx, ImportError> {
        let mut fx = NoteFx::default();
        let flags1 = self.r.u8()?;
        let flags2 = if self.version >= Version::V4 {
            self.r.u8()?
        } else {
            0
        };
        fx.hammer = flags1 & 0x02 != 0;
        if flags1 & 0x01 != 0 {
            self.bend()?;
            fx.techniques.push(Technique::Bend);
        }
        if flags1 & 0x10 != 0 {
            let grace = if self.version >= Version::V500 { 5 } else { 4 };
            self.r.skip(grace)?;
        }
        if self.version == Version::V3 {
            if flags1 & 0x04 != 0 {
                fx.techniques.push(Technique::Slide);
            }
            return Ok(fx);
        }

        if flags2 & 0x04 != 0 {
            self.r.skip(1)?; // tremolo picking
        }
        if flags2 & 0x08 != 0 {
            // Only a shift or legato slide goes to the next note; slides into the note or out
            // into nothing are not marked (LH-BEW-06 judges a slide where it arrives).
            let kind = self.r.u8()?;
            let to_next = if self.version == Version::V4 {
                matches!(kind, GP4_SLIDE_SHIFT | GP4_SLIDE_LEGATO)
            } else {
                kind & (GP5_SLIDE_SHIFT | GP5_SLIDE_LEGATO) != 0
            };
            if to_next {
                fx.techniques.push(Technique::Slide);
            }
        }
        if flags2 & 0x10 != 0 {
            fx.harmonic = Some(self.harmonic()?);
            fx.techniques.push(Technique::Harmonic);
        }
        if flags2 & 0x20 != 0 {
            self.r.skip(2)?; // trill
        }
        if flags2 & 0x01 != 0 {
            fx.techniques.push(Technique::Staccato);
        }
        if flags2 & 0x02 != 0 {
            fx.techniques.push(Technique::Muted); // palm mute
        }
        if flags2 & 0x40 != 0 {
            fx.techniques.push(Technique::Vibrato);
        }
        Ok(fx)
    }

    /// Reads a harmonic: GP4 stores its kind only, GP5 adds the pitch of artificial and the
    /// fret distance of tapped harmonics.
    fn harmonic(&mut self) -> Result<Harmonic, ImportError> {
        let kind = self.r.u8()?;
        if self.version < Version::V500 {
            return Ok(gp4_harmonic(kind));
        }
        match kind {
            HARMONIC_NATURAL => Ok(Harmonic::Natural),
            GP5_ARTIFICIAL => {
                // Natural pitch class (C = 0, D = 2 …), accidental (-1, 0, 1), octave (0 loco,
                // 1 8va, 2 15ma), e.g. E, 0, 1 over a fretted A1 → E3.
                let step = i32::from(self.r.i8()?);
                let accidental = i32::from(self.r.i8()?);
                let octaves = i32::from(self.r.i8()?);
                Ok(Harmonic::Class {
                    class: step + accidental,
                    octaves,
                })
            }
            GP5_TAPPED => Ok(Harmonic::Above(f64::from(self.r.u8()?))),
            _ => Ok(Harmonic::Unknown),
        }
    }

    /// Bend or tremolo bar: type, value, points of 9 bytes.
    fn bend(&mut self) -> Result<(), ImportError> {
        self.r.skip(1 + 4)?;
        let points = self.r.count()?;
        let size = points
            .checked_mul(9)
            .ok_or_else(|| ImportError::bad("bend point count"))?;
        self.r.skip(size)
    }
}

/// A GP4 harmonic kind: natural, artificial at a fixed distance, or unknown.
fn gp4_harmonic(kind: u8) -> Harmonic {
    if kind == HARMONIC_NATURAL {
        return Harmonic::Natural;
    }
    GP4_ARTIFICIAL
        .iter()
        .find(|(k, _)| *k == kind)
        .map_or(Harmonic::Unknown, |(_, frets)| Harmonic::Above(*frets))
}

/// The pitch a harmonic at `fret` of the string tuned to `open` sounds; `None` where unknown.
fn harmonic_pitch(harmonic: Harmonic, open: Pitch, fret: u8) -> Option<Pitch> {
    let fretted = open.transposed(i32::from(fret))?;
    match harmonic {
        Harmonic::Natural => crate::harmonic::natural_harmonic(open, f64::from(fret)),
        Harmonic::Above(frets) => crate::harmonic::natural_harmonic(fretted, frets),
        Harmonic::Class { class, octaves } => {
            crate::harmonic::harmonic_of_class(fretted, class, octaves)
        }
        Harmonic::Unknown => None,
    }
}

/// A GP5 direction: a sign jumped to, or a jump.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sign {
    Target(Target),
    Jump(Jump),
}

/// Order of the 19 direction slots in a GP5 file.
const SIGNS: [Sign; 19] = [
    Sign::Target(Target::Coda),
    Sign::Target(Target::DoubleCoda),
    Sign::Target(Target::Segno),
    Sign::Target(Target::SegnoSegno),
    Sign::Target(Target::Fine),
    Sign::Jump(Jump::DaCapo(Until::End)),
    Sign::Jump(Jump::DaCapo(Until::Coda)),
    Sign::Jump(Jump::DaCapo(Until::DoubleCoda)),
    Sign::Jump(Jump::DaCapo(Until::Fine)),
    Sign::Jump(Jump::DalSegno(Until::End)),
    Sign::Jump(Jump::DalSegno(Until::Coda)),
    Sign::Jump(Jump::DalSegno(Until::DoubleCoda)),
    Sign::Jump(Jump::DalSegno(Until::Fine)),
    Sign::Jump(Jump::DalSegnoSegno(Until::End)),
    Sign::Jump(Jump::DalSegnoSegno(Until::Coda)),
    Sign::Jump(Jump::DalSegnoSegno(Until::DoubleCoda)),
    Sign::Jump(Jump::DalSegnoSegno(Until::Fine)),
    Sign::Jump(Jump::ToCoda),
    Sign::Jump(Jump::ToDoubleCoda),
];

/// Attaches directions to their bars; numbers outside the song are ignored.
fn place_signs(bars: &mut [BarHead], signs: &[(Sign, i16)]) {
    for &(sign, number) in signs {
        let Some(bar) = usize::try_from(number)
            .ok()
            .and_then(|n| n.checked_sub(1))
            .and_then(|i| bars.get_mut(i))
        else {
            continue;
        };
        match sign {
            Sign::Target(target) => bar.targets.push(target),
            Sign::Jump(jump) => bar.jumps.push(jump),
        }
    }
}

fn positive(value: i8) -> Result<u8, ImportError> {
    u8::try_from(value)
        .ok()
        .filter(|v| *v > 0)
        .ok_or_else(|| ImportError::Malformed(format!("time signature value {value}")))
}

/// GP3/4 store the number of the ending; it covers all endings up to that number not yet
/// taken by earlier bars of the same repeat.
fn old_endings(number: u8, previous: &[BarHead]) -> u32 {
    let mut taken = 0u32;
    for (i, head) in previous.iter().enumerate().rev() {
        let last = i + 1 == previous.len();
        if head.repeat_plays > 0 && !last {
            break;
        }
        if head.repeat_open {
            break;
        }
        taken |= head.endings;
    }
    let all = 1u32
        .checked_shl(u32::from(number))
        .map_or(u32::MAX, |v| v - 1);
    all & !taken
}

/// General MIDI program of the track's channel, if valid.
fn program(programs: &[i32], port: i32, channel: i32) -> Option<u8> {
    let port = usize::try_from(port.checked_sub(1)?).ok()?;
    let channel = usize::try_from(channel.checked_sub(1)?).ok()?;
    let value = *programs.get(port.checked_mul(16)?.checked_add(channel)?)?;
    u8::try_from(value).ok().filter(|v| *v <= 127)
}

/// Beat length in ticks. `value`: -2 whole, -1 half, 0 quarter, 1 eighth, ... 6 = 256th.
fn duration(value: i8, dotted: bool, tuplet: i32) -> Result<u64, ImportError> {
    if !(-2..=6).contains(&value) {
        return Err(ImportError::Malformed(format!("duration value {value}")));
    }
    let shift = u32::try_from(i32::from(value) + 2).unwrap_or(0);
    let mut ticks = WHOLE_TICKS >> shift;
    if dotted {
        ticks = ticks * 3 / 2;
    }
    Ok(tuplet_ticks(ticks, tuplet))
}

/// Scales a duration for an n-tuplet: n notes in the time of the next lower power of two.
/// Example: triplet (3 in 2) of a quarter = 640 ticks.
pub(crate) fn tuplet_ticks(ticks: u64, enters: i32) -> u64 {
    let Ok(enters) = u64::try_from(enters) else {
        return ticks;
    };
    if enters < 2 {
        return ticks;
    }
    let times = if enters.is_power_of_two() {
        enters
    } else {
        1u64 << (63 - enters.leading_zeros())
    };
    ticks * times / enters
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations() {
        assert_eq!(duration(0, false, 1).unwrap(), 960);
        assert_eq!(duration(-2, false, 1).unwrap(), 3840);
        assert_eq!(duration(1, true, 1).unwrap(), 720);
        assert_eq!(duration(0, false, 3).unwrap(), 640);
        assert_eq!(duration(2, false, 5).unwrap(), 192);
        assert_eq!(duration(2, false, 6).unwrap(), 160);
        assert!(duration(9, false, 1).is_err());
    }

    #[test]
    fn gp4_artificial_harmonics() {
        assert_eq!(gp4_harmonic(1), Harmonic::Natural);
        assert_eq!(gp4_harmonic(15), Harmonic::Above(5.0));
        assert_eq!(gp4_harmonic(22), Harmonic::Above(12.0));
        assert_eq!(gp4_harmonic(4), Harmonic::Unknown, "pinch");
        // AH7 over a fretted A1: E3.
        let a = Pitch(33);
        let e3 = harmonic_pitch(gp4_harmonic(17), Pitch(28), 5);
        assert_eq!(e3, a.transposed(19));
    }

    #[test]
    fn old_style_endings() {
        let mut heads = vec![BarHead::new(TimeSignature::COMMON); 2];
        heads[0].repeat_open = true;
        heads[1].endings = old_endings(1, &heads[..1]);
        assert_eq!(heads[1].endings, 0b1);
        // Second ending numbered 2 covers ending 2 only (1 is taken).
        assert_eq!(old_endings(2, &heads), 0b10);
    }

    #[test]
    fn rejects_other_files() {
        let mut data = vec![24u8];
        data.extend_from_slice(b"FICHIER GUITAR PRO v2.21    ");
        data.extend_from_slice(&[0; 4]);
        assert!(matches!(parse(&data), Err(ImportError::Unsupported(_))));
        assert!(parse(b"hello").is_err());
    }
}
