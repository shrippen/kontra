//! Sound devices through cpal. Callbacks run on the audio thread: they must not allocate or
//! block. Input is delivered as mono f32 (the chosen channel or the loudest), output as
//! interleaved f32.

#[cfg(target_os = "linux")]
use crate::pipewire::Graph;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;

/// Largest block a callback receives at once; bigger device buffers are split.
const MAX_BLOCK: usize = 8192;
/// Output device id of a timer-driven output without sound (endurance tests, LH-NF-04).
pub const NULL_DEVICE: &str = "kontra:null";
/// The null output's block and rate: a typical PipeWire quantum.
const NULL_BLOCK: usize = 256;
const NULL_RATE: u32 = 48_000;
const NULL_CHANNELS: u16 = 2;
/// A callback this much later than one block after the previous one means the device ran dry
/// (with the usual two periods of buffer); the slack absorbs scheduler jitter.
const LATE_BLOCKS: f64 = 2.0;
const LATE_SLACK: Duration = Duration::from_millis(2);

/// Counts dropouts (LH-NF-04): xruns the audio backend reports and callbacks that came so
/// late that the buffer must have run empty. Lives in the callback: no allocation, no lock.
#[derive(Debug, Clone, Default)]
struct XrunCounter {
    count: Arc<AtomicU64>,
}

impl XrunCounter {
    fn add(&self) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    fn get(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }

    /// Wraps the error callback: xruns are counted (the backend recovers by itself), route
    /// changes and refused real-time priority are not errors; everything else is reported.
    fn errors<E: FnMut(String)>(&self, mut on_error: E) -> impl FnMut(cpal::Error) {
        let counter = self.clone();
        move |e: cpal::Error| match e.kind() {
            cpal::ErrorKind::Xrun => counter.add(),
            cpal::ErrorKind::DeviceChanged | cpal::ErrorKind::RealtimeDenied => {}
            _ => on_error(e.to_string()),
        }
    }
}

/// Measures the gap between callbacks against the previous block's length.
#[derive(Debug)]
struct LateCheck {
    counter: XrunCounter,
    last: Option<(Instant, Duration)>,
}

impl LateCheck {
    fn new(counter: &XrunCounter) -> LateCheck {
        LateCheck {
            counter: counter.clone(),
            last: None,
        }
    }

    /// Call at the start of every callback with the block's length in frames.
    fn block(&mut self, frames: usize, sample_rate: u32) {
        self.block_at(Instant::now(), frames, sample_rate);
    }

    /// `block` at a given time: tests set it, a sleeping test thread overshoots under load.
    fn block_at(&mut self, now: Instant, frames: usize, sample_rate: u32) {
        if let Some((at, length)) = self.last {
            if now.duration_since(at) > length.mul_f64(LATE_BLOCKS) + LATE_SLACK {
                self.counter.add();
            }
        }
        let length = Duration::from_secs_f64(frames as f64 / f64::from(sample_rate.max(1)));
        self.last = Some((now, length));
    }
}

/// Real-time priority of the audio thread: below PipeWire's own data thread (88), above
/// everything else Kontra runs.
#[cfg(target_os = "linux")]
const REALTIME_PRIORITY: i32 = 80;

/// Asks, in the first callback, for real-time scheduling of the audio thread. Without it a busy
/// system made the PipeWire host lose blocks at 128 frames (14.10.: up to 16 xruns per loopback
/// run, round trip spread 13 ms; with it 0 xruns). Granted when the user may (RLIMIT_RTPRIO,
/// e.g. the "realtime" group); otherwise nothing changes. One system call, no allocation.
/// rtkit (D-Bus) would serve everyone, but cpal's way there (audio_thread_priority) is MPL-2.0.
#[derive(Debug, Default)]
struct Realtime {
    asked: bool,
}

impl Realtime {
    fn once(&mut self) {
        if self.asked {
            return;
        }
        self.asked = true;
        let priority = if promote_thread() {
            Priority::Realtime
        } else {
            Priority::Normal
        };
        PRIORITY.store(priority as u8, Ordering::Relaxed);
    }
}

/// What the last audio thread got (reported by the hardware check).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Priority {
    /// No audio stream ran yet.
    Unknown,
    Realtime,
    /// Refused (no RLIMIT_RTPRIO): blocks may get lost under load at small quanta.
    Normal,
}

