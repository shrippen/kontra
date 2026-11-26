//! Building blocks in the app (DIDAKTIK I6): the `chain_*` calls on `Core`, which walk a piece
//! in short steps.

use crate::{Core, PracticeMode, UiEvent};
use kontra_session::{Blocks, Chain, ChainOrder, Workshop};
use serde_json::json;

/// Phrases from hesitating (DIDAKTIK I6): a bar counts when the player hesitated there in at
/// least `HESITATION_MIN` of the piece's last `HESITATION_RUNS` runs; once may be a slip.
const HESITATION_RUNS: usize = 5;
const HESITATION_MIN: usize = 2;

impl Core {
    /// Starts building blocks (DIDAKTIK I6), a few bars each or phrases, over the chosen bars
    /// (with blocks running: the bars they started from), or over the whole piece when they
    /// make only one block; the loop becomes the first step. Returns true if the notes changed,
    /// as `set_loop`.
    pub fn start_chain(&mut self, blocks: Blocks, order: ChainOrder) -> bool {
        let Some(bars) = self.arrangement().map(|a| a.bars.len()).filter(|n| *n > 0) else {
            return false;
        };
        // Running blocks loop one step: a new order or size cuts the bars chosen before them.
        let chosen = match self.chain.as_ref() {
            Some((_, chosen)) => *chosen,
            None => self.looping,
        };
        let Some(a) = self.arrangement() else {
            return false;
        };
        let hesitant = self.hesitant_bars();
        let cut = |bars| Chain::new(a, bars, blocks, &hesitant, order, &Workshop::default());
        // Chosen bars that make one block leave nothing to join: the whole piece instead.
        let chain = match chosen.map(cut) {
            Some(chain) if chain.count() > 1 => chain,
            _ => cut((0, bars - 1)),
        };
        let (from, to) = chain.step();
        self.chain = Some((chain, chosen));
        self.loop_to(from, to)
    }

    /// Bars (indices) the player keeps hesitating at, from the piece's last runs; phrases start
    /// there.
    fn hesitant_bars(&self) -> Vec<usize> {
        let (Some(loaded), Some(a)) = (self.loaded.as_ref(), self.arrangement()) else {
            return Vec::new();
        };
        let Ok(progress) = self.progress_store().load(&loaded.full) else {
            return Vec::new();
        };
        let mut counts = std::collections::BTreeMap::new();
        for run in progress.runs.iter().rev().take(HESITATION_RUNS) {
            for bar in &run.hesitant_bars {
                *counts.entry(*bar).or_insert(0) += 1;
            }
        }

        // Stored as printed numbers; with repeats a number names several bars.
        a.bars
            .iter()
            .enumerate()
            .filter(|(_, bar)| {
                counts
                    .get(&(bar.source_index + 1))
                    .is_some_and(|n| *n >= HESITATION_MIN)
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Ends the building blocks; the loop goes back to the one chosen before them (none, if
    /// none was). Returns true if the notes changed, as `set_loop`.
    pub fn stop_chain(&mut self) -> bool {
        let Some((_, chosen)) = self.chain.take() else {
            return false;
        };
        match chosen {
            Some((from, to)) => self.loop_to(from, to),
            None => self.clear_loop(),
        }
    }

    /// The building blocks for the UI, or `null`: `{step, steps, last, text}`, `text` like
    /// "Baustein: Takte 3–4 · Schritt 2 von 5".
    pub fn chain_json(&self) -> String {
        let Some((chain, _)) = self.chain.as_ref() else {
            return "null".into();
        };
        let (from, to) = chain.step();
        let number = |bar: usize| {
            self.arrangement()
                .and_then(|a| a.bars.get(bar))
                .map_or(bar + 1, |b| b.source_index as usize + 1)
        };
        let key = if chain.is_last() {
            "chain.whole"
        } else if from == to {
            "chain.step.one"
        } else {
            "chain.step"
        };
        let text = self.tr(
            key,
            &[
                ("from", number(from).to_string()),
                ("to", number(to).to_string()),
                ("step", (chain.index() + 1).to_string()),
                ("steps", chain.count().to_string()),
            ],
        );
        json!({
            "step": chain.index(),
            "steps": chain.count(),
            "last": chain.is_last(),
            "text": text,
        })
        .to_string()
    }

    /// After a workshop pass: when the step sits, its run ends (and is saved) and the next step
    /// starts with a count-in. Not within the run: clicks and backing are planned ahead.
    pub(crate) fn chain_pass(&mut self, hit_rate: f64) -> Vec<UiEvent> {
        if self.settings.mode != PracticeMode::Workshop || self.probe.is_some() {
            return Vec::new();
        }
        let Some((from, to)) = self.chain.as_mut().and_then(|(c, _)| c.on_pass(hit_rate)) else {
            return Vec::new();
        };
        self.stop_live();
        self.loop_to(from, to);
        let mut events = vec![UiEvent::ChainStep {
            chain_json: self.chain_json(),
        }];
        events.extend(self.play());
        events
    }
}
