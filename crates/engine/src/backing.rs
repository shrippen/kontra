//! Loads a backing track for the output: decode, resample to the device rate, stereo.

use crate::EngineError;
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

    /// The same track `semitones` higher (or lower), same length, so beat maps stay valid
    /// (LH-WIE-02). Stretched by the pitch ratio, then resampled back by it:
    ///
    /// ```text
    /// +2 semitones, r = 1.122: stretch to 1.122× length ─► read at 48 000 × 1.122 Hz as 48 kHz
    /// ```
    ///
    /// `semitones` within ±6 keeps the stretch in one pass (speed 0.71–1.41).
    pub fn transposed(&self, semitones: i32, resolution: Resolution) -> BackingTrack {
        if semitones == 0 {
            return self.clone();
        }
        let ratio = 2f64.powf(f64::from(semitones) / 12.0);
        let channels = [self.left.clone(), self.right.clone()];
        let longer = kontra_stretch::stretch_buffer_with(
            &channels,
            self.sample_rate,
            1.0 / ratio,
            resolution,
        );
        let read_rate = (f64::from(self.sample_rate) * ratio).round() as u32;
        let shift = |c: Option<&Vec<f32>>| {
            let mut out = resample(
                c.map_or(&[][..], Vec::as_slice),
                read_rate,
                self.sample_rate,
            );
            out.resize(self.left.len(), 0.0);
            out
        };
        BackingTrack {
            sample_rate: self.sample_rate,
            left: shift(longer.first()),
            right: shift(longer.get(1)),
        }
    }

    /// Interleaved stereo at `speed` (time-stretched, pitch kept), ready for `Output::play_backing`.
    /// `resolution` picks the stretch window; `Fine` for basses tuned below E1.
    pub fn interleaved(&self, speed: f64, resolution: Resolution) -> Vec<f32> {
        let channels = [self.left.clone(), self.right.clone()];
        let stretched = if (speed - 1.0).abs() < 1e-6 {
            channels.to_vec()
        } else {
            kontra_stretch::stretch_buffer_with(&channels, self.sample_rate, speed, resolution)
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

#[cfg(test)]
mod tests {
    use super::*;
    use kontra_dsp::Yin;
    use std::f64::consts::PI;

    const SR: u32 = 48_000;

    fn track(hz: f64) -> BackingTrack {
        let sine: Vec<f32> = (0..SR as usize)
            .map(|i| (2.0 * PI * hz * i as f64 / f64::from(SR)).sin() as f32 * 0.5)
            .collect();
        BackingTrack {
            sample_rate: SR,
            left: sine.clone(),
            right: sine,
        }
    }

    fn cents(samples: &[f32], expected_hz: f64) -> f64 {
        let mut yin = Yin::new(f64::from(SR), 50.0, 500.0, 0.04);
        let hz = yin.estimate(samples).map_or(0.0, |e| e.hz);
        1200.0 * (hz / expected_hz).log2()
    }

    #[test]
    fn transposing_changes_pitch_not_length() {
        let a2 = track(110.0);
        for semitones in [-6, -2, 3, 6] {
            let shifted = a2.transposed(semitones, Resolution::Standard);
            assert_eq!(shifted.left.len(), a2.left.len());
            let expected = 110.0 * 2f64.powf(f64::from(semitones) / 12.0);
            let off = cents(&shifted.left[12_000..36_000], expected);
            assert!(off.abs() < 5.0, "{semitones}: {off} cents");
        }
    }

    #[test]
    fn zero_is_the_same_track() {
        let a2 = track(110.0);
        assert_eq!(a2.transposed(0, Resolution::Standard), a2);
    }
}
