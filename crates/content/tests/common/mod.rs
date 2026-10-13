//! Test-only writers that produce small but valid files of every supported format, so tests
//! need no copyrighted material.

#![allow(dead_code)]

use midly::num::{u15, u24, u28, u4, u7};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};

// ---------------------------------------------------------------------------------------------
// Guitar Pro 5.10

/// One note of a GP5 beat. `string` counts like Guitar Pro: 1 = highest.
#[derive(Clone, Copy)]
pub struct GpNote {
    pub string: u8,
    pub fret: i8,
    pub kind: u8, // 1 normal, 2 tie, 3 dead
    pub hammer: bool,
    /// Harmonic bytes as stored: kind, then its data (e.g. `[1]` natural); empty for none.
    pub harmonic: &'static [u8],
}

pub const NORMAL: u8 = 1;
pub const TIE: u8 = 2;
pub const DEAD: u8 = 3;

pub fn note(string: u8, fret: i8) -> GpNote {
    GpNote {
        string,
        fret,
        kind: NORMAL,
        hammer: false,
        harmonic: &[],
    }
}

/// One beat: duration code (-2 whole, -1 half, 0 quarter, 1 eighth), notes, optional tempo.
#[derive(Clone)]
pub struct GpBeat {
    pub duration: i8,
    pub rest: bool,
    pub tempo: Option<i32>,
    pub notes: Vec<GpNote>,
}

pub fn beat(duration: i8, notes: Vec<GpNote>) -> GpBeat {
    GpBeat {
        duration,
        rest: false,
        tempo: None,
        notes,
    }
}

pub fn rest(duration: i8) -> GpBeat {
    GpBeat {
        duration,
        rest: true,
        tempo: None,
        notes: Vec::new(),
    }
}

#[derive(Clone, Default)]
pub struct GpBar {
    pub repeat_open: bool,
    /// Total plays; 0 = no repeat close.
    pub repeat_plays: i8,
    pub marker: Option<String>,
    /// Key change at this bar (fifths).
    pub key: Option<i8>,
    pub beats: Vec<GpBeat>,
}

pub struct GpTrack {
    pub name: String,
    /// Highest string first, as Guitar Pro stores it.
    pub tuning: Vec<i32>,
    pub program: i32,
    /// Beats per bar, indexed like the bars.
    pub bars: Vec<Vec<GpBeat>>,
}

pub struct GpSong {
    pub title: String,
    pub tempo: i32,
    /// Song key (fifths), used until a bar changes it.
    pub key: i8,
    /// GP5 direction slots (coda, double coda, segno, ..., da double coda): bar number
    /// (1-based) or -1.
    pub directions: [i16; 19],
    pub heads: Vec<GpBar>,
    pub tracks: Vec<GpTrack>,
}

#[derive(Default)]
struct Out(Vec<u8>);

