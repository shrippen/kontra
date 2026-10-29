//! A steady tone under the input (mains hum) apart from played notes (LH-NF-16, F3): an attack
//! that only leaves the background's pitch behind it has no pitch of its own.

use super::{NoteTracker, Pending, LEGATO_CENTS, RISE_GAP_S};
use crate::yin::power_at;
use kontra_model::hz_to_midi;

const CENTS_PER_SEMITONE: f64 = 100.0;
/// A pitch this close to the background's is the background's: the attack's tail pulls the
/// estimate on hum by up to 35 ct (50 Hz heard as 49.4 Hz).
const BACKGROUND_CENTS: f64 = 50.0;
/// A pitch near a steady background is a note of its own when this much stronger at that pitch
/// (power, +12 dB). Hum −50 dBFS against a pluck at −15 dBFS: +35 dB; a click leaves it as it was.
const BACKGROUND_RISE: f64 = 16.0;
/// How long before the attack a steady background tone must already sound (see `stands_out`).
const BACKGROUND_EARLIER_S: f64 = 0.2;
/// A background counts as a tone of its own only this clear: mains hum 0.98; a ringing or
/// changing note before an attack (laptop jack, band) 0.6–0.8.
const BACKGROUND_CONFIDENCE: f32 = 0.9;

impl NoteTracker {
    /// Whether the pitch found after the attack is new, not that of a steady tone before it.
    /// A click or scrape on mains hum leaves the hum's pitch after it: no note, the attack stays
    /// pitchless (LH-NF-16, idle-line: 31 false notes a minute at 50 Hz). Steady means heard
    /// just before the attack and `BACKGROUND_EARLIER_S` before that: a played pitch often
    /// sounds a little before its attack is found (legato, the fretting finger), rarely that
    /// long. The last note's pitch is that note ringing: struck again, it keeps its pitch.
    ///
    /// ```text
    ///   ├─ YIN ─┤ ··· 200 ms ··· ├─ YIN ─┤ gap │ attack ─ YIN (best)
    ///   both clear, both ≈ best, ≠ last note, not +12 dB at best ─► pitchless
    /// ```
    pub(super) fn stands_out(&mut self, p: &Pending) -> bool {
        let Some(best) = p.best else {
            return true;
        };
        let a4_hz = self.config.a4_hz;
        let found = hz_to_midi(best.hz, a4_hz);
        let near = |a: f64, b: f64| (a - b).abs() * CENTS_PER_SEMITONE < BACKGROUND_CENTS;
        // Closer to the last note than a new note would be: that note, glided a little (fretless).
        if self
            .last_midi
            .is_some_and(|m| (m - found).abs() * CENTS_PER_SEMITONE < LEGATO_CENTS)
        {
            return true;
        }
        let sr = self.sample_rate;
        let before_end = p.onset.saturating_sub((RISE_GAP_S * sr) as u64);
        let earlier_end = before_end.saturating_sub((BACKGROUND_EARLIER_S * sr) as u64);
        let steady = [before_end, earlier_end]
            .into_iter()
            .all(|end| self.background(end).is_some_and(|m| near(m, found)));
        if !steady {
            return true;
        }

        // A note played on top of it: much louder at its pitch than the background was (G1 at
        // 49 Hz on 50 Hz hum). A scrape is loud, but spread over all frequencies.
        let after_from = p.onset + (self.config.attack_skip_s * sr) as u64;
        let count = (self.yin.needed() as u64).min(self.pushed.saturating_sub(after_from));
        let before = self.power(before_end.saturating_sub(count), count, best.hz);
        let after = self.power(after_from, count, best.hz);
        after >= before * BACKGROUND_RISE
    }

    /// Power at `hz` over `count` samples of the ring from `from`.
    fn power(&mut self, from: u64, count: u64, hz: f64) -> f64 {
        let count = count as usize;
        self.copy_out(from, count);
        power_at(&self.scratch[..count], hz, self.sample_rate)
    }

    /// Pitch (MIDI) of a clear tone in the `needed()` samples before `end`, if the ring holds
    /// them.
    fn background(&mut self, end: u64) -> Option<f64> {
        let needed = self.yin.needed();
        let from = end.checked_sub(needed as u64)?;
        if from < self.pushed.saturating_sub(self.ring.len() as u64) {
            return None;
        }
        self.copy_out(from, needed);
        let tone = self.yin.estimate_attack(&self.scratch[..needed])?;
        (tone.confidence >= BACKGROUND_CONFIDENCE).then(|| hz_to_midi(tone.hz, self.config.a4_hz))
    }
}
