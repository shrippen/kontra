//! Progress (LH-FOR-01): recording runs, start page, progress page and practice spots.

use crate::{
    same_range, same_section, unix_now, Core, RecentEntry, SectionBars, PROBE_MODE,
    SECONDS_PER_DAY, WORKSHOP_MODE,
};
use kontra_session::{run_from, spacing, streak_days, LoopRange, PassageEvent, ProgressStore, Run};
use serde_json::json;
use std::path::Path;

impl Core {
    /// Progress over all pieces (LH-FOR-01): per piece best result, best tempo and time, and
    /// every run's time stamp and length so the view can sum days in local time.
    pub fn progress_json(&self) -> String {
        self.progress_json_at(unix_now())
    }

    /// In level order, the first bundled exercise that does not sit yet (see `sits`),
    /// apart from `skip` (the one just played): `{title, path, best, mastered, level}`.
    pub(crate) fn next_exercise(&self, skip: Option<&Path>) -> Option<serde_json::Value> {
        self.bundled_levels().into_iter().find_map(|level| {
            let mut item = level["items"]
                .as_array()?
                .iter()
                .filter(|i| skip.is_none_or(|p| i["path"].as_str() != p.to_str()))
                .find(|i| i["mastered"] != true)?
                .clone();
            item["level"] = level["level"].clone();
            Some(item)
        })
    }

    /// Start page: the last piece played, the first bundled exercise that does not sit yet,
    /// the next lesson (LH-LRN-07) and the practice of the last seven days.
    /// `{"last": {path, title, artist, best, practiceMin} | null,
    ///   "next": {title, path, best, level} | null, "lesson": {id, title} | null, "review": due,
    ///   "repeat": [{path, title, bar}], "streak": days, "week": {minutes, runs}}`.
    /// `utc_offset_s`: the local time zone, for the days of the streak.
    pub fn home_json(&self, utc_offset_s: i32) -> String {
        self.home_in(unix_now(), utc_offset_s)
    }

    #[cfg(test)]
    pub(crate) fn home_json_at(&self, now: u64) -> String {
        self.home_in(now, 0)
    }

    pub(crate) fn home_in(&self, now: u64, utc_offset_s: i32) -> String {
        let store = self.progress_store();
        let last = self
            .library
            .recent
            .iter()
            .max_by_key(|e| e.last_unix)
            .map(|e| {
                let progress = store.load_id(&e.song_id);
                json!({
                    "path": e.path,
                    "title": self.library_title(e),
                    "artist": e.artist,
                    "best": progress.as_ref().and_then(|p| p.best_hit_rate()),
                    "practiceMin": progress.as_ref().map_or(0.0, |p| p.practice_s() / 60.0),
                })
            });

        let next = self.next_exercise(None);

        let since = now.saturating_sub(WEEK_DAYS * SECONDS_PER_DAY);
        let week: Vec<_> = store
            .all()
            .into_iter()
            .flat_map(|p| p.runs)
            .filter(|r| r.finished_unix > since)
            .collect();
        let minutes = week.iter().map(|r| r.practice_s).sum::<f64>() / 60.0;
        let repeat = self.due_passages(&store, now);
        json!({
            "last": last,
            "next": next,
            "spots": self.hard_spots(&store, now, &repeat),
            "repeat": repeat,
            "streak": self.streak(&store, now, utc_offset_s),
            "probe": self.probe_offer(&store, now),
            "lesson": self.next_lesson(),
            "review": self.review_count(now),
            "week": { "minutes": minutes, "runs": week.len() },
        })
        .to_string()
    }