impl Out {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn i8(&mut self, v: i8) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn i16(&mut self, v: i16) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn i32(&mut self, v: i32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn zeros(&mut self, n: usize) {
        self.0.extend(std::iter::repeat_n(0, n));
    }
    fn byte_string(&mut self, text: &str, size: usize) {
        self.u8(text.len() as u8);
        let mut field = text.as_bytes().to_vec();
        field.resize(size, 0);
        self.0.extend_from_slice(&field);
    }
    fn int_byte_string(&mut self, text: &str) {
        self.i32(text.len() as i32 + 1);
        self.u8(text.len() as u8);
        self.0.extend_from_slice(text.as_bytes());
    }
}

/// Writes a Guitar Pro 5.10 file.
pub fn gp5(song: &GpSong) -> Vec<u8> {
    let mut o = Out::default();
    o.byte_string("FICHIER GUITAR PRO v5.10", 30);
    for field in [song.title.as_str(), "", "Kontra", "", "", "", "", "", ""] {
        o.int_byte_string(field);
    }
    o.i32(0); // notice lines
    o.i32(0); // lyrics track
    for _ in 0..5 {
        o.i32(0);
        o.i32(0);
    }
    o.zeros(19); // RSE master effect
    o.zeros(30); // page setup
    for _ in 0..10 {
        o.int_byte_string("");
    }
    o.int_byte_string("Moderate");
    o.i32(song.tempo);
    o.u8(0); // hide tempo
    o.i8(song.key);
    o.zeros(3); // key padding
    o.u8(0); // octave
    for channel in 0..64 {
        let program = song
            .tracks
            .iter()
            .enumerate()
            .find(|(i, _)| *i == channel)
            .map_or(0, |(_, t)| t.program);
        o.i32(program);
        o.zeros(8);
    }
    for number in song.directions {
        o.i16(number);
    }
    o.zeros(4); // master reverb
    o.i32(song.heads.len() as i32);
    o.i32(song.tracks.len() as i32);

    for (i, head) in song.heads.iter().enumerate() {
        if i > 0 {
            o.u8(0);
        }
        let mut flags = 0u8;
        if i == 0 {
            flags |= 0x03;
        }
        if head.repeat_open {
            flags |= 0x04;
        }
        if head.repeat_plays > 0 {
            flags |= 0x08;
        }
        if head.marker.is_some() {
            flags |= 0x20;
        }
        if head.key.is_some() {
            flags |= 0x40;
        }
        o.u8(flags);
        if i == 0 {
            o.i8(4);
            o.i8(4);
        }
        if head.repeat_plays > 0 {
            o.i8(head.repeat_plays);
        }
        if let Some(marker) = &head.marker {
            o.int_byte_string(marker);
            o.zeros(4);
        }
        if let Some(key) = head.key {
            o.i8(key);
            o.i8(0); // major
        }
        if flags & 0x03 != 0 {
            o.zeros(4); // beams
        }
        o.u8(0); // no alternate ending: placeholder
        o.u8(0); // triplet feel
    }

    for (i, track) in song.tracks.iter().enumerate() {
        if i == 0 {
            o.u8(0);
        }
        o.u8(0); // flags
        o.byte_string(&track.name, 40);
        o.i32(track.tuning.len() as i32);
        for slot in 0..7 {
            o.i32(*track.tuning.get(slot).unwrap_or(&0));
        }
        o.i32(1); // port
        o.i32(i as i32 + 1); // channel
        o.i32(i as i32 + 2); // effect channel
        o.i32(24); // frets
        o.i32(0); // capo
        o.zeros(4); // color
        o.zeros(2 + 1 + 1 + 1 + 12 + 12 + 12 + 4 + 4);
        o.int_byte_string("");
        o.int_byte_string("");
    }
    o.u8(0);

    for bar in 0..song.heads.len() {
        for track in &song.tracks {
            let beats = track.bars.get(bar).cloned().unwrap_or_default();
            o.i32(beats.len() as i32);
            for b in &beats {
                write_beat(&mut o, b, track.tuning.len());
            }
            o.i32(0); // second voice
            o.u8(0); // line break
        }
    }
    o.0
}

fn write_beat(o: &mut Out, beat: &GpBeat, strings: usize) {
    let mut flags = 0u8;
    if beat.rest {
        flags |= 0x40;
    }
    if beat.tempo.is_some() {
        flags |= 0x10;
    }
    o.u8(flags);
    if beat.rest {
        o.u8(2);
    }
    o.i8(beat.duration);
    if let Some(tempo) = beat.tempo {
        o.i8(-1); // instrument
        o.zeros(16); // RSE
        for _ in 0..6 {
            o.i8(-1);
        }
        o.int_byte_string("");
        o.i32(tempo);
        o.u8(0); // tempo transition
        o.u8(0); // hide tempo
        o.u8(0); // all tracks
        o.u8(0xff); // wah
        o.int_byte_string("");
        o.int_byte_string("");
    }
    let mut bits = 0u8;
    for n in &beat.notes {
        bits |= 1 << (7 - n.string);
    }
    o.u8(bits);
    // Notes in reader order: string 1 (highest) first.
    for number in 1..=strings as u8 {
        let Some(n) = beat.notes.iter().find(|n| n.string == number) else {
            continue;
        };
        let mut nflags = 0x20u8;
        let harmonic = !n.harmonic.is_empty();
        if n.hammer || harmonic {
            nflags |= 0x08;
        }
        o.u8(nflags);
        o.u8(n.kind);
        o.i8(n.fret);
        o.u8(0); // flags2
        if n.hammer || harmonic {
            o.u8(if n.hammer { 0x02 } else { 0 });
            o.u8(if harmonic { 0x10 } else { 0 });
            for byte in n.harmonic {
                o.u8(*byte);
            }
        }
    }
    o.i16(0);
}

/// Two bars of bass: `|: E G B~ | ~B rest D h E x :|`, tempo 120 → 90, marker "Verse".
pub fn bass_song() -> GpSong {
    let bar0 = vec![
        beat(0, vec![note(4, 0)]),
        beat(0, vec![note(4, 3)]),
        beat(-1, vec![note(3, 2)]),
    ];
    let mut tied = beat(
        0,
        vec![GpNote {
            kind: TIE,
            ..note(3, 2)
        }],
    );
    tied.tempo = Some(90);
    let bar1 = vec![
        tied,
        rest(0),
        beat(
            1,
            vec![GpNote {
                hammer: true,
                ..note(2, 0)
            }],
        ),
        beat(1, vec![note(2, 2)]),
        beat(
            0,
            vec![GpNote {
                kind: DEAD,
                ..note(4, 0)
            }],
        ),
    ];
    let guitar = vec![
        vec![beat(-2, vec![note(1, 0), note(2, 1)])],
        vec![beat(-2, vec![note(1, 3)])],
    ];
    GpSong {
        title: "Fixture".to_string(),
        tempo: 120,
        key: 0,
        directions: [-1; 19],
        heads: vec![
            GpBar {
                repeat_open: true,
                ..GpBar::default()
            },
            GpBar {
                repeat_plays: 2,
                marker: Some("Verse".to_string()),
                ..GpBar::default()
            },
        ],
        tracks: vec![
            GpTrack {
                name: "Guitar".to_string(),
                tuning: vec![64, 59, 55, 50, 45, 40],
                program: 29,
                bars: guitar,
            },
            GpTrack {
                name: "Electric Bass".to_string(),
                tuning: vec![43, 38, 33, 28],
                program: 33,
                bars: vec![bar0, bar1],
            },
        ],
    }
}

/// GP5 direction slots used by `directions_song`.
pub const SLOT_CODA: usize = 0;
pub const SLOT_SEGNO: usize = 2;
pub const SLOT_DS_AL_CODA: usize = 10;
pub const SLOT_TO_CODA: usize = 17;

/// Five bars of one whole note each (E string, fret = bar index) with a repeat and
/// directions: `0 | 1 :| 2 Segno | 3 to Coda | 4 D.S. al Coda | 5 Coda`.
pub fn directions_song() -> GpSong {
    let bars: Vec<Vec<GpBeat>> = (0..6i8).map(|i| vec![beat(-2, vec![note(4, i)])]).collect();
    let mut heads = vec![GpBar::default(); 6];
    heads[1].repeat_plays = 2;
    let mut directions = [-1; 19];
    directions[SLOT_SEGNO] = 3;
    directions[SLOT_TO_CODA] = 4;
    directions[SLOT_DS_AL_CODA] = 5;
    directions[SLOT_CODA] = 6;
    GpSong {
        title: "Directions".to_string(),
        tempo: 100,
        key: 0,
        directions,
        heads,
        tracks: vec![GpTrack {
            name: "Bass".to_string(),
            tuning: vec![43, 38, 33, 28],
            program: 33,
            bars,
        }],
    }
}

// ---------------------------------------------------------------------------------------------
// GPIF (Guitar Pro 6/7)

/// One bass track, one whole note per bar (E string, fret = bar index); `extras[i]` is
/// inserted into master bar `i` (repeats, directions).
pub fn gpif_bars(extras: &[&str]) -> String {
    let mut masters = String::new();
    let mut bars = String::new();
    let mut voices = String::new();
    let mut beats = String::new();
    let mut notes = String::new();
    for (i, extra) in extras.iter().enumerate() {
        masters.push_str(&format!(
            "<MasterBar><Time>4/4</Time>{extra}<Bars>{i}</Bars></MasterBar>"
        ));
        bars.push_str(&format!(
            r#"<Bar id="{i}"><Voices>{i} -1 -1 -1</Voices></Bar>"#
        ));
        voices.push_str(&format!(r#"<Voice id="{i}"><Beats>{i}</Beats></Voice>"#));
        beats.push_str(&format!(
            r#"<Beat id="{i}"><Rhythm ref="0"/><Notes>{i}</Notes></Beat>"#
        ));
        notes.push_str(&format!(
            r#"<Note id="{i}"><Properties><Property name="String"><String>0</String></Property><Property name="Fret"><Fret>{i}</Fret></Property></Properties></Note>"#
        ));
    }
    format!(
        r#"<GPIF><Score><Title>Flow</Title></Score>
<Tracks><Track id="0"><Name>Bass</Name><Staves><Staff><Properties><Property name="Tuning"><Pitches>28 33 38 43</Pitches></Property></Properties></Staff></Staves></Track></Tracks>
<MasterBars>{masters}</MasterBars><Bars>{bars}</Bars><Voices>{voices}</Voices><Beats>{beats}</Beats><Notes>{notes}</Notes>
<Rhythms><Rhythm id="0"><NoteValue>Whole</NoteValue></Rhythm></Rhythms></GPIF>"#
    )
}

/// A bass track with a repeat and two endings, tempo automations, a triplet, a dotted note,
/// a tie and a hammer-on.
///
/// ```text
/// bar 0 |: q A1(E5)  q. B1(A2)  e C2(A3) h  |   tempo 100
/// bar 1 | 1. h~ C2  triplet q D2 E2 F2(D)   :|  tempo 60 per half (= 120) at the middle
/// bar 2 | 2. w x E1                          |
/// ```
pub fn gpif() -> String {
    r#"<?xml version="1.0" encoding="utf-8"?>
<GPIF>
<Score><Title><![CDATA[Gpif Fixture]]></Title><Artist><![CDATA[Kontra]]></Artist></Score>
<MasterTrack><Tracks>0 1</Tracks><Automations>
<Automation><Type>Tempo</Type><Linear>false</Linear><Bar>0</Bar><Position>0</Position><Value>100 2</Value></Automation>
<Automation><Type>Tempo</Type><Linear>false</Linear><Bar>1</Bar><Position>0.5</Position><Value>60 4</Value></Automation>
</Automations></MasterTrack>
<Tracks>
<Track id="0"><Name><![CDATA[Keys]]></Name><Staves><Staff/><Staff/></Staves><Sounds><Sound><MIDI><Program>0</Program></MIDI></Sound></Sounds></Track>
<Track id="1"><Name><![CDATA[Tieftöner]]></Name><InstrumentSet><Type>electricBass</Type></InstrumentSet>
<Staves><Staff><Properties><Property name="Tuning"><Pitches>28 33 38 43</Pitches></Property></Properties></Staff></Staves>
<Sounds><Sound><MIDI><Program>33</Program></MIDI></Sound></Sounds></Track>
</Tracks>
<MasterBars>
<MasterBar><Time>4/4</Time><Repeat start="true" end="false" count="0"/><Section><Letter>A</Letter><Text>Intro</Text></Section><Bars>0 1 2</Bars></MasterBar>
<MasterBar><Time>4/4</Time><Repeat start="false" end="true" count="2"/><AlternateEndings>1</AlternateEndings><Bars>0 1 3</Bars></MasterBar>
<MasterBar><Time>4/4</Time><AlternateEndings>2</AlternateEndings><Bars>0 1 4</Bars></MasterBar>
</MasterBars>
<Bars>
<Bar id="0"><Voices>-1 -1 -1 -1</Voices></Bar>
<Bar id="1"><Voices>-1 -1 -1 -1</Voices></Bar>
<Bar id="2"><Voices>0 -1 -1 -1</Voices></Bar>
<Bar id="3"><Voices>1 -1 -1 -1</Voices></Bar>
<Bar id="4"><Voices>2 -1 -1 -1</Voices></Bar>
</Bars>
<Voices>
<Voice id="0"><Beats>0 1 2 3</Beats></Voice>
<Voice id="1"><Beats>4 5 6 7</Beats></Voice>
<Voice id="2"><Beats>8</Beats></Voice>
</Voices>
<Beats>
<Beat id="0"><Rhythm ref="0"/><Notes>0</Notes></Beat>
<Beat id="1"><Rhythm ref="1"/><Notes>1</Notes></Beat>
<Beat id="2"><Rhythm ref="2"/><Notes>2</Notes></Beat>
<Beat id="3"><GraceNotes>BeforeBeat</GraceNotes><Rhythm ref="2"/><Notes>9</Notes></Beat>
<Beat id="4"><Rhythm ref="3"/><Notes>3</Notes></Beat>
<Beat id="5"><Rhythm ref="4"/><Notes>4</Notes></Beat>
<Beat id="6"><Rhythm ref="4"/><Notes>5</Notes></Beat>
<Beat id="7"><Rhythm ref="4"/><Notes>6</Notes><Properties><Property name="Slapped"><Enable/></Property></Properties></Beat>
<Beat id="8"><Rhythm ref="5"/><Notes>7</Notes></Beat>
</Beats>
<Notes>
<Note id="0"><Properties><Property name="String"><String>0</String></Property><Property name="Fret"><Fret>5</Fret></Property></Properties></Note>
<Note id="1"><Properties><Property name="String"><String>1</String></Property><Property name="Fret"><Fret>2</Fret></Property></Properties><Accent>8</Accent></Note>
<Note id="2"><Tie origin="true" destination="false"/><Properties><Property name="String"><String>1</String></Property><Property name="Fret"><Fret>3</Fret></Property></Properties></Note>
<Note id="3"><Tie origin="false" destination="true"/><Properties><Property name="String"><String>1</String></Property><Property name="Fret"><Fret>3</Fret></Property></Properties></Note>
<Note id="4"><Properties><Property name="String"><String>1</String></Property><Property name="Fret"><Fret>5</Fret></Property></Properties></Note>
<Note id="5"><Properties><Property name="String"><String>1</String></Property><Property name="Fret"><Fret>7</Fret></Property><Property name="HopoDestination"><Enable/></Property></Properties></Note>
<Note id="6"><Properties><Property name="String"><String>2</String></Property><Property name="Fret"><Fret>3</Fret></Property></Properties><AntiAccent>Normal</AntiAccent></Note>
<Note id="7"><Properties><Property name="Midi"><Number>28</Number></Property><Property name="Muted"><Enable/></Property></Properties></Note>
<Note id="9"><Properties><Property name="String"><String>3</String></Property><Property name="Fret"><Fret>9</Fret></Property></Properties></Note>
</Notes>
<Rhythms>
<Rhythm id="0"><NoteValue>Quarter</NoteValue></Rhythm>
<Rhythm id="1"><NoteValue>Quarter</NoteValue><AugmentationDot count="1"/></Rhythm>
<Rhythm id="2"><NoteValue>Eighth</NoteValue></Rhythm>
<Rhythm id="3"><NoteValue>Half</NoteValue></Rhythm>
<Rhythm id="4"><NoteValue>Quarter</NoteValue><PrimaryTuplet num="3" den="2"/></Rhythm>
<Rhythm id="5"><NoteValue>Whole</NoteValue></Rhythm>
</Rhythms>
</GPIF>
"#
    .to_string()
}

// ---------------------------------------------------------------------------------------------
// Zip

/// How an entry is stored.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Stored,
    Deflated,
}

pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

/// Minimal zip writer (no data descriptors).
pub fn zip(entries: &[(&str, &[u8])], method: Method) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data) in entries {
        let packed = match method {
            Method::Stored => data.to_vec(),
            Method::Deflated => miniz_oxide::deflate::compress_to_vec(data, 6),
        };
        let code: u16 = if method == Method::Stored { 0 } else { 8 };
        let crc = crc32(data);
        let local = out.len() as u32;

        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&code.to_le_bytes());
        out.extend_from_slice(&[0; 4]); // time, date
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&(packed.len() as u32).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(&packed);