static PRIORITY: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// The scheduling the audio thread got.
pub fn audio_priority() -> Priority {
    match PRIORITY.load(Ordering::Relaxed) {
        x if x == Priority::Realtime as u8 => Priority::Realtime,
        x if x == Priority::Normal as u8 => Priority::Normal,
        _ => Priority::Unknown,
    }
}

/// True if the calling thread now runs with real-time priority.
#[cfg(target_os = "linux")]
#[allow(unsafe_code)]
fn promote_thread() -> bool {
    let param = libc::sched_param {
        sched_priority: REALTIME_PRIORITY,
    };
    // SAFETY: changes only the calling thread's (0) scheduling; `param` is a valid, initialised
    // struct that outlives the call. A refusal (EPERM) is returned, not raised. Child processes
    // (the stem separator) do not inherit it (RESET_ON_FORK).
    let result = unsafe {
        libc::sched_setscheduler(0, libc::SCHED_FIFO | libc::SCHED_RESET_ON_FORK, &param)
    };
    result == 0
}

#[cfg(not(target_os = "linux"))]
fn promote_thread() -> bool {
    false
}

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
    /// Opened through ALSA while PipeWire runs: the card has no PipeWire node (lost after
    /// "device busy"); replugging or restarting WirePlumber brings the node back.
    pub past_pipewire: bool,
}

/// What to open; `None` fields use the device defaults.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StreamConfig {
    pub device_id: Option<String>,
    pub sample_rate: Option<u32>,
    pub buffer_frames: Option<u32>,
    /// Input only: which channel carries the instrument; `None` follows the loudest.
    pub input_channel: Option<u16>,
}

/// Lists input or output devices of the audio host (PipeWire nodes, else ALSA devices), the
/// interfaces first (LH-EIN-02).
pub fn list_devices(direction: Direction) -> Vec<DeviceInfo> {
    let _quiet = QuietAlsa::new();
    let host = host();
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
    let mut list: Vec<DeviceInfo> = devices
        .unwrap_or_default()
        .iter()
        .filter_map(|d| describe(d, direction, default_id.as_deref()))
        .collect();
    // Read while `host` lives: it keeps the PipeWire library initialised.
    #[cfg(target_os = "linux")]
    if host.id() == cpal::HostId::PipeWire {
        if let Some(graph) = Graph::read() {
            list = pipewire_devices(list, &graph);
        }
        if direction == Direction::Input {
            let direct = direct_inputs(&list);
            list.extend(direct);
        }
    }
    interfaces_first(&mut list);
    list
}

/// Device id prefix of the PipeWire host; the rest is the node name, which stays the same
/// across restarts ("pipewire:alsa_input.usb-Focusrite_Scarlett_Solo_USB-00.analog-stereo").
#[cfg(target_os = "linux")]
const PIPEWIRE_PREFIX: &str = "pipewire:";

/// cpal's PipeWire list, cut to what the graph says: sources (or duplex nodes) for input, sinks
/// for output; no sink monitors, no other programs' streams, no stand-ins for the defaults
/// ("input_default"); the default is the node the system uses now.
#[cfg(target_os = "linux")]
fn pipewire_devices(listed: Vec<DeviceInfo>, graph: &Graph) -> Vec<DeviceInfo> {
    listed
        .into_iter()
        .filter_map(|mut device| {
            let node = device.id.strip_prefix(PIPEWIRE_PREFIX)?;
            if !graph.serves(node, device.direction) {
                return None;
            }
            device.is_default = graph.is_default(node, device.direction);
            Some(device)
        })
        .collect()
}

/// Interfaces (USB) first, then the built-in card, Bluetooth last; otherwise as listed.
fn interfaces_first(list: &mut [DeviceInfo]) {
    let rank = |connection: Connection| match connection {
        Connection::Usb => 0,
        Connection::BuiltIn => 1,
        Connection::Other => 2,
        Connection::Hdmi => 3,
        Connection::Bluetooth => 4,
    };
    list.sort_by_key(|d| rank(d.connection));
}

/// A USB sound card as ALSA lists it in /proc/asound/cards.
#[cfg(target_os = "linux")]
#[derive(Debug, Clone, PartialEq)]
struct UsbCard {
    index: u32,
    name: String,
}

