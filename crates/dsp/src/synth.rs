//! Deterministic bass-like test signals and input-path simulations.
//!
//! Used by tests, by the synthetic corpus (`kontra korpus`) and as the reference bass the player
//! can listen to (LH-WIE-03). Additive synthesis of a stiff string: partial n sits at
//! `n·f0·√(1 + B·n²)`, decays faster the higher it is, and the attack carries a short noise burst
//! like a finger or pick hitting the string.

use crate::biquad::{Biquad, FilterKind};
use std::f64::consts::PI;

/// Inharmonicity of a bass string (B in the stiff-string formula).
const INHARMONICITY: f64 = 0.000_08;
const ATTACK_NOISE_S: f64 = 0.004;

/// Timbre of the synthetic bass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToneShape {
    /// Round-wound strings, fingerstyle: many partials, strong 2nd harmonic.
    Rich,
    /// Flat-wound or tone knob down: few partials.
    Dull,
}

/// One note for `render`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SynthNote {
    pub onset_s: f64,
    pub hz: f64,
    pub duration_s: f64,
    /// 0..=1.
    pub velocity: f32,
}

/// Small deterministic noise source (xorshift64*).
#[derive(Debug, Clone)]
pub struct Noise(u64);

impl Noise {
    pub fn new(seed: u64) -> Noise {
        Noise(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    /// Uniform in [-1, 1).
    pub fn sample(&mut self) -> f32 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        let v = self.0.wrapping_mul(0x2545_F491_4F6C_DD1D);
        ((v >> 40) as f32 / (1u64 << 23) as f32) - 1.0
    }
}

/// A single tone of `duration_s` starting at sample 0.
pub fn bass_tone(
    hz: f64,
    sample_rate: f64,
    duration_s: f64,
    shape: ToneShape,
    seed: u64,
) -> Vec<f32> {
    let note = SynthNote {
        onset_s: 0.0,
        hz,
        duration_s,
        velocity: 0.8,
    };
    render(&[note], sample_rate, duration_s, shape, seed)
}

/// Renders notes into one buffer of `total_s`. A new note cuts the ringing of the previous one
/// (a bassist stops the old string), with a 15 ms fade.
pub fn render(
    notes: &[SynthNote],
    sample_rate: f64,
    total_s: f64,
    shape: ToneShape,
    seed: u64,
) -> Vec<f32> {
    let len = (total_s * sample_rate).ceil() as usize;
    let mut out = vec![0.0f32; len];
    let mut noise = Noise::new(seed);
    let mut sorted: Vec<&SynthNote> = notes.iter().collect();
    sorted.sort_by(|a, b| a.onset_s.total_cmp(&b.onset_s));
    for (i, note) in sorted.iter().enumerate() {
        let next = sorted.get(i + 1).map_or(f64::INFINITY, |n| n.onset_s);
        let end_s = (note.onset_s + note.duration_s).min(next);
        add_note(&mut out, note, end_s, sample_rate, shape, &mut noise);
    }
    out
}

fn add_note(
    out: &mut [f32],
    note: &SynthNote,
    end_s: f64,
    sr: f64,
    shape: ToneShape,
    noise: &mut Noise,
) {
    let start = (note.onset_s * sr).round() as usize;
    let end = ((end_s * sr).round() as usize).min(out.len());
    if start >= end {
        return;
    }
    let partials = partials(note.hz, sr, shape);
    // Muting a string takes a moment; a hard cut would click like a new attack.
    let fade = (0.015 * sr) as usize;
    let attack = (ATTACK_NOISE_S * sr) as usize;
    let mut burst = Biquad::new(FilterKind::HighPass, sr, 800.0);
    let gain = f64::from(note.velocity) * 0.5;
    for (i, slot) in out.iter_mut().enumerate().take(end).skip(start) {
        let t = (i - start) as f64 / sr;
        let mut v = 0.0;
        for p in &partials {
            v += p.amp * (-t / p.decay_s).exp() * (2.0 * PI * p.hz * t + p.phase).sin();
        }
        let mut sample = (v * gain) as f32;
        if i - start < attack {
            let env = 1.0 - (i - start) as f32 / attack as f32;
            sample += burst.process(noise.sample()) * env * note.velocity * 0.25;
        }
        let remaining = end - i;
        if remaining < fade {
            let x = remaining as f32 / fade as f32;
            sample *= 0.5 - 0.5 * (std::f32::consts::PI * x).cos();
        }
        *slot += sample;
    }
}

struct Partial {
    hz: f64,
    amp: f64,
    decay_s: f64,
    phase: f64,
}

fn partials(f0: f64, sr: f64, shape: ToneShape) -> Vec<Partial> {
    let (count, tilt) = match shape {
        ToneShape::Rich => (24, 1.1),
        ToneShape::Dull => (6, 1.8),
    };
    let mut list = Vec::new();
    for n in 1..=count {
        let n_f = f64::from(n);
        let hz = n_f * f0 * (1.0 + INHARMONICITY * n_f * n_f).sqrt();
        if hz > sr * 0.45 {
            break;
        }
        // Fingerstyle: the 2nd harmonic is often as strong as the fundamental.
        let boost = if n == 2 { 1.4 } else { 1.0 };
        list.push(Partial {
            hz,
            amp: boost / n_f.powf(tilt),
            decay_s: 2.5 / (1.0 + 0.35 * n_f),
            phase: 0.3 * n_f,
        });
    }
    list
}

/// Simulates a bass plugged into a laptop's combo jack: wrong impedance (treble loss),
/// low level, mains hum and noise.
pub fn laptop_jack(signal: &[f32], sample_rate: f64, seed: u64) -> Vec<f32> {
    let mut lowpass = Biquad::new(FilterKind::LowPass, sample_rate, 2500.0);
    let mut noise = Noise::new(seed);
    let gain = 0.18; // about -15 dB
    signal
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let t = i as f64 / sample_rate;
            let hum = (0.001 * (2.0 * PI * 50.0 * t).sin() + 0.0005 * (2.0 * PI * 150.0 * t).sin())
                as f32;
            lowpass.process(*s) * gain + hum + noise.sample() * 0.0005
        })
        .collect()
}

