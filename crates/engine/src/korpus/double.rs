//! Double stops in the corpus (R8, LH-BEW-11): sets where the tab announces two tones, and
//! how often the tracker's verdict was right.
//!
//! ```text
//! synthetic dyads (8 intervals on playable string pairs) ─┐
//! IDMT notes mixed in pairs (semi-real)                   ├─► announced ─► verdicts ─► tally
//! single-note sets, a second tone announced (false alarm) ┘
//! ```

use super::{band, Annotation, KorpusItem, KorpusSet, SAMPLE_RATE};
use crate::offline::{analyze_reported, cancel_echo};
use kontra_dsp::synth::{bass_tone, Noise, ToneShape};
use kontra_model::{DoubleStopHeard, DoubleStopVerdict, ExpectedStrike, ExpectedTone, Technique};

/// Intervals (semitones) a bassist plays as double stops: thirds, fourth, fifth, sixth,
/// octave, tenths.
pub const DOUBLE_STOP_INTERVALS: [u8; 8] = [3, 4, 5, 7, 9, 12, 15, 16];
/// Open strings of a four-string bass (E1 A1 D2 G2) and the frets used: up to the 9th, lower
/// fret even, the hand spanning at most four frets.
const OPEN_STRINGS: [u8; 4] = [28, 33, 38, 43];
const HIGHEST_FRET: u8 = 9;
const SPAN_FRETS: u8 = 4;
/// The upper string sounds up to this much later (a pluck across two strings) and softer.
const UPPER_LATE_S: f64 = 0.008;
const UPPER_SOFTEST: f32 = 0.6;
/// Each dyad rings this long; the next starts after a short rest.
const TONE_S: f64 = 0.5;
const SPACING_S: f64 = 0.6;
const LEAD_S: f64 = 0.3;
/// Two tones together reach twice one tone's level: halved.
const MIX_GAIN: f32 = 0.5;
/// Window of an announced double stop (the session's loose timing window).
const ANNOUNCE_WINDOW_S: f64 = 0.1;
/// A recorded note gives a segment up to the next note, this long at most and at least.
const SEGMENT_MAX_S: f64 = 1.0;
const SEGMENT_MIN_S: f64 = 0.3;
const SEMITONES_PER_OCTAVE: u8 = 12;

/// What was played where the tab announced a double stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoubleStopPlayed {
    Both,
    Lower,
    Upper,
}

/// A double stop the tab announces at `onset_s`, and what was played there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DoubleStopStrike {
    pub onset_s: f64,
    pub lower: u8,
    pub upper: u8,
    pub played: DoubleStopPlayed,
}

impl DoubleStopStrike {
    fn is_octave(&self) -> bool {
        (self.upper - self.lower).is_multiple_of(SEMITONES_PER_OCTAVE)
    }
}

/// Audio with announced double stops.
#[derive(Debug, Clone, PartialEq)]
pub struct DoubleStopSet {
    pub sample_rate: u32,
    pub samples: Vec<f32>,
    pub strikes: Vec<DoubleStopStrike>,
}

/// How the verdicts matched what was played. Octaves are counted apart: they get no statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DoubleStopTally {
    /// Both tones played (not octaves, told or not), and told as both.
    pub dyads: usize,
    pub dyads_both: usize,
    /// One tone played where two were announced (not octaves, told or not): told as both (a
    /// false alarm), told as the other tone.
    pub singles: usize,
    pub singles_both: usize,
    pub singles_other: usize,
    /// Octaves announced, and told anything but "no statement".
    pub octaves: usize,
    pub octaves_told: usize,
    /// Of `dyads` and `singles`, those without a verdict: no attack found there.
    pub untold: usize,
}

impl DoubleStopTally {
    pub fn add(&mut self, other: &DoubleStopTally) {
        self.dyads += other.dyads;
        self.dyads_both += other.dyads_both;
        self.singles += other.singles;
        self.singles_both += other.singles_both;
        self.singles_other += other.singles_other;
        self.octaves += other.octaves;
        self.octaves_told += other.octaves_told;
        self.untold += other.untold;
    }

    /// Share of double stops told as both.
    pub fn detected(&self) -> f64 {
        ratio(self.dyads_both, self.dyads)
    }