    /// "Spots from yesterday" (round 11, DIDAKTIK: places come back over days): from the piece
    /// practised last in the recent days, the bars that were among the hardest most often (ties:
    /// the later one). Runs of the last hours wait; a place comes back after a night.
    ///
    /// ```text
    /// now − 3 days ─────────── runs that count ─────────── now − 8 h ── today's runs ── now
    /// ```
    /// Bars already on the "repeat" card (`due_passages`) stay off this one.
    fn hard_spots(
        &self,
        store: &ProgressStore,
        now: u64,
        repeat: &[serde_json::Value],
    ) -> Option<serde_json::Value> {
        let from = now.saturating_sub(SPOT_DAYS * SECONDS_PER_DAY);
        let until = now.saturating_sub(SPOT_REST_S);
        let counts = |r: &Run| r.finished_unix > from && r.finished_unix <= until;
        let (song, runs) = store
            .all()
            .into_iter()
            .filter_map(|p| {
                let runs: Vec<Run> = p
                    .runs
                    .into_iter()
                    .filter(|r| counts(r) && !r.hardest_bars.is_empty())
                    .collect();
                let latest = runs.iter().map(|r| r.finished_unix).max()?;
                Some((latest, p.song_id, runs))
            })
            .max_by_key(|(latest, _, _)| *latest)
            .map(|(_, song, runs)| (song, runs))?;
        let entry = self.library.recent.iter().find(|e| e.song_id == song)?;
        if !entry.path.exists() {
            return None;
        }

        // Per bar: how often it was among the hardest, and when last.
        let mut bars: Vec<(u32, usize, u64)> = Vec::new();
        for run in &runs {
            for bar in &run.hardest_bars {
                match bars.iter_mut().find(|(b, _, _)| b == bar) {
                    Some(seen) => {
                        seen.1 += 1;
                        seen.2 = seen.2.max(run.finished_unix);
                    }
                    None => bars.push((*bar, 1, run.finished_unix)),
                }
            }
        }
        bars.sort_by_key(|(_, n, last)| std::cmp::Reverse((*n, *last)));
        let repeated = |bar: u32| {
            repeat
                .iter()
                .any(|r| r["path"] == json!(entry.path) && r["bar"] == bar)
        };
        let picked: Vec<u32> = bars
            .iter()
            .map(|(b, _, _)| *b)
            .filter(|b| !repeated(*b))
            .take(SPOTS)
            .collect();
        if picked.is_empty() {
            return None;
        }
        Some(json!({
            "path": entry.path,
            "title": self.library_title(entry),
            "bars": picked,
        }))
    }

    /// "Repeat" (LH-LRN-16): bars twice among the hardest come back when their interval is
    /// over (`kontra_session::spacing`), the longest waiting first, at most `REPEAT_MAX`.
    /// A pass is a run without help over a section with the bar, at the target tempo.
    fn due_passages(&self, store: &ProgressStore, now: u64) -> Vec<serde_json::Value> {
        let mut due: Vec<(u64, serde_json::Value)> = Vec::new();
        for progress in store.all() {
            let Some(entry) = self
                .library
                .recent
                .iter()
                .find(|e| e.song_id == progress.song_id && e.path.exists())
            else {
                continue;
            };
            let target = entry.target_speed.unwrap_or(1.0);

            // Per bar its history: among the hardest, or played in a probe.
            let mut events: std::collections::BTreeMap<u32, Vec<PassageEvent>> =
                std::collections::BTreeMap::new();
            for run in &progress.runs {
                let unix = run.finished_unix;
                for bar in &run.hardest_bars {
                    events
                        .entry(*bar)
                        .or_default()
                        .push(PassageEvent::Hardest { unix });
                }
                if run.mode != PROBE_MODE {
                    continue;
                }
                let Some(bars) = run.section.and_then(|(start_s, end_s)| {
                    entry
                        .sections
                        .iter()
                        .find(|b| same_section(b, start_s, end_s))
                }) else {
                    continue;
                };
                let passed = run.hit_rate >= PASS_HIT_RATE && run.speed >= target - SPEED_EPS;
                for bar in bars.from_bar..=bars.to_bar {
                    if let Some(list) = events.get_mut(&bar) {
                        list.push(PassageEvent::Probe { unix, passed });
                    }
                }
            }

            for (bar, list) in events {
                let Some(s) = spacing(&list).filter(|s| s.due_unix <= now) else {
                    continue;
                };
                let card =
                    json!({ "path": entry.path, "title": self.library_title(entry), "bar": bar });
                due.push((s.due_unix, card));
            }
        }
        due.sort_by_key(|(unix, _)| *unix);
        due.into_iter().take(REPEAT_MAX).map(|(_, c)| c).collect()
    }

