//! Output: metronome clicks, reference tones, the backing track and the monitored bass input,
//! scheduled on the engine clock.
//!
//! The control thread schedules sounds slightly ahead (≥ 100 ms); the audio callback starts each
//! one at the exact frame. Nothing in the callback allocates or frees: backing audio arrives as an
//! `Arc` that the control side keeps alive (see `Output::play_backing`).
//!
//! ```text
//! input callback ─► monitor ring (mono) ─┐
//! control ─► command ring ───────────────┴─► output callback: voices + backing + monitor ─► device
//! ```

use crate::clock::{settle, Clock};
use crate::plugin_chain::{ChainBuilder, PluginBank};
use crate::{EngineError, Event};
use kontra_drivers::plugins::{system_frame, PluginError};
use kontra_drivers::{OutputStream, StreamConfig};
use kontra_dsp::synth::Noise;
use kontra_dsp::{Chain, OutputLimiter, PitchShifter, Shaper, Shaping, PITCH_LATENCY};
use kontra_model::ChainSpec;
use kontra_model::DrumKind;
use levels::Levels;
use std::f32::consts::TAU;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;

const COMMANDS: usize = 1024;
const VOICES: usize = 48;
const CLICK_S: f32 = 0.02;
/// Drum voices: length, decay time constant of the body.
const KICK_S: f32 = 0.35;
const SNARE_S: f32 = 0.2;
const HIHAT_S: f32 = 0.08;
/// Click amplitude on the beat; the off-beat of a shuffle (LH-WIE-04) is half as loud.
const CLICK_LEVEL: f32 = 0.5;
const OFFBEAT_LEVEL: f32 = 0.25;
/// Monitor ring: half a second at the highest supported rate.
const MONITOR_RING: usize = 96_000;
/// What played, for a microphone input to subtract (LH-EIN-11): two seconds at 48 kHz.
const REFERENCE_RING: usize = 96_000;
/// More queued monitor audio than this means the clocks drifted apart; the excess is dropped
/// down to `MONITOR_KEEP_S`, so the bass never lags further behind.
const MONITOR_MAX_S: f64 = 0.08;
const MONITOR_KEEP_S: f64 = 0.02;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accent {
    /// First beat of a bar.
    Downbeat,
    Beat,
    /// Between the beats, e.g. the third triplet of a shuffle (LH-WIE-04): softer.
    Offbeat,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Sound {
    Click(Accent),
    /// Reference bass tone (LH-WIE-03).
    Tone {
        hz: f32,
        duration_s: f32,
        velocity: f32,
    },
    /// Accompaniment drum stroke (LH-LRN-01); `velocity` 0..=1, softer while Kontra plays a
    /// call (LH-LRN-19).
    Drum {
        kind: DrumKind,
        velocity: f32,
    },
}

impl Sound {
    /// A drum stroke at full velocity.
    pub fn drum(kind: DrumKind) -> Sound {
        Sound::Drum {
            kind,
            velocity: 1.0,
        }
    }
}

/// Interleaved backing audio at the output's sample rate: stereo, or after bass removal stereo
/// plus the removed bass (4 channels: L, R, bass L, bass R), which `Gains::bass` mixes back.
#[derive(Debug, Clone, PartialEq)]
pub struct BackingAudio {
    samples: Arc<Vec<f32>>,
    channels: usize,
}

const STEREO: usize = 2;
const WITH_BASS: usize = 4;

impl BackingAudio {
    pub fn stereo(samples: Vec<f32>) -> BackingAudio {
        BackingAudio {
            samples: Arc::new(samples),
            channels: STEREO,
        }
    }

    /// Frames of L, R, bass L, bass R.
    pub fn with_bass(samples: Vec<f32>) -> BackingAudio {
        BackingAudio {
            samples: Arc::new(samples),
            channels: WITH_BASS,
        }
    }

    /// Stereo frame `frame` with the removed bass at `bass` (0..=1), silence past the end.
    fn frame(&self, frame: usize, bass: f32) -> (f32, f32) {
        let start = frame * self.channels;
        let Some(f) = self.samples.get(start..start + self.channels) else {
            return (0.0, 0.0);
        };
        match f {
            [l, r, bl, br] => (l + bass * bl, r + bass * br),
            [l, r, ..] => (*l, *r),
            _ => (0.0, 0.0),
        }
    }
}

/// What the session layer needs from an output; `Player` is the device implementation.
pub trait Output {
    fn sample_rate(&self) -> u32;
    /// Plays `sound` at engine time `at_s`. Returns false if the command queue is full.
    fn schedule(&mut self, at_s: f64, sound: Sound) -> bool;
    /// Drops everything scheduled at or after `from_s` (seek, pause, tempo change).
    fn clear_from(&mut self, from_s: f64);
    /// Plays `audio` from stereo frame `from_frame` starting at engine time `at_s`; the running
    /// backing continues until then (one pending start, e.g. the next loop pass). The caller must keep a clone of `audio` until it is replaced, so the
    /// audio thread never frees memory.
    fn play_backing(&mut self, audio: BackingAudio, at_s: f64, from_frame: usize);
    fn stop_backing(&mut self, at_s: f64);
    fn set_gains(&mut self, gains: Gains);
    /// Pitch of the backing track in semitones (LH-WIE-02), changed while it plays; outputs
    /// without sound ignore it.
    fn set_transpose(&mut self, _semitones: f32) {}
    /// Changes the plugins in the effect chain made themselves since the last call, as
    /// (`PluginSpec::id`, parameter id, value); outputs without plugins have none. Also the tick
    /// that puts a chain in place once its plugins have loaded in the background, so call it
    /// regularly.
    fn take_param_changes(&mut self) -> Vec<(u32, u32, f64)> {
        Vec::new()
    }
    /// The chain's plugins' texts for values since the last call, as (`PluginSpec::id`,
    /// parameter id, value, text); outputs without plugins have none.
    fn take_param_texts(&mut self) -> Vec<(u32, u32, f64, String)> {
        Vec::new()
    }
    /// Opens the own editor of the chain's plugin `plugin` (`PluginSpec::id`); outputs without
    /// plugins have none.
    fn open_editor(&mut self, _plugin: u32, _title: &str) -> Result<(), PluginError> {
        Err(PluginError::NoEditor)
    }
}

/// Bus volumes, 0..=1.
#[derive(Debug, Clone, PartialEq)]
pub struct Gains {
    pub click: f32,
    pub tone: f32,
    pub backing: f32,
    /// Synthesised drum accompaniment (LH-LRN-01).
    pub drums: f32,
    /// The original bass mixed back into a backing track without it (LH-BIB-09); 0 = removed.
    pub bass: f32,
    /// The bass input played back (input monitoring); 0 = off.
    pub monitor: f32,
    /// Tone shaping of the monitored bass (LH-EIN-08); detection hears the dry input.
    pub shaping: Shaping,
    /// The plugins the chain of `shaping` names (LH-EIN-20, -21). The player keeps them; the
    /// callback never gets them.
    pub plugins: Option<PluginBank>,
}

impl Default for Gains {
    fn default() -> Self {
        Gains {
            click: 0.8,
            tone: 0.6,
            backing: 0.8,
            drums: 0.7,
            bass: 0.0,
            monitor: 0.0,
            shaping: Shaping::FLAT,
            plugins: None,
        }
    }
}

// `Gains` is large (the chain). Boxing it would free memory in the callback; the ring slots are
// all that size anyway.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
enum Command {
    Schedule {
        at_s: f64,
        sound: Sound,
    },
    /// Drops everything scheduled at or after `from_s`.
    ClearFrom {
        from_s: f64,
    },
    Backing {
        audio: BackingAudio,
        at_s: f64,
        from_frame: usize,
    },
    StopBacking {
        at_s: f64,
    },
    Gains(Gains),
    Transpose(f32),
}

#[derive(Debug, Clone)]
struct Backing {
    audio: BackingAudio,
    at_s: f64,
    from_frame: usize,
    stop_s: f64,
}

#[derive(Debug, Clone, Copy)]
struct Voice {
    start_s: f64,
    sound: Sound,
    phase: f32,
    /// Filter state of noise-based drums.
    memory: f32,
    active: bool,
}

const IDLE_VOICE: Voice = Voice {
    start_s: 0.0,
    sound: Sound::Click(Accent::Beat),
    phase: 0.0,
    memory: 0.0,
    active: false,
};

/// Chains in flight in each direction (a new shape waits here while one is fading out).
const CHAIN_RING: usize = 8;

/// Running output; dropping it stops playback.
#[derive(Debug)]
pub struct Player {
    commands: rtrb::Producer<Command>,
    _stream: OutputStream,
    started: Arc<AtomicU64>,
    /// Write end of the monitor ring, until an input takes it.
    monitor: Option<rtrb::Producer<f32>>,
    /// Read end of what played, until a microphone input takes it (boxed: rarely used).
    reference: Option<Box<Reference>>,
    /// New effect chains, built here, for the mixer.
    chains: rtrb::Producer<Box<Chain>>,
    /// Chains the mixer has faded out; dropped here, not in the callback.
    retired: rtrb::Consumer<Box<Chain>>,
    pub sample_rate: u32,
    /// Builds the chains from the instances of the bank's `PluginRack`. Last: dropped after the
    /// stream and the rings, so every processor this player ran is parked in the rack again.
    builder: ChainBuilder,
}

/// What the mixer played (without the monitored bass), for echo removal at a microphone. The
/// mixer writes only while `armed`; `start_s` is the engine time of its first sample then.
pub struct Reference {
    pub samples: rtrb::Consumer<f32>,
    pub armed: Arc<AtomicBool>,
    pub start_s: Arc<AtomicU64>,
}

impl std::fmt::Debug for Reference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Reference")
    }
}

