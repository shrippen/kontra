//! A band that listens (LH-WIE-05): in free play over generated drums, the drums answer the
//! bass. A phrase that ends with two free beats gets a fill in the second; a dense bar thins the
//! next one (no hi-hat). Only the sound changes, nothing is scored.
//!
//! ```text
//! beat     1    2    3    4  | 1 …
//! bass     ●  ● ●  ●            ← the phrase ends, beat 3 stays free
//! drums    k  h s  h    ░░░░    ← decided just before beat 4: a fill instead of the groove
//!                       ▲ decide (DECIDE_LEAD_S before beat 4; notes heard up to DETECT_LAG_S ago)
//! ```
//! The fill needs no lookahead into the future: Kontra waits until just before the last beat,
//! hears that the beat before it was free and plays the fill in the remaining one.

/// The last beat's drums are planned this long before it, so the free beat before is heard.
pub(crate) const DECIDE_LEAD_S: f64 = 0.1;
/// Notes are told about this long after their onset (LH-NF-15: 70 ms per note, plus margin).
const DETECT_LAG_S: f64 = 0.09;
/// An attack this early before the free beat (a late note) still counts as before it.
const FREE_MARGIN_S: f64 = 0.03;
/// At most one fill in this many bars.
pub(crate) const FILL_EVERY_BARS: f64 = 8.0;
/// Attacks per beat from which a bar is dense (eighths).
const DENSE_PER_BEAT: f64 = 2.0;
/// A bar is judged dense only after this many beats of it were heard.
const DENSE_MIN_BEATS: f64 = 2.0;
/// Attacks kept: a bar of sixteenths at a slow tempo and the bar before.
const HEARD_KEPT: usize = 64;

/// Whether the drums listen to the bass (LH-WIE-05; switchable, off unless chosen).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Band {
    /// The drums play as written.
    #[default]
    AsWritten,
    /// Fills after a phrase, thinner after a dense bar; in free play over generated drums.
    Reacting,
}

/// A bar on the engine clock: real start and length of a beat at the current speed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BarTime {
    pub start_s: f64,
    pub beat_s: f64,
    pub beats: u32,
}

impl BarTime {
    fn end_s(self) -> f64 {
        self.start_s + self.beat_s * f64::from(self.beats)
    }

    /// Start of beat `n` (0 = the downbeat).
    pub(crate) fn beat_start_s(self, n: u32) -> f64 {
        self.start_s + self.beat_s * f64::from(n)
    }
}

/// Listens to the bass and decides each bar once.
#[derive(Debug, Default)]
pub(crate) struct ReactingBand {
    /// Onsets heard, real seconds (latency taken off), oldest first.
    heard: Vec<f64>,
    /// Real start of the last bar with a fill.
    last_fill_s: Option<f64>,
    /// Decisions by (pass, bar index), so a bar plays as first decided.
    fills: Vec<((u32, usize), bool)>,
    thinned: Vec<((u32, usize), bool)>,
}

impl ReactingBand {
    /// An attack at real time `real_s`.
    pub(crate) fn heard(&mut self, real_s: f64) {
        if self.heard.len() >= HEARD_KEPT {
            self.heard.remove(0);
        }
        self.heard.push(real_s);
    }

    /// Forgets the decisions (seek, loop, speed): bars are decided anew.
    pub(crate) fn replan(&mut self) {
        self.fills.clear();
        self.thinned.clear();
    }

