//! Audio files: decode WAV, FLAC, Ogg Vorbis and MP3 to f32; write WAV.

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FileError {
    #[error("cannot open {path}: {source}")]
    Open {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("{path}: unsupported audio format (supported: wav, flac, ogg, mp3)")]
    Unsupported { path: PathBuf },
    #[error("{path}: cannot decode: {message}")]
    Decode { path: PathBuf, message: String },
    #[error("{path}: no audio in file")]
    Empty { path: PathBuf },
}

/// Decoded audio, one vector per channel.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioBuffer {
    pub sample_rate: u32,
    pub channels: Vec<Vec<f32>>,
}

impl AudioBuffer {
    pub fn frames(&self) -> usize {
        self.channels.first().map_or(0, Vec::len)
    }

    pub fn duration_s(&self) -> f64 {
        self.frames() as f64 / f64::from(self.sample_rate.max(1))
    }

    /// Average of all channels.
    pub fn mono(&self) -> Vec<f32> {
        let count = self.channels.len().max(1) as f32;
        (0..self.frames())
            .map(|i| self.channels.iter().map(|c| c[i]).sum::<f32>() / count)
            .collect()
    }

    fn from_interleaved(sample_rate: u32, channels: usize, data: &[f32]) -> AudioBuffer {
        let channels = channels.max(1);
        let mut out = vec![Vec::with_capacity(data.len() / channels); channels];
        for frame in data.chunks_exact(channels) {
            for (c, s) in frame.iter().enumerate() {
                out[c].push(*s);
            }
        }
        AudioBuffer {
            sample_rate,
            channels: out,
        }
    }
}

/// Reads an audio file, chosen by extension.
pub fn read_audio(path: &Path) -> Result<AudioBuffer, FileError> {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let buffer = match extension.as_str() {
        "wav" | "wave" => read_wav(path)?,
        "flac" => read_flac(path)?,
        "ogg" | "oga" => read_ogg(path)?,
        "mp3" => read_mp3(path)?,
        _ => {
            return Err(FileError::Unsupported {
                path: path.to_path_buf(),
            })
        }
    };
    if buffer.frames() == 0 {
        return Err(FileError::Empty {
            path: path.to_path_buf(),
        });
    }
    Ok(buffer)
}

fn open(path: &Path) -> Result<BufReader<File>, FileError> {
    File::open(path)
        .map(BufReader::new)
        .map_err(|source| FileError::Open {
            path: path.to_path_buf(),
            source,
        })
}

fn decode_error(path: &Path, error: impl std::fmt::Display) -> FileError {
    FileError::Decode {
        path: path.to_path_buf(),
        message: error.to_string(),
    }
}

fn read_wav(path: &Path) -> Result<AudioBuffer, FileError> {
    let reader = hound::WavReader::new(open(path)?).map_err(|e| decode_error(path, e))?;
    let spec = reader.spec();
    let data: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .into_samples::<f32>()
            .collect::<Result<_, _>>()
            .map_err(|e| decode_error(path, e))?,
        hound::SampleFormat::Int => {
            let scale = 1.0 / (1u64 << (spec.bits_per_sample.clamp(1, 32) - 1)) as f32;
            reader
                .into_samples::<i32>()
                .map(|s| s.map(|v| v as f32 * scale))
                .collect::<Result<_, _>>()
                .map_err(|e| decode_error(path, e))?
        }
    };
    Ok(AudioBuffer::from_interleaved(
        spec.sample_rate,
        usize::from(spec.channels),
        &data,
    ))
}

fn read_flac(path: &Path) -> Result<AudioBuffer, FileError> {
    let mut reader = claxon::FlacReader::new(open(path)?).map_err(|e| decode_error(path, e))?;
    let info = reader.streaminfo();
    let scale = 1.0 / (1u64 << (info.bits_per_sample.clamp(1, 32) - 1)) as f32;
    let data: Vec<f32> = reader
        .samples()
        .map(|s| s.map(|v| v as f32 * scale))
        .collect::<Result<_, _>>()
        .map_err(|e| decode_error(path, e))?;
    Ok(AudioBuffer::from_interleaved(
        info.sample_rate,
        info.channels as usize,
        &data,
    ))
}