impl Player {
    pub fn start(
        config: &StreamConfig,
        clock: &Clock,
        events: Sender<Event>,
    ) -> Result<Player, EngineError> {
        let (producer, mut consumer) = rtrb::RingBuffer::<Command>::new(COMMANDS);
        let (monitor, monitor_rx) = rtrb::RingBuffer::<f32>::new(MONITOR_RING);
        let (reference_tx, reference_rx) = rtrb::RingBuffer::<f32>::new(REFERENCE_RING);
        let (chains_tx, chains_rx) = rtrb::RingBuffer::<Box<Chain>>::new(CHAIN_RING);
        let (retired_tx, retired_rx) = rtrb::RingBuffer::<Box<Chain>>::new(CHAIN_RING);
        let armed = Arc::new(AtomicBool::new(false));
        let reference_start = Arc::new(AtomicU64::new(f64::NAN.to_bits()));
        let started = Arc::new(AtomicU64::new(f64::NAN.to_bits()));
        let started_cb = Arc::clone(&started);
        let clock = clock.clone();
        let mut mixer = Mixer {
            voices: [IDLE_VOICE; VOICES],
            gains: Gains::default(),
            levels: Levels::of(&Gains::default()),
            backing: None,
            next: None,
            monitor: Some(monitor_rx),
            shaper: Shaper::new(48_000.0),
            chains: Some(ChainLink {
                incoming: chains_rx,
                retired: retired_tx,
            }),
            shifter: Box::new(PitchShifter::new()),
            shift_next: None,
            reference: Some(ReferenceTap {
                samples: reference_tx,
                armed: Arc::clone(&armed),
                start_s: Arc::clone(&reference_start),
            }),
            noise: Noise::new(1),
            sum_limit: OutputLimiter::new(48_000.0),
        };
        let mut frame: u64 = 0;
        // The device's rate is known only after opening; the callback reads it from here.
        let rate = Arc::new(AtomicU64::new(48_000));
        let rate_cb = Arc::clone(&rate);
        let stream = OutputStream::open(
            config,
            move |data: &mut [f32], channels: usize, lead_s: f64| {
                let sr = rate_cb.load(Ordering::Relaxed) as f64;
                // When the first frame sounds: the device's lead after now, less what played
                // before (R3).
                let played_s = frame as f64 / sr;
                let t0 = f64::from_bits(started_cb.load(Ordering::Relaxed));
                let t0 = settle(t0, clock.now_s() + lead_s - played_s, played_s);
                started_cb.store(t0.to_bits(), Ordering::Relaxed);
                while let Ok(command) = consumer.pop() {
                    mixer.apply(command);
                }
                mixer.trim_monitor(sr);
                mixer.swap_chains();
                for out in data.chunks_mut(channels.max(1)) {
                    let now = t0 + frame as f64 / sr;
                    let (left, right) = mixer.render(now, sr);
                    match out {
                        [mono] => *mono = 0.5 * (left + right),
                        [l, r, rest @ ..] => {
                            *l = left;
                            *r = right;
                            rest.iter_mut().for_each(|s| *s = 0.0);
                        }
                        [] => {}
                    }
                    frame += 1;
                }
            },
            move |message| {
                let _ = events.send(Event::Error(kontra_drivers::Direction::Output, message));
            },
        )?;
        rate.store(u64::from(stream.sample_rate), Ordering::Relaxed);
        let rate_hz = f64::from(stream.sample_rate);
        Ok(Player {
            commands: producer,
            sample_rate: stream.sample_rate,
            _stream: stream,
            started,
            monitor: Some(monitor),
            reference: Some(Box::new(Reference {
                samples: reference_rx,
                armed,
                start_s: reference_start,
            })),
            chains: chains_tx,
            retired: retired_rx,
            builder: ChainBuilder::new(rate_hz),
        })
    }

