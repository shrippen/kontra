//! Stretches a WAV file, or measures speed.
//!
//! ```text
//! cargo run --release -p kontra-stretch --example stretch_wav -- in.wav out.wav 0.75
//! cargo run --release -p kontra-stretch --example stretch_wav -- --bench
//! ```
//!
//! Reads 16/24/32-bit PCM and 32-bit float WAV; writes 16-bit PCM for
//! 16-bit input, 32-bit float otherwise.

use std::fmt;
use std::fs;
use std::time::Instant;

use kontra_stretch::{stretch_buffer, Stretcher};

const FORMAT_PCM: u16 = 1;
const FORMAT_FLOAT: u16 = 3;
const FORMAT_EXTENSIBLE: u16 = 0xFFFE;

#[derive(Debug)]
enum Error {
    Usage,
    Io(std::io::Error),
    Wav(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Usage => write!(f, "usage: stretch_wav <in.wav> <out.wav> <speed> | --bench"),
            Error::Io(e) => write!(f, "{e}"),
            Error::Wav(m) => write!(f, "wav: {m}"),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

/// Sample encoding of a WAV file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Encoding {
    Pcm(u16),
    Float,
}

#[derive(Debug)]
struct Wav {
    rate: u32,
    encoding: Encoding,
    channels: Vec<Vec<f32>>,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Error> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--bench") {
        bench();
        return Ok(());
    }
    let [input, output, speed] = &args[..] else {
        return Err(Error::Usage);
    };
    let speed: f64 = speed.parse().map_err(|_| Error::Usage)?;

    let wav = read(&fs::read(input)?)?;
    let secs = wav.channels.first().map_or(0, Vec::len) as f64 / f64::from(wav.rate);

    let t = Instant::now();
    let channels = stretch_buffer(&wav.channels, wav.rate, speed);
    let took = t.elapsed().as_secs_f64();

    let out = Wav { channels, ..wav };
    fs::write(output, write(&out))?;
    println!(
        "{secs:.1} s in {took:.2} s ({:.0}x real time of the input)",
        secs / took
    );
    Ok(())
}

/// Streams 60 s of synthetic stereo at 48 kHz through push/pull in
/// 512-sample blocks and reports the real-time factor per speed.
fn bench() {
    const SR: u32 = 48_000;
    const BLOCK: usize = 512;
    let len = SR as usize * 60;
    let (left, right) = music(len, SR);

    for &speed in &[1.0, 0.75, 0.5, 0.25] {
        let mut st = Stretcher::new(2, SR, BLOCK);
        st.set_speed(speed);
        let mut out = [vec![0.0; BLOCK], vec![0.0; BLOCK]];

        let t = Instant::now();
        let (mut fed, mut made) = (0, 0);
        while fed < len {
            let [l, r] = &mut out;
            let got = st.pull(&mut [&mut l[..], &mut r[..]]);
            made += got;
            if got > 0 {
                continue;
            }
            let need = st.required_input().clamp(1, BLOCK).min(len - fed);
            fed += st.push(&[&left[fed..fed + need], &right[fed..fed + need]]);
        }
        let took = t.elapsed().as_secs_f64();
        let audio = made as f64 / f64::from(SR);
        println!(
            "speed {speed:.2}: {audio:.1} s output in {took:.3} s → {:.0}x real time",
            audio / took
        );
    }
}

/// Chord, bass and noise bursts (stand-in for a backing track).
fn music(len: usize, rate: u32) -> (Vec<f32>, Vec<f32>) {
    let mut seed = 1_u32;
    let mut left = Vec::with_capacity(len);
    let mut right = Vec::with_capacity(len);
    for n in 0..len {
        let t = n as f64 / f64::from(rate);
        let tone = |f: f64| (std::f64::consts::TAU * f * t).sin();
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let noise = f64::from(seed >> 8) / f64::from(1_u32 << 24) - 0.5;
        let beat = (-(t % 0.25) * 40.0).exp();
        let bass = 0.3 * tone(55.0);
        left.push((bass + 0.1 * tone(220.0) + 0.2 * noise * beat) as f32);
        right.push((bass + 0.1 * tone(329.6) + 0.2 * noise * beat) as f32);
    }
    (left, right)
}

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