        central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&code.to_le_bytes());
        central.extend_from_slice(&[0; 4]);
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&(packed.len() as u32).to_le_bytes());
        central.extend_from_slice(&(data.len() as u32).to_le_bytes());
        central.extend_from_slice(&(name.len() as u16).to_le_bytes());
        central.extend_from_slice(&[0; 2 + 2 + 2 + 2 + 4]); // extra, comment, disk, attrs
        central.extend_from_slice(&local.to_le_bytes());
        central.extend_from_slice(name.as_bytes());
    }
    let start = out.len() as u32;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(central.len() as u32).to_le_bytes());
    out.extend_from_slice(&start.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

// ---------------------------------------------------------------------------------------------
// GPX (Guitar Pro 6)

const SECTOR: usize = 0x1000;

/// Uncompressed BCFS file system holding `files`.
pub fn bcfs(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut system = vec![0xffu8; SECTOR]; // sector 0 unused
    for (name, data) in files {
        let entry_sector = system.len() / SECTOR;
        let data_sectors = data.len().div_ceil(SECTOR).max(1);
        let mut entry = vec![0u8; SECTOR];
        entry[..4].copy_from_slice(&2u32.to_le_bytes());
        entry[4..4 + name.len()].copy_from_slice(name.as_bytes());
        entry[0x8c..0x90].copy_from_slice(&(data.len() as u32).to_le_bytes());
        for i in 0..data_sectors {
            let sector = (entry_sector + 1 + i) as u32;
            let at = 0x94 + 4 * i;
            entry[at..at + 4].copy_from_slice(&sector.to_le_bytes());
        }
        system.extend_from_slice(&entry);
        let mut content = data.to_vec();
        content.resize(data_sectors * SECTOR, 0);
        system.extend_from_slice(&content);
    }
    let mut out = b"BCFS".to_vec();
    out.extend_from_slice(&system);
    out
}

/// Bit writer, most significant bit first.
#[derive(Default)]
struct BitWriter {
    bytes: Vec<u8>,
    used: usize,
}

impl BitWriter {
    fn bits(&mut self, value: u32, count: u32, reversed: bool) {
        for i in 0..count {
            let shift = if reversed { i } else { count - 1 - i };
            if self.used.is_multiple_of(8) {
                self.bytes.push(0);
            }
            if (value >> shift) & 1 == 1 {
                let last = self.bytes.len() - 1;
                self.bytes[last] |= 1 << (7 - self.used % 8);
            }
            self.used += 1;
        }
    }
}

/// BCFZ-compresses `data`: back references for repeats 64 bytes back, literals otherwise.
pub fn bcfz(data: &[u8]) -> Vec<u8> {
    const BACK: usize = 64;
    const WORD: u32 = 7; // 7 bits hold 64
    let mut w = BitWriter::default();
    let mut i = 0;
    while i < data.len() {
        let repeat = i >= BACK && i + BACK <= data.len() && data[i..i + BACK] == data[i - BACK..i];
        if repeat {
            w.bits(1, 1, false);
            w.bits(WORD, 4, false);
            w.bits(BACK as u32, WORD, true);
            w.bits(BACK as u32, WORD, true);
            i += BACK;
            continue;
        }
        let count = (data.len() - i).min(3);
        w.bits(0, 1, false);
        w.bits(count as u32, 2, true);
        for byte in &data[i..i + count] {
            w.bits(u32::from(*byte), 8, false);
        }
        i += count;
    }
    let mut out = b"BCFZ".to_vec();
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&w.bytes);
    out
}

