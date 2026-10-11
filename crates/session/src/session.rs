//! A practice session: one arrangement, one mode, scored live.

use crate::expected::expected_notes;
use crate::matcher::{Matcher, NoteResult, Outcome};
use crate::settings::ScoreSettings;
use crate::summary::Summary;
use crate::technique::{TechniqueJudge, TechniqueResult};
use crate::transport::{LoopRange, Transport};
use kontra_model::{Arrangement, DetectedNote};

/// Real seconds to wait before declaring a note missed: detected notes arrive up to ~150 ms
/// after their onset (concept 6.3), plus margin.
const DETECTION_GRACE_S: f64 = 0.25;

/// Riff-Werkstatt (LH-MOD-04): keeps the hit rate in a target band (decision round 5, after
/// DIDAKTIK.md idea I7). Above the band for `passes_needed` passes in a row → one step faster
/// (up to `target_speed`); below it as often → one step slower (down to `min_speed`); inside it
/// the tempo stays. Practice that is neither trivial nor overwhelming is where people learn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Workshop {
    pub target_speed: f64,
    pub min_speed: f64,
    pub step: f64,
    /// Hit rate band (low, high), e.g. 80–90 %.
    pub band: (f64, f64),
    pub passes_needed: u32,
}

impl Default for Workshop {
    fn default() -> Self {
        Workshop {
            target_speed: 1.0,
            min_speed: 0.25,
            step: 0.05,
            band: (0.8, 0.9),
            passes_needed: 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    /// Runs through, scores silently (LH-MOD-01).
    Play,
    /// Runs through with scoring and summary (LH-MOD-02).
    Song,
    /// Holds before each note until it is played (LH-MOD-03).
    Wait,
    /// Loop with automatic speed steps (LH-MOD-04).
    Workshop(Workshop),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SessionEvent {
    Result(NoteResult),
    /// How a note's technique was played (LH-BEW-06); apart from its `Result`.
    Technique(TechniqueResult),
    /// Wait mode: holding before this note (index into `Arrangement::notes`).
    Holding {
        note: usize,
    },
    Released,
    PassComplete {
        pass: u32,
        clean: bool,
        /// Hits among the pass's notes and extra notes (0..=1).
        hit_rate: f64,
    },
    SpeedChanged(f64),
    Finished,
}

#[derive(Debug)]
pub struct Session {
    arrangement: Arrangement,
    mode: Mode,
    settings: ScoreSettings,
    transport: Transport,
    matcher: Matcher,
    techniques: TechniqueJudge,
    results: Vec<NoteResult>,
    events: Vec<SessionEvent>,
    cursor_pass: u32,
    cursor_slot: usize,
    /// Passes in a row above (positive) or below (negative) the workshop band.
    band_streak: i32,
    holding: bool,
    finished: bool,
    /// Real time of the last `update`.
    now_s: f64,
}

impl Session {
    pub fn new(arrangement: Arrangement, mode: Mode, settings: ScoreSettings) -> Session {
        let matcher = Matcher::new(expected_notes(&arrangement));
        let techniques = TechniqueJudge::new(&arrangement, settings.source);
        let transport = Transport::new(arrangement.duration_s());
        Session {
            arrangement,
            mode,
            settings,
            transport,
            matcher,
            techniques,
            results: Vec::new(),
            events: Vec::new(),
            cursor_pass: 0,
            cursor_slot: 0,
            band_streak: 0,
            holding: false,
            finished: false,
            now_s: 0.0,
        }
    }

    pub fn arrangement(&self) -> &Arrangement {
        &self.arrangement
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn transport(&self) -> &Transport {
        &self.transport
    }

    pub fn settings(&self) -> &ScoreSettings {
        &self.settings
    }

    pub fn results(&self) -> &[NoteResult] {
        &self.results
    }

    /// Technique results so far (LH-BEW-06), in the order they were judged.
    pub fn technique_results(&self) -> &[TechniqueResult] {
        self.techniques.results()
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn play(&mut self, real_s: f64) {
        self.transport.play(real_s);
    }

    pub fn pause(&mut self, real_s: f64) {
        self.transport.pause(real_s);
    }

    pub fn set_speed(&mut self, real_s: f64, speed: f64) {
        self.transport.set_speed(real_s, speed);
        self.events
            .push(SessionEvent::SpeedChanged(self.transport.speed()));
    }

    /// Sets the loop (Workshop, LH-MOD-04) and restarts counting at its first note.
    pub fn set_loop(&mut self, real_s: f64, range: Option<LoopRange>) {
        self.transport.set_loop(real_s, range);
        let (position, pass) = self.transport.position_at(real_s);
        self.cursor_pass = pass;
        self.cursor_slot = self.first_slot_from(position);
        self.techniques.rewind(pass, position);
        self.band_streak = 0;
    }

    /// Jumps to `position_s`; counting restarts at the first note from there.
    pub fn seek(&mut self, real_s: f64, position_s: f64) {
        self.transport.seek(real_s, position_s);
        let (position, pass) = self.transport.position_at(real_s);
        self.cursor_pass = pass;
        self.cursor_slot = self.first_slot_from(position);
        self.techniques.rewind(pass, position);
        self.holding = false;
    }

    /// Scores a detected note. Its onset is on the same real clock as the transport.
    pub fn on_note(&mut self, note: &DetectedNote) {
        let real_s = note.onset_s - self.settings.latency_ms / 1000.0;
        if matches!(self.mode, Mode::Wait) {
            self.wait_note(real_s, note);
            return;
        }
        let (mut position, mut pass) = self.transport.position_at(real_s);
        let range = self.range();
        let speed = self.transport.speed();
        // Just before a loop wraps, a note with nothing to match in this pass is the next pass's
        // first note played early (QA P2: it was "extra" plus a "missed" first note).
        if let Some(LoopRange { start_s, end_s }) = self.transport.loop_range() {
            let window = self.settings.strictness.window_ms() / 1000.0 * speed;
            let wrapped = start_s - (end_s - position);
            if end_s - position <= window
                && !self.matcher.has_candidate(pass, position, window, range)
                && self.matcher.has_candidate(pass + 1, wrapped, window, range)
            {
                position = wrapped;
                pass += 1;
            }
        }
        // A hammer-on or pull-off target has no attack: a note there is judged for its technique
        // only, and is not an extra note.
        let window = self.settings.strictness.window_ms() / 1000.0 * speed;
        let legato =
            self.techniques
                .judge_legato(pass, position, note.midi, note.technique, window);
        if let Some(t) = legato {
            self.events.push(SessionEvent::Technique(t));
            return;
        }
        let result = self.matcher.score(
            pass,
            position,
            note.midi,
            note.technique,
            speed,
            &self.settings,
            range,
        );
        self.record(result);
        self.judge_struck(&result, Some(note.technique));
    }

    /// Advances the clock to `real_s`; returns what happened since the last call.
    pub fn update(&mut self, real_s: f64) -> Vec<SessionEvent> {
        self.now_s = real_s;
        let hold = self.hold_position();
        self.transport.advance(real_s, hold);
        if matches!(self.mode, Mode::Wait) && self.transport.is_held() && !self.holding {
            self.holding = true;
            if let Some(e) = self.matcher.expected().get(self.cursor_slot) {
                self.events.push(SessionEvent::Holding { note: e.index });
            }
        }
        if !matches!(self.mode, Mode::Wait) {
            self.expire(real_s - DETECTION_GRACE_S);
        }
        self.check_finished();
        std::mem::take(&mut self.events)
    }

    /// The next note the player has to strike in the current pass.
    pub fn next_expected(&self) -> Option<crate::expected::ExpectedNote> {
        self.matcher.expected().get(self.cursor_slot).copied()
    }

    pub fn summary(&self) -> Summary {
        let mut summary = Summary::from_results(&self.results, &self.arrangement);
        summary.techniques = self.techniques.tally();
        summary.untold_techniques = self.techniques.untold().to_vec();
        summary
    }

    /// The technique of a struck note, judged with the matcher's result; `played`: the detected
    /// technique, `None` for a missed note.
    fn judge_struck(
        &mut self,
        result: &NoteResult,
        played: Option<Option<kontra_model::Technique>>,
    ) {
        let Some(note) = result.note else {
            return;
        };
        let played = if matches!(result.outcome, Outcome::Missed) {
            None
        } else {
            played
        };
        if let Some(t) = self.techniques.judge_struck(result.pass, note, played) {
            self.events.push(SessionEvent::Technique(t));
        }
    }

    fn range(&self) -> (f64, f64) {
        self.transport
            .loop_range()
            .map_or((0.0, f64::INFINITY), |l| (l.start_s, l.end_s))
    }

    fn first_slot_from(&self, position_s: f64) -> usize {
        self.matcher
            .expected()
            .partition_point(|e| e.position_s < position_s)
    }

    fn slot_in_range(&self, slot: usize) -> bool {
        let (start, end) = self.range();
        self.matcher
            .expected()
            .get(slot)
            .is_some_and(|e| e.position_s >= start && e.position_s < end)
    }

    fn hold_position(&self) -> Option<f64> {
        if !matches!(self.mode, Mode::Wait) {
            return None;
        }
        self.matcher
            .expected()
            .get(self.cursor_slot)
            .map(|e| e.position_s)
    }

    /// Records `result`; for a chord the same result is recorded for each further chord tone.
    fn record(&mut self, result: NoteResult) {
        let chord = result.note.and_then(|index| {
            let expected = self.matcher.expected();
            let from = expected.partition_point(|e| e.position_s < result.position_s);
            expected[from..]
                .iter()
                .take_while(|e| e.position_s <= result.position_s)
                .find(|e| e.index == index)
                .map(|e| e.chord)
        });
        self.results.push(result);
        self.events.push(SessionEvent::Result(result));
        for (index, _) in chord.iter().flat_map(|c| c.iter()) {
            let tone = NoteResult {
                note: Some(index),
                ..result
            };
            self.results.push(tone);
            self.events.push(SessionEvent::Result(tone));
        }
    }

    /// Wait mode: compare with the note we wait for, whenever it is played.
    fn wait_note(&mut self, real_s: f64, note: &DetectedNote) {
        let midi = note.midi;
        let Some(target) = self.matcher.expected().get(self.cursor_slot).copied() else {
            return;
        };
        // A hammer-on or pull-off before the note we wait for is that legato note, not a wrong
        // attempt at the next one: judged between the last struck note and this one.
        let pass = self.transport.pass();
        let from = self
            .cursor_slot
            .checked_sub(1)
            .and_then(|slot| self.matcher.expected().get(slot))
            .map_or(0.0, |e| e.position_s);
        let middle = (from + target.position_s) / 2.0;
        let half = (target.position_s - from) / 2.0;
        if let Some(t) = self
            .techniques
            .judge_legato(pass, middle, midi, note.technique, half)
        {
            self.events.push(SessionEvent::Technique(t));
            return;
        }
        // A ghost note played as one is right whatever its pitch (a muted string has none).
        let muted = target.ghost && note.technique == Some(kontra_model::Technique::Ghost);
        let cents = target.cents_from(midi);
        let outcome = if muted {
            Outcome::Hit
        } else {
            crate::matcher::pitch_outcome(cents, self.settings.pitch_cents, midi)
        };
        // An off-pitch note is still the right note: the wait is over.
        let in_tune = !matches!(outcome, Outcome::WrongPitch { .. });
        self.record(NoteResult {
            pass,
            note: Some(target.index),
            bar: target.bar,
            outcome,
            offset_ms: None,
            cents: (!muted).then_some(cents),
            position_s: target.position_s,
        });
        if !in_tune {
            return;
        }
        if let Some(t) = self
            .techniques
            .judge_struck(pass, target.index, Some(note.technique))
        {
            self.events.push(SessionEvent::Technique(t));
        }
        self.matcher.resolve(pass, self.cursor_slot);
        self.cursor_slot += 1;
        if self.holding {
            self.holding = false;
            self.transport.release(real_s);
            self.events.push(SessionEvent::Released);
        }
    }

    /// Declares notes missed once the clock (minus the detection grace) has passed their window,
    /// and closes loop passes.
    fn expire(&mut self, real_cutoff: f64) {
        let (position, pass) = self.transport.position_at(real_cutoff);
        let window = self.settings.strictness.window_ms() / 1000.0 * self.transport.speed();
        let range = self.range();
        for t in self.techniques.expire(pass, position, window, range) {
            self.events.push(SessionEvent::Technique(t));
        }
        loop {
            if self.cursor_pass > pass {
                return;
            }
            if !self.slot_in_range(self.cursor_slot) {
                if self.cursor_pass == pass {
                    return;
                }
                self.complete_pass();
                continue;
            }
            let Some(e) = self.matcher.expected().get(self.cursor_slot).copied() else {
                return;
            };
            if self.cursor_pass == pass && e.position_s + window >= position {
                return;
            }
            if !self.matcher.is_resolved(self.cursor_pass, self.cursor_slot) {
                self.matcher.resolve(self.cursor_pass, self.cursor_slot);
                let missed = NoteResult {
                    pass: self.cursor_pass,
                    note: Some(e.index),
                    bar: e.bar,
                    outcome: Outcome::Missed,
                    offset_ms: None,
                    cents: None,
                    position_s: e.position_s,
                };
                self.record(missed);
                self.judge_struck(&missed, None);
            }
            self.cursor_slot += 1;
        }
    }

    fn complete_pass(&mut self) {
        let pass = self.cursor_pass;
        let in_pass = || self.results.iter().filter(|r| r.pass == pass);
        let clean = in_pass().all(NoteResult::is_hit);
        let judged = in_pass().count();
        let hit_rate = if judged == 0 {
            1.0
        } else {
            in_pass().filter(|r| r.is_hit()).count() as f64 / judged as f64
        };
        self.events.push(SessionEvent::PassComplete {
            pass,
            clean,
            hit_rate,
        });
        self.cursor_pass += 1;
        self.cursor_slot = self.first_slot_from(self.range().0);
        let Mode::Workshop(workshop) = self.mode else {
            return;
        };
        let (low, high) = workshop.band;
        self.band_streak = if hit_rate > high {
            self.band_streak.max(0) + 1
        } else if hit_rate < low {
            self.band_streak.min(0) - 1
        } else {
            0
        };
        let needed = workshop.passes_needed.max(1) as i32;
        let speed = self.transport.speed();
        let next = if self.band_streak >= needed {
            (speed + workshop.step).min(workshop.target_speed)
        } else if self.band_streak <= -needed {
            (speed - workshop.step).max(workshop.min_speed)
        } else {
            return;
        };
        self.band_streak = 0;
        if (next - speed).abs() < 1e-9 {
            return;
        }
        // Applies from now; the pass that just started gets the new speed almost from its start.
        self.transport.set_speed(self.now_s, next);
        self.events.push(SessionEvent::SpeedChanged(next));
    }

    fn check_finished(&mut self) {
        if self.finished || self.transport.is_running() || self.transport.loop_range().is_some() {
            return;
        }
        let (position, _) = self.transport.position_at(f64::INFINITY);
        if position < self.transport.end_s() {
            return;
        }
        let done =
            self.cursor_slot >= self.matcher.expected().len() || matches!(self.mode, Mode::Wait);
        if done {
            self.finished = true;
            self.events.push(SessionEvent::Finished);
        }
    }
}