#[cfg(target_os = "linux")]
const CARDS_FILE: &str = "/proc/asound/cards";
/// Marks a USB card's line: ` 2 [USB            ]: USB-Audio - Scarlett Solo USB`.
#[cfg(target_os = "linux")]
const USB_DRIVER: &str = "USB-Audio - ";

/// USB cards from /proc/asound/cards; reading it opens no device.
#[cfg(target_os = "linux")]
fn usb_cards(text: &str) -> Vec<UsbCard> {
    text.lines()
        .filter_map(|line| {
            let (head, driver) = line.split_once("]: ")?;
            let name = driver.strip_prefix(USB_DRIVER)?;
            let (index, _) = head.trim_start().split_once(' ')?;
            Some(UsbCard {
                index: index.parse().ok()?,
                name: name.trim().to_string(),
            })
        })
        .collect()
}

/// Cards no PipeWire input stands for: none of the USB inputs carries the card's name
/// ("Scarlett Solo USB" in "Scarlett Solo USB Analoges Stereo").
#[cfg(target_os = "linux")]
fn uncovered<'a>(cards: &'a [UsbCard], pipewire: &[DeviceInfo]) -> Vec<&'a UsbCard> {
    let covered = |card: &UsbCard| {
        pipewire
            .iter()
            .any(|d| d.connection == Connection::Usb && d.name.contains(&card.name))
    };
    cards.iter().filter(|card| !covered(card)).collect()
}

/// An interface ALSA sees but PipeWire offers no input for (review #73: the Scarlett's
/// source was gone after "device busy" and "Automatic" took the built-in line-in). Listed
/// through ALSA's `plughw` (any format and rate), marked so Kontra can say why and how to fix it.
#[cfg(target_os = "linux")]
fn direct_inputs(pipewire: &[DeviceInfo]) -> Vec<DeviceInfo> {
    let text = std::fs::read_to_string(CARDS_FILE).unwrap_or_default();
    let cards = usb_cards(&text);
    let missing: Vec<_> = uncovered(&cards, pipewire)
        .into_iter()
        .filter(|card| {
            let capture = format!("/proc/asound/card{}/pcm0c", card.index);
            std::path::Path::new(&capture).exists()
        })
        .collect();
    if missing.is_empty() {
        return Vec::new();
    }
    let Ok(alsa) = cpal::host_from_id(cpal::HostId::Alsa) else {
        return Vec::new();
    };
    let devices = alsa.input_devices().map(|d| d.collect::<Vec<_>>());
    let devices = devices.unwrap_or_default();
    missing
        .iter()
        .filter_map(|card| {
            let id = format!("alsa:plughw:CARD={},DEV=0", card.index);
            let device = devices
                .iter()
                .find(|d| d.id().is_ok_and(|i| i.to_string() == id))?;
            let mut info = describe(device, Direction::Input, None)?;
            info.connection = Connection::Usb;
            info.past_pipewire = true;
            Some(info)
        })
        .collect()
}

/// Probing every ALSA device makes alsa-lib print to stderr for those that cannot open (the
/// "oss" plugin without /dev/dsp, a card's dmix while PipeWire holds the card); they are left
/// out of the list anyway. Silenced on this thread while it lives; errors still reach cpal.
#[cfg(target_os = "linux")]
#[allow(unsafe_code)]
mod quiet_alsa {
    use std::ffi::{c_char, c_int};

    pub(super) struct QuietAlsa(alsa_sys::snd_local_error_handler_t);

    impl std::fmt::Debug for QuietAlsa {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("QuietAlsa")
        }
    }

    unsafe extern "C" fn ignore(
        _file: *const c_char,
        _line: c_int,
        _function: *const c_char,
        _err: c_int,
        _fmt: *const c_char,
        _args: *mut alsa_sys::__va_list_tag,
    ) {
    }

    impl QuietAlsa {
        pub(super) fn new() -> QuietAlsa {
            // SAFETY: sets this thread's alsa-lib error handler to a function of the declared
            // signature that touches none of its arguments; the previous one is kept to restore.
            QuietAlsa(unsafe { alsa_sys::snd_lib_error_set_local(Some(ignore)) })
        }
    }

    impl Drop for QuietAlsa {
        fn drop(&mut self) {
            // SAFETY: restores the handler this thread had before (`None` is alsa-lib's default).
            unsafe {
                alsa_sys::snd_lib_error_set_local(self.0);
            }
        }
    }
}

