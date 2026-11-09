//! Note ends judged (LH-BEW-10): a note the tab wants stopped, written staccato or followed by
//! a rest, is judged by where the tracker heard it end.
//!
//! ```text
//! NoteEnd (onset, end, how) ─► the struck note at that onset ─► wants an end? ─► written end
//!   Muted or Decayed by the written end + window ─► InTime
//!   Muted later                                 ─► Late (ms)
//!   Decayed later, or the next note came first  ─► RangOn
//! ```
//!
//! Only active muting is reliable (a fast fall of 10 dB): only cable inputs are judged; a
//! microphone hears the room ring on, MIDI sends no ends yet. The summary says so.

use super::{same_onset, Session, MAX_ANALYSIS_LAG_S, MS_PER_S};
use crate::summary::{EndHeard, MutingTally, NoteEndMark, Summary};
use kontra_model::{Ending, InputSource, Judging, NoteEnd, Technique, TICKS_PER_QUARTER};

/// A rest at least this long after a note asks for it to be stopped (an eighth).
const REST_TICKS: u64 = TICKS_PER_QUARTER / 2;
/// The end of a muted note is told this long after it at most: the fall (60 ms), the check
/// behind the audio (80 ms), the hold (100 ms), plus margin. The run ends after it.
const END_TOLD_S: f64 = 0.4;

/// The ends judged (detected onset, pass, note, heard), and whether the input tells ends at
/// all: a run scored from notes alone gets none and judges none (review #227).
#[derive(Debug, Clone, Default)]
pub(super) struct Ends {
    heard: Vec<(f64, u32, usize, EndHeard)>,
    fed: bool,
}

impl std::ops::Deref for Ends {
    type Target = Vec<(f64, u32, usize, EndHeard)>;
    fn deref(&self) -> &Self::Target {
        &self.heard
    }
}

impl std::ops::DerefMut for Ends {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.heard
    }
}

impl Session {
    /// Where a note ended (E5): judged if the tab wants that note stopped.
    pub fn on_end(&mut self, end: &NoteEnd) {
        if !tells_ends(self.settings.source) || self.arrangement.judging == Judging::Open {
            return;
        }
        self.ends.fed = true;
        let Some(&(onset_s, pass, note)) = self
            .struck
            .iter()
            .rev()
            .find(|(onset, _, _)| same_onset(*onset, end.onset_s))
        else {
            return;
        };
        let Some(written_s) = self.written_end_s(note) else {
            return;
        };
        let (told_s, told_pass) = self.transport.position_at(self.real_s(end.end_s));
        let speed = self.transport.speed();
        let window_ms = self.settings.strictness.window_ms();
        // Real ms past the written end; a loop wrapped meanwhile: far past it.
        let past_ms = if told_pass == pass {
            (told_s - written_s) / speed * MS_PER_S
        } else {
            f64::INFINITY
        };
        let heard = match end.ending {
            Ending::Replaced => EndHeard::RangOn,
            _ if past_ms <= window_ms => EndHeard::InTime,
            Ending::Muted if past_ms.is_finite() => EndHeard::Late {
                late_ms: past_ms - window_ms,
            },
            _ => EndHeard::RangOn,
        };
        self.ends.retain(|(_, p, n, _)| !(*p == pass && *n == note));
        self.ends.push((onset_s, pass, note, heard));
    }

    /// How the notes the tab wants stopped ended (detected onset, pass, note, heard).
    pub fn ends_heard(&self) -> &[(f64, u32, usize, EndHeard)] {
        &self.ends
    }

    /// The ends judged, for the note list, and their tally.
    pub(super) fn end_summary(&self, summary: &mut Summary) {
        summary.ends = self
            .ends
            .iter()
            .map(|&(_, pass, note, heard)| NoteEndMark { pass, note, heard })
            .collect();
        // A finished run: a note to stop that told no end rang on (the last one has no next
        // note to end it, its decay takes seconds).
        if self.finished && self.ends.fed {
            let untold = self.struck.iter().filter(|&&(onset, pass, note)| {
                self.written_end_s(note).is_some()
                    && !self
                        .ends
                        .iter()
                        .any(|(o, p, n, _)| *p == pass && *n == note && same_onset(*o, onset))
            });
            summary
                .ends
                .extend(untold.map(|&(_, pass, note)| NoteEndMark {
                    pass,
                    note,
                    heard: EndHeard::RangOn,
                }));
        }
        summary.muting = MutingTally::of(&summary.ends, self.untold_ends());
    }

    /// Notes the tab wants stopped that this input cannot judge (microphone, MIDI): said in
    /// the summary, not hidden.
    fn untold_ends(&self) -> u32 {
        if tells_ends(self.settings.source) || self.arrangement.judging == Judging::Open {
            return 0;
        }
        let wanted = (0..self.arrangement.notes.len()).filter(|&i| self.written_end_s(i).is_some());
        wanted.count() as u32
    }

    /// Whether the last struck note's end may still be told: the analysis has not heard
    /// `END_TOLD_S` past its written end yet.
    pub(super) fn end_pending(&self) -> bool {
        if !tells_ends(self.settings.source) {
            return false;
        }
        let (Some(heard), Some(&(onset, pass, note))) = (self.heard_s, self.struck.last()) else {
            return false;
        };
        let told = self
            .ends
            .iter()
            .any(|(o, p, n, _)| *p == pass && *n == note && same_onset(*o, onset));
        let Some(written_s) = self.written_end_s(note).filter(|_| !told) else {
            return false;
        };
        let start_s = self
            .arrangement
            .tempo
            .seconds_at(self.arrangement.notes[note].start_ticks);
        let length_s = (written_s - start_s) / self.transport.speed();
        let due_s = self.real_s(onset) + length_s + END_TOLD_S;
        heard < due_s && self.now_s < due_s + MAX_ANALYSIS_LAG_S
    }

    /// The written end (arrangement seconds) of `note` when the tab wants it stopped: written
    /// staccato, or a rest of an eighth or more after it (its ties included). Not for chords,
    /// muted notes (dead, ghost, palm mute) or harmonics: they stop by themselves or ring on
    /// purpose. The last note only when staccato: the piece's end is no rest.
    fn written_end_s(&self, note: usize) -> Option<f64> {
        let notes = &self.arrangement.notes;
        let n = notes.get(note)?;
        let rings = |t: &Technique| {
            matches!(
                t,
                Technique::Muted | Technique::Ghost | Technique::PalmMute | Technique::Harmonic
            )
        };
        if !n.needs_attack() || n.techniques.iter().any(rings) {
            return None;
        }
        if notes
            .iter()
            .enumerate()
            .any(|(i, o)| i != note && o.start_ticks == n.start_ticks)
        {
            return None;
        }

        // Its ties continue it: the end is the last one's.
        let mut end = n.end_ticks();
        for o in &notes[note + 1..] {
            if o.tied && o.pitch == n.pitch && o.start_ticks == end {
                end = o.end_ticks();
            }
        }
        let staccato = n.techniques.contains(&Technique::Staccato);
        let next = notes[note + 1..]
            .iter()
            .filter(|o| !o.tied && o.start_ticks > n.start_ticks)
            .map(|o| o.start_ticks)
            .min();
        let rest = next.is_some_and(|s| s >= end + REST_TICKS);
        (staccato || rest).then(|| self.arrangement.tempo.seconds_at(end))
    }
}

/// Inputs whose ends are reliable: a cable carries the string's own fall.
fn tells_ends(source: InputSource) -> bool {
    matches!(source, InputSource::Line | InputSource::LaptopJack)
}