/// How a note is played, for the technique corpus (LH-BEW-06).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Articulation {
    /// Plucked with a finger (as `render`).
    Pluck,
    /// Thumb slap: a hard broadband thump and a brighter tone.
    Slap,
    /// Finger pop: a sharp, bright snap.
    Pop,
    /// Muted: a short thump without a clear pitch.
    Ghost,
    /// The pitch changes on the ringing string without a new attack (hammer-on, pull-off).
    Legato,
    /// The pitch glides from the note before over `glide_s`.
    Slide { glide_s: f64 },
}

/// One played note of an articulated line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Played {
    pub note: SynthNote,
    pub how: Articulation,
}

/// Slap and pop transients: (length s, high-pass Hz, level).
const SLAP_THUMP: (f64, f64, f32) = (0.006, 120.0, 0.9);
const POP_SNAP: (f64, f64, f32) = (0.003, 2500.0, 0.8);
/// A muted note sounds this long and decays this fast.
const GHOST_S: f64 = 0.04;
const GHOST_DECAY_S: f64 = 0.012;

/// Renders an articulated line: legato notes and slides continue the oscillator of the note
/// before (no new attack, phases run on), the others start fresh. One voice, like a bassist on
/// one string at a time.
pub fn render_articulated(line: &[Played], sample_rate: f64, total_s: f64, seed: u64) -> Vec<f32> {
    let len = (total_s * sample_rate).ceil() as usize;
    let mut out = vec![0.0f32; len];
    let mut noise = Noise::new(seed);
    let mut i = 0;
    while i < line.len() {
        // A phrase: one attacked note and the legato and slide notes that follow it.
        let mut end = i + 1;
        while end < line.len()
            && matches!(
                line[end].how,
                Articulation::Legato | Articulation::Slide { .. }
            )
        {
            end += 1;
        }
        phrase(&mut out, &line[i..end], sample_rate, &mut noise);
        i = end;
    }
    out
}