#[cfg(target_os = "linux")]
use quiet_alsa::QuietAlsa;

/// Elsewhere there is no alsa-lib to silence.
#[cfg(not(target_os = "linux"))]
#[derive(Debug)]
struct QuietAlsa;

#[cfg(not(target_os = "linux"))]
impl QuietAlsa {
    fn new() -> QuietAlsa {
        QuietAlsa
    }
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
        past_pipewire: false,
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

/// Environment variable that forces ALSA ("alsa"), e.g. to compare with PipeWire.
const HOST_VARIABLE: &str = "KONTRA_AUDIO_HOST";

/// PipeWire when it runs (cpal finds its socket and nodes), else ALSA, e.g. without PipeWire
/// or in a sandbox without the socket.
fn host() -> cpal::Host {
    if std::env::var(HOST_VARIABLE).as_deref() == Ok("alsa") {
        if let Ok(alsa) = cpal::host_from_id(cpal::HostId::Alsa) {
            return alsa;
        }
    }
    cpal::default_host()
}

/// The device with `id` on the host the id names (an ALSA device also while PipeWire runs);
/// `None` is the system default.
fn find_device(direction: Direction, id: Option<&str>) -> Result<cpal::Device, AudioError> {
    let Some(id) = id else {
        let host = host();
        let default = match direction {
            Direction::Input => host.default_input_device(),
            Direction::Output => host.default_output_device(),
        };
        return default.ok_or(AudioError::NoDevice(direction));
    };
    let host = host_of(id)
        .and_then(|h| cpal::host_from_id(h).ok())
        .ok_or(AudioError::NoDevice(direction))?;
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

/// The host a saved id belongs to ("alsa:hw:2,0", "pipewire:alsa_input…").
fn host_of(id: &str) -> Option<cpal::HostId> {
    id.parse::<cpal::DeviceId>()
        .ok()
        .map(|parsed| parsed.host())
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
    let mut config = cpal::StreamConfig {
        channels: default.channels(),
        sample_rate: wanted.sample_rate.unwrap_or(default.sample_rate()),
        buffer_size: buffer_size(wanted.buffer_frames, default.buffer_size()),
    };
    // PipeWire takes any block in its quantum range (cpal checks it) as a wish (node.latency).
    let probe = !is_pipewire(device) && config.buffer_size != cpal::BufferSize::Default;
    if probe && !accepts(device, direction, &config) {
        config.buffer_size = cpal::BufferSize::Default;
    }
    Ok(config)
}

#[cfg(target_os = "linux")]
fn is_pipewire(device: &cpal::Device) -> bool {
    device
        .id()
        .is_ok_and(|id| id.host() == cpal::HostId::PipeWire)
}

#[cfg(not(target_os = "linux"))]
fn is_pipewire(_: &cpal::Device) -> bool {
    false
}

/// Whether the device opens with this config. Devices may report wider limits than they take:
/// ALSA's `sysdefault` of a Scarlett Solo (dsnoop) says 85..91 M frames and opens only 1024
/// ("Buffer size 128 is not in the supported range", 14.10.). A stream that is never started
/// is built and dropped, so the real one can fall back to the default block.
fn accepts(device: &cpal::Device, direction: Direction, config: &cpal::StreamConfig) -> bool {
    let quiet = |_: cpal::Error| {};
    match direction {
        Direction::Input => device
            .build_input_stream(
                *config,
                |_: &[f32], _: &cpal::InputCallbackInfo| {},
                quiet,
                None,
            )
            .is_ok(),
        Direction::Output => device
            .build_output_stream(
                *config,
                |_: &mut [f32], _: &cpal::OutputCallbackInfo| {},
                quiet,
                None,
            )
            .is_ok(),
    }
}

/// The wanted block, clamped to what the device takes; unknown limits keep the default (a
/// fixed size outside them makes opening fail).
fn buffer_size(wanted: Option<u32>, supported: &cpal::SupportedBufferSize) -> cpal::BufferSize {
    let (Some(frames), cpal::SupportedBufferSize::Range { min, max }) = (wanted, supported) else {
        return cpal::BufferSize::Default;
    };
    cpal::BufferSize::Fixed(frames.clamp(*min, *max))
}

/// A running input stream; dropping it stops capture.
pub struct InputStream {
    _stream: cpal::Stream,
    xruns: XrunCounter,
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
        on_error: E,
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
        let mut downmix = Downmix::new(channels, pick);
        let mut mono = vec![0.0f32; MAX_BLOCK];
        let xruns = XrunCounter::default();
        let mut late = LateCheck::new(&xruns);
        let rate = stream_config.sample_rate;
        let mut realtime = Realtime::default();
        let callback = move |data: &[f32], _: &cpal::InputCallbackInfo| {
            realtime.once();
            late.block(data.len() / channels, rate);
            for chunk in data.chunks(MAX_BLOCK * channels) {
                let frames = chunk.len() / channels;
                downmix.mono(chunk, &mut mono[..frames]);
                on_samples(&mono[..frames]);
            }
        };
        let stream = device
            .build_input_stream(stream_config, callback, xruns.errors(on_error), None)
            .map_err(|e| device_error(&device, e))?;
        stream.play().map_err(|e| device_error(&device, e))?;
        Ok(InputStream {
            _stream: stream,
            xruns,
            sample_rate: stream_config.sample_rate,
            channels: stream_config.channels,
            device_name: device
                .description()
                .map(|d| d.name().to_string())
                .unwrap_or_default(),
        })
    }
}

/// Turns interleaved frames into the instrument's mono signal: the chosen channel, or else the
/// loudest one. An average would halve a bass on one input of a two-input interface and hide its
/// clipping (hardware check 13.10.: Scarlett Solo, peaks at −6 dBFS while clipping).
///
/// ```text
/// in 1 (empty)  ─────────            held peak per channel, decaying;
/// in 2 (bass)   ─╱╲╱╲╱╲──  ──► out   switch only when another is 6 dB louder
/// ```
struct Downmix {
    channels: usize,
    pick: Option<usize>,
    /// Recent peak per channel; allocated before the audio thread starts.
    held: Vec<f32>,
    current: usize,
}

/// Per block, a held peak keeps this share (a few seconds at usual block sizes).
const HOLD_DECAY: f32 = 0.995;
/// Another channel takes over only this much louder (6 dB): no flipping between equal inputs.
const SWITCH_RATIO: f32 = 2.0;

impl Downmix {
    fn new(channels: usize, pick: Option<usize>) -> Downmix {
        Downmix {
            channels,
            pick,
            held: vec![0.0; channels],
            current: 0,
        }
    }