    /// Hands what plays to a microphone input (once); it starts flowing when armed.
    pub fn take_reference(&mut self) -> Option<Reference> {
        self.reference.take().map(|r| *r)
    }

    /// Hands the monitor ring's write end to an input (once).
    pub(crate) fn take_monitor(&mut self) -> Option<rtrb::Producer<f32>> {
        self.monitor.take()
    }

    /// Makes a new chain when the effects, their order or on/off differ from the last; the
    /// mixer fades over to it. Values alone travel with the gains. A full ring keeps the old
    /// shape, so the next call tries again.
    fn fit_chain(&mut self, spec: &ChainSpec, bank: &PluginBank) {
        // Dropped here, on the control thread: a plugin processor never ends in the callback.
        while self.retired.pop().is_ok() {}
        self.builder.reap();
        if self.chains.slots() == 0 {
            return;
        }
        if let Some(chain) = self.builder.fit(spec, bank) {
            // A free slot was checked, and this is the only producer.
            let _ = self.chains.push(chain);
        }
    }

    /// Hands over the chain whose plugins finished loading since `fit_chain` (they load on the
    /// plugin thread). Waits for a free slot like `fit_chain`.
    fn poll_chain(&mut self) {
        while self.retired.pop().is_ok() {}
        self.builder.reap();
        if !self.builder.is_loading() || self.chains.slots() == 0 {
            return;
        }
        if let Some(chain) = self.builder.poll() {
            // A free slot was checked, and this is the only producer.
            let _ = self.chains.push(chain);
        }
    }