// ---------------------------------------------------------------------------------------------
// MIDI

fn event(delta: u32, kind: TrackEventKind<'static>) -> TrackEvent<'static> {
    TrackEvent {
        delta: u28::new(delta),
        kind,
    }
}

fn on(channel: u8, key: u8, vel: u8) -> TrackEventKind<'static> {
    TrackEventKind::Midi {
        channel: u4::new(channel),
        message: MidiMessage::NoteOn {
            key: u7::new(key),
            vel: u7::new(vel),
        },
    }
}

fn off(channel: u8, key: u8) -> TrackEventKind<'static> {
    TrackEventKind::Midi {
        channel: u4::new(channel),
        message: MidiMessage::NoteOff {
            key: u7::new(key),
            vel: u7::new(0),
        },
    }
}

/// Type 1 file at 480 PPQ: conductor track (3/4, 100 bpm, 150 bpm from bar 2), a piano
/// track playing chords on channel 1, a bass track (program 33) on channel 2.
pub fn midi() -> Vec<u8> {
    let header = Header::new(Format::Parallel, Timing::Metrical(u15::new(480)));
    let mut smf = Smf::new(header);
    let end = || event(0, TrackEventKind::Meta(MetaMessage::EndOfTrack));

    smf.tracks.push(vec![
        event(
            0,
            TrackEventKind::Meta(MetaMessage::TimeSignature(3, 2, 24, 8)),
        ),
        event(
            0,
            TrackEventKind::Meta(MetaMessage::Tempo(u24::new(600_000))),
        ),
        event(
            1440,
            TrackEventKind::Meta(MetaMessage::Tempo(u24::new(400_000))),
        ),
        end(),
    ]);

    // Piano: C major triad twice.
    let mut piano = vec![event(
        0,
        TrackEventKind::Meta(MetaMessage::TrackName(b"Piano")),
    )];
    for _ in 0..2 {
        piano.push(event(0, on(0, 60, 90)));
        piano.push(event(0, on(0, 64, 90)));
        piano.push(event(0, on(0, 67, 90)));
        piano.push(event(1440, off(0, 60)));
        piano.push(event(0, off(0, 64)));
        piano.push(event(0, off(0, 67)));
    }
    piano.push(end());
    smf.tracks.push(piano);

    // Bass: E1 A1 B1 quarter notes, then E2 (note-on velocity 0 as note-off), then the
    // chord E1+B1.
    let mut bass = vec![
        event(0, TrackEventKind::Meta(MetaMessage::TrackName(b"Track 3"))),
        event(
            0,
            TrackEventKind::Midi {
                channel: u4::new(1),
                message: MidiMessage::ProgramChange {
                    program: u7::new(33),
                },
            },
        ),
    ];
    for key in [28, 33, 35] {
        bass.push(event(0, on(1, key, 100)));
        bass.push(event(480, off(1, key)));
    }
    bass.push(event(0, on(1, 40, 100)));
    bass.push(event(480, on(1, 40, 0)));
    bass.push(event(0, on(1, 35, 100)));
    bass.push(event(0, on(1, 28, 100)));
    bass.push(event(960, off(1, 28)));
    bass.push(event(0, off(1, 35)));
    bass.push(end());
    smf.tracks.push(bass);

    let mut out = Vec::new();
    smf.write_std(&mut out).unwrap();
    out
}