    /// `out` gets one sample per frame of `frames`.
    fn mono(&mut self, frames: &[f32], out: &mut [f32]) {
        let channel = self.pick.unwrap_or_else(|| self.follow(frames));
        for (i, frame) in frames.chunks_exact(self.channels).enumerate() {
            out[i] = frame[channel];
        }
    }

    /// The loudest channel of the last blocks.
    fn follow(&mut self, frames: &[f32]) -> usize {
        for held in &mut self.held {
            *held *= HOLD_DECAY;
        }
        for frame in frames.chunks_exact(self.channels) {
            for (held, sample) in self.held.iter_mut().zip(frame) {
                *held = held.max(sample.abs());
            }
        }
        let loudest = (0..self.channels)
            .max_by(|a, b| self.held[*a].total_cmp(&self.held[*b]))
            .unwrap_or(0);
        if self.held[loudest] > self.held[self.current] * SWITCH_RATIO {
            self.current = loudest;
        }
        self.current
    }
}

impl InputStream {
    /// Dropouts since the stream started (LH-NF-04).
    pub fn xruns(&self) -> u64 {
        self.xruns.get()
    }
}

/// Where an output's callbacks come from.
enum Backend {
    /// Kept only so the stream lives as long as the `OutputStream`.
    Device(#[allow(dead_code)] cpal::Stream),
    /// A thread calling the fill function on a timer; stops when dropped.
    Null {
        stop: Arc<AtomicBool>,
        thread: Option<std::thread::JoinHandle<()>>,
    },
}

impl Drop for Backend {
    fn drop(&mut self) {
        if let Backend::Null { stop, thread } = self {
            stop.store(true, Ordering::Relaxed);
            if let Some(t) = thread.take() {
                let _ = t.join();
            }
        }
    }
}

/// A running output stream; dropping it stops playback.
pub struct OutputStream {
    _stream: Backend,
    xruns: XrunCounter,
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
        on_error: E,
    ) -> Result<OutputStream, AudioError>
    where
        F: FnMut(&mut [f32], usize) + Send + 'static,
        E: FnMut(String) + Send + 'static,
    {
        if config.device_id.as_deref() == Some(NULL_DEVICE) {
            return Ok(Self::open_null(fill));
        }
        let device = find_device(Direction::Output, config.device_id.as_deref())?;
        let stream_config = stream_config(&device, Direction::Output, config)?;
        let channels = usize::from(stream_config.channels.max(1));
        let xruns = XrunCounter::default();
        let mut late = LateCheck::new(&xruns);
        let rate = stream_config.sample_rate;
        let mut realtime = Realtime::default();
        let callback = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            realtime.once();
            late.block(data.len() / channels, rate);
            fill(data, channels);
        };
        let stream = device
            .build_output_stream(stream_config, callback, xruns.errors(on_error), None)
            .map_err(|e| device_error(&device, e))?;
        stream.play().map_err(|e| device_error(&device, e))?;
        Ok(OutputStream {
            _stream: Backend::Device(stream),
            xruns,
            sample_rate: stream_config.sample_rate,
            channels: stream_config.channels,
            device_name: device
                .description()
                .map(|d| d.name().to_string())
                .unwrap_or_default(),
        })
    }