/// One attacked note and its legato continuation, as a single oscillator bank whose frequency
/// follows the notes (gliding for slides).
fn phrase(out: &mut [f32], notes: &[Played], sr: f64, noise: &mut Noise) {
    let Some(first) = notes.first() else {
        return;
    };
    let start = (first.note.onset_s * sr).round() as usize;
    let last = notes.last().map_or(first.note, |p| p.note);
    let end = (((last.onset_s + last.duration_s) * sr).round() as usize).min(out.len());
    if start >= end {
        return;
    }
    let (count, tilt) = match first.how {
        Articulation::Slap | Articulation::Pop => (24, 0.8),
        _ => (24, 1.1),
    };
    let ghost = first.how == Articulation::Ghost;
    let end = if ghost {
        (start + (GHOST_S * sr) as usize).min(out.len())
    } else {
        end
    };
    let mut phases = vec![0.0f64; count];
    let gain = f64::from(first.note.velocity) * 0.5;
    let fade = (0.015 * sr) as usize;
    let mut transient = match first.how {
        Articulation::Slap => Some((
            SLAP_THUMP,
            Biquad::new(FilterKind::HighPass, sr, SLAP_THUMP.1),
        )),
        Articulation::Pop => Some((POP_SNAP, Biquad::new(FilterKind::HighPass, sr, POP_SNAP.1))),
        _ => None,
    };
    let mut burst = Biquad::new(FilterKind::HighPass, sr, 800.0);
    let attack = (ATTACK_NOISE_S * sr) as usize;
    for (k, slot) in out.iter_mut().enumerate().take(end).skip(start) {
        let t_abs = k as f64 / sr;
        let t = (k - start) as f64 / sr;
        // The note sounding now, and the frequency (gliding during a slide).
        let idx = notes
            .iter()
            .rposition(|p| p.note.onset_s <= t_abs)
            .unwrap_or(0);
        let hz = match notes[idx].how {
            Articulation::Slide { glide_s } if idx > 0 => {
                let from = notes[idx - 1].note.hz;
                let x = ((t_abs - notes[idx].note.onset_s) / glide_s).clamp(0.0, 1.0);
                from * (notes[idx].note.hz / from).powf(x)
            }
            _ => notes[idx].note.hz,
        };
        let mut v = 0.0;
        for (n, phase) in phases.iter_mut().enumerate() {
            let n_f = (n + 1) as f64;
            let f = n_f * hz * (1.0 + INHARMONICITY * n_f * n_f).sqrt();
            if f > sr * 0.45 {
                break;
            }
            *phase += 2.0 * PI * f / sr;
            let boost = if n == 1 { 1.4 } else { 1.0 };
            let decay = if ghost {
                GHOST_DECAY_S
            } else {
                2.5 / (1.0 + 0.35 * n_f)
            };
            v += boost / n_f.powf(tilt) * (-t / decay).exp() * (*phase + 0.3 * n_f).sin();
        }
        let mut sample = (v * gain) as f32;
        if k - start < attack && !matches!(first.how, Articulation::Slap | Articulation::Pop) {
            let env = 1.0 - (k - start) as f32 / attack as f32;
            sample += burst.process(noise.sample()) * env * first.note.velocity * 0.25;
        }
        if let Some(((len_s, _, level), filter)) = transient.as_mut() {
            let len = (*len_s * sr) as usize;
            if k - start < len {
                let env = 1.0 - (k - start) as f32 / len as f32;
                sample += filter.process(noise.sample()) * env * *level * first.note.velocity;
            }
        }
        let remaining = end - k;
        if remaining < fade {
            let x = remaining as f32 / fade as f32;
            sample *= 0.5 - 0.5 * (std::f32::consts::PI * x).cos();
        }
        *slot += sample;
    }
}

/// Comb delays of the room's tail in ms (Schroeder; mutually prime-ish, as in Freeverb).
const ROOM_COMBS_MS: [f64; 4] = [29.7, 37.1, 41.1, 43.7];
/// All-pass delays that smear the tail's echoes, in ms.
const ROOM_ALLPASS_MS: [f64; 2] = [5.0, 1.7];
/// Early reflections: (delay ms, gain) from floor, near wall and ceiling.
const ROOM_EARLY: [(f64, f32); 3] = [(7.0, 0.5), (13.0, 0.35), (23.0, 0.25)];
/// Reverberation time of a furnished living room.
const ROOM_RT60_S: f64 = 0.45;
/// How much of the tail reaches the microphone.
const ROOM_WET: f32 = 0.35;
/// A small microphone loses the deepest lows.
const MIC_LOW_CUT_HZ: f64 = 50.0;
/// Fan and street, peak level.
const ROOM_NOISE_DB: f32 = -50.0;

