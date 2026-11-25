//! Backing track, beat grid and transposition in the app (LH-BIB-06): the `backing_*` and
//! `transpose` calls on `Core`.

use crate::{quarters_per_bar, store, Core, SURE_ON_BEAT_SHARE};
use serde_json::json;
use std::path::PathBuf;

impl Core {
    /// Detects the beats of the backing track and lines the bars up with them (LH-BIB-06).
    pub fn backing_analyse(&mut self) -> Result<(), String> {
        let (backing, _) = self
            .backing()
            .ok_or_else(|| self.tr("backing.nobacking", &[]))?;
        let audio =
            kontra_session::BackingTrack::load(&backing, 44_100).map_err(|e| e.to_string())?;
        let mono: Vec<f32> = audio
            .left
            .iter()
            .zip(&audio.right)
            .map(|(l, r)| 0.5 * (l + r))
            .collect();
        let grid = kontra_session::track_beats(&mono, audio.sample_rate);
        if grid.beats.len() < 4 {
            return Err(self.tr("backing.nobeats", &[]));
        }
        let path = self.loaded_path().ok_or("nothing loaded")?.to_path_buf();
        self.library.beat_maps.insert(
            path,
            store::BeatMap {
                beats: grid.beats,
                bpm: grid.bpm,
                on_beat_share: grid.on_beat_share,
                ..store::BeatMap::default()
            },
        );
        self.apply_beat_map()
    }

    /// Changes the beat map of the loaded piece: `downbeat` beats later (negative: earlier),
    /// every bar by `shift_ms`, or one bar by `bar_ms`.
    pub fn backing_adjust(
        &mut self,
        downbeat: i32,
        shift_ms: f64,
        bar: Option<(usize, f64)>,
    ) -> Result<(), String> {
        let path = self.loaded_path().ok_or("nothing loaded")?.to_path_buf();
        let Some(map) = self.library.beat_maps.get_mut(&path) else {
            return Ok(());
        };
        let last = map.beats.len().saturating_sub(2) as i64;
        map.first = (map.first as i64 + i64::from(downbeat)).clamp(0, last.max(0)) as usize;
        map.shift_ms += shift_ms;
        if let Some((bar, ms)) = bar {
            *map.nudges.entry(bar).or_insert(0.0) += ms;
        }
        self.apply_beat_map()
    }

    /// Moves every bar half a beat later, or back (QA 27.10., J3: the grid sat on the offbeat).
    pub fn backing_half_beat(&mut self) -> Result<(), String> {
        let path = self.loaded_path().ok_or("nothing loaded")?.to_path_buf();
        let Some(map) = self.library.beat_maps.get_mut(&path) else {
            return Ok(());
        };
        map.half_beat = !map.half_beat;
        self.apply_beat_map()
    }

    /// Back to the piece's own tempo, the backing starting at its old offset.
    pub fn backing_clear(&mut self) -> Result<(), String> {
        let path = self.loaded_path().ok_or("nothing loaded")?.to_path_buf();
        self.library.beat_maps.remove(&path);
        if let Some(entry) = self.library.entry_mut(&path) {
            entry.backing_offset_s = 0.0;
        }
        self.apply_beat_map()
    }

