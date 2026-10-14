//! Loads a backing track for the output: decode, resample to the device rate, stereo.

use crate::{BackingAudio, EngineError};
use kontra_drivers::read_audio;
use kontra_dsp::resample;
pub use kontra_stretch::Resolution;
use std::path::Path;

/// Stereo backing audio at `sample_rate` (mono files are duplicated, extra channels dropped).
#[derive(Debug, Clone, PartialEq)]
pub struct BackingTrack {
    pub sample_rate: u32,
    pub left: Vec<f32>,
    pub right: Vec<f32>,
    /// The removed bass (left, right), when this is a song without its bass (LH-BIB-09).
    bass: Option<[Vec<f32>; 2]>,
}

impl BackingTrack {
    pub fn load(path: &Path, sample_rate: u32) -> Result<BackingTrack, EngineError> {
        let buffer = read_audio(path)?;
        let left = buffer.channels.first().cloned().unwrap_or_default();
        let right = buffer
            .channels
            .get(1)
            .cloned()
            .unwrap_or_else(|| left.clone());
        Ok(BackingTrack {
            sample_rate,
            left: resample(&left, buffer.sample_rate, sample_rate),
            right: resample(&right, buffer.sample_rate, sample_rate),
            bass: None,
        })
    }

    /// This song without its bass, plus the bass taken from `original` (the difference), so the
    /// mixer can bring it back (`Gains::bass`). Example: original 0.5, without bass 0.1 ─► 0.4.
    pub fn with_bass_of(mut self, original: &BackingTrack) -> BackingTrack {
        let difference = |with: &[f32], without: &[f32]| -> Vec<f32> {
            let with = with.iter().chain(std::iter::repeat(&0.0));
            without.iter().zip(with).map(|(w, o)| o - w).collect()
        };
        self.bass = Some([
            difference(&original.left, &self.left),
            difference(&original.right, &self.right),
        ]);
        self
    }

    pub fn duration_s(&self) -> f64 {
        self.left.len() as f64 / f64::from(self.sample_rate.max(1))
    }

    /// Interleaved at `speed` (time-stretched, pitch kept), ready for `Output::play_backing`;
    /// with the removed bass as two more channels, stretched together so they stay in step.
    /// `resolution` picks the stretch window; `Fine` for basses tuned below E1.
    pub fn interleaved(&self, speed: f64, resolution: Resolution) -> BackingAudio {
        let mut channels = vec![self.left.clone(), self.right.clone()];
        channels.extend(self.bass.iter().flatten().cloned());
        let stretched = if (speed - 1.0).abs() < 1e-6 {
            channels
        } else {
            kontra_stretch::stretch_buffer_with(&channels, self.sample_rate, speed, resolution)
        };
        let frames = stretched.first().map_or(0, Vec::len);
        let mut out = Vec::with_capacity(frames * stretched.len());
        for i in 0..frames {
            out.extend(stretched.iter().map(|c| c.get(i).copied().unwrap_or(0.0)));
        }
        match self.bass {
            Some(_) => BackingAudio::with_bass(out),
            None => BackingAudio::stereo(out),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(left: Vec<f32>) -> BackingTrack {
        BackingTrack {
            sample_rate: 1000,
            right: left.clone(),
            left,
            bass: None,
        }
    }

    #[test]
    fn removed_bass_is_the_difference() {
        let without = track(vec![0.1; 4]);
        let original = track(vec![0.5; 3]);
        let audio = without
            .with_bass_of(&original)
            .interleaved(1.0, Resolution::default());
        // Frames: L, R, bass L, bass R; at full bass it is the original (silent past its end).
        let bass = 0.5 - 0.1;
        let mut frames = [0.1, 0.1, bass, bass].repeat(3);
        frames.extend([0.1, 0.1, -0.1, -0.1]);
        assert_eq!(audio, BackingAudio::with_bass(frames));
    }
}
