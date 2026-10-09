//! Sound devices through cpal. Callbacks run on the audio thread: they must not allocate or
//! block. Input is delivered as mono f32 (one chosen channel or the average), output as
//! interleaved f32.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::fmt;
use thiserror::Error;

/// Largest block a callback receives at once; bigger device buffers are split.
const MAX_BLOCK: usize = 8192;

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("no audio device found for {0:?}")]
    NoDevice(Direction),
    #[error("audio device {name}: {message}")]
    Device { name: String, message: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Input,
    Output,
}

/// How a device is connected; used for hints (Bluetooth adds latency, USB is likely an interface).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Connection {
    BuiltIn,
    Usb,
    Bluetooth,
    Hdmi,
    Other,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub direction: Direction,
    pub is_default: bool,
    pub connection: Connection,
    pub sample_rate: u32,
    pub channels: u16,
}

/// What to open; `None` fields use the device defaults.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StreamConfig {
    pub device_id: Option<String>,
    pub sample_rate: Option<u32>,
    pub buffer_frames: Option<u32>,
    /// Input only: which channel carries the instrument; `None` averages all channels.
    pub input_channel: Option<u16>,
}

/// Lists input or output devices of the default host.
pub fn list_devices(direction: Direction) -> Vec<DeviceInfo> {
    let host = cpal::default_host();
    let default_id = match direction {
        Direction::Input => host.default_input_device(),
        Direction::Output => host.default_output_device(),
    }
    .and_then(|d| d.id().ok())
    .map(|id| id.to_string());
    let devices = match direction {
        Direction::Input => host.input_devices().map(|d| d.collect::<Vec<_>>()),
        Direction::Output => host.output_devices().map(|d| d.collect::<Vec<_>>()),
    };
    devices
        .unwrap_or_default()
        .iter()
        .filter_map(|d| describe(d, direction, default_id.as_deref()))
        .collect()
}

fn describe(
    device: &cpal::Device,
    direction: Direction,
    default_id: Option<&str>,
) -> Option<DeviceInfo> {
    let id = device.id().ok()?.to_string();
    let description = device.description().ok()?;
    let config = match direction {
        Direction::Input => device.default_input_config(),
        Direction::Output => device.default_output_config(),
    }
    .ok()?;
    let connection = match description.interface_type() {
        cpal::InterfaceType::BuiltIn | cpal::InterfaceType::Pci => Connection::BuiltIn,
        cpal::InterfaceType::Usb => Connection::Usb,
        cpal::InterfaceType::Bluetooth => Connection::Bluetooth,
        cpal::InterfaceType::Hdmi => Connection::Hdmi,
        _ => guess_connection(description.name()),
    };
    Some(DeviceInfo {
        is_default: default_id == Some(id.as_str()),
        id,
        name: description.name().to_string(),
        direction,
        connection,
        sample_rate: config.sample_rate(),
        channels: config.channels(),
    })
}

fn guess_connection(name: &str) -> Connection {
    let lower = name.to_lowercase();
    if lower.contains("bluez") || lower.contains("bluetooth") {
        return Connection::Bluetooth;
    }
    if lower.contains("usb") || lower.contains("scarlett") || lower.contains("focusrite") {
        return Connection::Usb;
    }
    if lower.contains("hdmi") || lower.contains("displayport") {
        return Connection::Hdmi;
    }
    Connection::Other
}

fn find_device(direction: Direction, id: Option<&str>) -> Result<cpal::Device, AudioError> {
    let host = cpal::default_host();
    let fallback = || match direction {
        Direction::Input => host.default_input_device(),
        Direction::Output => host.default_output_device(),
    };
    let Some(id) = id else {
        return fallback().ok_or(AudioError::NoDevice(direction));
    };
    let devices = match direction {
        Direction::Input => host.input_devices().map(|d| d.collect::<Vec<_>>()),
        Direction::Output => host.output_devices().map(|d| d.collect::<Vec<_>>()),
    };
    devices
        .unwrap_or_default()
        .into_iter()
        .find(|d| d.id().map(|i| i.to_string() == id).unwrap_or(false))
        .ok_or(AudioError::NoDevice(direction))
}

fn device_error(device: &cpal::Device, error: impl fmt::Display) -> AudioError {
    let name = device
        .description()
        .map(|d| d.name().to_string())
        .unwrap_or_else(|_| "?".to_string());
    AudioError::Device {
        name,
        message: error.to_string(),
    }
}