/// A double bass in a living room through a microphone (QA corpus "Kontrabass Raum",
/// LH-EIN-11): early reflections, a Schroeder tail of `ROOM_RT60_S`, a low cut at
/// `MIC_LOW_CUT_HZ` and room noise. Deterministic for a given `seed`.
///
/// ```text
/// dry ─┬────────────────────────────────────────────┐
///      ├─ early reflections (7, 13, 23 ms) ──────────┤
///      └─ 4 combs ─► 2 all-passes ─► × wet ──────────┴─► low cut ─► + noise
/// ```
pub fn room(signal: &[f32], sample_rate: f64, seed: u64) -> Vec<f32> {
    let delay = |ms: f64| ((ms / 1000.0) * sample_rate).round() as usize;
    let mut combs: Vec<(Vec<f32>, usize, f32)> = ROOM_COMBS_MS
        .iter()
        .map(|&ms| {
            let d = delay(ms);
            // Feedback for a 60 dB decay over RT60.
            let g = 10f64.powf(-3.0 * ms / 1000.0 / ROOM_RT60_S) as f32;
            (vec![0.0; d], 0, g)
        })
        .collect();
    let mut allpasses: Vec<(Vec<f32>, usize)> = ROOM_ALLPASS_MS
        .iter()
        .map(|&ms| (vec![0.0; delay(ms)], 0))
        .collect();
    let mut low_cut = Biquad::new(FilterKind::HighPass, sample_rate, MIC_LOW_CUT_HZ);
    let mut noise = Noise::new(seed);
    let noise_amp = 10f32.powf(ROOM_NOISE_DB / 20.0);
    let early: Vec<(usize, f32)> = ROOM_EARLY.iter().map(|&(ms, g)| (delay(ms), g)).collect();

    (0..signal.len())
        .map(|i| {
            let x = signal[i];
            let reflections: f32 = early
                .iter()
                .filter_map(|&(d, g)| i.checked_sub(d).map(|j| signal[j] * g))
                .sum();
            let mut tail = 0.0;
            for (buf, pos, g) in combs.iter_mut() {
                let out = buf[*pos];
                buf[*pos] = x + out * *g;
                *pos = (*pos + 1) % buf.len();
                tail += out;
            }
            tail /= ROOM_COMBS_MS.len() as f32;
            for (buf, pos) in allpasses.iter_mut() {
                let delayed = buf[*pos];
                let input = tail;
                buf[*pos] = input + delayed * 0.5;
                tail = delayed - input * 0.5;
                *pos = (*pos + 1) % buf.len();
            }
            let wet = x + reflections + tail * ROOM_WET;
            low_cut.process(wet) + noise.sample() * noise_amp
        })
        .collect()
}

/// Adds white noise at `level_db` dBFS (peak).
pub fn add_noise(signal: &mut [f32], level_db: f32, seed: u64) {
    let amp = 10f32.powf(level_db / 20.0);
    let mut noise = Noise::new(seed);
    for s in signal.iter_mut() {
        *s += noise.sample() * amp;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::{peak_db, rms_db};

    #[test]
    fn deterministic() {
        let a = bass_tone(41.2, 48_000.0, 0.2, ToneShape::Rich, 1);
        let b = bass_tone(41.2, 48_000.0, 0.2, ToneShape::Rich, 1);
        assert_eq!(a, b);
        assert!(peak_db(&a) < 0.0);
    }

    #[test]
    fn laptop_is_quieter() {
        let a = bass_tone(55.0, 48_000.0, 0.5, ToneShape::Rich, 1);
        let b = laptop_jack(&a, 48_000.0, 2);
        assert!(rms_db(&b) < rms_db(&a) - 10.0);
    }

    #[test]
    fn next_note_cuts_previous() {
        let notes = [
            SynthNote {
                onset_s: 0.0,
                hz: 41.2,
                duration_s: 2.0,
                velocity: 0.8,
            },
            SynthNote {
                onset_s: 0.5,
                hz: 55.0,
                duration_s: 0.1,
                velocity: 0.8,
            },
        ];
        let out = render(&notes, 48_000.0, 1.0, ToneShape::Rich, 1);
        assert!(rms_db(&out[36_000..]) < -100.0);
    }
}