    /// Output without a device: a thread asks for a block every block length, like a sound
    /// card. A block that is ready later than its deadline counts as a dropout.
    fn open_null<F>(mut fill: F) -> OutputStream
    where
        F: FnMut(&mut [f32], usize) + Send + 'static,
    {
        let xruns = XrunCounter::default();
        let counter = xruns.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = Arc::clone(&stop);
        let channels = usize::from(NULL_CHANNELS);
        let block = Duration::from_secs_f64(NULL_BLOCK as f64 / f64::from(NULL_RATE));
        let thread = std::thread::spawn(move || {
            let mut buffer = vec![0.0f32; NULL_BLOCK * channels];
            let mut deadline = Instant::now() + block;
            while !stop_thread.load(Ordering::Relaxed) {
                fill(&mut buffer, channels);
                let now = Instant::now();
                if now > deadline + LATE_SLACK {
                    counter.add();
                    // Start over from now, as a device does after an xrun.
                    deadline = now;
                }
                std::thread::sleep(deadline.saturating_duration_since(Instant::now()));
                deadline += block;
            }
        });
        OutputStream {
            _stream: Backend::Null {
                stop,
                thread: Some(thread),
            },
            xruns,
            sample_rate: NULL_RATE,
            channels: NULL_CHANNELS,
            device_name: NULL_DEVICE.to_string(),
        }
    }

