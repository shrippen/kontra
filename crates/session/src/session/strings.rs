//! The string a struck note was heard on (R7, LH-BEW-16): feedback beside the score.

use super::{same_onset, Session, MAX_ANALYSIS_LAG_S, REVISION_MATCH_S, STRUCK_KEPT};
use kontra_model::{StringHeard, StringVerdict, Technique};

/// A note's string is told this long after its onset at most (R7: the peak up to 30 ms late,
/// the window from 30 ms after it for 250 ms, a block), plus margin. The run ends after it.
const STRING_TOLD_S: f64 = 0.4;

impl Session {
    /// The string a struck note was heard on (R7): feedback, the note's result stays. A note
    /// the tab mutes with the palm is not told: its partials die before they tell the string.
    pub fn on_string(&mut self, verdict: &StringVerdict) {
        if self.told_strings.len() >= STRUCK_KEPT {
            self.told_strings.remove(0);
        }
        self.told_strings.push(*verdict);
        self.attach_string(verdict);
    }

    /// Records `verdict` for the struck note detected at its onset, if any.
    pub(super) fn attach_string(&mut self, verdict: &StringVerdict) {
        let Some(&(_, pass, note)) = self
            .struck
            .iter()
            .rev()
            .find(|(onset, _, _)| (onset - verdict.onset_s).abs() < REVISION_MATCH_S)
        else {
            return;
        };
        if !self.tells_string(note) {
            return;
        }
        self.strings
            .push((verdict.onset_s, pass, note, verdict.heard));
    }

    /// Whether the tab puts `note` on a string that is told (palm-muted notes die too fast).
    fn tells_string(&self, note: usize) -> bool {
        self.arrangement
            .notes
            .get(note)
            .is_some_and(|n| n.string.is_some() && !n.techniques.contains(&Technique::PalmMute))
    }

    /// Whether the last struck note's string may still be told: the analysis has not heard
    /// `STRING_TOLD_S` past it yet. Without word from the analysis (a file ended) nothing comes.
    pub(super) fn string_pending(&self) -> bool {
        let (Some(heard), Some(&(onset, pass, note))) = (self.heard_s, self.struck.last()) else {
            return false;
        };
        let told = self
            .strings
            .iter()
            .any(|(o, p, n, _)| *p == pass && *n == note && same_onset(*o, onset));
        if told || !self.tells_string(note) {
            return false;
        }
        let due_s = self.real_s(onset) + STRING_TOLD_S;
        heard < due_s && self.now_s < due_s + MAX_ANALYSIS_LAG_S
    }

    /// The strings struck notes were heard on (detected onset, pass, note, heard), in the order
    /// told.
    pub fn strings_heard(&self) -> &[(f64, u32, usize, StringHeard)] {
        &self.strings
    }
}
