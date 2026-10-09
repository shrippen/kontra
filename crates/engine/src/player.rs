//! Output: metronome clicks and reference tones, scheduled on the engine clock.
//!
//! The control thread schedules sounds slightly ahead (≥ 100 ms); the audio callback starts each
//! one at the exact frame. Nothing in the callback allocates.

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

#[derive(Debug, Clone, Copy, PartialEq)]
enum Command {
    Schedule {
        at_s: f64,
        sound: Sound,
    },
    /// Drops everything scheduled at or after `from_s` (seek, pause, tempo change).
    ClearFrom {
        from_s: f64,
    },
    Gain {
        bus: Bus,
        gain: f32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Bus {
    Click,
    Tone,
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
        let mut voices = [IDLE_VOICE; VOICES];
        let mut gains = (0.8f32, 0.6f32);
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
                    apply(command, &mut voices, &mut gains);
                }
                data.iter_mut().for_each(|s| *s = 0.0);
                let sr = rate_cb.load(Ordering::Relaxed) as f64;
                for out in data.chunks_mut(channels.max(1)) {
                    let now = t0 + frame as f64 / sr;
                    let value = render(&mut voices, now, sr as f32, gains);
                    out.iter_mut().for_each(|s| *s = value);
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

    /// Plays `sound` at engine time `at_s`. Returns false if the command queue is full.
    pub fn schedule(&mut self, at_s: f64, sound: Sound) -> bool {
        self.commands
            .push(Command::Schedule { at_s, sound })
            .is_ok()
    }

    pub fn clear_from(&mut self, from_s: f64) {
        let _ = self.commands.push(Command::ClearFrom { from_s });
    }

    pub fn set_click_gain(&mut self, gain: f32) {
        let _ = self.commands.push(Command::Gain {
            bus: Bus::Click,
            gain,
        });
    }

    pub fn set_tone_gain(&mut self, gain: f32) {
        let _ = self.commands.push(Command::Gain {
            bus: Bus::Tone,
            gain,
        });
    }
}

fn apply(command: Command, voices: &mut [Voice; VOICES], gains: &mut (f32, f32)) {
    match command {
        Command::Schedule { at_s, sound } => {
            if let Some(free) = voices.iter_mut().find(|v| !v.active) {
                *free = Voice {
                    start_s: at_s,
                    sound,
                    phase: 0.0,
                    active: true,
                };
            }
        }
        Command::ClearFrom { from_s } => {
            for v in voices
                .iter_mut()
                .filter(|v| v.active && v.start_s >= from_s)
            {
                v.active = false;
            }
        }
        Command::Gain {
            bus: Bus::Click,
            gain,
        } => gains.0 = gain.clamp(0.0, 1.0),
        Command::Gain {
            bus: Bus::Tone,
            gain,
        } => gains.1 = gain.clamp(0.0, 1.0),
    }
}

/// Mixes all voices for one frame at engine time `now`.
fn render(voices: &mut [Voice; VOICES], now: f64, sr: f32, gains: (f32, f32)) -> f32 {
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
                mix += gains.0 * 0.5 * (TAU * v.phase).sin() * (-t / 0.004).exp();
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
                mix += gains.1 * velocity * 0.25 * wave * (-t / 1.2).exp() * release;
            }
        }
    }
    mix.clamp(-1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn click_starts_on_time_and_ends() {
        let mut voices = [IDLE_VOICE; VOICES];
        let mut gains = (1.0, 1.0);
        apply(
            Command::Schedule {
                at_s: 0.01,
                sound: Sound::Click(Accent::Downbeat),
            },
            &mut voices,
            &mut gains,
        );
        let sr = 48_000.0;
        let before: f32 = (0..400)
            .map(|i| render(&mut voices, i as f64 / 48_000.0, sr, gains).abs())
            .sum();
        assert_eq!(before, 0.0);
        let during: f32 = (480..960)
            .map(|i| render(&mut voices, i as f64 / 48_000.0, sr, gains).abs())
            .sum();
        assert!(during > 1.0);
        render(&mut voices, 1.0, sr, gains);
        assert!(voices.iter().all(|v| !v.active));
    }

    #[test]
    fn clear_drops_future_sounds() {
        let mut voices = [IDLE_VOICE; VOICES];
        let mut gains = (1.0, 1.0);
        apply(
            Command::Schedule {
                at_s: 1.0,
                sound: Sound::Click(Accent::Beat),
            },
            &mut voices,
            &mut gains,
        );
        apply(
            Command::Schedule {
                at_s: 2.0,
                sound: Sound::Click(Accent::Beat),
            },
            &mut voices,
            &mut gains,
        );
        apply(Command::ClearFrom { from_s: 1.5 }, &mut voices, &mut gains);
        assert_eq!(voices.iter().filter(|v| v.active).count(), 1);
    }
}