    /// Dropouts since the stream started (LH-NF-04).
    pub fn xruns(&self) -> u64 {
        self.xruns.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Hardware check 13.10.: Scarlett Solo, bass in input 2, input 1 empty. The average halved
    // the bass, so clipping (full scale) never reached the clip mark.
    #[test]
    fn empty_second_input_keeps_the_level() {
        let mut downmix = Downmix::new(2, None);
        let frames = [0.0, 1.0, 0.0, -1.0, 0.0, 0.5];
        let mut out = [0.0; 3];
        downmix.mono(&frames, &mut out);
        assert_eq!(out, [1.0, -1.0, 0.5]);

        let mut fixed = Downmix::new(2, Some(0));
        fixed.mono(&frames, &mut out);
        assert_eq!(out, [0.0; 3]);
    }

    // Two inputs with the same level (a stereo jack): one of them, at full level, no flipping.
    #[test]
    fn equal_inputs_stay_on_one() {
        let mut downmix = Downmix::new(2, None);
        let mut out = [0.0; 2];
        downmix.mono(&[0.8, 0.6, 0.4, 0.5], &mut out);
        assert_eq!(out, [0.8, 0.4]);
        downmix.mono(&[0.6, 0.9, 0.5, 0.4], &mut out);
        assert_eq!(out, [0.6, 0.5]);
    }

    // A device that cannot take the wanted block would refuse to open; it gets the nearest one.
    #[test]
    fn buffer_fits_the_device() {
        let range = cpal::SupportedBufferSize::Range {
            min: 256,
            max: 4096,
        };
        assert_eq!(buffer_size(Some(128), &range), cpal::BufferSize::Fixed(256));
        assert_eq!(buffer_size(Some(512), &range), cpal::BufferSize::Fixed(512));
        assert_eq!(buffer_size(None, &range), cpal::BufferSize::Default);
        let unknown = cpal::SupportedBufferSize::Unknown;
        assert_eq!(buffer_size(Some(128), &unknown), cpal::BufferSize::Default);
    }

    fn device(id: &str, direction: Direction, connection: Connection) -> DeviceInfo {
        DeviceInfo {
            id: id.into(),
            name: id.into(),
            direction,
            is_default: false,
            connection,
            sample_rate: 48_000,
            channels: 2,
            past_pipewire: false,
        }
    }

    // Hardware check 13.10. (H8): the Scarlett could not be chosen next to another default.
    // PipeWire lists it by node; sink monitors, program streams and the stand-ins go.
    #[cfg(target_os = "linux")]
    #[test]
    fn pipewire_lists_sources_as_inputs() {
        use crate::pipewire::NodeClass;
        let scarlett = "pipewire:alsa_input.usb-Focusrite_Scarlett_Solo_USB-00.analog-stereo";
        let graph = Graph::fake(
            &[
                (
                    "alsa_input.pci-0000_12_00.4.analog-stereo",
                    NodeClass::Source,
                ),
                (
                    "alsa_input.usb-Focusrite_Scarlett_Solo_USB-00.analog-stereo",
                    NodeClass::Source,
                ),
                (
                    "alsa_output.pci-0000_12_00.4.iec958-stereo",
                    NodeClass::Sink,
                ),
            ],
            Some("alsa_input.pci-0000_12_00.4.analog-stereo"),
            Some("alsa_output.pci-0000_12_00.4.iec958-stereo"),
        );
        let listed = vec![
            device(
                "pipewire:input_default",
                Direction::Input,
                Connection::Other,
            ),
            device(
                "pipewire:alsa_output.pci-0000_12_00.4.iec958-stereo",
                Direction::Input,
                Connection::BuiltIn,
            ),
            device(
                "pipewire:alsa_input.pci-0000_12_00.4.analog-stereo",
                Direction::Input,
                Connection::BuiltIn,
            ),
            device(scarlett, Direction::Input, Connection::Usb),
            device("pipewire:mpv", Direction::Input, Connection::Other),
        ];
        let mut list = pipewire_devices(listed, &graph);
        interfaces_first(&mut list);
        let ids: Vec<_> = list.iter().map(|d| d.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                scarlett,
                "pipewire:alsa_input.pci-0000_12_00.4.analog-stereo"
            ]
        );
        assert!(!list[0].is_default && list[1].is_default);
    }

    #[test]
    fn interfaces_come_first() {
        let mut list = vec![
            device("bt", Direction::Output, Connection::Bluetooth),
            device("card", Direction::Output, Connection::BuiltIn),
            device("hdmi", Direction::Output, Connection::Hdmi),
            device("solo", Direction::Output, Connection::Usb),
        ];
        interfaces_first(&mut list);
        let ids: Vec<_> = list.iter().map(|d| d.id.as_str()).collect();
        assert_eq!(ids, ["solo", "card", "hdmi", "bt"]);
    }

    // A saved id opens on its own host: an ALSA fallback device while PipeWire runs.
    #[test]
    fn ids_name_their_host() {
        assert_eq!(host_of("alsa:hw:CARD=2,DEV=0"), Some(cpal::HostId::Alsa));
        #[cfg(target_os = "linux")]
        assert_eq!(
            host_of("pipewire:alsa_input.usb-x"),
            Some(cpal::HostId::PipeWire)
        );
        assert_eq!(host_of("Scarlett"), None);
    }

    #[cfg(target_os = "linux")]
    const CARDS: &str = " 0 [NVidia         ]: HDA-Intel - HDA NVidia
                      HDA NVidia at 0xfb080000 irq 174
 1 [Generic        ]: HDA-Intel - HD-Audio Generic
                      HD-Audio Generic at 0xfbf00000 irq 176
 2 [USB            ]: USB-Audio - Scarlett Solo USB
                      Focusrite Scarlett Solo USB at usb-0000:12:00.3-4.4.3, high speed
";

