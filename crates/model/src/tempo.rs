//! Tempo map: converts ticks to seconds and back.

use crate::arrangement::TICKS_PER_QUARTER;
use serde::{Deserialize, Serialize};

const SECONDS_PER_MINUTE: f64 = 60.0;
const DEFAULT_BPM: f64 = 120.0;

/// A tempo that applies from `tick` on, in quarter notes per minute.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TempoChange {
    pub tick: u64,
    pub bpm: f64,
}

/// Sorted tempo changes; the first one starts at tick 0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TempoMap {
    changes: Vec<TempoChange>,
}

impl Default for TempoMap {
    fn default() -> Self {
        TempoMap::constant(DEFAULT_BPM)
    }
}

impl TempoMap {
    pub fn constant(bpm: f64) -> TempoMap {
        TempoMap {
            changes: vec![TempoChange {
                tick: 0,
                bpm: sane_bpm(bpm),
            }],
        }
    }

    /// Builds a map from unsorted changes; adds tick 0 with the first tempo if missing.
    pub fn from_changes(mut changes: Vec<TempoChange>) -> TempoMap {
        changes.retain(|c| c.bpm.is_finite() && c.bpm > 0.0);
        changes.sort_by_key(|c| c.tick);
        changes.dedup_by_key(|c| c.tick);
        let first_bpm = changes.first().map_or(DEFAULT_BPM, |c| c.bpm);
        if changes.first().is_none_or(|c| c.tick != 0) {
            changes.insert(
                0,
                TempoChange {
                    tick: 0,
                    bpm: first_bpm,
                },
            );
        }
        TempoMap { changes }
    }

    pub fn changes(&self) -> &[TempoChange] {
        &self.changes
    }

    /// Tempo in effect at `tick`.
    pub fn bpm_at(&self, tick: u64) -> f64 {
        self.changes
            .iter()
            .take_while(|c| c.tick <= tick)
            .last()
            .map_or(DEFAULT_BPM, |c| c.bpm)
    }

    /// Seconds from tick 0 to `tick`.
    pub fn seconds_at(&self, tick: u64) -> f64 {
        let mut seconds = 0.0;
        for (i, change) in self.changes.iter().enumerate() {
            if change.tick >= tick {
                break;
            }
            let end = self
                .changes
                .get(i + 1)
                .map_or(tick, |next| next.tick.min(tick));
            seconds += ticks_to_seconds(end - change.tick, change.bpm);
        }
        seconds
    }

    /// Tick at `seconds` (rounded down).
    pub fn tick_at(&self, seconds: f64) -> u64 {
        if seconds <= 0.0 {
            return 0;
        }
        let mut elapsed = 0.0;
        for (i, change) in self.changes.iter().enumerate() {
            let segment = self
                .changes
                .get(i + 1)
                .map(|next| ticks_to_seconds(next.tick - change.tick, change.bpm));
            match segment {
                Some(len) if elapsed + len <= seconds => elapsed += len,
                _ => {
                    let ticks = (seconds - elapsed) * change.bpm / SECONDS_PER_MINUTE
                        * TICKS_PER_QUARTER as f64;
                    return change.tick + ticks as u64;
                }
            }
        }
        0
    }
}

fn sane_bpm(bpm: f64) -> f64 {
    if bpm.is_finite() && bpm > 0.0 {
        bpm
    } else {
        DEFAULT_BPM
    }
}

fn ticks_to_seconds(ticks: u64, bpm: f64) -> f64 {
    ticks as f64 / TICKS_PER_QUARTER as f64 * SECONDS_PER_MINUTE / bpm
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_tempo() {
        let map = TempoMap::constant(120.0);
        assert!((map.seconds_at(TICKS_PER_QUARTER * 4) - 2.0).abs() < 1e-9);
        assert_eq!(map.tick_at(2.0), TICKS_PER_QUARTER * 4);
    }

    #[test]
    fn tempo_change_roundtrip() {
        let map = TempoMap::from_changes(vec![
            TempoChange {
                tick: TICKS_PER_QUARTER * 4,
                bpm: 60.0,
            },
            TempoChange {
                tick: 0,
                bpm: 120.0,
            },
        ]);
        // 4 quarters at 120 = 2 s, then 2 quarters at 60 = 2 s.
        let tick = TICKS_PER_QUARTER * 6;
        assert!((map.seconds_at(tick) - 4.0).abs() < 1e-9);
        assert_eq!(map.tick_at(4.0), tick);
        assert_eq!(map.bpm_at(tick), 60.0);
    }

    #[test]
    fn inserts_start() {
        let map = TempoMap::from_changes(vec![TempoChange {
            tick: 100,
            bpm: 90.0,
        }]);
        assert_eq!(map.changes()[0].tick, 0);
        assert_eq!(map.bpm_at(0), 90.0);
    }
}
