//! Whole-buffer convenience on top of the streaming [`Stretcher`].

use crate::{Stretcher, SPEED_MAX, SPEED_MIN};

/// Block size used to feed the stretcher.
const BLOCK: usize = 4096;

/// Stretches whole channels at a constant speed (0.25–1.5, clamped).
/// The result has `round(len / speed)` samples per channel and is time
/// aligned: output sample `o` corresponds to input sample `o * speed`.
/// Shorter channels are padded with silence.
pub fn stretch_buffer(channels: &[Vec<f32>], sample_rate: u32, speed: f64) -> Vec<Vec<f32>> {
    stretch_buffer_with(channels, sample_rate, speed, crate::Resolution::Standard)
}

/// `stretch_buffer` with a chosen analysis resolution.
pub fn stretch_buffer_with(
    channels: &[Vec<f32>],
    sample_rate: u32,
    speed: f64,
    resolution: crate::Resolution,
) -> Vec<Vec<f32>> {
    let speed = match speed.is_finite() {
        true => speed.clamp(SPEED_MIN, SPEED_MAX),
        false => 1.0,
    };
    let len_in = channels.iter().map(Vec::len).max().unwrap_or(0);
    let len_out = (len_in as f64 / speed).round() as usize;
    let mut out = vec![vec![0.0; len_out]; channels.len()];
    if channels.is_empty() || len_in == 0 {
        return out;
    }

    let mut st = Stretcher::with_resolution(channels.len(), sample_rate, BLOCK, resolution);
    st.set_speed(speed);

    let mut chunk = vec![vec![0.0; BLOCK]; channels.len()];
    let mut fed = 0;
    let mut made = 0;
    while made < len_out {
        let mut views: Vec<&mut [f32]> = out.iter_mut().map(|c| &mut c[made..]).collect();
        let got = st.pull(&mut views);
        made += got;
        if got > 0 {
            continue;
        }

        // Feed the next piece of input, silence once it is used up.
        let need = st.required_input().clamp(1, BLOCK);
        for (dst, src) in chunk.iter_mut().zip(channels) {
            dst[..need].fill(0.0);
            let end = (fed + need).min(src.len());
            if fed < end {
                dst[..end - fed].copy_from_slice(&src[fed..end]);
            }
        }
        let views: Vec<&[f32]> = chunk.iter().map(|c| &c[..need]).collect();
        let took = st.push(&views);
        if took == 0 {
            break;
        }
        fed += took;
    }
    out
}