    #[cfg(target_os = "linux")]
    #[test]
    fn finds_usb_cards() {
        let cards = usb_cards(CARDS);
        let solo = UsbCard {
            index: 2,
            name: "Scarlett Solo USB".into(),
        };
        assert_eq!(cards, [solo]);
    }

    // Review #73 (hardware 14.10.): ALSA sees the Scarlett, PipeWire has no source for it
    // (lost after "device busy"). It must stay choosable, past PipeWire, not vanish.
    #[cfg(target_os = "linux")]
    #[test]
    fn interface_without_node_is_missing() {
        let cards = usb_cards(CARDS);
        let builtin = device(
            "pipewire:alsa_input.pci-0000_12_00.4.analog-stereo",
            Direction::Input,
            Connection::BuiltIn,
        );
        let missing = uncovered(&cards, std::slice::from_ref(&builtin));
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].index, 2);

        let mut solo = device(
            "pipewire:alsa_input.usb-Focusrite_Scarlett_Solo_USB-00.analog-stereo",
            Direction::Input,
            Connection::Usb,
        );
        solo.name = "Scarlett Solo USB Analoges Stereo".into();
        assert!(uncovered(&cards, &[builtin, solo]).is_empty());
    }

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
    fn late_callbacks_count_as_xruns() {
        let counter = XrunCounter::default();
        let mut late = LateCheck::new(&counter);
        // 480 frames at 48 kHz = 10 ms; 12 ms later is fine, 40 ms later is a dropout.
        let start = Instant::now();
        late.block_at(start, 480, 48_000);
        late.block_at(start + Duration::from_millis(12), 480, 48_000);
        assert_eq!(counter.get(), 0);
        late.block_at(start + Duration::from_millis(52), 480, 48_000);
        assert_eq!(counter.get(), 1);
    }

    #[test]
    fn null_output_runs_on_time() {
        // Blocks, the time of the last one in µs after the first, and whether the thread was
        // ever starved past an xrun: the output's own view.
        let blocks = Arc::new(AtomicU64::new(0));
        let last_us = Arc::new(AtomicU64::new(0));
        let starved = Arc::new(AtomicBool::new(false));
        let (seen, last, gap) = (
            Arc::clone(&blocks),
            Arc::clone(&last_us),
            Arc::clone(&starved),
        );
        let block = Duration::from_secs_f64(NULL_BLOCK as f64 / f64::from(NULL_RATE));
        let mut first: Option<Instant> = None;
        let mut previous: Option<Instant> = None;
        let config = StreamConfig {
            device_id: Some(NULL_DEVICE.into()),
            ..StreamConfig::default()
        };
        let stream = OutputStream::open(
            &config,
            move |data: &mut [f32], _| {
                data.fill(0.0);
                let now = Instant::now();
                // Three blocks: a gap a slow clock (two blocks) cannot explain.
                if previous.is_some_and(|p| now - p > 3 * block) {
                    gap.store(true, Ordering::Relaxed);
                }
                previous = Some(now);
                let first = *first.get_or_insert(now);
                last.store((now - first).as_micros() as u64, Ordering::Relaxed);
                seen.fetch_add(1, Ordering::Relaxed);
            },
            |_| {},
        )
        .unwrap();
        std::thread::sleep(Duration::from_millis(200));
        drop(stream);

        // Between its first and last block the output keeps 5.3 ms blocks, about 37 in 200 ms.
        // Measured from the output, not the test: under load (load average 36, 25.10.) the
        // thread starts or ends late, and the test's clock counted blocks it never had time for.
        // Starved in between, it skips blocks like a device after an xrun: then only "never too
        // many" holds.
        let span_s = last_us.load(Ordering::Relaxed) as f64 / 1e6;
        let expected = span_s / block.as_secs_f64() + 1.0;
        let n = blocks.load(Ordering::Relaxed) as f64;
        assert!(n <= 1.2 * expected + 2.0, "{n} of {expected:.0}");
        if !starved.load(Ordering::Relaxed) {
            assert!(n >= 0.7 * expected, "{n} of {expected:.0}");
        }
    }

    #[test]
    fn listing_without_hardware_does_not_panic() {
        let _ = list_devices(Direction::Input);
        let _ = list_devices(Direction::Output);
    }
}
