//! A practice session: one arrangement, one mode, scored live.

use crate::expected::expected_notes;
use crate::matcher::{Claim, Matcher, NoteResult, Outcome};
use crate::settings::ScoreSettings;
use crate::summary::Summary;
use crate::technique::{TechniqueJudge, TechniqueResult};
use crate::transport::{LoopRange, Transport};
use kontra_model::{
    Arrangement, DetectedNote, NoteRetraction, PitchRevision, Technique, TechniqueRevision,
};

mod double_stop;

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
const MS_PER_S: f64 = 1000.0;
/// Notes this soon after one taken back are scored again with it gone: the tracker takes a note
/// back before the full window would have told it (about 90 ms), at the pluck's rise check.
const RESCORE_S: f64 = 0.3;

/// How a scored note stands: open to a pitch or double stop told later, or judged by both
/// tones of a double stop (R8), final and possibly different per tone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Judgement {
    Provisional,
    BothTones,
}

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
    /// A note taken back (R1) is open again: its verdict goes until the next one.
    Reopened {
        note: usize,
    },
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
    /// Notes scored lately: (detected onset, index into `results`, technique, detected pitch,
    /// judgement), so a pitch or double stop told later can judge them again
    /// (`on_pitch_revision`, `on_double_stop`) and a retraction take them off.
    scored: Vec<(f64, usize, Option<Technique>, f64, Judgement)>,
    /// Notes heard lately, so those scored after a note taken back are scored again
    /// (`on_retraction`).
    heard: Vec<DetectedNote>,
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
    /// Real time the input stalled at, until it resumes (LH-EIN-15).
    stalled_s: Option<f64>,
    /// Stretches the input did not deliver, as (pass, position) from and to: their notes are
    /// not judged, missing data is no mistake.
    unheard: Vec<((u32, f64), (u32, f64))>,
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
            heard: Vec::new(),
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
            stalled_s: None,
            unheard: Vec::new(),
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
        if self.heard.len() >= STRUCK_KEPT {
            self.heard.remove(0);
        }
        self.heard.push(*note);
        let real_s = self.real_s(note.onset_s);
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
        // Without a pitch of its own (none, or the background's) an attack judges nothing on its
        // own: where no note is due it is a click on hum or a scrape, not an extra note
        // (LH-NF-16). Near a due note it is scored as before.
        if !note.stands_out() && !self.matcher.has_candidate(pass, position, window, range) {
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
        for entry in self.scored.iter_mut().filter(|(_, i, ..)| *i == at) {
            entry.1 = self.results.len();
        }
        self.results.push(extra);
        self.events.push(SessionEvent::Result(extra));
        if self.scored.len() >= STRUCK_KEPT {
            self.scored.remove(0);
        }
        self.scored.push((
            note.onset_s,
            at,
            note.technique,
            note.midi,
            Judgement::Provisional,
        ));

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
        // A stalled input holds judging where it stopped until it resumes (LH-EIN-15).
        if self.stalled_s.is_some() {
            return;
        }
        let real_s = self.real_s(at_s);
        self.heard_s = Some(self.heard_s.map_or(real_s, |h| h.max(real_s)));
    }

    /// The input stopped delivering at `at_s` (engine clock, LH-EIN-15): nothing after it is
    /// judged until it resumes, however long that takes.
    pub fn on_stalled(&mut self, at_s: f64) {
        self.stalled_s = Some(self.real_s(at_s));
    }

    /// The input delivers again from `at_s`: the notes since the stall are not judged.
    pub fn on_resumed(&mut self, at_s: f64) {
        let Some(from_s) = self.stalled_s.take() else {
            return;
        };
        let to_s = self.real_s(at_s);
        let at = |real_s| {
            let (position, pass) = self.transport.position_at(real_s);
            (pass, position)
        };
        self.unheard.push((at(from_s), at(to_s)));
    }

    /// True if the note at `position_s` in `pass` lay in a stretch the input did not deliver.
    fn unheard(&self, pass: u32, position_s: f64) -> bool {
        let at = (pass, position_s);
        self.unheard
            .iter()
            .any(|(from, to)| *from <= at && at < *to)
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
        let Some(entry) = self
            .scored
            .iter_mut()
            .rev()
            .find(|(onset, ..)| (onset - revision.onset_s).abs() < REVISION_MATCH_S)
        else {
            return;
        };
        if entry.4 == Judgement::BothTones {
            return;
        }
        entry.3 = revision.midi;
        let (_, at, technique, ..) = *entry;
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

    /// A note taken back (R1): told early, it proved to be no note of its own, e.g. the finger
    /// tapping the string before the pluck. Its result goes (with its chord tones), and the
    /// note it took is open again: for the pluck that follows, or missed. A pluck already
    /// scored behind it is scored again.
    ///
    /// ```text
    /// tap told early ─► hit (60 ms early) ─► taken back: no result, D2 open
    /// pluck          ─────────────────────────────────► hit (on time)
    /// ```
    pub fn on_retraction(&mut self, retraction: &NoteRetraction) {
        let onset_s = retraction.onset_s;

        // Notes scored after it may have been judged behind it (the engine can tell the pluck
        // before the tap's retraction): taken off too, and scored again once it is gone.
        let later: Vec<DetectedNote> = self
            .heard
            .iter()
            .filter(|n| n.onset_s > onset_s && n.onset_s - onset_s <= RESCORE_S)
            .filter(|n| self.scored_at(n.onset_s).is_some())
            .copied()
            .collect();
        if !self.unscore(onset_s) {
            return;
        }
        self.heard.retain(|n| !same_onset(n.onset_s, onset_s));
        for note in &later {
            self.unscore(note.onset_s);
        }
        self.heard
            .retain(|n| !later.iter().any(|l| same_onset(l.onset_s, n.onset_s)));
        for note in &later {
            self.on_note(note);
        }
    }

    /// Where the note detected at `onset_s` is in `scored`.
    fn scored_at(&self, onset_s: f64) -> Option<usize> {
        self.scored
            .iter()
            .rposition(|(onset, ..)| same_onset(*onset, onset_s))
    }

    /// Takes the result of the note detected at `onset_s` off (with its chord tones); the notes
    /// it took are open again. Returns whether it had one.
    fn unscore(&mut self, onset_s: f64) -> bool {
        let Some(entry) = self.scored_at(onset_s) else {
            return false;
        };
        let (_, at, .., judgement) = self.scored.remove(entry);
        let taken = self.results[at];

        // The result and the chord tones recorded with it (a double stop's tones may differ).
        let tone = |r: &NoteResult| {
            r.pass == taken.pass
                && r.position_s == taken.position_s
                && (r.outcome == taken.outcome || judgement == Judgement::BothTones)
                && r.note.is_some()
                && taken.note.is_some()
        };
        let end = (at + 1..self.results.len())
            .find(|&i| !tone(&self.results[i]))
            .unwrap_or(self.results.len());
        let removed: Vec<NoteResult> = self.results.drain(at..end).collect();
        for entry in self.scored.iter_mut().filter(|(_, i, ..)| *i >= end) {
            entry.1 -= end - at;
        }
        self.struck
            .retain(|(onset, _, _)| !same_onset(*onset, onset_s));

        // Each note it took is open again; Wait mode waits for the first of them in this pass.
        let mut first: Option<usize> = None;
        for r in removed {
            let current = r.pass == self.transport.pass();
            if let Some(note) = r.note.filter(|_| current) {
                self.events.push(SessionEvent::Reopened { note });
            }
            let Some(slot) = r.note.and_then(|note| self.slot_of(note, r.position_s)) else {
                continue;
            };
            self.reopen(r.pass, slot);
            if r.pass == self.transport.pass() {
                first = Some(first.map_or(slot, |f| f.min(slot)));
            }
        }
        if let Some(slot) = first.filter(|_| matches!(self.mode, Mode::Wait)) {
            self.cursor_slot = self.cursor_slot.min(slot);
        }
        true
    }

    /// The note at `slot` of `pass` lost its detection: open for the next one, or missed when
    /// the clock already passed it (not in Wait mode, which waits for it again).
    fn reopen(&mut self, pass: u32, slot: usize) {
        self.matcher.release(pass, slot);
        if matches!(self.mode, Mode::Wait) {
            return;
        }
        let passed = (self.cursor_pass, self.cursor_slot) > (pass, slot);
        if !passed {
            return;
        }
        let e = self.matcher.expected()[slot];
        self.matcher.resolve(pass, slot);
        let missed = NoteResult {
            pass,
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
        self.scored.push((
            note.onset_s,
            self.results.len(),
            note.technique,
            note.midi,
            Judgement::Provisional,
        ));
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
            // Up to where the analysis has heard, if it runs behind (a loaded machine); a
            // stalled input holds it there.
            let heard = match (self.heard_s, self.stalled_s) {
                (Some(h), Some(_)) => h,
                (Some(h), None) => h.max(real_s - MAX_ANALYSIS_LAG_S),
                (None, _) => real_s,
            };
            self.expire(real_s.min(heard) - DETECTION_GRACE_S);
        }
        self.check_finished();
        std::mem::take(&mut self.events)
    }

    /// Real (transport) time of a time on the input's clock: the measured latency taken off
    /// (LH-EIN-05). `input_s` is the way back.
    fn real_s(&self, input_s: f64) -> f64 {
        input_s - self.settings.latency_ms / MS_PER_S
    }

    /// When a strike due at real time `real_s` reaches the input (R2): plus the latency.
    pub(crate) fn input_s(&self, real_s: f64) -> f64 {
        real_s + self.settings.latency_ms / MS_PER_S
    }

    /// The notes the player is to strike, one entry per attack.
    pub(crate) fn expected(&self) -> &[crate::expected::ExpectedNote] {
        self.matcher.expected()
    }

    /// The next note the player has to strike in the current pass.
    pub fn next_expected(&self) -> Option<crate::expected::ExpectedNote> {
        self.matcher.expected().get(self.cursor_slot).copied()
    }

    pub fn summary(&self) -> Summary {
        let mut summary = Summary::from_results(&self.results, &self.arrangement);
        summary.techniques = self.techniques.tally();
        summary.untold_techniques = self.techniques.untold().to_vec();
        summary.untold_double_stops = self.untold_double_stops();
        summary.outside_offset_ms =
            crate::summary::outside_offset_ms(&self.results, self.transport.speed());
        summary.outside_notes =
            crate::summary::outside_notes(&self.results, self.transport.speed());
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
        // Notes already played after a note taken back and played again (`on_retraction`).
        while self.matcher.is_resolved(pass, self.cursor_slot) {
            self.cursor_slot += 1;
        }
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
            // Not delivered by the input: resolved without a result, it is no mistake.
            let unheard = self.unheard(self.cursor_pass, e.position_s);
            if !self.matcher.is_resolved(self.cursor_pass, self.cursor_slot) && unheard {
                self.matcher.resolve(self.cursor_pass, self.cursor_slot);
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

/// Whether two detections are the same (the engine's clock, exact).
fn same_onset(a: f64, b: f64) -> bool {
    (a - b).abs() < REVISION_MATCH_S
}

/// Arrangement position a judged detection was played at.
fn played_at(result: &NoteResult, speed: f64) -> f64 {
    result.position_s + result.offset_ms.unwrap_or(0.0) / 1000.0 * speed
}
