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
use kontra_model::DrumKind;
use std::f32::consts::TAU;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;

const COMMANDS: usize = 1024;
const VOICES: usize = 48;
const CLICK_S: f32 = 0.02;
/// Drum voices: length, decay time constant of the body.
const KICK_S: f32 = 0.35;
const SNARE_S: f32 = 0.2;
const HIHAT_S: f32 = 0.08;
/// Monitor ring: half a second at the highest supported rate.
const MONITOR_RING: usize = 96_000;
/// More queued monitor audio than this means the clocks drifted apart; the excess is dropped
/// down to `MONITOR_KEEP_S`, so the bass never lags further behind.
const MONITOR_MAX_S: f64 = 0.08;
const MONITOR_KEEP_S: f64 = 0.02;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accent {
    /// First beat of a bar.
    Downbeat,
    Beat,
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

/// Interleaved stereo audio at the output's sample rate.
pub type BackingAudio = Arc<Vec<f32>>;

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
}

/// Bus volumes, 0..=1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gains {
    pub click: f32,
    pub tone: f32,
    pub backing: f32,
    /// Synthesised drum accompaniment (LH-LRN-01).
    pub drums: f32,
    /// The bass input played back (input monitoring); 0 = off.
    pub monitor: f32,
}

impl Default for Gains {
    fn default() -> Self {
        Gains {
            click: 0.8,
            tone: 0.6,
            backing: 0.8,
            drums: 0.7,
            monitor: 0.0,
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
    pub sample_rate: u32,
}

impl Player {
    pub fn start(
        config: &StreamConfig,
        clock: &Clock,
        events: Sender<Event>,
    ) -> Result<Player, EngineError> {
        let (producer, mut consumer) = rtrb::RingBuffer::<Command>::new(COMMANDS);
        let (monitor, monitor_rx) = rtrb::RingBuffer::<f32>::new(MONITOR_RING);
        let started = Arc::new(AtomicU64::new(f64::NAN.to_bits()));
        let started_cb = Arc::clone(&started);
        let clock = clock.clone();
        let mut mixer = Mixer {
            voices: [IDLE_VOICE; VOICES],
            gains: Gains::default(),
            backing: None,
            next: None,
            monitor: Some(monitor_rx),
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
                let _ = events.send(Event::Error(message));
            },
        )?;
        rate.store(u64::from(stream.sample_rate), Ordering::Relaxed);
        Ok(Player {
            commands: producer,
            sample_rate: stream.sample_rate,
            _stream: stream,
            started,
            monitor: Some(monitor),
        })
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

/// State owned by the audio callback.
struct Mixer {
    voices: [Voice; VOICES],
    gains: Gains,
    backing: Option<Backing>,
    /// Starts at its `at_s` and replaces `backing`.
    next: Option<Backing>,
    /// Read end of the monitor ring (mono input samples at the output rate).
    monitor: Option<rtrb::Consumer<f32>>,
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
            Command::Gains(gains) => self.gains = gains,
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
                if let Some(pair) = b.audio.get(frame * 2..frame * 2 + 2) {
                    left += pair[0] * self.gains.backing;
                    right += pair[1] * self.gains.backing;
                }
            }
        }
        // Monitoring: one input sample per frame; an empty ring (underrun) stays silent.
        if let Some(ring) = self.monitor.as_mut() {
            let sample = ring.pop().unwrap_or(0.0) * self.gains.monitor;
            left += sample;
            right += sample;
        }
        (left.clamp(-1.0, 1.0), right.clamp(-1.0, 1.0))
    }
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
                let hz = if accent == Accent::Downbeat {
                    2000.0
                } else {
                    1400.0
                };
                v.phase = (v.phase + hz / sr).fract();
                mix += gains.click * 0.5 * (TAU * v.phase).sin() * (-t / 0.004).exp();
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
                monitor: 1.0,
            },
            backing: None,
            next: None,
            monitor: None,
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
        let audio: BackingAudio = Arc::new(
            (0..2000)
                .flat_map(|i| [i as f32 / 4000.0, -(i as f32) / 4000.0])
                .collect(),
        );
        m.apply(Command::Backing {
            audio: Arc::clone(&audio),
            at_s: 1.0,
            from_frame: 100,
        });
        let sr = 1000.0;
        assert_eq!(m.render(0.5, sr), (0.0, 0.0));
        let (l, r) = m.render(1.010, sr);
        assert!((l - 110.0 / 4000.0).abs() < 1e-6 && (r + 110.0 / 4000.0).abs() < 1e-6);
        // A pending start (next loop pass) takes over at its time.
        m.apply(Command::Backing {
            audio: Arc::clone(&audio),
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