    /// The beat map in words for the alignment sheet:
    /// `{"analysed", "bpm", "firstS", "mapped", "bars", "sure", "halfBeat", "text"}`. Not `sure`: kick and
    /// bass do not show where the beat is, the grid may sit on the offbeat.
    pub fn backing_json(&self) -> String {
        let Some(path) = self.loaded_path() else {
            return "{}".into();
        };
        let bars = self.arrangement().map_or(0, |a| a.bars.len());
        let Some(map) = self.library.beat_maps.get(path) else {
            return json!({ "analysed": false, "bars": bars }).to_string();
        };
        let times = self
            .arrangement()
            .map(|a| map.bar_times(&quarters_per_bar(a)))
            .unwrap_or_default();
        let first = times.first().copied().unwrap_or(0.0);
        // Moved by hand, the player has listened: no warning, but the text says so.
        let sure = map.on_beat_share >= SURE_ON_BEAT_SHARE || map.half_beat;
        let key = match (map.half_beat, sure) {
            (true, _) => "backing.aligned.half",
            (false, true) => "backing.aligned",
            (false, false) => "backing.unsure",
        };
        json!({
            "analysed": true,
            "bpm": map.bpm,
            "firstS": first,
            "mapped": times.len(),
            "bars": bars,
            "sure": sure,
            "halfBeat": map.half_beat,
            "text": self.tr(key, &[
                ("bpm", format!("{:.0}", map.bpm)),
                ("first", format!("{first:.2}")),
                ("mapped", times.len().to_string()),
                ("bars", bars.to_string()),
            ]),
        })
        .to_string()
    }

    /// Backing track of the loaded piece and its offset; without its bass when chosen and
    /// separated (LH-BIB-09).
    pub(crate) fn backing(&self) -> Option<(PathBuf, f64)> {
        let path = self.loaded_path()?;
        let entry = self.library.entry(path)?;
        let song = entry.backing.clone()?;
        let separated = entry
            .backing_without_bass
            .then(|| self.without_bass(&song))
            .flatten();
        let played = separated.unwrap_or(song);
        Some((played, entry.backing_offset_s))
    }

    /// The backing track with its bass, when it plays without it: the mixer brings the bass
    /// back from it (LH-BIB-09).
    pub(crate) fn backing_original(&self) -> Option<PathBuf> {
        let entry = self.library.entry(self.loaded_path()?)?;
        let song = entry.backing.clone()?;
        let separated = entry.backing_without_bass && self.without_bass(&song).is_some();
        separated.then_some(song).filter(|p| p.exists())
    }

    /// File name of the chosen backing track (also when it plays without its bass).
    pub fn backing_name(&self) -> String {
        let path = self.loaded_path();
        path.and_then(|p| self.library.entry(p))
            .and_then(|e| e.backing.as_ref())
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_default()
    }

    /// Largest transposition offered, for the UI.
    pub fn transpose_max() -> i32 {
        kontra_session::TRANSPOSE_MAX
    }

    /// Semitones the loaded piece and its backing track are moved by (LH-WIE-02).
    pub fn transpose(&self) -> i32 {
        self.loaded_path()
            .and_then(|p| self.library.entry(p))
            .map_or(0, |e| e.transpose_semitones)
    }

    /// Moves the loaded piece and its backing by `semitones` (±6) and lays out the notes again.
    /// A running practice goes on with the moved notes: what was judged stays, the backing
    /// shifts in the mixer. Only if the notes changed in more than pitch does the run end.
    pub fn set_transpose(&mut self, semitones: i32) -> Result<(), String> {
        let Some(loaded) = self.loaded.as_ref().map(|l| (l.path.clone(), l.track)) else {
            return Ok(());
        };
        let max = kontra_session::TRANSPOSE_MAX;
        if let Some(entry) = self.library.entry_mut(&loaded.0) {
            entry.transpose_semitones = semitones.clamp(-max, max);
        }
        self.save_library();
        // Kept out of `reload`, which would end it.
        let live = self.live.take();
        let looping = self.looping;
        let reloaded = self.reload(loaded.1);
        // The loop stays, and its own note density (LH-MOD-06) needs it back before thinning.
        self.looping = looping;
        self.rethin();
        let Some(mut live) = live else {
            return reloaded;
        };
        let moved = self.arrangement().cloned();
        let goes_on = reloaded.is_ok()
            && moved.is_some_and(|a| live.practice_mut().retarget(a, self.transpose()));
        self.live = Some(live);
        if !goes_on {
            self.stop_live();
        }
        reloaded
    }
}