    /// Practice days in a row (LH-FOR-05), 0 when switched off; days in local time.
    fn streak(&self, store: &ProgressStore, now: u64, utc_offset_s: i32) -> u32 {
        if !self.settings.streak {
            return 0;
        }
        let day = |unix: u64| {
            i64::try_from(unix)
                .unwrap_or(i64::MAX)
                .saturating_add(i64::from(utc_offset_s))
                .div_euclid(SECONDS_PER_DAY as i64)
        };
        let days: Vec<i64> = store
            .all()
            .iter()
            .flat_map(|p| p.runs.iter().map(|r| day(r.finished_unix)))
            .collect();
        streak_days(&days, day(now))
    }

    /// "Check without help" (DIDAKTIK I1): the section practised last in the workshop in the
    /// recent days, rested a night like the spots, and not probed since.
    fn probe_offer(&self, store: &ProgressStore, now: u64) -> Option<serde_json::Value> {
        let from = now.saturating_sub(SPOT_DAYS * SECONDS_PER_DAY);
        let until = now.saturating_sub(SPOT_REST_S);
        let mut latest: Option<(u64, &RecentEntry, &SectionBars)> = None;
        for progress in store.all() {
            // Only pieces the library can open, with sections it can name (a newer one it
            // cannot must not hide an older one).
            let Some(entry) = self
                .library
                .recent
                .iter()
                .find(|e| e.song_id == progress.song_id && e.path.exists())
            else {
                continue;
            };
            for run in &progress.runs {
                let Some((start_s, end_s)) = run.section else {
                    continue;
                };
                if run.mode != WORKSHOP_MODE
                    || run.finished_unix <= from
                    || run.finished_unix > until
                    || latest.is_some_and(|(t, _, _)| t >= run.finished_unix)
                {
                    continue;
                }
                let range = LoopRange { start_s, end_s };
                let probed = progress.runs.iter().any(|r| {
                    r.mode == PROBE_MODE
                        && r.finished_unix > run.finished_unix
                        && same_range(r.section, range)
                });
                if probed {
                    continue;
                }
                let Some(bars) = entry
                    .sections
                    .iter()
                    .find(|b| same_section(b, start_s, end_s))
                else {
                    continue;
                };
                latest = Some((run.finished_unix, entry, bars));
            }
        }
        let (_, entry, bars) = latest?;
        Some(json!({
            "path": entry.path,
            "title": self.library_title(entry),
            "from": bars.from_bar,
            "to": bars.to_bar,
            "startS": bars.start_s,
            "endS": bars.end_s,
        }))
    }

    /// Bars of the sections practised in the piece with `song_id`, as the library knows them.
    fn section_names(&self, song_id: &str) -> &[SectionBars] {
        self.library
            .recent
            .iter()
            .find(|e| e.song_id == song_id)
            .map_or(&[], |e| e.sections.as_slice())
    }

    /// A piece's title for progress and report: a bundled exercise in the UI's language (the
    /// stored `title` is the German progress key), else as stored.
    pub(crate) fn song_title(&self, song_id: &str, title: &str) -> String {
        self.library
            .recent
            .iter()
            .find(|e| e.song_id == song_id)
            .map_or_else(|| title.to_string(), |e| self.library_title(e))
    }

    pub(crate) fn progress_json_at(&self, now: u64) -> String {
        let mut songs = self.progress_store().all();
        songs.sort_by_key(|p| std::cmp::Reverse(p.runs.iter().map(|r| r.finished_unix).max()));
        let pieces: Vec<serde_json::Value> = songs
            .iter()
            .map(|p| {
                json!({
                    "songId": p.song_id,
                    "title": self.song_title(&p.song_id, &p.title),
                    "artist": p.artist,
                    "best": p.best_hit_rate(),
                    "bestSpeed": p.best_speed(0.9),
                    "practiceMin": p.practice_s() / 60.0,
                    "runs": p.runs.len(),
                    "lastUnix": p.runs.iter().map(|r| r.finished_unix).max(),
                    "sections": self.sections_json(&p.runs, self.section_names(&p.song_id), now),
                })
            })
            .collect();
        let runs: Vec<serde_json::Value> = songs
            .iter()
            .flat_map(|p| p.runs.iter())
            .map(|r| json!({ "unix": r.finished_unix, "s": r.practice_s }))
            .collect();
        json!({ "pieces": pieces, "runs": runs }).to_string()
    }

