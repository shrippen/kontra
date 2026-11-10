//! Streams in the device's own sample format. Kontra's audio code speaks f32, and WASAPI,
//! PipeWire and ALSA give it; an ASIO driver wants its own (Focusrite USB ASIO: i32,
//! "Sample format f32 is not supported; expected i32", Win2, 10.11.2026).
//!
//! ```text
//! device (i32) ─► to_f32 ─► callback(&[f32])          input
//! callback(&mut [f32]) ─► from_f32 ─► device (i32)    output
//! ```
//!
//! The scratch buffer is made before the stream starts: no allocation on the audio thread.

use cpal::traits::DeviceTrait;
use cpal::{FromSample, Sample, SampleFormat, SizedSample};

/// Samples converted per call. A device block larger than this (32768 stereo frames; no
/// driver asks for that) is handed on in parts.
const SCRATCH: usize = 1 << 16;

pub(super) fn build_input<F, E>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    format: SampleFormat,
    callback: F,
    on_error: E,
) -> Result<cpal::Stream, cpal::Error>
where
    F: FnMut(&[f32], &cpal::InputCallbackInfo) + Send + 'static,
    E: FnMut(cpal::Error) + Send + 'static,
{
    match format {
        SampleFormat::I16 => converted_input::<i16, _, _>(device, config, callback, on_error),
        SampleFormat::I32 => converted_input::<i32, _, _>(device, config, callback, on_error),
        // f32, or a format Kontra does not convert: cpal says what the device wants.
        _ => device.build_input_stream(config, callback, on_error, None),
    }
}

pub(super) fn build_output<F, E>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    format: SampleFormat,
    callback: F,
    on_error: E,
) -> Result<cpal::Stream, cpal::Error>
where
    F: FnMut(&mut [f32], &cpal::OutputCallbackInfo) + Send + 'static,
    E: FnMut(cpal::Error) + Send + 'static,
{
    match format {
        SampleFormat::I16 => converted_output::<i16, _, _>(device, config, callback, on_error),
        SampleFormat::I32 => converted_output::<i32, _, _>(device, config, callback, on_error),
        _ => device.build_output_stream(config, callback, on_error, None),
    }
}

fn converted_input<T, F, E>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    mut callback: F,
    on_error: E,
) -> Result<cpal::Stream, cpal::Error>
where
    T: SizedSample + Send + 'static,
    f32: FromSample<T>,
    F: FnMut(&[f32], &cpal::InputCallbackInfo) + Send + 'static,
    E: FnMut(cpal::Error) + Send + 'static,
{
    let mut scratch = vec![0.0f32; SCRATCH];
    let convert = move |data: &[T], info: &cpal::InputCallbackInfo| {
        for part in data.chunks(SCRATCH) {
            let heard = &mut scratch[..part.len()];
            to_f32(part, heard);
            callback(heard, info);
        }
    };
    device.build_input_stream(config, convert, on_error, None)
}

fn converted_output<T, F, E>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    mut callback: F,
    on_error: E,
) -> Result<cpal::Stream, cpal::Error>
where
    T: SizedSample + FromSample<f32> + Send + 'static,
    F: FnMut(&mut [f32], &cpal::OutputCallbackInfo) + Send + 'static,
    E: FnMut(cpal::Error) + Send + 'static,
{
    let mut scratch = vec![0.0f32; SCRATCH];
    let convert = move |data: &mut [T], info: &cpal::OutputCallbackInfo| {
        for part in data.chunks_mut(SCRATCH) {
            let played = &mut scratch[..part.len()];
            callback(played, info);
            from_f32(played, part);
        }
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
