//! Loads a backing track for the output: decode, resample to the device rate, stereo.

use crate::EngineError;
use kontra_drivers::read_audio;
use kontra_dsp::resample;
use std::path::Path;

/// Stereo backing audio at `sample_rate` (mono files are duplicated, extra channels dropped).
#[derive(Debug, Clone, PartialEq)]
pub struct BackingTrack {
    pub sample_rate: u32,
    pub left: Vec<f32>,
    pub right: Vec<f32>,
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
        })
    }

    pub fn duration_s(&self) -> f64 {
        self.left.len() as f64 / f64::from(self.sample_rate.max(1))
    }

    /// Interleaved stereo at `speed` (time-stretched, pitch kept), ready for `Output::play_backing`.
    pub fn interleaved(&self, speed: f64) -> Vec<f32> {
        let channels = [self.left.clone(), self.right.clone()];
        let stretched = if (speed - 1.0).abs() < 1e-6 {
            channels.to_vec()
        } else {
            kontra_stretch::stretch_buffer(&channels, self.sample_rate, speed)
        };
        let frames = stretched.first().map_or(0, Vec::len);
        let mut out = Vec::with_capacity(frames * 2);
        for i in 0..frames {
            out.push(stretched[0][i]);
            out.push(stretched.get(1).map_or(0.0, |c| c[i]));
        }
        out
    }
}
