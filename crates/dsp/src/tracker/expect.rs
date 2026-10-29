//! Strikes the tab expects (R2): a gatekeeper for the first report, never the decider.
//!
//! An attack near an expected strike is tried from 8 ms after the attack skip on, on a short
//! stretch that holds one period of the expected tone and lags up to just past twice it. The
//! expected pitch goes out early only when everything agrees, twice in a row:
//!
//! ```text
//! attack ─ skip ─ every 5 ms: CMNDF at the expected period < 0.25,
//!                             the octave below not clearly deeper,
//!                             YIN's own pick within 30 ct of the expected pitch ─► twice? ─► emit
//!          … full window there ─► the usual search (two agree) has the last word:
//!                                 another pitch ─► PitchRevision
//! ```
//!
//! A loose rule (any expected period that repeats) told wrongly played notes as the expected
//! ones three times as often (IDMT, expectation a semitone off: 16 → 47); with the free estimate
//! deciding and the full window confirming, the final pitch stays as without expectation
//! (experiment pitch-fast, 29.10.2026).

use super::{
    NoteTracker, Pending, FOLLOW_STEP_S, MERGE_S, PRECURSOR_FACTOR, RETRY_S, RISE_AFTER_S,
};
use crate::intonation::refine_hz;
use crate::yin::PitchEstimate;
use kontra_model::{cents_between, hz_to_midi, midi_to_hz, ExpectedStrike, PitchRevision};

/// Strikes kept at once: the session tells them about 0.25 s ahead; sixteenths at 240 BPM are
/// 16 a second.
const SLOTS: usize = 16;
/// An expected strike is kept this long after its window: an attack is decided up to the
/// pitch timeout (150 ms) after its onset.
const KEEP_S: f64 = 0.25;
/// First early try after the attack skip.
const EARLY_FROM_S: f64 = 0.008;
/// The short stretch: one period of the expected tone as the window, lags up to just past
/// twice it (so the octave below is seen), plus two samples for the dip search.
const WINDOW_PERIODS: f64 = 1.0;
const LAG_PERIODS: f64 = 2.06;
const LAG_EXTRA: usize = 2;
/// The expected period must repeat this well (normalized difference) …
const PERIOD_DIP: f64 = 0.25;
/// … and the octave below must not repeat better by more than this (the player plays lower).
const OCTAVE_MARGIN: f64 = 0.05;
/// YIN's own pick must round to the expected semitone …
const HALF_SEMITONE: f64 = 0.5;
/// … and, refined, lie this close to it (cents).
const EXPECTED_CENTS: f64 = 30.0;
/// The full window revises an early pitch on another semitone or this far off (cents).
const CONFIRM_CENTS: f64 = 2.0;
const CENTS_PER_SEMITONE: f64 = 100.0;

/// Expected strikes on the tracker's sample clock.
#[derive(Debug, Default)]
pub(super) struct Expectations {
    slots: [Option<Slot>; SLOTS],
}

#[derive(Debug, Clone, Copy)]
struct Slot {
    onset: u64,
    window: u64,
    strike: ExpectedStrike,
}

impl Expectations {
    /// Keeps `strike`; strikes behind `pushed` make room first, then the earliest.
    pub(super) fn add(&mut self, strike: &ExpectedStrike, sample_rate: f64, pushed: u64) {
        let window = (strike.window_s.max(0.0) * sample_rate) as u64;
        let keep = (KEEP_S * sample_rate) as u64;
        if strike.onset_s + strike.window_s < 0.0 || strike.tones().is_empty() {
            return;
        }
        let onset = (strike.onset_s.max(0.0) * sample_rate) as u64;
        for slot in &mut self.slots {
            if slot.is_some_and(|s| s.onset + s.window + keep < pushed) {
                *slot = None;
            }
        }
        let free = self.slots.iter().position(Option::is_none);
        let earliest = || {
            let first = self.slots.iter().flatten().map(|s| s.onset).min();
            self.slots
                .iter()
                .position(|s| s.map(|s| s.onset) == first)
                .unwrap_or(0)
        };
        let at = free.unwrap_or_else(earliest);
        self.slots[at] = Some(Slot {
            onset,
            window,
            strike: *strike,
        });
    }

    pub(super) fn clear(&mut self) {
        self.slots = [None; SLOTS];
    }