    /// Share of single tones told as both.
    pub fn false_alarms(&self) -> f64 {
        ratio(self.singles_both, self.singles)
    }

    fn count(&mut self, strike: &DoubleStopStrike, heard: Option<DoubleStopHeard>) {
        if strike.is_octave() {
            self.octaves += 1;
            if heard.is_some_and(|h| h != DoubleStopHeard::Unknown) {
                self.octaves_told += 1;
            }
            return;
        }
        // Untold ones count in the shares too: a double stop not judged is not detected.
        match strike.played {
            DoubleStopPlayed::Both => self.dyads += 1,
            DoubleStopPlayed::Lower | DoubleStopPlayed::Upper => self.singles += 1,
        }
        let Some(heard) = heard else {
            self.untold += 1;
            return;
        };
        let other = match strike.played {
            DoubleStopPlayed::Both => {
                if heard == DoubleStopHeard::Both {
                    self.dyads_both += 1;
                }
                return;
            }
            DoubleStopPlayed::Lower => DoubleStopHeard::Upper,
            DoubleStopPlayed::Upper => DoubleStopHeard::Lower,
        };
        if heard == DoubleStopHeard::Both {
            self.singles_both += 1;
        }
        if heard == other {
            self.singles_other += 1;
        }
    }
}

fn ratio(a: usize, b: usize) -> f64 {
    if b == 0 {
        return 0.0;
    }
    a as f64 / b as f64
}

/// Synthetic double stops on a four-string bass: every interval of `DOUBLE_STOP_INTERVALS` on
/// the string pairs and frets a hand reaches, round-wound and flat-wound, the upper string
/// 0–8 ms later and softer (deterministic).
pub fn generate_double_stops() -> DoubleStopSet {
    let sr = f64::from(SAMPLE_RATE);
    let mut noise = Noise::new(8);
    let mut pairs = Vec::new();
    for shape in [ToneShape::Rich, ToneShape::Dull] {
        for (s, low) in OPEN_STRINGS.iter().enumerate() {
            for high in &OPEN_STRINGS[s + 1..] {
                for a in (0..=HIGHEST_FRET).step_by(2) {
                    for b in 0..=HIGHEST_FRET {
                        let (lower, upper) = (low + a, high + b);
                        if a.abs_diff(b) > SPAN_FRETS
                            || !DOUBLE_STOP_INTERVALS.contains(&(upper - lower))
                        {
                            continue;
                        }
                        pairs.push((shape, lower, upper));
                    }
                }
            }
        }
    }
    let total_s = LEAD_S + pairs.len() as f64 * SPACING_S + LEAD_S;
    let mut samples = vec![0.0f32; (total_s * sr) as usize];
    let mut strikes = Vec::with_capacity(pairs.len());
    for (i, (shape, lower, upper)) in pairs.into_iter().enumerate() {
        let onset_s = LEAD_S + i as f64 * SPACING_S;
        let at = (onset_s * sr) as usize;
        let late = (unit(&mut noise) * UPPER_LATE_S * sr) as usize;
        let gain = UPPER_SOFTEST + (1.0 - UPPER_SOFTEST) * unit(&mut noise) as f32;
        let hz = |midi: u8| kontra_model::midi_to_hz(f64::from(midi), kontra_model::A4_HZ);
        let seed = i as u64 * 2;
        let low = bass_tone(hz(lower), sr, TONE_S, shape, seed);
        let high = bass_tone(hz(upper), sr, TONE_S, shape, seed + 1);
        mix_in(&mut samples, &low, at, MIX_GAIN);
        mix_in(&mut samples, &high, at + late, MIX_GAIN * gain);
        strikes.push(DoubleStopStrike {
            onset_s,
            lower,
            upper,
            played: DoubleStopPlayed::Both,
        });
    }
    DoubleStopSet {
        sample_rate: SAMPLE_RATE,
        samples,
        strikes,
    }
}

/// Uniform in [0, 1).
fn unit(noise: &mut Noise) -> f64 {
    f64::from(noise.sample()) * 0.5 + 0.5
}

fn mix_in(out: &mut [f32], tone: &[f32], at: usize, gain: f32) {
    for (o, x) in out.iter_mut().skip(at).zip(tone) {
        *o += x * gain;
    }
}