fn read_ogg(path: &Path) -> Result<AudioBuffer, FileError> {
    let mut reader =
        lewton::inside_ogg::OggStreamReader::new(open(path)?).map_err(|e| decode_error(path, e))?;
    let sample_rate = reader.ident_hdr.audio_sample_rate;
    let channels = usize::from(reader.ident_hdr.audio_channels);
    let mut data = Vec::new();
    while let Some(packet) = reader
        .read_dec_packet_itl()
        .map_err(|e| decode_error(path, e))?
    {
        data.extend(packet.iter().map(|s| f32::from(*s) / 32_768.0));
    }
    Ok(AudioBuffer::from_interleaved(sample_rate, channels, &data))
}

fn read_mp3(path: &Path) -> Result<AudioBuffer, FileError> {
    let mut decoder = minimp3::Decoder::new(open(path)?);
    let mut data = Vec::new();
    let mut format = None;
    loop {
        match decoder.next_frame() {
            Ok(frame) => {
                format.get_or_insert((frame.sample_rate, frame.channels));
                data.extend(frame.data.iter().map(|s| f32::from(*s) / 32_768.0));
            }
            Err(minimp3::Error::Eof) => break,
            Err(minimp3::Error::SkippedData) => continue,
            Err(e) => return Err(decode_error(path, e)),
        }
    }
    let Some((sample_rate, channels)) = format else {
        return Err(FileError::Empty {
            path: path.to_path_buf(),
        });
    };
    Ok(AudioBuffer::from_interleaved(
        sample_rate.max(0) as u32,
        channels,
        &data,
    ))
}

/// Writes 32-bit float WAV.
pub fn write_wav(path: &Path, buffer: &AudioBuffer) -> Result<(), FileError> {
    let spec = hound::WavSpec {
        channels: buffer.channels.len().max(1) as u16,
        sample_rate: buffer.sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(path, spec).map_err(|e| decode_error(path, e))?;
    for i in 0..buffer.frames() {
        for channel in &buffer.channels {
            writer
                .write_sample(channel[i])
                .map_err(|e| decode_error(path, e))?;
        }
    }
    writer.finalize().map_err(|e| decode_error(path, e))
}

/// Writes 16-bit WAV: half the size of float, for audio that is only listened to (a cache).
pub fn write_wav_16(path: &Path, buffer: &AudioBuffer) -> Result<(), FileError> {
    let spec = hound::WavSpec {
        channels: buffer.channels.len().max(1) as u16,
        sample_rate: buffer.sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).map_err(|e| decode_error(path, e))?;
    let full = f32::from(i16::MAX);
    for i in 0..buffer.frames() {
        for channel in &buffer.channels {
            let sample = (channel[i].clamp(-1.0, 1.0) * full).round() as i16;
            writer
                .write_sample(sample)
                .map_err(|e| decode_error(path, e))?;
        }
    }
    writer.finalize().map_err(|e| decode_error(path, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_roundtrip() {
        let path = std::env::temp_dir().join(format!("kontra-drivers-{}.wav", std::process::id()));
        let buffer = AudioBuffer {
            sample_rate: 48_000,
            channels: vec![vec![0.0, 0.5, -0.5], vec![0.25, 0.0, 1.0]],
        };
        write_wav(&path, &buffer).unwrap();
        let back = read_audio(&path).unwrap();
        assert_eq!(back, buffer);
        assert_eq!(back.mono(), vec![0.125, 0.25, 0.25]);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn unsupported_and_missing() {
        assert!(matches!(
            read_audio(Path::new("x.aiff")),
            Err(FileError::Unsupported { .. })
        ));
        assert!(matches!(
            read_audio(Path::new("/nonexistent.wav")),
            Err(FileError::Open { .. })
        ));
    }
}