// ---------------------------------------------------------------------------------------------
// MusicXML

/// Bass part written an octave up (`octave-change -1`), with tab for the first bar.
///
/// ```text
/// bar 1 |: q E1(s4 f0)  q A1(s3 f0) chord with E2  backup/forward  h G1~ |  tempo 90
/// bar 2 | 1. h ~G1  q rest  q C2                                        :|
/// bar 3 | 2. w E1                                                        |
/// ```
pub fn musicxml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE score-partwise PUBLIC "-//Recordare//DTD MusicXML 4.0 Partwise//EN" "http://www.musicxml.org/dtds/partwise.dtd">
<score-partwise version="4.0">
  <work><work-title>Walking &amp; Talking</work-title></work>
  <identification><creator type="composer">Kontra</creator></identification>
  <part-list>
    <score-part id="P1"><part-name>Flute</part-name><midi-instrument id="P1-I1"><midi-program>74</midi-program></midi-instrument></score-part>
    <score-part id="P2"><part-name>Electric Bass</part-name><midi-instrument id="P2-I1"><midi-channel>2</midi-channel><midi-program>34</midi-program></midi-instrument></score-part>
  </part-list>
  <part id="P1">
    <measure number="1">
      <attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time></attributes>
      <direction><sound tempo="90"/></direction>
      <barline location="left"><repeat direction="forward"/></barline>
      <note><pitch><step>C</step><octave>5</octave></pitch><duration>4</duration></note>
    </measure>
    <measure number="2">
      <barline location="left"><ending number="1" type="start"/></barline>
      <note><pitch><step>D</step><octave>5</octave></pitch><duration>4</duration></note>
      <barline location="right"><ending number="1" type="stop"/><repeat direction="backward"/></barline>
    </measure>
    <measure number="3">
      <barline location="left"><ending number="2" type="start"/></barline>
      <note><pitch><step>E</step><octave>5</octave></pitch><duration>4</duration></note>
      <barline location="right"><ending number="2" type="discontinue"/></barline>
    </measure>
  </part>
  <part id="P2">
    <measure number="1">
      <attributes>
        <divisions>2</divisions>
        <time><beats>4</beats><beat-type>4</beat-type></time>
        <clef><sign>F</sign><line>4</line></clef>
        <staff-details><staff-lines>4</staff-lines>
          <staff-tuning line="1"><tuning-step>E</tuning-step><tuning-octave>1</tuning-octave></staff-tuning>
          <staff-tuning line="2"><tuning-step>A</tuning-step><tuning-octave>1</tuning-octave></staff-tuning>
          <staff-tuning line="3"><tuning-step>D</tuning-step><tuning-octave>2</tuning-octave></staff-tuning>
          <staff-tuning line="4"><tuning-step>G</tuning-step><tuning-octave>2</tuning-octave></staff-tuning>
        </staff-details>
        <transpose><diatonic>0</diatonic><chromatic>0</chromatic><octave-change>-1</octave-change></transpose>
      </attributes>
      <note><pitch><step>E</step><octave>2</octave></pitch><duration>2</duration>
        <notations><technical><string>4</string><fret>0</fret></technical></notations></note>
      <note><pitch><step>A</step><octave>2</octave></pitch><duration>2</duration>
        <notations><technical><string>3</string><fret>0</fret></technical></notations></note>
      <note><chord/><pitch><step>E</step><octave>3</octave></pitch><duration>2</duration></note>
      <backup><duration>2</duration></backup>
      <forward><duration>2</duration></forward>
      <note><pitch><step>G</step><octave>2</octave></pitch><duration>4</duration><tie type="start"/>
        <notations><tied type="start"/><articulations><staccato/></articulations></notations></note>
    </measure>
    <measure number="2">
      <note><pitch><step>G</step><octave>2</octave></pitch><duration>4</duration><tie type="stop"/></note>
      <note><rest/><duration>2</duration></note>
      <note><grace/><pitch><step>B</step><octave>2</octave></pitch></note>
      <note><pitch><step>C</step><octave>3</octave></pitch><duration>2</duration></note>
    </measure>
    <measure number="3">
      <note><pitch><step>E</step><octave>2</octave></pitch><duration>8</duration></note>
    </measure>
  </part>