    /// What the effect chain running now delays the bass by, in ms. The chain fades in a moment
    /// after it was built; the figure is the new one from then on.
    pub fn chain_latency_ms(&self) -> f64 {
        self.builder.latency_ms()
    }

    /// Output dropouts since the start (LH-NF-04).
    pub fn xruns(&self) -> u64 {
        self._stream.xruns()
    }

    /// True once the device asked for its first block (before that, nothing plays).
    pub fn is_running(&self) -> bool {
        !f64::from_bits(self.started.load(Ordering::Relaxed)).is_nan()
    }
}

impl Output for Player {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn schedule(&mut self, at_s: f64, sound: Sound) -> bool {
        self.commands
            .push(Command::Schedule { at_s, sound })
            .is_ok()
    }

    fn clear_from(&mut self, from_s: f64) {
        let _ = self.commands.push(Command::ClearFrom { from_s });
    }

    fn play_backing(&mut self, audio: BackingAudio, at_s: f64, from_frame: usize) {
        let _ = self.commands.push(Command::Backing {
            audio,
            at_s,
            from_frame,
        });
    }

    fn stop_backing(&mut self, at_s: f64) {
        let _ = self.commands.push(Command::StopBacking { at_s });
    }

    fn set_gains(&mut self, mut gains: Gains) {
        // The bank stays on this side: the callback must never drop its last reference.
        let bank = gains.plugins.take().unwrap_or_default();
        self.fit_chain(&gains.shaping.chain, &bank);
        let _ = self.commands.push(Command::Gains(gains));
    }