    pub(crate) fn progress_store(&self) -> ProgressStore {
        ProgressStore::new(&self.data_dir.join("progress"))
    }

    /// The file at `path` changed (a regenerated exercise, an edited score): its runs move from
    /// the id the library knew to `song_id`, unless another entry still uses the old one.
    pub(crate) fn adopt_progress(&self, path: &Path, song_id: &str) {
        let Some((known, old)) = self
            .library
            .entry(path)
            .map(|e| (e.path.clone(), e.song_id.clone()))
        else {
            return;
        };
        let shared = self
            .library
            .recent
            .iter()
            .any(|e| e.path != known && e.song_id == old);
        if shared {
            return;
        }
        // Progress is a convenience here: a failed move leaves the old runs where they were.
        let _ = self.progress_store().adopt(&old, song_id);
    }

    /// Saves the finished run (LH-FOR-01).
    pub(crate) fn record_run(&mut self) {
        let Some(live) = self.live.as_ref() else {
            return;
        };
        // An exercise of a lesson while a course folder is open live: nothing of the folder
        // is stored, its runs neither (LH-LRN-21).
        if self.previewing_course() && self.loaded_goal_of_lesson().is_some() {
            self.run_started = None;
            return;
        }
        let session = live.practice().session();
        let summary = session.summary();
        // An open run judges no written notes, it counts its attacks (LH-BEW-13).
        let counted = summary.improvisation.map_or(0, |i| i.played());
        if summary.judged == 0 && counted == 0 {
            self.run_started = None;
            return;
        }
        let practice_s = self
            .run_started
            .take()
            .map_or(0.0, |t| t.elapsed().as_secs_f64());
        let section = session
            .transport()
            .loop_range()
            .map(|l| (l.start_s, l.end_s));
        // The speed played: the workshop changes it on its own after a pass (LH-FOR-02 charts
        // it, the probe takes it over).
        let mut run = run_from(
            &summary,
            practice_s,
            session.played_speed(),
            self.run_mode(),
            section,
            unix_now(),
        );
        // Progress belongs to the piece, not to a thinned copy (LH-MOD-06).
        let arrangement = self
            .loaded
            .as_ref()
            .map_or_else(|| session.arrangement().clone(), |l| l.full.clone());
        // Bar numbers as printed in the score (repeats are unrolled into more bars).
        run.hardest_bars = summary
            .hardest_bars
            .iter()
            .filter_map(|b| arrangement.bars.get(*b))
            .map(|bar| bar.source_index + 1)
            .collect();
        run.hesitant_bars = summary
            .hesitant_bars
            .iter()
            .filter_map(|b| arrangement.bars.get(*b))
            .map(|bar| bar.source_index + 1)
            .collect();
        let _ = self.progress_store().record(&arrangement, run);
        if let (Some((start_s, end_s)), Some((from, to))) = (section, self.looping) {
            self.remember_section(start_s, end_s, from, to);
        }
    }

    /// The mode a run is saved under: a probe apart from the mode chosen.
    fn run_mode(&self) -> &str {
        if self.probe.is_some() {
            return PROBE_MODE;
        }
        &self.settings.mode
    }

    /// Keeps the written bar numbers of a practised section with the piece (LH-FOR-01).
    pub(crate) fn remember_section(&mut self, start_s: f64, end_s: f64, from: usize, to: usize) {
        let Some(a) = self.arrangement() else {
            return;
        };
        let number = |i: usize| a.bars.get(i).map_or(0, |b| b.source_index + 1);
        let section = SectionBars {
            start_s,
            end_s,
            from_bar: number(from),
            to_bar: number(to),
        };
        let Some(path) = self.loaded_path().map(Path::to_path_buf) else {
            return;
        };
        let Some(entry) = self.library.entry_mut(&path) else {
            return;
        };
        if entry
            .sections
            .iter()
            .any(|s| same_section(s, start_s, end_s))
        {
            return;
        }
        entry.sections.push(section);
        self.save_library();
    }