/// Every pitched note of `notes` announced as a double stop with a second tone `semitones`
/// away (above when positive): what was played is the single note (false alarms).
pub fn announce_second_tone(notes: &[Annotation], semitones: i8) -> Vec<DoubleStopStrike> {
    notes
        .iter()
        .filter(|n| n.technique != Some(Technique::Muted))
        .filter_map(|n| {
            let played = n.midi.round() as i16;
            let other = u8::try_from(played + i16::from(semitones)).ok()?;
            let played = u8::try_from(played).ok()?;
            let (lower, upper, which) = match semitones > 0 {
                true => (played, other, DoubleStopPlayed::Lower),
                false => (other, played, DoubleStopPlayed::Upper),
            };
            Some(DoubleStopStrike {
                onset_s: n.onset_s,
                lower,
                upper,
                played: which,
            })
        })
        .collect()
}

/// Semi-real double stops from a recording of single notes: pairs of its notes at each interval
/// of `DOUBLE_STOP_INTERVALS` (at most `per_interval` each), mixed, the upper 0–8 ms later.
pub fn double_stop_mixtures(item: &KorpusItem, per_interval: usize, seed: u64) -> DoubleStopSet {
    let sr = f64::from(item.sample_rate);
    let segments = segments(item);
    let mut noise = Noise::new(seed);
    let mut samples = vec![0.0f32; (LEAD_S * sr) as usize];
    let mut strikes = Vec::new();
    for interval in DOUBLE_STOP_INTERVALS {
        let mut made = 0;
        for (lower, a) in &segments {
            for (upper, b) in &segments {
                if made >= per_interval || upper.checked_sub(*lower) != Some(interval) {
                    continue;
                }
                let at = samples.len();
                let late = (unit(&mut noise) * UPPER_LATE_S * sr) as usize;
                let len = a.len().max(b.len() + late);
                samples.resize(at + len + (LEAD_S * sr) as usize, 0.0);
                mix_in(&mut samples, a, at, MIX_GAIN);
                mix_in(&mut samples, b, at + late, MIX_GAIN);
                strikes.push(DoubleStopStrike {
                    onset_s: at as f64 / sr,
                    lower: *lower,
                    upper: *upper,
                    played: DoubleStopPlayed::Both,
                });
                made += 1;
            }
        }
    }
    DoubleStopSet {
        sample_rate: item.sample_rate,
        samples,
        strikes,
    }
}

/// Each pitched note's audio from its onset to the next note (`SEGMENT_*`), with its pitch.
fn segments(item: &KorpusItem) -> Vec<(u8, &[f32])> {
    let sr = f64::from(item.sample_rate);
    let notes = &item.notes;
    let mut out = Vec::new();
    for (i, n) in notes.iter().enumerate() {
        if n.technique.is_some() || n.slide_cents.is_some() {
            continue;
        }
        let next = notes.get(i + 1).map_or(f64::INFINITY, |m| m.onset_s);
        let end = next.min(n.onset_s + SEGMENT_MAX_S);
        if end - n.onset_s < SEGMENT_MIN_S {
            continue;
        }
        let from = (n.onset_s * sr) as usize;
        let to = ((end * sr) as usize).min(item.samples.len());
        let Ok(midi) = u8::try_from(n.midi.round() as i64) else {
            continue;
        };
        if from < to {
            out.push((midi, &item.samples[from..to]));
        }
    }
    out
}

/// Runs the tracker over `samples` with `strikes` announced and counts its verdicts; a verdict
/// belongs to the strike within `window_s` of its note.
pub fn evaluate_double_stops(
    set: KorpusSet,
    samples: &[f32],
    sample_rate: u32,
    strikes: &[DoubleStopStrike],
    window_s: f64,
) -> DoubleStopTally {
    let rate = f64::from(sample_rate);
    let cleaned;
    let heard = match set {
        KorpusSet::RoomDoubleBacking => {
            cleaned = cancel_echo(samples, &band(samples.len(), rate));
            &cleaned
        }
        _ => samples,
    };
    let tone = |midi| ExpectedTone { midi, string: None };
    let mut expected: Vec<ExpectedStrike> = strikes
        .iter()
        .map(|s| {
            ExpectedStrike::new(s.onset_s, ANNOUNCE_WINDOW_S)
                .with(tone(s.lower))
                .with(tone(s.upper))
        })
        .collect();
    expected.sort_by(|a, b| a.onset_s.total_cmp(&b.onset_s));
    let told = analyze_reported(heard, rate, &set.instrument(), set.source(), &expected);
    let mut used = vec![false; told.double_stops.len()];
    let mut tally = DoubleStopTally::default();
    for strike in strikes {
        let heard = nearest(&told.double_stops, &used, strike.onset_s, window_s).map(|i| {
            used[i] = true;
            told.double_stops[i].heard
        });
        tally.count(strike, heard);
    }
    tally
}

