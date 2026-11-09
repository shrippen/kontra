//! Song map (LH-FOR-06): the hit rate of each bar over the last runs that played it, for
//! `KanteSongMap`. Shown between runs and in the summary, never while playing.
//!
//! ```text
//! runs (newest first)   bar 1   bar 2   bar 3
//!   today               4/4     1/4     –        a run of a loop judges only its bars
//!   yesterday           4/4     2/4     3/4
//!   map                 100 %   38 %    75 %     at most RECENT_RUNS runs per bar
//! ```

use crate::Core;
use serde_json::json;

/// Runs per bar that count: the recent ones show where the piece stands now.
const RECENT_RUNS: usize = 5;

impl Core {
    /// `{ values: [rate | null], labels: ["1", …], tips: […], loop: [from, to] | null,
    /// played: bool }` for the loaded piece, one entry per bar (repeats unrolled).
    pub fn song_map_json(&self) -> String {
        let Some(bars) = self.loaded.as_ref().map(|l| &l.full.bars) else {
            return json!({ "values": [], "labels": [], "tips": [], "played": false }).to_string();
        };
        let runs = self
            .loaded_path()
            .and_then(|p| self.library.entry(p))
            .and_then(|e| self.progress_store().load_id(&e.song_id))
            .map(|p| p.runs)
            .unwrap_or_default();

        // Per bar: hits and judged notes of its last RECENT_RUNS runs, newest first.
        let mut tally = vec![(0u32, 0u32, 0usize); bars.len()];
        for run in runs.iter().rev() {
            for b in &run.bars {
                let Some(t) = tally.get_mut(b.bar as usize) else {
                    continue;
                };
                if t.2 >= RECENT_RUNS || b.judged == 0 {
                    continue;
                }
                *t = (t.0 + b.hits, t.1 + b.judged, t.2 + 1);
            }
        }

        let labels: Vec<String> = bars
            .iter()
            .map(|b| (b.source_index + 1).to_string())
            .collect();
        let values: Vec<Option<f64>> = tally
            .iter()
            .map(|(hits, judged, _)| (*judged > 0).then(|| f64::from(*hits) / f64::from(*judged)))
            .collect();
        let tips: Vec<String> = tally
            .iter()
            .zip(&labels)
            .map(|((hits, judged, runs), bar)| self.song_map_tip(bar, *hits, *judged, *runs))
            .collect();
        json!({
            "values": values,
            "labels": labels,
            "tips": tips,
            "loop": self.looping.map(|(from, to)| [from, to]),
            "played": tally.iter().any(|t| t.1 > 0),
        })
        .to_string()
    }

    /// "Takt 5: 82 % in den letzten 3 Durchläufen (14 von 17 Noten)", or "Takt 5: noch nicht
    /// gespielt".
    fn song_map_tip(&self, bar: &str, hits: u32, judged: u32, runs: usize) -> String {
        if judged == 0 {
            return self.tr("songmap.none", &[("bar", bar.to_string())]);
        }
        let percent = format!("{:.0}", f64::from(hits) / f64::from(judged) * 100.0);
        let key = if runs == 1 {
            "songmap.bar.one"
        } else {
            "songmap.bar"
        };
        self.tr(
            key,
            &[
                ("bar", bar.to_string()),
                ("percent", percent),
                ("runs", runs.to_string()),
                ("hits", hits.to_string()),
                ("judged", judged.to_string()),
            ],
        )
    }
}
