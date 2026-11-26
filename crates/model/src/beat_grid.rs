//! Tempo and beats found in a recording.

/// Tempo and beat times (seconds) of a recording.
#[derive(Debug, Clone, PartialEq)]
pub struct BeatGrid {
    pub bpm: f64,
    pub beats: Vec<f64>,
    /// Share (0–1) of the kick and bass onsets on the beats rather than half a beat off. Near 0.5
    /// (or 0 without any): the grid may sit on the offbeat, the player should check by ear.
    pub on_beat_share: f64,
}
