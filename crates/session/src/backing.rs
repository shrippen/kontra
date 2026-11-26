//! A backing track for the output: decoded, at the device rate, stereo. Owns the engine's.

use crate::EngineError;
use kontra_engine as engine;
use kontra_model::BeatGrid;
use std::path::Path;

/// Rate the beat finder listens at.
const BEAT_RATE: u32 = 44_100;

#[derive(Debug, Clone, PartialEq)]
pub struct BackingTrack(engine::BackingTrack);

impl BackingTrack {
    /// Decodes `path` and resamples it to `sample_rate`.
    pub fn load(path: &Path, sample_rate: u32) -> Result<BackingTrack, EngineError> {
        Ok(BackingTrack(engine::BackingTrack::load(path, sample_rate)?))
    }

    /// Test: the engine's track, to look at its samples.
    #[cfg(test)]
    pub(crate) fn engine(&self) -> &engine::BackingTrack {
        &self.0
    }

    /// Keeps the track as a float WAV at its rate (the backing cache).
    pub(crate) fn save(&self, path: &Path) -> Result<(), EngineError> {
        Ok(self.0.save(path)?)
    }

    /// This song without its bass, plus the bass taken from `original` (`Gains::bass`).
    pub(crate) fn with_bass_of(self, original: &BackingTrack) -> BackingTrack {
        BackingTrack(self.0.with_bass_of(&original.0))
    }

    /// Interleaved at `speed` (pitch kept), ready for `Output::play_backing`.
    pub(crate) fn interleaved(
        &self,
        speed: f64,
        resolution: engine::Resolution,
    ) -> engine::BackingAudio {
        self.0.interleaved(speed, resolution)
    }
}

/// Tempo and beats of the audio file `path`.
pub fn beat_grid(path: &Path) -> Result<BeatGrid, EngineError> {
    let audio = engine::BackingTrack::load(path, BEAT_RATE)?;
    let mono: Vec<f32> = audio
        .left
        .iter()
        .zip(&audio.right)
        .map(|(l, r)| 0.5 * (l + r))
        .collect();
    Ok(engine::track_beats(&mono, audio.sample_rate))
}