</score-partwise>
"#
    .to_string()
}

/// One bass part, one whole note per measure (E2 written, E1 + measure index sounding);
/// `extras[i]` is inserted into measure `i` (directions, barlines).
pub fn musicxml_bars(extras: &[&str]) -> String {
    let steps = ["E", "F", "G", "A", "B", "C"];
    let mut measures = String::new();
    for (i, extra) in extras.iter().enumerate() {
        let attributes = if i == 0 {
            "<attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time></attributes>"
        } else {
            ""
        };
        let octave = if i < 5 { 1 } else { 2 };
        measures.push_str(&format!(
            r#"<measure number="{n}">{attributes}{extra}<note><pitch><step>{step}</step><octave>{octave}</octave></pitch><duration>4</duration></note></measure>"#,
            n = i + 1,
            step = steps[i % steps.len()],
        ));
    }
    format!(
        r#"<?xml version="1.0"?><score-partwise><part-list><score-part id="P1"><part-name>Bass</part-name></score-part></part-list><part id="P1">{measures}</part></score-partwise>"#
    )
}

/// `.mxl` archive with container pointing to `score.musicxml`.
pub fn mxl() -> Vec<u8> {
    let container = br#"<?xml version="1.0"?><container><rootfiles><rootfile full-path="score.musicxml"/></rootfiles></container>"#;
    let score = musicxml();
    zip(
        &[
            ("META-INF/container.xml", container.as_slice()),
            ("score.musicxml", score.as_bytes()),
        ],
        Method::Deflated,
    )
}

// ---------------------------------------------------------------------------------------------
// Corruption

/// Deterministic pseudo-random generator (xorshift).
pub struct Noise(pub u64);

impl Noise {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    /// Copy of `data` with `count` bytes replaced.
    pub fn flip(&mut self, data: &[u8], count: usize) -> Vec<u8> {
        let mut out = data.to_vec();
        if out.is_empty() {
            return out;
        }
        for _ in 0..count {
            let at = (self.next() % out.len() as u64) as usize;
            out[at] = self.next() as u8;
        }
        out
    }
}
