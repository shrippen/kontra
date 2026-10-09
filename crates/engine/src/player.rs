//! Output: metronome clicks, reference tones and the backing track, scheduled on the engine clock.
//!
//! The control thread schedules sounds slightly ahead (≥ 100 ms); the audio callback starts each
//! one at the exact frame. Nothing in the callback allocates or frees: backing audio arrives as an
//! `Arc` that the control side keeps alive (see `Output::play_backing`).

use crate::clock::Clock;
use crate::{EngineError, Event};
use kontra_drivers::{OutputStream, StreamConfig};
use std::f32::consts::TAU;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;

const COMMANDS: usize = 1024;
const VOICES: usize = 48;
const CLICK_S: f32 = 0.02;

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
}

impl Default for Gains {
    fn default() -> Self {
        Gains {
            click: 0.8,
            tone: 0.6,
            backing: 0.8,
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
    active: bool,
}

const IDLE_VOICE: Voice = Voice {
    start_s: 0.0,
    sound: Sound::Click(Accent::Beat),
    phase: 0.0,
    active: false,
};

/// Running output; dropping it stops playback.
#[derive(Debug)]
pub struct Player {
    commands: rtrb::Producer<Command>,
    _stream: OutputStream,
    started: Arc<AtomicU64>,
    pub sample_rate: u32,
}

impl Player {
    pub fn start(
        config: &StreamConfig,
        clock: &Clock,
        events: Sender<Event>,
    ) -> Result<Player, EngineError> {
        let (producer, mut consumer) = rtrb::RingBuffer::<Command>::new(COMMANDS);
        let started = Arc::new(AtomicU64::new(f64::NAN.to_bits()));
        let started_cb = Arc::clone(&started);
        let clock = clock.clone();
        let mut mixer = Mixer {
            voices: [IDLE_VOICE; VOICES],
            gains: Gains::default(),
            backing: None,
            next: None,
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
        })
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

    /// One stereo frame at engine time `now`.
    fn render(&mut self, now: f64, sr: f64) -> (f32, f32) {
        let mono = render(&mut self.voices, now, sr as f32, self.gains);
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
        (left.clamp(-1.0, 1.0), right.clamp(-1.0, 1.0))
    }
}

/// Mixes all voices for one frame at engine time `now`.
fn render(voices: &mut [Voice; VOICES], now: f64, sr: f32, gains: Gains) -> f32 {
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
        }
    }
    mix
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
            },
            backing: None,
            next: None,
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
}