    fn set_transpose(&mut self, semitones: f32) {
        let _ = self.commands.push(Command::Transpose(semitones));
    }

    fn take_param_changes(&mut self) -> Vec<(u32, u32, f64)> {
        self.poll_chain();
        self.builder.take_param_changes()
    }

    fn open_editor(&mut self, plugin: u32, title: &str) -> Result<(), PluginError> {
        self.builder.open_editor(plugin, title, system_frame())
    }

    fn take_param_texts(&mut self) -> Vec<(u32, u32, f64, String)> {
        self.builder.take_param_texts()
    }
}

/// Output that plays nothing (no device, CI, file-only QA runs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NullOutput {
    pub sample_rate: u32,
}

impl Output for NullOutput {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    fn schedule(&mut self, _at_s: f64, _sound: Sound) -> bool {
        true
    }
    fn clear_from(&mut self, _from_s: f64) {}
    fn play_backing(&mut self, _audio: BackingAudio, _at_s: f64, _from_frame: usize) {}
    fn stop_backing(&mut self, _at_s: f64) {}
    fn set_gains(&mut self, _gains: Gains) {}
}

/// The mixer's end of `Reference`.
struct ReferenceTap {
    samples: rtrb::Producer<f32>,
    armed: Arc<AtomicBool>,
    start_s: Arc<AtomicU64>,
}

impl ReferenceTap {
    /// One played frame at engine time `now`; a full ring (no reader) drops it.
    fn push(&mut self, now: f64, sample: f32) {
        if !self.armed.load(Ordering::Relaxed) {
            return;
        }
        if f64::from_bits(self.start_s.load(Ordering::Relaxed)).is_nan() {
            self.start_s.store(now.to_bits(), Ordering::Relaxed);
        }
        let _ = self.samples.push(sample);
    }
}

/// The mixer's ends of the rings that carry effect chains.
struct ChainLink {
    incoming: rtrb::Consumer<Box<Chain>>,
    retired: rtrb::Producer<Box<Chain>>,
}

/// State owned by the audio callback.
struct Mixer {
    voices: [Voice; VOICES],
    gains: Gains,
    /// The volumes heard now; they follow `gains` over a few ms.
    levels: Levels,
    backing: Option<Backing>,
    /// Starts at its `at_s` and replaces `backing`.
    next: Option<Backing>,
    /// Read end of the monitor ring (mono input samples at the output rate).
    monitor: Option<rtrb::Consumer<f32>>,
    /// Shapes the monitored bass.
    shaper: Shaper,
    /// Where new effect chains come from and old ones go; `None` in tests.
    chains: Option<ChainLink>,
    /// Write side of `Reference`.
    reference: Option<ReferenceTap>,
    /// Transposes the backing track (made with the mixer: no allocation in the callback).
    shifter: Box<PitchShifter>,
    /// The backing frame the shifter expects next; `None`: start over (seek, loop, new shift).
    shift_next: Option<usize>,
    /// Source of snare and hi-hat noise.
    noise: Noise,
    /// Holds the sum of everything under full scale (LH-WIE-06).
    sum_limit: OutputLimiter,
}

impl Mixer {
    fn apply(&mut self, command: Command) {
        match command {
            Command::Schedule { at_s, sound } => {
                if let Some(free) = self.voices.iter_mut().find(|v| !v.active) {
                    *free = Voice {
                        start_s: at_s,
                        sound,
                        phase: 0.0,
                        memory: 0.0,
                        active: true,
                    };
                }
            }
            Command::ClearFrom { from_s } => {
                for v in self
                    .voices
                    .iter_mut()
                    .filter(|v| v.active && v.start_s >= from_s)
                {
                    v.active = false;
                }
            }
            Command::Backing {
                audio,
                at_s,
                from_frame,
            } => {
                self.next = Some(Backing {
                    audio,
                    at_s,
                    from_frame,
                    stop_s: f64::INFINITY,
                });
            }
            Command::StopBacking { at_s } => {
                if let Some(b) = self.backing.as_mut() {
                    b.stop_s = at_s;
                }
                if self.next.as_ref().is_some_and(|n| n.at_s >= at_s) {
                    self.next = None;
                }
            }
            Command::Gains(gains) => {
                self.shaper.set(gains.shaping);
                self.gains = gains;
            }
            Command::Transpose(semitones) => {
                if self.shifter.semitones() == 0.0 {
                    self.shift_next = None;
                }
                self.shifter.set_semitones(semitones);
            }
        }
    }