    /// Whether `bar` (of `key`: pass, bar index) ends with a fill, decided at `now_s`: the
    /// beat before its last was free, the player played in the beat before that or earlier in
    /// the bar (a phrase ended), and the last fill is `FILL_EVERY_BARS` bars ago. `None` while
    /// it is too early to tell.
    pub(crate) fn fill(&mut self, key: (u32, usize), bar: BarTime, now_s: f64) -> Option<bool> {
        if let Some((_, fill)) = self.fills.iter().find(|(k, _)| *k == key) {
            return Some(*fill);
        }
        let last_beat = bar.beat_start_s(bar.beats.saturating_sub(1));
        if now_s < last_beat - DECIDE_LEAD_S || bar.beats < 2 {
            return None;
        }
        let free_from = bar.beat_start_s(bar.beats - 2);
        let heard_until = now_s - DETECT_LAG_S;
        let free = !self.any_in(free_from - FREE_MARGIN_S, heard_until);
        let phrase = self.any_in(bar.start_s - bar.beat_s, free_from - FREE_MARGIN_S);
        let bar_s = bar.end_s() - bar.start_s;
        let rested = self
            .last_fill_s
            .is_none_or(|last| bar.start_s - last >= FILL_EVERY_BARS * bar_s - 1e-6);
        let fill = free && phrase && rested;
        if fill {
            self.last_fill_s = Some(bar.start_s);
        }
        self.fills.push((key, fill));
        Some(fill)
    }

    /// Whether `bar` (of `key`) leaves out the hi-hat: the bar before it (`before`) was dense
    /// as far as it was heard at `now_s`. Decided once.
    pub(crate) fn thin(&mut self, key: (u32, usize), before: BarTime, now_s: f64) -> bool {
        if let Some((_, thin)) = self.thinned.iter().find(|(k, _)| *k == key) {
            return *thin;
        }
        let heard_until = (now_s - DETECT_LAG_S).min(before.end_s());
        let beats = (heard_until - before.start_s) / before.beat_s;
        let attacks = self
            .heard
            .iter()
            .filter(|t| **t >= before.start_s - FREE_MARGIN_S && **t < heard_until)
            .count() as f64;
        let thin = beats >= DENSE_MIN_BEATS && attacks >= DENSE_PER_BEAT * beats.floor();
        self.thinned.push((key, thin));
        thin
    }