fn stream_config(
    device: &cpal::Device,
    direction: Direction,
    wanted: &StreamConfig,
) -> Result<cpal::StreamConfig, AudioError> {
    let default = match direction {
        Direction::Input => device.default_input_config(),
        Direction::Output => device.default_output_config(),
    }
    .map_err(|e| device_error(device, e))?;
    Ok(cpal::StreamConfig {
        channels: default.channels(),
        sample_rate: wanted.sample_rate.unwrap_or(default.sample_rate()),
        buffer_size: wanted
            .buffer_frames
            .map_or(cpal::BufferSize::Default, cpal::BufferSize::Fixed),
    })
}

/// A running input stream; dropping it stops capture.
pub struct InputStream {
    _stream: cpal::Stream,
    pub sample_rate: u32,
    pub channels: u16,
    pub device_name: String,
}

impl fmt::Debug for InputStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InputStream")
            .field("sample_rate", &self.sample_rate)
            .field("device", &self.device_name)
            .finish()
    }
}

impl InputStream {
    /// Opens capture; `on_samples` gets mono blocks on the audio thread, `on_error` reports
    /// device errors (e.g. unplugged interface).
    pub fn open<F, E>(
        config: &StreamConfig,
        mut on_samples: F,
        mut on_error: E,
    ) -> Result<InputStream, AudioError>
    where
        F: FnMut(&[f32]) + Send + 'static,
        E: FnMut(String) + Send + 'static,
    {
        let device = find_device(Direction::Input, config.device_id.as_deref())?;
        let stream_config = stream_config(&device, Direction::Input, config)?;
        let channels = usize::from(stream_config.channels.max(1));
        let pick = config
            .input_channel
            .map(usize::from)
            .filter(|c| *c < channels);
        let mut mono = vec![0.0f32; MAX_BLOCK];
        let callback = move |data: &[f32], _: &cpal::InputCallbackInfo| {
            for chunk in data.chunks(MAX_BLOCK * channels) {
                let frames = chunk.len() / channels;
                for (i, frame) in chunk.chunks_exact(channels).enumerate() {
                    mono[i] = match pick {
                        Some(c) => frame[c],
                        None => frame.iter().sum::<f32>() / channels as f32,
                    };
                }
                on_samples(&mono[..frames]);
            }
        };
        let stream = device
            .build_input_stream(
                stream_config,
                callback,
                move |e| on_error(e.to_string()),
                None,
            )
            .map_err(|e| device_error(&device, e))?;
        stream.play().map_err(|e| device_error(&device, e))?;
        Ok(InputStream {
            _stream: stream,
            sample_rate: stream_config.sample_rate,
            channels: stream_config.channels,
            device_name: device
                .description()
                .map(|d| d.name().to_string())
                .unwrap_or_default(),
        })
    }
}

/// A running output stream; dropping it stops playback.
pub struct OutputStream {
    _stream: cpal::Stream,
    pub sample_rate: u32,
    pub channels: u16,
    pub device_name: String,
}

impl fmt::Debug for OutputStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OutputStream")
            .field("sample_rate", &self.sample_rate)
            .field("device", &self.device_name)
            .finish()
    }
}

impl OutputStream {
    /// Opens playback; `fill` writes interleaved samples (`channels` per frame) on the audio thread.
    pub fn open<F, E>(
        config: &StreamConfig,
        mut fill: F,
        mut on_error: E,
    ) -> Result<OutputStream, AudioError>
    where
        F: FnMut(&mut [f32], usize) + Send + 'static,
        E: FnMut(String) + Send + 'static,
    {
        let device = find_device(Direction::Output, config.device_id.as_deref())?;
        let stream_config = stream_config(&device, Direction::Output, config)?;
        let channels = usize::from(stream_config.channels.max(1));
        let callback = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| fill(data, channels);
        let stream = device
            .build_output_stream(
                stream_config,
                callback,
                move |e| on_error(e.to_string()),
                None,
            )
            .map_err(|e| device_error(&device, e))?;
        stream.play().map_err(|e| device_error(&device, e))?;
        Ok(OutputStream {
            _stream: stream,
            sample_rate: stream_config.sample_rate,
            channels: stream_config.channels,
            device_name: device
                .description()
                .map(|d| d.name().to_string())
                .unwrap_or_default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guesses_connections() {
        assert_eq!(
            guess_connection("bluez_output.AA_BB"),
            Connection::Bluetooth
        );
        assert_eq!(guess_connection("Scarlett Solo USB"), Connection::Usb);
        assert_eq!(guess_connection("HDA Intel HDMI 0"), Connection::Hdmi);
    }

    #[test]
    fn listing_without_hardware_does_not_panic() {
        let _ = list_devices(Direction::Input);
        let _ = list_devices(Direction::Output);
    }
}