    /// Once per block: hands a faded-out chain back and takes the next new one, when the
    /// shaper is ready for it (one fade at a time). Nothing here allocates or frees.
    fn swap_chains(&mut self) {
        let Some(link) = self.chains.as_mut() else {
            return;
        };
        if let Some(old) = self.shaper.take_retired() {
            // Only a control side that stopped listening leaves the ring full: then the
            // chain is freed here, once, rather than kept for ever.
            let _ = link.retired.push(old);
        }
        if !self.shaper.can_swap() {
            return;
        }
        if let Ok(new) = link.incoming.pop() {
            // `can_swap` held, so the shaper takes it.
            let _ = self.shaper.swap_chain(new);
        }
    }

    /// Once per block: drops queued monitor audio that is muted or lags too far behind.
    /// Example at 48 kHz: 5000 queued frames (104 ms) are cut to the newest 960 (20 ms).
    fn trim_monitor(&mut self, sr: f64) {
        let Some(ring) = self.monitor.as_mut() else {
            return;
        };
        let queued = ring.slots();
        // Still heard while the volume ramps down: kept until it is silent (review #285).
        let heard = self.gains.monitor > 0.0 || self.levels.monitor > 0.0;
        let keep = if heard {
            (MONITOR_KEEP_S * sr) as usize
        } else {
            0
        };
        if heard && queued <= (MONITOR_MAX_S * sr) as usize {
            return;
        }
        if let Ok(chunk) = ring.read_chunk(queued.saturating_sub(keep)) {
            chunk.commit_all();
        }
    }

    /// One stereo frame at engine time `now`.
    fn render(&mut self, now: f64, sr: f64) -> (f32, f32) {
        self.levels.follow(&Levels::of(&self.gains), sr as f32);
        let mono = render(
            &mut self.voices,
            now,
            sr as f32,
            &self.levels,
            &mut self.noise,
        );
        let (mut left, mut right) = (mono, mono);
        if self.next.as_ref().is_some_and(|n| now >= n.at_s) {
            // The control side still holds the old Arc, so this drop frees nothing here.
            self.backing = self.next.take();
        }
        if let Some(b) = &self.backing {
            if now >= b.at_s && now < b.stop_s {
                let frame = b.from_frame + ((now - b.at_s) * sr) as usize;
                let bass = self.levels.bass;
                let (l, r) = if self.shifter.semitones() == 0.0 {
                    b.audio.frame(frame, bass)
                } else {
                    let shift = (&mut *self.shifter, &mut self.shift_next);
                    shifted_frame(shift, &b.audio, frame, bass)
                };
                left += l * self.levels.backing;
                right += r * self.levels.backing;
            }
        }
        // What plays from the speakers, before the monitored bass (that one the mic hears anyway).
        let played = 0.5 * (left + right);
        // Monitoring: one input sample per frame; an empty ring (underrun) stays silent.
        if let Some(ring) = self.monitor.as_mut() {
            let dry = ring.pop().unwrap_or(0.0);
            let (l, r) = self.shaper.process(dry, sr);
            left += l * self.levels.monitor;
            right += r * self.levels.monitor;
        }

        // The sum never reaches full scale: a loud spot gets quieter for a moment instead of
        // being cut off (a crackle). The echo reference follows, it is what the speakers play.
        let (left, right) = self.sum_limit.process((left, right), sr);
        if let Some(tap) = self.reference.as_mut() {
            tap.push(now, played * self.sum_limit.gain());
        }
        (left.clamp(-1.0, 1.0), right.clamp(-1.0, 1.0))
    }
}