    /// The strike nearest to an attack reported at `report`, if within its window.
    fn near(&self, report: u64) -> Option<ExpectedStrike> {
        self.slots
            .iter()
            .flatten()
            .filter(|s| s.onset.abs_diff(report) <= s.window)
            .min_by_key(|s| s.onset.abs_diff(report))
            .map(|s| s.strike)
    }
}

/// A note told early, and the usual search that has the last word on it.
#[derive(Debug, Clone, Copy)]
pub(super) struct Confirm {
    /// The emitted note's onset (sample) and pitch.
    pub(super) report: u64,
    midi: f64,
    pub(super) search: Pending,
}

impl NoteTracker {
    /// When an attack at `onset` is first tried: early, on a short window (R1, R2).
    pub(super) fn first_try(&self, onset: u64) -> u64 {
        let skip = (self.config.attack_skip_s * self.sample_rate) as u64;
        let early = onset + skip + (EARLY_FROM_S * self.sample_rate) as u64;
        early.min(self.full_try(onset))
    }

    /// Early tries due for `p`, until the full window is there; true when an expected pitch or
    /// an unambiguous one (`hear_free`, R1) held twice in a row (`p.best` is it then).
    pub(super) fn try_early(&mut self, p: &mut Pending) -> bool {
        let full = self.full_try(p.onset);
        let step = (RETRY_S * self.sample_rate) as u64;
        while p.next_try < full && p.next_try <= self.pushed {
            let heard = match self.hear_expected(p) {
                Some(heard) => Some(heard),
                None => self.hear_free(p),
            };
            let twice = heard.zip(p.early).is_some_and(|(now, last)| {
                cents_between(last.hz, now.hz).abs() < self.config.agree_cents
            });
            if twice && self.attack_start(p.onset) == p.onset {
                p.best = heard;
                return true;
            }
            p.early = heard;
            p.next_try = (p.next_try + step).min(full);
        }
        false
    }

    /// The expected pitch on the audio up to `p.next_try`, if YIN hears it (any tone of the
    /// strike: a double stop has two).
    fn hear_expected(&mut self, p: &Pending) -> Option<PitchEstimate> {
        let strike = self.expected.near(p.report)?;
        let skip = (self.config.attack_skip_s * self.sample_rate) as u64;
        let avail = p.next_try.saturating_sub(p.onset + skip) as usize;
        strike
            .tones()
            .iter()
            .find_map(|tone| self.hear_tone(f64::from(tone.midi), p.next_try, avail))
    }

    /// Whether the `avail` samples before `until` hold `midi` (see the module comment).
    fn hear_tone(&mut self, midi: f64, until: u64, avail: usize) -> Option<PitchEstimate> {
        let a4_hz = self.config.a4_hz;
        let period = self.sample_rate / midi_to_hz(midi, a4_hz);
        let lag = (LAG_PERIODS * period).ceil() as usize + LAG_EXTRA;
        let len = (WINDOW_PERIODS * period).ceil() as usize + lag;
        if len > avail || len > self.scratch.len() {
            return None;
        }
        self.copy_out(until - len as u64, len);
        let samples = &self.scratch[..len];
        let heard = self.yin.estimate_expected(samples, lag, period)?;
        if heard.at_period >= PERIOD_DIP || heard.at_octave_below < heard.at_period - OCTAVE_MARGIN
        {
            return None;
        }

        // The free estimate has the last word: the expected semitone, refined near its pitch.
        if (hz_to_midi(heard.free.hz, a4_hz) - midi).abs() >= HALF_SEMITONE {
            return None;
        }
        let mut estimate = heard.free;
        if let Some(hz) = refine_hz(samples, self.sample_rate, estimate.hz) {
            estimate.hz = hz;
        }
        let cents = (hz_to_midi(estimate.hz, a4_hz) - midi).abs() * CENTS_PER_SEMITONE;
        (cents < EXPECTED_CENTS).then_some(estimate)
    }

    /// Emits a note told early; the usual search goes on for it.
    pub(super) fn emit_early(&mut self, p: Pending) {
        let midi = p.best.map(|b| hz_to_midi(b.hz, self.config.a4_hz));
        self.emit(p);
        let Some(midi) = midi else {
            return;
        };
        let search = Pending {
            last: None,
            best: None,
            next_try: self.full_try(p.onset),
            checked: false,
            early: None,
            ..p
        };
        self.confirm = Some(Confirm {
            report: p.report,
            midi,
            search,
        });
    }