/// Parses RIFF/WAVE: `fmt ` and `data` chunks, other chunks skipped.
fn read(b: &[u8]) -> Result<Wav, Error> {
    if b.get(0..4) != Some(b"RIFF") || b.get(8..12) != Some(b"WAVE") {
        return Err(Error::Wav("not RIFF/WAVE"));
    }

    let mut fmt: Option<(u16, u16, u32, u16)> = None;
    let mut at = 12;
    while let (Some(id), Some(size)) = (b.get(at..at + 4), u32_at(b, at + 4)) {
        let body = at + 8;
        let size = size as usize;
        let end = (body + size).min(b.len());

        if id == b"fmt " {
            let short = Error::Wav("short fmt chunk");
            let mut format = u16_at(b, body).ok_or(short)?;
            let channels = u16_at(b, body + 2).ok_or(Error::Wav("short fmt"))?;
            let rate = u32_at(b, body + 4).ok_or(Error::Wav("short fmt"))?;
            let bits = u16_at(b, body + 14).ok_or(Error::Wav("short fmt"))?;
            // Extensible: the real format is the start of the sub-format GUID.
            if format == FORMAT_EXTENSIBLE {
                format = u16_at(b, body + 24).ok_or(Error::Wav("short fmt"))?;
            }
            fmt = Some((format, channels, rate, bits));
        }

        if id == b"data" {
            let (format, channels, rate, bits) = fmt.ok_or(Error::Wav("data before fmt"))?;
            return decode(&b[body..end], format, channels, rate, bits);
        }

        // Chunks are padded to even length.
        at = body + size + (size & 1);
    }
    Err(Error::Wav("no data chunk"))
}

fn decode(data: &[u8], format: u16, channels: u16, rate: u32, bits: u16) -> Result<Wav, Error> {
    let encoding = match (format, bits) {
        (FORMAT_PCM, 16 | 24 | 32) => Encoding::Pcm(bits),
        (FORMAT_FLOAT, 32) => Encoding::Float,
        _ => return Err(Error::Wav("only 16/24/32-bit PCM or 32-bit float")),
    };
    let count = usize::from(channels.max(1));
    let width = usize::from(bits / 8);

    let mut out = vec![Vec::with_capacity(data.len() / width / count); count];
    for (i, s) in data.chunks_exact(width).enumerate() {
        out[i % count].push(sample(s, encoding));
    }
    // Drop a trailing partial frame.
    let frames = out.iter().map(Vec::len).min().unwrap_or(0);
    for ch in &mut out {
        ch.truncate(frames);
    }

    Ok(Wav {
        rate,
        encoding,
        channels: out,
    })
}

fn sample(s: &[u8], encoding: Encoding) -> f32 {
    match (encoding, s) {
        (Encoding::Float, &[a, b, c, d]) => f32::from_le_bytes([a, b, c, d]),
        (Encoding::Pcm(16), &[a, b]) => f32::from(i16::from_le_bytes([a, b])) / 32_768.0,
        (Encoding::Pcm(24), &[a, b, c]) => {
            (i32::from_le_bytes([0, a, b, c]) >> 8) as f32 / 8_388_608.0
        }
        (Encoding::Pcm(_), &[a, b, c, d]) => {
            i32::from_le_bytes([a, b, c, d]) as f32 / 2_147_483_648.0
        }
        _ => 0.0,
    }
}

/// Interleaved WAV: 16-bit PCM when the source was 16-bit, else float.
fn write(wav: &Wav) -> Vec<u8> {
    let (format, bits) = match wav.encoding {
        Encoding::Pcm(16) => (FORMAT_PCM, 16_u16),
        _ => (FORMAT_FLOAT, 32_u16),
    };
    let channels = wav.channels.len().max(1);
    let frames = wav.channels.first().map_or(0, Vec::len);
    let width = usize::from(bits / 8);
    let data_len = frames * channels * width;

    let mut b = Vec::with_capacity(44 + data_len);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&((36 + data_len) as u32).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16_u32.to_le_bytes());
    b.extend_from_slice(&format.to_le_bytes());
    b.extend_from_slice(&(channels as u16).to_le_bytes());
    b.extend_from_slice(&wav.rate.to_le_bytes());
    b.extend_from_slice(&(wav.rate * (channels * width) as u32).to_le_bytes());
    b.extend_from_slice(&((channels * width) as u16).to_le_bytes());
    b.extend_from_slice(&bits.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&(data_len as u32).to_le_bytes());

    for i in 0..frames {
        for ch in &wav.channels {
            let v = ch[i];
            match format {
                FORMAT_PCM => {
                    let q = (v.clamp(-1.0, 1.0) * 32_767.0).round() as i16;
                    b.extend_from_slice(&q.to_le_bytes());
                }
                _ => b.extend_from_slice(&v.to_le_bytes()),
            }
        }
    }
    b
}
