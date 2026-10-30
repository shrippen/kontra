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

use crate::clock::Clock;
use crate::{EngineError, Event};
use kontra_drivers::{OutputStream, StreamConfig};
use kontra_dsp::synth::Noise;
use kontra_dsp::{PitchShifter, Shaper, Shaping, PITCH_LATENCY};
use kontra_model::DrumKind;
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
    /// Accompaniment drum stroke (LH-LRN-01).
    Drum(DrumKind),
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
}

/// Bus volumes, 0..=1.
#[derive(Debug, Clone, Copy, PartialEq)]
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
        }
    }
}

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
    pub sample_rate: u32,
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
        let armed = Arc::new(AtomicBool::new(false));
        let reference_start = Arc::new(AtomicU64::new(f64::NAN.to_bits()));
        let started = Arc::new(AtomicU64::new(f64::NAN.to_bits()));
        let started_cb = Arc::clone(&started);
        let clock = clock.clone();
        let mut mixer = Mixer {
            voices: [IDLE_VOICE; VOICES],
            gains: Gains::default(),
            backing: None,
            next: None,
            monitor: Some(monitor_rx),
            shaper: Shaper::new(48_000.0),
            shifter: Box::new(PitchShifter::new()),
            shift_next: None,
            reference: Some(ReferenceTap {
                samples: reference_tx,
                armed: Arc::clone(&armed),
                start_s: Arc::clone(&reference_start),
            }),
            noise: Noise::new(1),
        };
        let mut frame: u64 = 0;
        let mut first_block = true;
        // The device's rate is known only after opening; the callback reads it from here.
        let rate = Arc::new(AtomicU64::new(48_000));
        let rate_cb = Arc::clone(&rate);
        let stream = OutputStream::open(
            config,
            move |data: &mut [f32], channels: usize| {
                if first_block {
                    started_cb.store(clock.now_s().to_bits(), Ordering::Relaxed);
                    first_block = false;
                }
                let t0 = f64::from_bits(started_cb.load(Ordering::Relaxed));
                while let Ok(command) = consumer.pop() {
                    mixer.apply(command);
                }
                let sr = rate_cb.load(Ordering::Relaxed) as f64;
                mixer.trim_monitor(sr);
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

    fn set_gains(&mut self, gains: Gains) {
        let _ = self.commands.push(Command::Gains(gains));
    }

    fn set_transpose(&mut self, semitones: f32) {
        let _ = self.commands.push(Command::Transpose(semitones));
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

/// State owned by the audio callback.
struct Mixer {
    voices: [Voice; VOICES],
    gains: Gains,
    backing: Option<Backing>,
    /// Starts at its `at_s` and replaces `backing`.
    next: Option<Backing>,
    /// Read end of the monitor ring (mono input samples at the output rate).
    monitor: Option<rtrb::Consumer<f32>>,
    /// Shapes the monitored bass.
    shaper: Shaper,
    /// Write side of `Reference`.
    reference: Option<ReferenceTap>,
    /// Transposes the backing track (made with the mixer: no allocation in the callback).
    shifter: Box<PitchShifter>,
    /// The backing frame the shifter expects next; `None`: start over (seek, loop, new shift).
    shift_next: Option<usize>,
    /// Source of snare and hi-hat noise.
    noise: Noise,
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
                self.gains = gains;
                self.shaper.set(gains.shaping);
            }
            Command::Transpose(semitones) => {
                if self.shifter.semitones() == 0.0 {
                    self.shift_next = None;
                }
                self.shifter.set_semitones(semitones);
            }
        }
    }

    /// Once per block: drops queued monitor audio that is muted or lags too far behind.
    /// Example at 48 kHz: 5000 queued frames (104 ms) are cut to the newest 960 (20 ms).
    fn trim_monitor(&mut self, sr: f64) {
        let Some(ring) = self.monitor.as_mut() else {
            return;
        };
        let queued = ring.slots();
        let keep = if self.gains.monitor > 0.0 {
            (MONITOR_KEEP_S * sr) as usize
        } else {
            0
        };
        if self.gains.monitor > 0.0 && queued <= (MONITOR_MAX_S * sr) as usize {
            return;
        }
        if let Ok(chunk) = ring.read_chunk(queued.saturating_sub(keep)) {
            chunk.commit_all();
        }
    }

    /// One stereo frame at engine time `now`.
    fn render(&mut self, now: f64, sr: f64) -> (f32, f32) {
        let mono = render(
            &mut self.voices,
            now,
            sr as f32,
            self.gains,
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
                let bass = self.gains.bass;
                let (l, r) = if self.shifter.semitones() == 0.0 {
                    b.audio.frame(frame, bass)
                } else {
                    let shift = (&mut *self.shifter, &mut self.shift_next);
                    shifted_frame(shift, &b.audio, frame, bass)
                };
                left += l * self.gains.backing;
                right += r * self.gains.backing;
            }
        }
        // What plays from the speakers, before the monitored bass (that one the mic hears anyway).
        if let Some(tap) = self.reference.as_mut() {
            tap.push(now, 0.5 * (left + right));
        }
        // Monitoring: one input sample per frame; an empty ring (underrun) stays silent.
        if let Some(ring) = self.monitor.as_mut() {
            let dry = ring.pop().unwrap_or(0.0);
            let sample = self.shaper.process(dry, sr) * self.gains.monitor;
            left += sample;
            right += sample;
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
fn render(voices: &mut [Voice; VOICES], now: f64, sr: f32, gains: Gains, noise: &mut Noise) -> f32 {
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
            Sound::Drum(kind) => {
                if t > drum_length_s(kind) {
                    v.active = false;
                    continue;
                }
                mix += gains.drums * drum(v, kind, t, sr, noise.sample());
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

#[cfg(test)]
mod tests {
    use super::*;

    fn mixer() -> Mixer {
        Mixer {
            voices: [IDLE_VOICE; VOICES],
            gains: Gains {
                click: 1.0,
                tone: 1.0,
                backing: 1.0,
                drums: 1.0,
                bass: 0.0,
                monitor: 1.0,
                shaping: Shaping::FLAT,
            },
            backing: None,
            next: None,
            monitor: None,
            shaper: Shaper::new(48_000.0),
            shifter: Box::new(PitchShifter::new()),
            shift_next: None,
            reference: None,
            noise: Noise::new(1),
        }
    }

    #[test]
    fn click_starts_on_time_and_ends() {
        let mut m = mixer();
        m.apply(Command::Schedule {
            at_s: 0.01,
            sound: Sound::Click(Accent::Downbeat),
        });
        let sr = 48_000.0;
        let before: f32 = (0..400).map(|i| m.render(i as f64 / sr, sr).0.abs()).sum();
        assert_eq!(before, 0.0);
        let during: f32 = (480..960)
            .map(|i| m.render(i as f64 / sr, sr).0.abs())
            .sum();
        assert!(during > 1.0);
        m.render(1.0, sr);
        assert!(m.voices.iter().all(|v| !v.active));
    }

    // LH-WIE-04: the shuffle's off-beat click is softer than the beat, so the pulse stays clear.
    #[test]
    fn offbeat_click_is_softer() {
        let sr = 48_000.0;
        let energy = |accent| {
            let mut m = mixer();
            m.apply(Command::Schedule {
                at_s: 0.0,
                sound: Sound::Click(accent),
            });
            (0..960)
                .map(|i| m.render(i as f64 / sr, sr).0.abs())
                .sum::<f32>()
        };
        let (beat, offbeat) = (energy(Accent::Beat), energy(Accent::Offbeat));
        assert!(offbeat > 0.1 * beat, "audible: {offbeat} vs {beat}");
        assert!(offbeat < 0.75 * beat, "softer: {offbeat} vs {beat}");
    }

    #[test]
    fn drums_sound_and_end() {
        let sr = 48_000.0;
        for kind in [DrumKind::Kick, DrumKind::Snare, DrumKind::HiHat] {
            let mut m = mixer();
            m.apply(Command::Schedule {
                at_s: 0.0,
                sound: Sound::Drum(kind),
            });
            let energy: f32 = (0..2400).map(|i| m.render(i as f64 / sr, sr).0.abs()).sum();
            assert!(energy > 1.0, "{kind:?}: {energy}");
            m.render(1.0, sr);
            assert!(m.voices.iter().all(|v| !v.active), "{kind:?}");
        }
        // The drum bus has its own volume.
        let mut m = mixer();
        m.gains.drums = 0.0;
        m.apply(Command::Schedule {
            at_s: 0.0,
            sound: Sound::Drum(DrumKind::Kick),
        });
        assert_eq!(m.render(0.001, sr), (0.0, 0.0));
    }

    #[test]
    fn clear_drops_future_sounds() {
        let mut m = mixer();
        m.apply(Command::Schedule {
            at_s: 1.0,
            sound: Sound::Click(Accent::Beat),
        });
        m.apply(Command::Schedule {
            at_s: 2.0,
            sound: Sound::Click(Accent::Beat),
        });
        m.apply(Command::ClearFrom { from_s: 1.5 });
        assert_eq!(m.voices.iter().filter(|v| v.active).count(), 1);
    }

    #[test]
    fn backing_starts_at_frame_and_stops() {
        let mut m = mixer();
        let audio = BackingAudio::stereo(
            (0..2000)
                .flat_map(|i| [i as f32 / 4000.0, -(i as f32) / 4000.0])
                .collect(),
        );
        m.apply(Command::Backing {
            audio: audio.clone(),
            at_s: 1.0,
            from_frame: 100,
        });
        let sr = 1000.0;
        assert_eq!(m.render(0.5, sr), (0.0, 0.0));
        let (l, r) = m.render(1.010, sr);
        assert!((l - 110.0 / 4000.0).abs() < 1e-6 && (r + 110.0 / 4000.0).abs() < 1e-6);
        // A pending start (next loop pass) takes over at its time.
        m.apply(Command::Backing {
            audio: audio.clone(),
            at_s: 1.2,
            from_frame: 0,
        });
        assert!((m.render(1.1, sr).0 - 200.0 / 4000.0).abs() < 1e-6);
        assert!((m.render(1.201, sr).0 - 1.0 / 4000.0).abs() < 1e-6);
        m.apply(Command::StopBacking { at_s: 1.5 });
        assert_eq!(m.render(1.6, sr), (0.0, 0.0));
    }

    /// A mixer with a monitor ring; returns the ring's write end.
    fn monitored(gain: f32) -> (Mixer, rtrb::Producer<f32>) {
        let (tx, rx) = rtrb::RingBuffer::<f32>::new(MONITOR_RING);
        let mut m = mixer();
        m.gains.monitor = gain;
        m.monitor = Some(rx);
        (m, tx)
    }

    #[test]
    fn monitor_plays_input_at_its_gain() {
        let (mut m, mut tx) = monitored(0.5);
        for _ in 0..10 {
            tx.push(0.4).unwrap();
        }
        let sr = 48_000.0;
        m.trim_monitor(sr);
        let (l, r) = m.render(0.0, sr);
        assert!((l - 0.2).abs() < 1e-6 && (r - 0.2).abs() < 1e-6);
        // Underrun: silence, not stale audio.
        for i in 1..10 {
            m.render(f64::from(i) / sr, sr);
        }
        assert_eq!(m.render(1.0, sr), (0.0, 0.0));
    }

    /// LH-WIE-02 (round 8): the mixer transposes the backing while it plays; a note in the
    /// backing still sounds at its own time (the shifter is fed ahead by its latency).
    #[test]
    fn backing_is_transposed_in_time() {
        use kontra_dsp::Yin;
        let sr = 48_000.0;
        // Silence, then A2 from 0.5 s, as interleaved stereo.
        let mut audio = vec![0.0f32; 2 * 24_000];
        for i in 0..48_000 {
            let x = 0.5 * (std::f64::consts::TAU * 110.0 * f64::from(i) / sr).sin() as f32;
            audio.extend([x, x]);
        }
        let mut m = mixer();
        m.gains.click = 0.0;
        m.apply(Command::Backing {
            audio: BackingAudio::stereo(audio),
            at_s: 0.0,
            from_frame: 0,
        });
        m.apply(Command::Transpose(3.0));
        let out: Vec<f32> = (0..72_000)
            .map(|i| m.render(f64::from(i) / sr, sr).0)
            .collect();

        let peak = out.iter().fold(0.0f32, |p, s| p.max(s.abs()));
        let onset = out.iter().position(|s| s.abs() > peak * 0.3).unwrap_or(0);
        assert!(onset.abs_diff(24_000) < 240, "starts at {onset}, not 24000");
        let mut yin = Yin::new(sr, 40.0, 800.0, 0.04);
        let hz = yin.estimate(&out[40_000..64_000]).map_or(0.0, |e| e.hz);
        let cents = 1200.0 * (hz / (110.0 * 2f64.powf(3.0 / 12.0))).log2();
        assert!(cents.abs() < 5.0, "{hz} Hz, {cents} cents");
    }

    /// LH-BIB-09: the removed bass comes back at `Gains::bass`, also while it plays.
    #[test]
    fn removed_bass_follows_its_gain() {
        let mut m = mixer();
        // Every frame: song without bass 0.1, removed bass 0.4 (left), -0.4 (right).
        let audio = BackingAudio::with_bass([0.1, 0.1, 0.4, -0.4].repeat(1000));
        m.apply(Command::Backing {
            audio,
            at_s: 0.0,
            from_frame: 0,
        });
        let sr = 1000.0;
        assert_eq!(m.render(0.1, sr), (0.1, 0.1));

        m.apply(Command::Gains(Gains {
            bass: 0.5,
            ..m.gains
        }));
        let (l, r) = m.render(0.2, sr);
        assert!((l - 0.3).abs() < 1e-6 && (r + 0.1).abs() < 1e-6, "{l} {r}");
    }

    #[test]
    fn monitor_is_shaped() {
        // A 6 dB low boost doubles a slow ramp (low frequency) once the change has glided in.
        let (mut m, mut tx) = monitored(1.0);
        m.apply(Command::Gains(Gains {
            shaping: Shaping {
                low_db: 6.0,
                ..Shaping::FLAT
            },
            ..m.gains
        }));
        let sr = 48_000.0;
        let mut last = 0.0;
        for i in 0..4000 {
            tx.push(0.1).unwrap();
            last = m.render(f64::from(i) / sr, sr).0;
        }
        assert!((last - 0.2).abs() < 0.01, "{last}");
    }

    #[test]
    fn muted_monitor_drains_its_ring() {
        let (mut m, mut tx) = monitored(0.0);
        for _ in 0..1000 {
            tx.push(0.4).unwrap();
        }
        m.trim_monitor(48_000.0);
        assert_eq!(m.render(0.0, 48_000.0), (0.0, 0.0));
        // Switched on later: starts with fresh input, not with what queued while muted.
        m.apply(Command::Gains(Gains {
            monitor: 1.0,
            ..m.gains
        }));
        assert_eq!(m.render(0.0, 48_000.0), (0.0, 0.0));
    }

    #[test]
    fn lagging_monitor_is_cut_to_the_newest_audio() {
        let (mut m, mut tx) = monitored(1.0);
        let sr = 1000.0;
        // 200 ms queued at 1 kHz: older samples 0.1, the newest 20 ms 0.3.
        for i in 0..200 {
            tx.push(if i < 180 { 0.1 } else { 0.3 }).unwrap();
        }
        m.trim_monitor(sr);
        assert!((m.render(0.0, sr).0 - 0.3).abs() < 1e-6);
        // Within the limit nothing is dropped.
        for _ in 0..50 {
            tx.push(0.2).unwrap();
        }
        m.trim_monitor(sr);
        assert!((m.render(0.0, sr).0 - 0.3).abs() < 1e-6);
    }
}