    fn any_in(&self, from_s: f64, to_s: f64) -> bool {
        self.heard.iter().any(|t| *t >= from_s && *t < to_s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BEAT_S: f64 = 0.5;

    fn bar(n: u32) -> BarTime {
        BarTime {
            start_s: f64::from(n) * 4.0 * BEAT_S,
            beat_s: BEAT_S,
            beats: 4,
        }
    }

    /// The moment the last beat of bar `n` is decided.
    fn decide_at(n: u32) -> f64 {
        bar(n).beat_start_s(3) - DECIDE_LEAD_S
    }

    #[test]
    fn a_free_beat_after_a_phrase_brings_a_fill() {
        let mut band = ReactingBand::default();
        // Bar 0: beats 1 and 2 played, 3 and 4 free.
        band.heard(0.0);
        band.heard(0.5);
        assert_eq!(
            band.fill((0, 0), bar(0), decide_at(0) - 0.2),
            None,
            "too early"
        );
        assert_eq!(band.fill((0, 0), bar(0), decide_at(0)), Some(true));
        // Asked again: the same answer, the fill is not counted twice.
        assert_eq!(band.fill((0, 0), bar(0), decide_at(0) + 0.05), Some(true));
    }

    #[test]
    fn no_fill_without_a_phrase_or_while_playing() {
        let mut band = ReactingBand::default();
        // Silence: no phrase ended.
        assert_eq!(band.fill((0, 0), bar(0), decide_at(0)), Some(false));
        // Bar 1 played through beat 3.
        for beat in 4..7 {
            band.heard(f64::from(beat) * BEAT_S);
        }
        assert_eq!(band.fill((0, 1), bar(1), decide_at(1)), Some(false));
    }

    #[test]
    fn fills_come_at_most_every_eight_bars() {
        let mut band = ReactingBand::default();
        let mut fills = Vec::new();
        for n in 0..20 {
            band.heard(bar(n).start_s);
            band.heard(bar(n).beat_start_s(1));
            if band.fill((0, n as usize), bar(n), decide_at(n)) == Some(true) {
                fills.push(n);
            }
        }
        assert_eq!(fills, [0, 8, 16]);
    }

    /// A recording (LH-WIE-05 acceptance): two-bar phrases at 120 BPM, the second bar leaves
    /// beats 3 and 4 free; one phrase plays beat 3 and leaves only beat 4; one bar in eighths.
    /// The detector hears it as the app does; each bar is decided with the notes told by then.
    #[test]
    fn a_recorded_line_gets_fills_only_after_rests_and_eight_bars_apart() {
        use kontra_dsp::synth::{render, SynthNote, ToneShape};
        const SAMPLE_RATE: f64 = 48_000.0;
        const BARS: u32 = 24;
        const A1_HZ: f64 = 55.0;
        // Beats played per bar (of four; eighths in bar 20).
        let beats = |n: u32| -> Vec<f64> {
            match n {
                20 => (0..8).map(|i| f64::from(i) / 2.0).collect(),
                // Bar 13: beat 3 played, only beat 4 free (no fill).
                13 => vec![0.0, 1.0, 2.0],
                n if n % 2 == 1 => vec![0.0, 1.0],
                _ => vec![0.0, 1.0, 2.0, 3.0],
            }
        };
        let notes: Vec<SynthNote> = (0..BARS)
            .flat_map(|n| beats(n).into_iter().map(move |b| (n, b)))
            .map(|(n, b)| SynthNote {
                onset_s: bar(n).start_s + b * BEAT_S + 0.01,
                hz: A1_HZ,
                duration_s: BEAT_S * 0.4,
                velocity: 0.8,
            })
            .collect();
        let total_s = bar(BARS).start_s + 1.0;
        let audio = render(&notes, SAMPLE_RATE, total_s, ToneShape::Rich, 7);
        let heard = kontra_engine::analyze(
            &audio,
            SAMPLE_RATE,
            &kontra_model::Instrument::default_bass(),
            kontra_model::InputSource::Line,
        );
        assert!(
            heard.len() >= notes.len() - 2,
            "{} of {}",
            heard.len(),
            notes.len()
        );

        // Told 70 ms after the onset (LH-NF-15).
        const TOLD_S: f64 = 0.07;
        let mut band = ReactingBand::default();
        let mut told = 0;
        let mut fills = Vec::new();
        let mut thinned = Vec::new();
        for n in 0..BARS {
            let mut tell_until = |band: &mut ReactingBand, now: f64| {
                while told < heard.len() && heard[told].onset_s + TOLD_S <= now {
                    band.heard(heard[told].onset_s);
                    told += 1;
                }
            };
            let thin_at = bar(n).start_s - 0.25;
            tell_until(&mut band, thin_at);
            if n > 0 && band.thin((0, n as usize), bar(n - 1), thin_at) {
                thinned.push(n);
            }
            tell_until(&mut band, decide_at(n));
            if band.fill((0, n as usize), bar(n), decide_at(n)) == Some(true) {
                fills.push(n);
            }
        }
        // Phrase ends in bars 1, 3, 5 …; 8 bars apart: 1, 9, 17. Bar 13 has no free beat 3.
        assert_eq!(fills, [1, 9, 17]);
        assert_eq!(thinned, [21]);
    }

    #[test]
    fn a_dense_bar_thins_the_next() {
        let mut band = ReactingBand::default();
        // Bar 0 in eighths, as heard 0.25 s before bar 1 starts.
        for i in 0..8 {
            band.heard(f64::from(i) * BEAT_S / 2.0);
        }
        assert!(band.thin((0, 1), bar(0), bar(1).start_s - 0.25));
        // Quarters are not dense.
        let mut calm = ReactingBand::default();
        for i in 0..4 {
            calm.heard(f64::from(i) * BEAT_S);
        }
        assert!(!calm.thin((0, 1), bar(0), bar(1).start_s - 0.25));
    }
}
