//! A practice session: one arrangement, one mode, scored live.

use crate::expected::expected_notes;
use crate::matcher::{Claim, Matcher, NoteResult, Outcome};
use crate::settings::ScoreSettings;
use crate::summary::Summary;
use crate::technique::{TechniqueJudge, TechniqueResult};
use crate::transport::{LoopRange, Transport};
use kontra_model::{Arrangement, DetectedNote, PitchRevision, Technique, TechniqueRevision};

/// Real seconds to wait before declaring a note missed: detected notes arrive up to ~150 ms
/// after their onset (concept 6.3), plus margin.
const DETECTION_GRACE_S: f64 = 0.25;
/// A loaded machine may analyse behind the clock: notes wait for the analysis this long at
/// most (a stalled input must not hold the run forever).
const MAX_ANALYSIS_LAG_S: f64 = 2.0;

/// Struck notes kept for techniques told later (they come within about 110 ms of the note).
const STRUCK_KEPT: usize = 16;
/// A revision belongs to the note detected at the same onset (the engine's clock, exact).
const REVISION_MATCH_S: f64 = 1e-6;

/// Riff-Werkstatt (LH-MOD-04): keeps the hit rate in a target band (decision round 5, after
/// planung/DIDAKTIK.md idea I7). Above the band for `passes_needed` passes in a row → one step faster
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
    /// Blind probe (DIDAKTIK I1): scores like `Song`, but ends after one pass of the loop, so a
    /// practised section is played once without help.
    Probe,
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
    /// Struck notes judged lately: (detected onset, pass, note), so a technique told later can
    /// be judged again (`on_revision`).
    struck: Vec<(f64, u32, usize)>,
    /// Notes scored lately: (detected onset, index into `results`, technique), so a pitch told
    /// later can judge them again (`on_pitch_revision`).
    scored: Vec<(f64, usize, Option<Technique>)>,
    results: Vec<NoteResult>,
    events: Vec<SessionEvent>,
    cursor_pass: u32,
    cursor_slot: usize,
    /// Passes in a row above (positive) or below (negative) the workshop band.
    band_streak: i32,
    holding: bool,
    finished: bool,
    /// Speed of the last completed pass; the workshop may already have changed the transport's.
    played_speed: Option<f64>,
    /// Real time of the last `update`.
    now_s: f64,
    /// Real time up to which the analysis has heard; `None`: as far as the clock (MIDI, ended).
    heard_s: Option<f64>,
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
            struck: Vec::new(),
            scored: Vec::new(),
            results: Vec::new(),
            events: Vec::new(),
            cursor_pass: 0,
            cursor_slot: 0,
            band_streak: 0,
            holding: false,
            finished: false,
            played_speed: None,
            now_s: 0.0,
            heard_s: None,
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

    /// Swaps in the same piece transposed (LH-WIE-02 while playing): with the same attacks at
    /// the same times, clock, loop and results go on and only the pitches change. Returns false
    /// (nothing changed) when the notes differ in more than pitch.
    pub fn retarget(&mut self, arrangement: Arrangement) -> bool {
        let same_times = arrangement.notes.len() == self.arrangement.notes.len()
            && arrangement
                .notes
                .iter()
                .zip(&self.arrangement.notes)
                .all(|(a, b)| {
                    a.start_ticks == b.start_ticks && a.duration_ticks == b.duration_ticks
                })
            && arrangement.tempo == self.arrangement.tempo;
        if !same_times {
            return false;
        }
        if !self.techniques.retarget(&arrangement, self.settings.source) {
            return false;
        }
        self.matcher.retarget(expected_notes(&arrangement));
        self.arrangement = arrangement;
        true
    }

    /// Technique results so far (LH-BEW-06), in the order they were judged.
    pub fn technique_results(&self) -> &[TechniqueResult] {
        self.techniques.results()
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// The speed the player last played a whole pass at; before one, the transport's.
    pub fn played_speed(&self) -> f64 {
        self.played_speed.unwrap_or(self.transport.speed())
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
        // After a probe's one pass the run is over; a note then belongs to no pass.
        if self.finished && matches!(self.mode, Mode::Probe) {
            return;
        }
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

            // The mirror case: just after the wrap, the last pass's final note played late. Both
            // lie in the window; the pitch tells them apart (PR #99 review).
            let unwrapped = end_s + (position - start_s);
            if pass > 0
                && position - start_s <= window
                && !self
                    .matcher
                    .has_pitch(pass, position, window, range, note.midi)
                && self
                    .matcher
                    .has_pitch(pass - 1, unwrapped, window, range, note.midi)
            {
                position = unwrapped;
                pass -= 1;
            }
        }
        // A probe has one pass: playing on past the section is not a second one.
        if matches!(self.mode, Mode::Probe) && pass > 0 {
            return;
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
        let (result, claim) = self.matcher.score(
            pass,
            position,
            note.midi,
            note.technique,
            speed,
            &self.settings,
            range,
        );
        if claim == Claim::TakenOver {
            self.take_over(note, result);
            return;
        }
        self.remember_scored(note);
        self.record(result);
        self.judge_struck(&result, Some(note.technique));
        if let Some(index) = result.note {
            self.remember_struck(note.onset_s, result.pass, index);
        }
    }

    /// `result` of `note` replaces the worse one an earlier detection got for the same expected
    /// note (and its chord tones); the earlier detection becomes an extra note (H5).
    fn take_over(&mut self, note: &DetectedNote, result: NoteResult) {
        let taken = |r: &NoteResult| {
            r.pass == result.pass
                && r.position_s == result.position_s
                && r.note.is_some()
                && !matches!(r.outcome, Outcome::Missed)
        };
        let Some(at) = self
            .results
            .iter()
            .rposition(|r| taken(r) && r.note == result.note)
        else {
            return;
        };
        let earlier = self.results[at];

        // The note and each chord tone recorded with it.
        let tones: Vec<usize> = (at..self.results.len())
            .take_while(|&i| taken(&self.results[i]) && self.results[i].outcome == earlier.outcome)
            .collect();
        for i in tones {
            let tone = NoteResult {
                note: self.results[i].note,
                ..result
            };
            self.results[i] = tone;
            self.events.push(SessionEvent::Result(tone));
        }

        // The earlier detection is an extra note now; its late pitch judges nothing.
        let extra = NoteResult {
            note: None,
            bar: None,
            outcome: Outcome::Extra {
                played_midi: self.played_midi(&earlier),
            },
            offset_ms: None,
            cents: None,
            position_s: played_at(&earlier, self.transport.speed()),
            ..earlier
        };
        for entry in self.scored.iter_mut().filter(|(_, i, _)| *i == at) {
            entry.1 = self.results.len();
        }
        self.results.push(extra);
        self.events.push(SessionEvent::Result(extra));
        if self.scored.len() >= STRUCK_KEPT {
            self.scored.remove(0);
        }
        self.scored.push((note.onset_s, at, note.technique));

        // The technique is judged on the attack, not the artifact.
        self.struck
            .retain(|(_, pass, n)| !(*pass == result.pass && Some(*n) == result.note));
        self.judge_struck(&result, Some(note.technique));
        if let Some(index) = result.note {
            self.remember_struck(note.onset_s, result.pass, index);
        }
    }

    /// The pitch a judged detection was played at: kept by a wrong note, else the expected one
    /// moved by the cents.
    fn played_midi(&self, result: &NoteResult) -> f64 {
        if let Outcome::WrongPitch { played_midi } = result.outcome {
            return played_midi;
        }
        let expected = result
            .note
            .and_then(|note| self.slot_of(note, result.position_s))
            .map_or(0.0, |slot| f64::from(self.matcher.expected()[slot].midi));
        expected + result.cents.unwrap_or(0.0) / 100.0
    }

    /// The analysis has heard the input up to `at_s` (engine clock); nothing after it is
    /// missed yet.
    pub fn on_heard(&mut self, at_s: f64) {
        let real_s = at_s - self.settings.latency_ms / 1000.0;
        self.heard_s = Some(self.heard_s.map_or(real_s, |h| h.max(real_s)));
    }

    /// The input ended (a file): from now on only the clock counts.
    pub fn on_input_ended(&mut self) {
        self.heard_s = None;
    }

    /// A technique told after its note (LH-BEW-06): the note it belongs to is judged again.
    pub fn on_revision(&mut self, revision: &TechniqueRevision) {
        let Some(&(_, pass, note)) = self
            .struck
            .iter()
            .rev()
            .find(|(onset, _, _)| (onset - revision.onset_s).abs() < REVISION_MATCH_S)
        else {
            return;
        };
        if let Some(t) = self
            .techniques
            .judge_struck(pass, note, Some(Some(revision.technique)))
        {
            self.events.push(SessionEvent::Technique(t));
        }
        if revision.technique == kontra_model::Technique::Muted {
            self.unmute(pass, note);
        }
    }

    /// A ghost or dead note judged by a pitch it does not have (its muted verdict came later):
    /// it hits after all, and in Wait mode the wait is over.
    fn unmute(&mut self, pass: u32, note: usize) {
        let Some(slot) = self
            .matcher
            .expected()
            .iter()
            .position(|e| e.index == note && e.ghost)
        else {
            return;
        };
        // The earlier result is replaced, not followed: a pass counts every result it holds.
        let Some(at) = self
            .results
            .iter()
            .rposition(|r| r.pass == pass && r.note == Some(note))
        else {
            return;
        };
        let last = self.results[at];
        if !matches!(last.outcome, Outcome::WrongPitch { .. } | Outcome::OffPitch) {
            return;
        }
        let hit = NoteResult {
            outcome: Outcome::Hit,
            cents: None,
            ..last
        };
        self.results[at] = hit;
        self.events.push(SessionEvent::Result(hit));
        self.matcher.rejudge(pass, slot, hit.outcome);
        self.end_wait(pass, slot);
    }

    /// A pitch told after its note, once the tone settled (decision round 11): the note is
    /// judged again with it, e.g. a flat start judged a wrong note becomes a hit.
    pub fn on_pitch_revision(&mut self, revision: &PitchRevision) {
        let Some(&(_, at, technique)) = self
            .scored
            .iter()
            .rev()
            .find(|(onset, _, _)| (onset - revision.onset_s).abs() < REVISION_MATCH_S)
        else {
            return;
        };
        let last = self.results[at];

        // Only a note judged by its pitch (a dead note played as one has none).
        let (Some(note), Some(_)) = (last.note, last.cents) else {
            return;
        };
        let Some(slot) = self.slot_of(note, last.position_s) else {
            return;
        };
        let cents = self.matcher.expected()[slot].cents_from(revision.midi);
        let outcome =
            crate::matcher::pitch_outcome(cents, self.settings.pitch_cents, revision.midi);
        let revised = NoteResult {
            outcome,
            cents: Some(cents),
            ..last
        };

        // The earlier result is replaced, not followed: a pass counts every result it holds.
        self.results[at] = revised;
        self.events.push(SessionEvent::Result(revised));
        self.matcher.rejudge(last.pass, slot, outcome);

        // Wait mode: a wrong attempt that was the right note after all ends the wait.
        let wrong = |o: Outcome| matches!(o, Outcome::WrongPitch { .. });
        if !wrong(last.outcome) || wrong(outcome) || !matches!(self.mode, Mode::Wait) {
            return;
        }
        if self.cursor_slot != slot {
            return;
        }
        if let Some(t) = self
            .techniques
            .judge_struck(last.pass, note, Some(technique))
        {
            self.events.push(SessionEvent::Technique(t));
        }
        self.end_wait(last.pass, slot);
    }

    /// Wait mode: the note at `slot` was played after all; the wait is over.
    fn end_wait(&mut self, pass: u32, slot: usize) {
        if !matches!(self.mode, Mode::Wait) || self.cursor_slot != slot {
            return;
        }
        self.matcher.resolve(pass, slot);
        self.cursor_slot += 1;
        if self.holding {
            self.holding = false;
            self.transport.release(self.now_s);
            self.events.push(SessionEvent::Released);
        }
    }

    /// Keeps the last scored notes for `on_pitch_revision`; call right before `record`.
    fn remember_scored(&mut self, note: &DetectedNote) {
        if self.scored.len() >= STRUCK_KEPT {
            self.scored.remove(0);
        }
        self.scored
            .push((note.onset_s, self.results.len(), note.technique));
    }

    /// The slot in the matcher's expected notes of `note` at `position_s`.
    fn slot_of(&self, note: usize, position_s: f64) -> Option<usize> {
        let expected = self.matcher.expected();
        let from = expected.partition_point(|e| e.position_s < position_s);
        expected[from..]
            .iter()
            .take_while(|e| e.position_s <= position_s)
            .position(|e| e.index == note)
            .map(|i| from + i)
    }

    /// Keeps the last struck notes for `on_revision`.
    fn remember_struck(&mut self, onset_s: f64, pass: u32, note: usize) {
        if self.struck.len() >= STRUCK_KEPT {
            self.struck.remove(0);
        }
        self.struck.push((onset_s, pass, note));
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
        // A finished probe judges nothing more: its clock stopped inside the next pass.
        if !matches!(self.mode, Mode::Wait) && !self.finished {
            // Up to where the analysis has heard, if it runs behind (a loaded machine).
            let heard = self
                .heard_s
                .map_or(real_s, |h| h.max(real_s - MAX_ANALYSIS_LAG_S));
            self.expire(real_s.min(heard) - DETECTION_GRACE_S);
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
        // A ghost note played as one is right whatever its pitch (a muted string has none); so is
        // any note judged by time only.
        let muted = target.any_pitch
            || (target.ghost && note.technique == Some(kontra_model::Technique::Muted));
        let cents = target.cents_from(midi);
        let outcome = if muted {
            Outcome::Hit
        } else {
            crate::matcher::pitch_outcome(cents, self.settings.pitch_cents, midi)
        };
        // An off-pitch note is still the right note: the wait is over.
        let in_tune = !matches!(outcome, Outcome::WrongPitch { .. });
        self.remember_scored(note);
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
            // A dead note heard with a pitch may still be told muted later (`on_revision`).
            if target.ghost {
                self.remember_struck(note.onset_s, pass, target.index);
            }
            return;
        }
        if let Some(t) = self
            .techniques
            .judge_struck(pass, target.index, Some(note.technique))
        {
            self.events.push(SessionEvent::Technique(t));
        }
        self.remember_struck(note.onset_s, pass, target.index);
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
                if self.finished {
                    return;
                }
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
        self.played_speed = Some(self.transport.speed());
        // The notes of the score, judged as the summary does: extra notes (a real bass makes
        // about one per note, H5) do not count, the last result per note does.
        let of_pass: Vec<NoteResult> = self
            .results
            .iter()
            .filter(|r| r.pass == pass)
            .copied()
            .collect();
        let judged = crate::summary::final_per_note(&of_pass);
        let in_pass = || judged.iter();
        let clean = in_pass().all(NoteResult::is_hit);
        let count = in_pass().count();
        let hit_rate = if count == 0 {
            1.0
        } else {
            in_pass().filter(|r| r.is_hit()).count() as f64 / count as f64
        };
        self.events.push(SessionEvent::PassComplete {
            pass,
            clean,
            hit_rate,
        });
        self.cursor_pass += 1;
        self.cursor_slot = self.first_slot_from(self.range().0);
        if matches!(self.mode, Mode::Probe) {
            self.transport.pause(self.now_s);
            self.finished = true;
            self.events.push(SessionEvent::Finished);
            return;
        }
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

/// Arrangement position a judged detection was played at.
fn played_at(result: &NoteResult, speed: f64) -> f64 {
    result.position_s + result.offset_ms.unwrap_or(0.0) / 1000.0 * speed
}
