//! A practice session: one arrangement, one mode, scored live.

use crate::expected::expected_notes;
use crate::matcher::{Matcher, NoteResult, Outcome};
use crate::settings::ScoreSettings;
use crate::summary::Summary;
use crate::transport::{LoopRange, Transport};
use kontra_model::{Arrangement, DetectedNote};

/// Real seconds to wait before declaring a note missed: detected notes arrive up to ~150 ms
/// after their onset (concept 6.3), plus margin.
const DETECTION_GRACE_S: f64 = 0.25;

/// Riff-Werkstatt: speed up after enough clean passes (LH-MOD-04).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Workshop {
    pub target_speed: f64,
    pub step: f64,
    pub clean_needed: u32,
}

impl Default for Workshop {
    fn default() -> Self {
        Workshop {
            target_speed: 1.0,
            step: 0.05,
            clean_needed: 3,
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
    /// Wait mode: holding before this note (index into `Arrangement::notes`).
    Holding {
        note: usize,
    },
    Released,
    PassComplete {
        pass: u32,
        clean: bool,
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
    results: Vec<NoteResult>,
    events: Vec<SessionEvent>,
    cursor_pass: u32,
    cursor_slot: usize,
    clean_streak: u32,
    holding: bool,
    finished: bool,
    /// Real time of the last `update`.
    now_s: f64,
}

impl Session {
    pub fn new(arrangement: Arrangement, mode: Mode, settings: ScoreSettings) -> Session {
        let matcher = Matcher::new(expected_notes(&arrangement));
        let transport = Transport::new(arrangement.duration_s());
        Session {
            arrangement,
            mode,
            settings,
            transport,
            matcher,
            results: Vec::new(),
            events: Vec::new(),
            cursor_pass: 0,
            cursor_slot: 0,
            clean_streak: 0,
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
        self.clean_streak = 0;
    }

    /// Jumps to `position_s`; counting restarts at the first note from there.
    pub fn seek(&mut self, real_s: f64, position_s: f64) {
        self.transport.seek(real_s, position_s);
        let (position, pass) = self.transport.position_at(real_s);
        self.cursor_pass = pass;
        self.cursor_slot = self.first_slot_from(position);
        self.holding = false;
    }

    /// Scores a detected note. Its onset is on the same real clock as the transport.
    pub fn on_note(&mut self, note: &DetectedNote) {
        let real_s = note.onset_s - self.settings.latency_ms / 1000.0;
        if matches!(self.mode, Mode::Wait) {
            self.wait_note(real_s, note.midi);
            return;
        }
        let (position, pass) = self.transport.position_at(real_s);
        let range = self.range();
        let result = self.matcher.score(
            pass,
            position,
            note.midi,
            self.transport.speed(),
            &self.settings,
            range,
        );
        self.record(result);
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
        Summary::from_results(&self.results, &self.arrangement)
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
    fn wait_note(&mut self, real_s: f64, midi: f64) {
        let Some(target) = self.matcher.expected().get(self.cursor_slot).copied() else {
            return;
        };
        let cents = target.cents_from(midi);
        let in_tune = cents.abs() <= self.settings.pitch_cents;
        let outcome = if in_tune {
            Outcome::Hit
        } else {
            Outcome::WrongPitch { played_midi: midi }
        };
        let pass = self.transport.pass();
        self.record(NoteResult {
            pass,
            note: Some(target.index),
            bar: target.bar,
            outcome,
            offset_ms: None,
            cents: Some(cents),
            position_s: target.position_s,
        });
        if !in_tune {
            return;
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
                self.record(NoteResult {
                    pass: self.cursor_pass,
                    note: Some(e.index),
                    bar: e.bar,
                    outcome: Outcome::Missed,
                    offset_ms: None,
                    cents: None,
                    position_s: e.position_s,
                });
            }
            self.cursor_slot += 1;
        }
    }

    fn complete_pass(&mut self) {
        let pass = self.cursor_pass;
        let clean = self
            .results
            .iter()
            .filter(|r| r.pass == pass)
            .all(NoteResult::is_hit);
        self.events.push(SessionEvent::PassComplete { pass, clean });
        self.cursor_pass += 1;
        self.cursor_slot = self.first_slot_from(self.range().0);
        let Mode::Workshop(workshop) = self.mode else {
            return;
        };
        self.clean_streak = if clean { self.clean_streak + 1 } else { 0 };
        let speed = self.transport.speed();
        if self.clean_streak < workshop.clean_needed || speed >= workshop.target_speed {
            return;
        }
        self.clean_streak = 0;
        let next = (speed + workshop.step).min(workshop.target_speed);
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
