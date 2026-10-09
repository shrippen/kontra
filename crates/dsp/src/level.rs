//! Levels in dBFS.

/// Level reported for digital silence.
pub const SILENCE_DB: f32 = -120.0;

pub fn amplitude_to_db(amplitude: f32) -> f32 {
    if amplitude <= 0.0 {
        return SILENCE_DB;
    }
    (20.0 * amplitude.log10()).max(SILENCE_DB)
}

pub fn peak_db(samples: &[f32]) -> f32 {
    let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    amplitude_to_db(peak)
}

pub fn rms_db(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return SILENCE_DB;
    }
    let sum: f64 = samples.iter().map(|s| f64::from(*s) * f64::from(*s)).sum();
    amplitude_to_db((sum / samples.len() as f64).sqrt() as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_scale_sine_rms() {
        let sine: Vec<f32> = (0..4800).map(|i| (i as f32 * 0.1).sin()).collect();
        assert!((rms_db(&sine) + 3.01).abs() < 0.1);
        assert!(peak_db(&sine).abs() < 0.01);
        assert_eq!(rms_db(&[]), SILENCE_DB);
    }
}
