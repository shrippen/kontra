//! Streams in the device's own sample format. Kontra's audio code speaks f32, and WASAPI,
//! PipeWire and ALSA give it; an ASIO driver wants its own (Focusrite USB ASIO: i32,
//! "Sample format f32 is not supported; expected i32", Win2, 10.11.2026).
//!
//! ```text
//! device (i32) ─► to_f32 ─► callback(&[f32])          input
//! callback(&mut [f32]) ─► from_f32 ─► device (i32)    output
//! ```
//!
//! The scratch buffer is made before the stream starts, for the largest block the device
//! names: no allocation on the audio thread. A block that is larger still is cut to the
//! frames that fit (never split: the callback sees each device block once, so its time stamp
//! and the dropout check stay right); what does not fit is lost like a dropout.

use cpal::traits::DeviceTrait;
use cpal::{FromSample, Sample, SampleFormat, SizedSample};

/// Frames the scratch buffer holds when the device does not say how large its blocks get.
const UNKNOWN_BLOCK_FRAMES: usize = 16_384;
/// The most frames it is made for, whatever a driver reports.
const MAX_BLOCK_FRAMES: usize = 65_536;

/// Samples of the scratch buffer for `channels` (an ASIO driver delivers all its channels,
/// 18 or more on large interfaces) and the device's largest block, if it names one.
pub(super) fn scratch_len(channels: u16, largest_block_frames: Option<u32>) -> usize {
    let frames = largest_block_frames.map_or(UNKNOWN_BLOCK_FRAMES, |f| f as usize);
    usize::from(channels.max(1)) * frames.clamp(1, MAX_BLOCK_FRAMES)
}

/// How many samples of a block of `len` are converted: all, or the whole frames that fit.
fn fitting(len: usize, channels: usize, scratch: usize) -> usize {
    let most = len.min(scratch);
    most - most % channels.max(1)
}

pub(super) fn build_input<F, E>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    (format, scratch): (SampleFormat, usize),
    callback: F,
    on_error: E,
) -> Result<cpal::Stream, cpal::Error>
where
    F: FnMut(&[f32], &cpal::InputCallbackInfo) + Send + 'static,
    E: FnMut(cpal::Error) + Send + 'static,
{
    match format {
        SampleFormat::I16 => {
            converted_input::<i16, _, _>(device, config, scratch, callback, on_error)
        }
        SampleFormat::I32 => {
            converted_input::<i32, _, _>(device, config, scratch, callback, on_error)
        }
        // f32, or a format Kontra does not convert: cpal says what the device wants.
        _ => device.build_input_stream(config, callback, on_error, None),
    }
}

pub(super) fn build_output<F, E>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    (format, scratch): (SampleFormat, usize),
    callback: F,
    on_error: E,
) -> Result<cpal::Stream, cpal::Error>
where
    F: FnMut(&mut [f32], &cpal::OutputCallbackInfo) + Send + 'static,
    E: FnMut(cpal::Error) + Send + 'static,
{
    match format {
        SampleFormat::I16 => {
            converted_output::<i16, _, _>(device, config, scratch, callback, on_error)
        }
        SampleFormat::I32 => {
            converted_output::<i32, _, _>(device, config, scratch, callback, on_error)
        }
        _ => device.build_output_stream(config, callback, on_error, None),
    }
}

fn converted_input<T, F, E>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    scratch: usize,
    mut callback: F,
    on_error: E,
) -> Result<cpal::Stream, cpal::Error>
where
    T: SizedSample + Send + 'static,
    f32: FromSample<T>,
    F: FnMut(&[f32], &cpal::InputCallbackInfo) + Send + 'static,
    E: FnMut(cpal::Error) + Send + 'static,
{
    let channels = usize::from(config.channels);
    let mut scratch = vec![0.0f32; scratch];
    let convert = move |data: &[T], info: &cpal::InputCallbackInfo| {
        let fits = fitting(data.len(), channels, scratch.len());
        let heard = &mut scratch[..fits];
        to_f32(&data[..fits], heard);
        callback(heard, info);
    };
    device.build_input_stream(config, convert, on_error, None)
}

fn converted_output<T, F, E>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    scratch: usize,
    mut callback: F,
    on_error: E,
) -> Result<cpal::Stream, cpal::Error>
where
    T: SizedSample + FromSample<f32> + Send + 'static,
    F: FnMut(&mut [f32], &cpal::OutputCallbackInfo) + Send + 'static,
    E: FnMut(cpal::Error) + Send + 'static,
{
    let channels = usize::from(config.channels);
    let mut scratch = vec![0.0f32; scratch];
    let convert = move |data: &mut [T], info: &cpal::OutputCallbackInfo| {
        let fits = fitting(data.len(), channels, scratch.len());
        let (filled, beyond) = data.split_at_mut(fits);
        let played = &mut scratch[..fits];
        callback(played, info);
        from_f32(played, filled);
        // What the scratch buffer cannot hold stays silent.
        beyond.fill(T::EQUILIBRIUM);
    };
    device.build_output_stream(config, convert, on_error, None)
}

/// Device samples as f32: full scale is ±1.
fn to_f32<T>(from: &[T], to: &mut [f32])
where
    T: Sample,
    f32: FromSample<T>,
{
    for (out, sample) in to.iter_mut().zip(from) {
        *out = f32::from_sample(*sample);
    }
}

/// f32 as device samples; above full scale it clips.
fn from_f32<T>(from: &[f32], to: &mut [T])
where
    T: Sample + FromSample<f32>,
{
    for (out, sample) in to.iter_mut().zip(from) {
        *out = T::from_sample(*sample);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Review #247: a block larger than the scratch buffer was handed on in parts, with one
    /// time stamp for all and, with 18 channels, split in the middle of a frame. Now the buffer
    /// follows channels and block size, and a block beyond it is cut to whole frames.
    #[test]
    fn the_scratch_buffer_holds_whole_blocks() {
        // 18 channels at 4096 frames (a large interface over ASIO) fit.
        assert_eq!(scratch_len(18, Some(4096)), 18 * 4096);
        assert_eq!(scratch_len(2, None), 2 * UNKNOWN_BLOCK_FRAMES);
        assert_eq!(scratch_len(2, Some(u32::MAX)), 2 * MAX_BLOCK_FRAMES);
        assert_eq!(scratch_len(0, Some(0)), 1);

        assert_eq!(
            fitting(18 * 4096, 18, scratch_len(18, Some(4096))),
            18 * 4096
        );
        // Larger than the buffer: whole frames only, never half of one.
        assert_eq!(fitting(100, 18, 64), 54);
        assert_eq!(fitting(10, 2, 64), 10);
    }

    /// Win2: the Focusrite ASIO driver speaks i32; Kontra's f32 streams did not open on it.
    #[test]
    fn device_samples_become_f32_and_back() {
        let mut heard = [0.0f32; 3];
        to_f32(&[i32::MAX, 0, i32::MIN / 2], &mut heard);
        assert!(
            (heard[0] - 1.0).abs() < 1e-6 && heard[1] == 0.0,
            "{heard:?}"
        );
        assert!((heard[2] + 0.5).abs() < 1e-6, "{heard:?}");

        let mut played = [0i16; 4];
        from_f32(&[1.0, -1.0, 0.5, 2.0], &mut played);
        assert_eq!(played[1], i16::MIN);
        assert!((played[2] - 16384).abs() <= 1, "{played:?}");
        assert_eq!(played[3], played[0], "above full scale clips");
        assert!(played[0] >= i16::MAX - 1);
    }
}
