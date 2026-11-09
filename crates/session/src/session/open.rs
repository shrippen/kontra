//! Attacks that are not matched: open runs (LH-BEW-13), where the player makes up the line and
//! attacks are counted by how they fit the chords (`crate::improv`), and calls (LH-LRN-19),
//! where Kontra plays.

use super::{Session, DETECTION_GRACE_S};
use crate::improv::Improvisation;
use crate::summary::Summary;
use kontra_model::DetectedNote;

impl Session {
    /// Takes an attack that is not matched: during a call Kontra plays and a microphone hears
    /// it (LH-LRN-19), ignored; in an open run counted by its fit. True if taken.
    pub(super) fn count_open(&mut self, note: &DetectedNote, real_s: f64) -> bool {
        // An answer's first note played a little early is the answer's, not the call's.
        let (position, _) = self.transport.position_at(real_s);
        let window =
            self.settings.strictness.window_ms() / super::MS_PER_S * self.transport.speed();
        let tick = self.arrangement.tempo.tick_at(position + window);
        if self.arrangement.in_call(tick) {
            return true;
        }
        let Some(open) = self.open.as_mut() else {
            return false;
        };
        // A click on hum or a scrape has no tone of its own to fit (LH-NF-16).
        if !note.stands_out() {
            return true;
        }
        let (position, pass) = self.transport.position_at(real_s);
        open.attack(&self.arrangement, pass, position, note.midi);
        true
    }

    /// The tally of `passes` (all: `None`) so far; `None` unless the run is open.
    fn open_tally(&self, passes: Option<u32>) -> Option<Improvisation> {
        let open = self.open.as_ref()?;
        let (position, pass) = self.transport.position_at(self.now_s - DETECTION_GRACE_S);
        let window_s = self.settings.strictness.window_ms() / super::MS_PER_S;
        Some(open.tally(passes, (pass, position), self.range(), window_s))
    }

    /// The fitting share of a finished pass of an open run: its hit rate.
    pub(super) fn open_pass_rate(&self, pass: u32) -> Option<f64> {
        self.open_tally(Some(pass))?.fitting_share()
    }

    /// An open run's summary: the tally, its fitting share as the hit rate.
    pub(super) fn open_summary(&self, summary: &mut Summary) {
        let Some(tally) = self.open_tally(None) else {
            return;
        };
        summary.hit_rate = tally.fitting_share().unwrap_or(0.0);
        summary.improvisation = Some(tally);
    }
}