    /// Per section of a piece (LH-FOR-01): the best run with its tempo and date, and the
    /// workshop runs of the last 30 days for the tempo chart (LH-FOR-02). The whole piece
    /// comes first, then sections by start.
    fn sections_json(
        &self,
        runs: &[Run],
        names: &[SectionBars],
        now_unix: u64,
    ) -> Vec<serde_json::Value> {
        // Sections as centiseconds, so float noise does not split them.
        let key = |r: &Run| {
            r.section
                .map(|(a, b)| ((a * 100.0).round() as i64, (b * 100.0).round() as i64))
        };
        let mut groups: std::collections::BTreeMap<Option<(i64, i64)>, Vec<&Run>> =
            std::collections::BTreeMap::new();
        for run in runs {
            groups.entry(key(run)).or_default().push(run);
        }
        let since = now_unix.saturating_sub(HISTORY_DAYS * SECONDS_PER_DAY);
        groups
            .into_values()
            .filter_map(|runs| {
                let best = runs.iter().max_by(|a, b| {
                    a.hit_rate.total_cmp(&b.hit_rate).then(a.speed.total_cmp(&b.speed))
                })?;
                let mut history: Vec<&&Run> = runs
                    .iter()
                    .filter(|r| r.mode == "workshop" && r.finished_unix >= since)
                    .collect();
                history.sort_by_key(|r| r.finished_unix);
                let history: Vec<serde_json::Value> = history
                    .iter()
                    .map(|r| json!({ "unix": r.finished_unix, "speed": r.speed, "hitRate": r.hit_rate }))
                    .collect();
                Some(json!({
                    "label": self.section_label(best.section, names),
                    "best": best.hit_rate,
                    "speed": best.speed,
                    "unix": best.finished_unix,
                    "runs": runs.len(),
                    "history": history,
                }))
            })
            .collect()
    }

    /// "Ganzes Stück", "Takt 5–8" or, for a section without known bars, "0:12–0:24".
    fn section_label(&self, section: Option<(f64, f64)>, names: &[SectionBars]) -> String {
        let Some((start_s, end_s)) = section else {
            return self.tr("progress.section.whole", &[]);
        };
        let Some(bars) = names.iter().find(|s| same_section(s, start_s, end_s)) else {
            let clock = |s: f64| format!("{}:{:02}", (s / 60.0) as u32, (s % 60.0) as u32);
            return self.tr(
                "progress.section.time",
                &[("from", clock(start_s)), ("to", clock(end_s))],
            );
        };
        if bars.from_bar == bars.to_bar {
            return self.tr(
                "progress.section.bar",
                &[("bar", bars.from_bar.to_string())],
            );
        }
        self.tr(
            "progress.section.bars",
            &[
                ("from", bars.from_bar.to_string()),
                ("to", bars.to_bar.to_string()),
            ],
        )
    }
}

/// Days of workshop history in the tempo chart (LH-FOR-02).
const HISTORY_DAYS: u64 = 30;

/// Days the start page sums up as "this week".
const WEEK_DAYS: u64 = 7;

/// "Spots from yesterday": runs of the last days count, the last hours wait (one night).
const SPOT_DAYS: u64 = 3;

const SPOT_REST_S: u64 = 8 * 60 * 60;

/// Bars on the card.
const SPOTS: usize = 2;

/// Passages on the "repeat" card (LH-LRN-16).
const REPEAT_MAX: usize = 3;

/// A probe passes a passage from this hit rate on (LH-LRN-16).
const PASS_HIT_RATE: f64 = 0.8;

/// Speeds are stored in 5 % steps; float noise must not fail a pass at the target tempo.
const SPEED_EPS: f64 = 1e-6;
