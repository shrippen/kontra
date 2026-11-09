//! Improvisation scoring (LH-BEW-13): in an open exercise the player makes up the line, so no
//! note is right or wrong. Each attack counts by how it fits the bar's chord: a chord tone, a
//! tone of the key, or a foreign tone. Where the written example line rests for a beat or more,
//! a rest window asks for air between phrases (optional: a line without rests has none).
//!
//! ```text
//! chord  | Am7            | D7             |
//! played   A  C  E  F#      D  F# C  A#
//!          ●  ●  ●  ○       ●  ●  ●  ×      ● chord tone  ○ in the key  × foreign
//! fitting 7/8, chord tones 6/8
//! ```

use kontra_model::{in_key, Arrangement, Chord, MarkerKind};
use serde::{Deserialize, Serialize};

/// The shortest rest of the example line that counts as a rest window, in beats.
const REST_BEATS: f64 = 1.0;

/// How an attack fits its bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fit {
    ChordTone,
    /// In the key, not in the chord.
    ScaleTone,
    Foreign,
}

/// Attacks of an open run by fit, and the rest windows kept free.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Improvisation {
    pub chord_tones: u32,
    pub scale_tones: u32,
    pub foreign: u32,
    /// Rest windows the run passed through, and those without an attack.
    pub rests: u32,
    pub rests_kept: u32,
}

impl Improvisation {
    /// Attacks counted.
    pub fn played(&self) -> u32 {
        self.chord_tones + self.scale_tones + self.foreign
    }

    /// Chord and scale tones among the attacks, 0..=1; none played: `None`.
    pub fn fitting_share(&self) -> Option<f64> {
        self.share(self.chord_tones + self.scale_tones)
    }

    /// Chord tones among the attacks, 0..=1.
    pub fn chord_share(&self) -> Option<f64> {
        self.share(self.chord_tones)
    }

    fn share(&self, count: u32) -> Option<f64> {
        let played = self.played();
        (played > 0).then(|| f64::from(count) / f64::from(played))
    }
}

/// Follows an open run: attacks with their fit and the rest windows.
#[derive(Debug, Default)]
pub(crate) struct OpenRun {
    /// (pass, position s, fit) of each attack.
    attacks: Vec<(u32, f64, Fit)>,
    /// Rest windows of the example line as positions (from, to).
    windows: Vec<(f64, f64)>,
}

impl OpenRun {
    pub(crate) fn new(arrangement: &Arrangement) -> OpenRun {
        OpenRun {
            windows: rest_windows(arrangement),
            ..OpenRun::default()
        }
    }

    /// Counts an attack at `position_s` of `pass` with the pitch `midi`.
    pub(crate) fn attack(
        &mut self,
        arrangement: &Arrangement,
        pass: u32,
        position_s: f64,
        midi: f64,
    ) {
        let tick = arrangement.tempo.tick_at(position_s);
        let midi = midi.round().clamp(0.0, f64::from(u8::MAX)) as u8;
        self.attacks
            .push((pass, position_s, fit(arrangement, tick, midi)));
    }

    /// The tally of the passes `passes` (all: `None`) up to `reached` (pass, position), within
    /// `range` (the loop or the whole piece); `margin_s` (the timing window) keeps an attack
    /// just before or after a rest window out of it.
    pub(crate) fn tally(
        &self,
        passes: Option<u32>,
        reached: (u32, f64),
        range: (f64, f64),
        margin_s: f64,
    ) -> Improvisation {
        let mut out = Improvisation::default();
        let counted = |pass: u32| passes.is_none_or(|p| p == pass);
        for (_, _, fit) in self.attacks.iter().filter(|(p, ..)| counted(*p)) {
            match fit {
                Fit::ChordTone => out.chord_tones += 1,
                Fit::ScaleTone => out.scale_tones += 1,
                Fit::Foreign => out.foreign += 1,
            }
        }
        let (start, end) = range;
        let windows: Vec<&(f64, f64)> = self
            .windows
            .iter()
            .filter(|(from, to)| *from >= start && *to <= end)
            .collect();
        let (last_pass, last_position) = reached;
        for pass in (0..=last_pass).filter(|p| counted(*p)) {
            for (from, to) in windows.iter().copied() {
                if pass == last_pass && *to > last_position {
                    continue;
                }
                out.rests += 1;
                let inside = |(p, at, _): &&(u32, f64, Fit)| {
                    *p == pass && *at > from + margin_s && *at < to - margin_s
                };
                if !self.attacks.iter().any(|a| inside(&a)) {
                    out.rests_kept += 1;
                }
            }
        }
        out
    }
}

/// How `midi` at `tick` fits: the chord last named at or before it, else the bar's key alone.
pub fn fit(arrangement: &Arrangement, tick: u64, midi: u8) -> Fit {
    let chord = arrangement
        .markers
        .iter()
        .filter(|m| m.kind == MarkerKind::Chord && m.tick <= tick)
        .filter_map(|m| Chord::parse(&m.name))
        .next_back();
    if chord.is_some_and(|c| c.has(midi)) {
        return Fit::ChordTone;
    }
    let key = arrangement
        .bar_at(tick)
        .and_then(|b| arrangement.bars.get(b))
        .map_or(0, |b| b.key_fifths);
    if in_key(key, midi) {
        Fit::ScaleTone
    } else {
        Fit::Foreign
    }
}

/// Rests of at least `REST_BEATS` between the example line's attacks, as positions.
fn rest_windows(arrangement: &Arrangement) -> Vec<(f64, f64)> {
    let tempo = &arrangement.tempo;
    let beat_s = |tick: u64| 60.0 / tempo.bpm_at(tick);
    let mut windows = Vec::new();
    let mut sounding_until: Option<u64> = None;
    for note in &arrangement.notes {
        if let Some(end) = sounding_until.filter(|end| *end < note.start_ticks) {
            let (from, to) = (tempo.seconds_at(end), tempo.seconds_at(note.start_ticks));
            if to - from >= REST_BEATS * beat_s(end) - 1e-9 {
                windows.push((from, to));
            }
        }
        sounding_until = Some(sounding_until.unwrap_or(0).max(note.end_ticks()));
    }
    windows
}