/// The unused verdict nearest to `onset_s`, within `window_s`.
fn nearest(
    verdicts: &[DoubleStopVerdict],
    used: &[bool],
    onset_s: f64,
    window_s: f64,
) -> Option<usize> {
    verdicts
        .iter()
        .enumerate()
        .filter(|(i, v)| !used[*i] && (v.onset_s - onset_s).abs() <= window_s)
        .min_by(|a, b| {
            (a.1.onset_s - onset_s)
                .abs()
                .total_cmp(&(b.1.onset_s - onset_s).abs())
        })
        .map(|(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::korpus::{generate, MATCH_S};

    /// Every interval on every string pair, both string types.
    #[test]
    fn synthetic_set_covers_the_intervals() {
        let set = generate_double_stops();
        for interval in DOUBLE_STOP_INTERVALS {
            let count = set
                .strikes
                .iter()
                .filter(|s| s.upper - s.lower == interval)
                .count();
            assert!(count >= 2, "{interval}: {count}");
        }
        let last = set.strikes.last().map_or(0.0, |s| s.onset_s);
        assert!(last + TONE_S < set.samples.len() as f64 / f64::from(set.sample_rate));
    }

    /// A slice of the synthetic set and of line-4 with a fifth announced: double stops told,
    /// single notes not, octaves never.
    #[test]
    fn verdicts_on_a_slice() {
        let set = generate_double_stops();
        let sr = f64::from(set.sample_rate);
        let take = 24;
        let end = ((LEAD_S + take as f64 * SPACING_S) * sr) as usize;
        let strikes = &set.strikes[..take];
        let tally = evaluate_double_stops(
            KorpusSet::Line4,
            &set.samples[..end],
            set.sample_rate,
            strikes,
            MATCH_S,
        );
        assert!(tally.dyads > 0 && tally.detected() >= 0.9, "{tally:?}");
        assert_eq!(tally.octaves_told, 0, "{tally:?}");

        let item = generate(KorpusSet::Line4);
        let end = (12.0 * f64::from(item.sample_rate)) as usize;
        let notes: Vec<Annotation> = item
            .notes
            .iter()
            .filter(|n| n.onset_s < 11.5)
            .copied()
            .collect();
        for semitones in [7, -7, 12] {
            let strikes = announce_second_tone(&notes, semitones);
            let tally = evaluate_double_stops(
                KorpusSet::Line4,
                &item.samples[..end],
                item.sample_rate,
                &strikes,
                MATCH_S,
            );
            assert_eq!(tally.singles_both, 0, "{semitones}: {tally:?}");
            assert_eq!(tally.singles_other, 0, "{semitones}: {tally:?}");
            assert_eq!(tally.octaves_told, 0, "{semitones}: {tally:?}");
        }
    }

    /// Review #205: a double stop without a verdict counts against detection; one told as
    /// both out of two announced is half, not all.
    #[test]
    fn untold_counts_in_the_shares() {
        let strike = |played| DoubleStopStrike {
            onset_s: 0.0,
            lower: 33,
            upper: 40,
            played,
        };
        let mut tally = DoubleStopTally::default();
        tally.count(&strike(DoubleStopPlayed::Both), Some(DoubleStopHeard::Both));
        tally.count(&strike(DoubleStopPlayed::Both), None);
        tally.count(
            &strike(DoubleStopPlayed::Lower),
            Some(DoubleStopHeard::Both),
        );
        tally.count(&strike(DoubleStopPlayed::Lower), None);
        let shares = (tally.detected(), tally.false_alarms());
        assert_eq!(shares, (0.5, 0.5), "{tally:?}");
    }
}