/// Backing frame `frame`, transposed. The shifter delays by `PITCH_LATENCY`, so it is fed that
/// far ahead; after a jump (seek, loop start) it is filled again first, which costs one
/// window of work once. Frames one off (rounding of the clock) count as continuous.
fn shifted_frame(
    (shifter, next): (&mut PitchShifter, &mut Option<usize>),
    audio: &BackingAudio,
    frame: usize,
    bass: f32,
) -> (f32, f32) {
    if !next.is_some_and(|n| n.abs_diff(frame) <= 1) {
        shifter.reset();
        for f in frame..frame + PITCH_LATENCY {
            let (l, r) = audio.frame(f, bass);
            shifter.process(l, r);
        }
    }
    *next = Some(frame + 1);
    let (l, r) = audio.frame(frame + PITCH_LATENCY, bass);
    shifter.process(l, r)
}

/// Mixes all voices for one frame at engine time `now`.
fn render(
    voices: &mut [Voice; VOICES],
    now: f64,
    sr: f32,
    gains: &Levels,
    noise: &mut Noise,
) -> f32 {
    let mut mix = 0.0;
    for v in voices.iter_mut().filter(|v| v.active && v.start_s <= now) {
        let t = (now - v.start_s) as f32;
        match v.sound {
            Sound::Click(accent) => {
                if t > CLICK_S {
                    v.active = false;
                    continue;
                }
                // The downbeat is higher; the off-beat between beats is softer.
                let (hz, level) = match accent {
                    Accent::Downbeat => (2000.0, CLICK_LEVEL),
                    Accent::Beat => (1400.0, CLICK_LEVEL),
                    Accent::Offbeat => (1400.0, OFFBEAT_LEVEL),
                };
                v.phase = (v.phase + hz / sr).fract();
                mix += gains.click * level * (TAU * v.phase).sin() * (-t / 0.004).exp();
            }
            Sound::Tone {
                hz,
                duration_s,
                velocity,
            } => {
                if t > duration_s {
                    v.active = false;
                    continue;
                }
                v.phase = (v.phase + hz / sr).fract();
                let p = TAU * v.phase;
                let wave = p.sin()
                    + 0.6 * (2.0 * p).sin()
                    + 0.3 * (3.0 * p).sin()
                    + 0.15 * (4.0 * p).sin();
                let release = ((duration_s - t) / 0.01).min(1.0);
                mix += gains.tone * velocity * 0.25 * wave * (-t / 1.2).exp() * release;
            }
            Sound::Drum { kind, velocity } => {
                if t > drum_length_s(kind) {
                    v.active = false;
                    continue;
                }
                mix += gains.drums * velocity * drum(v, kind, t, sr, noise.sample());
            }
        }
    }
    mix
}

fn drum_length_s(kind: DrumKind) -> f32 {
    match kind {
        DrumKind::Kick => KICK_S,
        DrumKind::Snare => SNARE_S,
        DrumKind::HiHat => HIHAT_S,
    }
}

/// One sample of a drum voice `t` seconds after its start; `white` is fresh noise in [-1, 1).
///
/// Kick: a sine falling from 150 Hz to 50 Hz. Snare: a 190 Hz body plus noise. Hi-hat: noise
/// through a first-difference high-pass (only the sizzle stays).
fn drum(v: &mut Voice, kind: DrumKind, t: f32, sr: f32, white: f32) -> f32 {
    // A short fade at the end: no click when the voice stops.
    let fade = ((drum_length_s(kind) - t) / 0.01).min(1.0);
    match kind {
        DrumKind::Kick => {
            let hz = 50.0 + 100.0 * (-t / 0.03).exp();
            v.phase = (v.phase + hz / sr).fract();
            0.8 * (TAU * v.phase).sin() * (-t / 0.12).exp() * fade
        }
        DrumKind::Snare => {
            v.phase = (v.phase + 190.0 / sr).fract();
            let body = 0.3 * (TAU * v.phase).sin() * (-t / 0.03).exp();
            let rattle = 0.35 * white * (-t / 0.05).exp();
            (body + rattle) * fade
        }
        DrumKind::HiHat => {
            let high = white - v.memory;
            v.memory = white;
            0.12 * high * (-t / 0.015).exp() * fade
        }
    }
}

mod levels;
#[cfg(test)]
mod tests;