    /// Goes on with the search for a note told early; tells its pitch once found.
    pub(super) fn check_confirm(&mut self) {
        let Some(mut c) = self.confirm.take() else {
            return;
        };
        // An attack waiting for its rise check may still show the note was a precursor
        // (`retracts_early`); the search catches up after it.
        if self.candidate.is_some_and(|onset| onset > c.report) {
            self.confirm = Some(c);
            return;
        }
        if !self.search(&mut c.search) {
            self.confirm = Some(c);
            return;
        }

        // The ringing note is followed from here on, as when told at this point.
        let step = (FOLLOW_STEP_S * self.sample_rate) as u64;
        let next_try = self.pushed.max(c.search.next_try) + step;
        if let Some(s) = self.sounding.as_mut() {
            s.next_try = next_try;
        }
        self.confirmed(&c);
    }

    /// Ends the search for a note told early with what it found (a new attack, the end).
    pub(super) fn finish_confirm(&mut self) {
        if let Some(c) = self.confirm.take() {
            self.confirmed(&c);
        }
    }

    /// A second burst of an attack told early (within `MERGE_S`, not a much louder pluck): the
    /// search moves to it, as for a note still pending (`close_pending`). Returns whether it
    /// joined.
    pub(super) fn joins_early(&mut self, onset: u64) -> bool {
        let Some(c) = self.confirm else {
            return false;
        };
        let merge = (MERGE_S * self.sample_rate) as u64;
        if onset >= c.report + merge || c.search.best.is_some() {
            return false;
        }
        let before = self.peak(c.report, onset);
        let after = self.peak(onset, onset + (RISE_AFTER_S * self.sample_rate) as u64);
        if after >= before * PRECURSOR_FACTOR {
            return false;
        }
        let search = Pending {
            onset,
            last: None,
            next_try: self.full_try(onset),
            checked: false,
            ..c.search
        };
        self.confirm = Some(Confirm { search, ..c });
        true
    }

    /// The search's pitch replaces the early one where they differ.
    fn confirmed(&mut self, c: &Confirm) {
        let Some(best) = c.search.best else {
            return;
        };
        let midi = hz_to_midi(best.hz, self.config.a4_hz);
        let same = midi.round() == c.midi.round()
            && (midi - c.midi).abs() * CENTS_PER_SEMITONE < CONFIRM_CENTS;
        if same {
            return;
        }
        self.pitch_revised.push(PitchRevision {
            onset_s: c.report as f64 / self.sample_rate,
            midi,
        });
        let Some(s) = self.sounding.as_mut() else {
            return;
        };
        let Some(t) = s.settle.as_mut().filter(|t| t.onset == c.report) else {
            return;
        };
        t.midi = midi;
        s.midi = midi;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracker::TrackerConfig;
    use kontra_model::{InputSource, Instrument};

    const SR: f64 = 48_000.0;

    /// The revisions when the full window hears `heard` for a note told early at `told`.
    fn revised(told: f64, heard: f64) -> Vec<PitchRevision> {
        let config = TrackerConfig::for_instrument(&Instrument::default_bass(), InputSource::Line);
        let mut t = NoteTracker::new(SR, config);
        let search = Pending {
            onset: 0,
            report: 0,
            last: None,
            best: Some(PitchEstimate {
                hz: midi_to_hz(heard, config.a4_hz),
                confidence: 0.9,
            }),
            next_try: 0,
            checked: true,
            early: None,
        };
        let c = Confirm {
            report: 0,
            midi: told,
            search,
        };
        t.confirmed(&c);
        t.pitch_revised().to_vec()
    }

    #[test]
    fn full_window_revises_both_ways_and_cents() {
        // Up and down a semitone or an octave, and the same semitone a few cents off (R1).
        for (told, heard) in [(37.0, 38.0), (38.9, 38.0), (50.0, 38.0), (38.0, 38.05)] {
            let r = revised(told, heard);
            assert_eq!(r.len(), 1, "{told} → {heard}: {r:?}");
            assert!((r[0].midi - heard).abs() < 1e-6, "{r:?}");
        }
        assert!(revised(38.0, 38.01).is_empty(), "within 2 ct it stands");
    }
}
