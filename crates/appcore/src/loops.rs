//! Loops in the app (LH-MOD): the section being practised, saved loops and the target speed.

use crate::{clamp_speed, Core, LoopEdge, PracticeLoop, RecentEntry, SECTION_TOLERANCE_S};
use kontra_session::LoopRange;
use serde_json::json;

impl Core {
    /// Returns true if the notes changed: the section has another note density (LH-MOD-06).
    pub fn set_loop(&mut self, from: usize, to: usize) -> bool {
        // Bars chosen by hand end the building blocks.
        self.chain = None;
        self.loop_to(from, to)
    }

    pub(crate) fn loop_to(&mut self, from: usize, to: usize) -> bool {
        self.looping = Some((from.min(to), from.max(to)));
        let range = self.loop_range();
        if let Some(live) = self.live.as_mut() {
            live.practice_mut().set_loop(range);
        }
        self.rethin()
    }

    /// Sets one end of the loop to the bar at `position_s` (keys `[` and `]`, LH-MOD-07); the
    /// other end stays where it is unless it would cross, e.g. loop 3–6, `]` in bar 2 → 2–2.
    /// Returns true if the notes changed, as `set_loop`.
    pub fn set_loop_edge(&mut self, edge: LoopEdge, position_s: f64) -> bool {
        let Some(a) = self.arrangement() else {
            return false;
        };
        let Some(bar) = a.bar_at(a.tempo.tick_at(position_s.max(0.0))) else {
            return false;
        };
        let (from, to) = match (edge, self.looping) {
            (LoopEdge::Start, Some((_, to))) if to >= bar => (bar, to),
            (LoopEdge::End, Some((from, _))) if from <= bar => (from, bar),
            _ => (bar, bar),
        };
        self.set_loop(from, to)
    }

    /// Back to the start: the loop's first bar if a loop is set, else the piece's (LH-MOD-07).
    /// Ends the run; returns the position the next `play` starts from.
    pub fn rewind(&mut self) -> f64 {
        self.stop_live();
        self.loop_range().map_or(0.0, |r| r.start_s)
    }

    /// Returns true if the notes changed, as `set_loop`.
    pub fn clear_loop(&mut self) -> bool {
        self.looping = None;
        self.chain = None;
        if let Some(live) = self.live.as_mut() {
            live.practice_mut().set_loop(None);
        }
        self.rethin()
    }

    /// Keeps the current loop with the piece (LH-BIB-07: a package hands it on); a loop kept
    /// already is not kept twice. Returns true if one was added.
    pub fn save_loop(&mut self) -> bool {
        let Some((from, to)) = self.looping else {
            return false;
        };
        let Some(entry) = self.loaded_entry_mut() else {
            return false;
        };
        let practice = PracticeLoop { from, to };
        if entry.loops.contains(&practice) {
            return false;
        }
        entry.loops.push(practice);
        self.save_library();
        true
    }

    /// Drops the kept loop at `index` (as `saved_loops` lists them).
    pub fn remove_loop(&mut self, index: usize) {
        let Some(entry) = self.loaded_entry_mut() else {
            return;
        };
        if index >= entry.loops.len() {
            return;
        }
        entry.loops.remove(index);
        self.save_library();
    }

    /// Loops kept with the loaded piece, as bar indices `(from, to)`.
    pub fn saved_loops(&self) -> Vec<(usize, usize)> {
        self.loaded_path()
            .and_then(|p| self.library.entry(p))
            .map_or_else(Vec::new, |e| {
                e.loops.iter().map(|l| (l.from, l.to)).collect()
            })
    }

    /// Kept loops for the UI: `[{"from", "to"}]`, bar indices from 0.
    pub fn saved_loops_json(&self) -> String {
        let loops: Vec<_> = self
            .saved_loops()
            .into_iter()
            .map(|(from, to)| json!({ "from": from, "to": to }))
            .collect();
        serde_json::Value::from(loops).to_string()
    }

    /// The workshop's target speed for the loaded piece (1.0 = as written).
    pub fn target_speed(&self) -> f64 {
        self.loaded_path()
            .and_then(|p| self.library.entry(p))
            .and_then(|e| e.target_speed)
            .unwrap_or(1.0)
    }

    /// Sets the target speed of the loaded piece, in the workshop's 5 % steps.
    pub fn set_target_speed(&mut self, speed: f64) {
        let Some(entry) = self.loaded_entry_mut() else {
            return;
        };
        entry.target_speed = Some(clamp_speed(speed));
        self.save_library();
    }

    fn loaded_entry_mut(&mut self) -> Option<&mut RecentEntry> {
        let path = self.loaded_path()?.to_path_buf();
        self.library.entry_mut(&path)
    }

    /// Indices of the bars from `start_s` to `end_s` (a saved section), if bars begin and end
    /// there.
    pub fn bars_at(&self, start_s: f64, end_s: f64) -> Option<(usize, usize)> {
        let a = self.arrangement()?;
        let at = |s: f64, t: u64| (a.tempo.seconds_at(t) - s).abs() < SECTION_TOLERANCE_S;
        let from = a.bars.iter().position(|b| at(start_s, b.start_ticks))?;
        let to = a.bars.iter().position(|b| at(end_s, b.end_ticks()))?;
        Some((from, to))
    }

    /// Loop range in seconds, or (-1, -1).
    pub fn loop_seconds(&self) -> (f64, f64) {
        self.loop_range()
            .map_or((-1.0, -1.0), |r| (r.start_s, r.end_s))
    }

    pub(crate) fn loop_range(&self) -> Option<LoopRange> {
        let (from, to) = self.looping?;
        let a = self.arrangement()?;
        let first = a.bars.get(from)?;
        let last = a.bars.get(to)?;
        Some(LoopRange {
            start_s: a.tempo.seconds_at(first.start_ticks),
            end_s: a.tempo.seconds_at(last.end_ticks()),
        })
    }
}
