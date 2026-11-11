//! Effects for the chain on the monitored bass (LH-EIN-19): chorus, envelope filter, octaver,
//! delay, reverb and fuzz. Each takes one mono sample and its values (`a`, `b` in 0..=1, see
//! `kontra_model::EffectKind`); the chain blends in the dry signal.
//!
//! ```text
//! Chorus      x ─► delay 7 ms ± LFO ─────────────────────────────► wet
//! EnvFilter   x ─► |x| follower ─► cutoff 80 Hz … 2.5 kHz ─┐
//!             x ───────────────────────► low-pass (SVF) ◄───┘ ─► wet
//! Octaver     x ─► low-pass ─► flip-flop (÷2 at each rise) ─► × envelope ─► low-pass ─► x + sub
//! Delay       x ─► line ─┬─► tap (time) ─► wet
//!                  ▲     └─► damp ─► × feedback ─┘
//! Reverb      x ─► 8 parallel combs (damped) ─► 4 allpasses in series ─► wet   (Freeverb)
//! Fuzz        x ─► × gain ─► asymmetric tanh ─► DC block ─► low-pass ─► wet
//! ```
//!
//! Runs in the audio callback: all memory is made in `new` (for the highest device rate),
//! then one sample at a time without allocation.

use crate::biquad::{Biquad, FilterKind};
use std::f32::consts::TAU;

/// Highest device rate the delay lines are sized for.
const MAX_RATE_HZ: f64 = 192_000.0;
/// Level below which a signal counts as silence (nothing to follow, no sub-octave).
const SILENCE: f32 = 1e-4;

/// Which side of a stereo pair an instance serves; spreads the modulation and the tunings,
/// so the two sides of a chorus or reverb are not identical.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Side {
    Mono,
    Left,
    Right,
}

/// Seconds to a one-pole coefficient: the part of the old value kept per sample.
fn keep(time_s: f64, sample_rate: f64) -> f32 {
    (-1.0 / (time_s * sample_rate)).exp() as f32
}

/// One-pole low-pass coefficient (weight of the new input) for `hz`.
fn pole(hz: f32, sample_rate: f64) -> f32 {
    1.0 - (-TAU * hz / sample_rate as f32).exp()
}

/// A circular buffer read at fractional delays.
#[derive(Debug, Clone)]
struct Line {
    buf: Vec<f32>,
    at: usize,
}

impl Line {
    fn new(len: usize) -> Line {
        Line {
            buf: vec![0.0; len.max(4)],
            at: 0,
        }
    }

    fn push(&mut self, x: f32) {
        self.buf[self.at] = x;
        self.at = (self.at + 1) % self.buf.len();
    }

    /// The sample pushed `delay` samples ago (1 is the latest), linearly interpolated.
    fn tap(&self, delay: f32) -> f32 {
        let len = self.buf.len();
        let delay = delay.clamp(1.0, (len - 2) as f32);
        let whole = delay as usize;
        let frac = delay - whole as f32;
        let newer = self.buf[(self.at + len - whole) % len];
        let older = self.buf[(self.at + len - whole - 1) % len];
        newer + (older - newer) * frac
    }
}

// ---------------------------------------------------------------------------------- chorus

const CHORUS_BASE_S: f32 = 0.007;
/// Deepest swing of the delay around its base, in seconds.
const CHORUS_SWING_S: f32 = 0.004;
const CHORUS_RATE_MIN_HZ: f32 = 0.1;
const CHORUS_RATE_MAX_HZ: f32 = 4.0;

/// Chorus: a short delay swept by a slow sine, so the copy drifts in pitch and beats against
/// the dry signal. `a` rate (0.1 to 4 Hz), `b` depth (swing up to ±4 ms).
#[derive(Debug, Clone)]
pub(crate) struct Chorus {
    line: Line,
    phase: f32,
    rate: f64,
}

impl Chorus {
    pub(crate) fn new(sample_rate: f64, side: Side) -> Chorus {
        let size = (MAX_RATE_HZ * f64::from(CHORUS_BASE_S + CHORUS_SWING_S)) as usize + 4;
        Chorus {
            line: Line::new(size),
            phase: if side == Side::Right { 0.5 } else { 0.0 },
            rate: sample_rate,
        }
    }

    pub(crate) fn retime(&mut self, sample_rate: f64) {
        self.rate = sample_rate;
    }

    pub(crate) fn process(&mut self, x: f32, a: f32, b: f32) -> f32 {
        let hz = CHORUS_RATE_MIN_HZ + (CHORUS_RATE_MAX_HZ - CHORUS_RATE_MIN_HZ) * a;
        self.phase = (self.phase + hz / self.rate as f32).fract();
        let seconds = CHORUS_BASE_S + CHORUS_SWING_S * b * (TAU * self.phase).sin();
        self.line.push(x);
        self.line.tap(seconds * self.rate as f32)
    }
}

// ----------------------------------------------------------------------- envelope filter

const FOLLOW_ATTACK_S: f64 = 0.003;
const FOLLOW_RELEASE_S: f64 = 0.100;
const WAH_LOW_HZ: f32 = 80.0;
const WAH_HIGH_HZ: f32 = 2500.0;
/// How many times the envelope is amplified at sensitivity 0 and 1 (a pluck peaks near 0.3).
const WAH_GAIN_MIN: f32 = 2.0;
const WAH_GAIN_MAX: f32 = 20.0;
/// Quality of the filter at resonance 0 and 1.
const WAH_Q_MIN: f32 = 0.8;
const WAH_Q_MAX: f32 = 8.0;

/// Envelope filter (auto-wah): a resonant low-pass whose cutoff follows the level of the
/// playing, so each pick attack sweeps it open and it closes as the note dies away.
/// `a` sensitivity, `b` resonance. Filter: state variable, trapezoidal.
#[derive(Debug, Clone)]
pub(crate) struct EnvelopeFilter {
    envelope: f32,
    attack: f32,
    release: f32,
    s1: f32,
    s2: f32,
    rate: f32,
}

impl EnvelopeFilter {
    pub(crate) fn new(sample_rate: f64) -> EnvelopeFilter {
        let mut filter = EnvelopeFilter {
            envelope: 0.0,
            attack: 0.0,
            release: 0.0,
            s1: 0.0,
            s2: 0.0,
            rate: 0.0,
        };
        filter.retime(sample_rate);
        filter
    }

    pub(crate) fn retime(&mut self, sample_rate: f64) {
        self.rate = sample_rate as f32;
        self.attack = keep(FOLLOW_ATTACK_S, sample_rate);
        self.release = keep(FOLLOW_RELEASE_S, sample_rate);
    }

    pub(crate) fn process(&mut self, x: f32, a: f32, b: f32) -> f32 {
        let level = x.abs();
        let coeff = if level > self.envelope {
            self.attack
        } else {
            self.release
        };
        self.envelope = level + coeff * (self.envelope - level);

        let gain = WAH_GAIN_MIN + (WAH_GAIN_MAX - WAH_GAIN_MIN) * a;
        let open = (self.envelope * gain).clamp(0.0, 1.0);
        let hz = WAH_LOW_HZ * (WAH_HIGH_HZ / WAH_LOW_HZ).powf(open);
        let q = WAH_Q_MIN + (WAH_Q_MAX - WAH_Q_MIN) * b;

        let g = (std::f32::consts::PI * hz / self.rate).tan();
        let k = 1.0 / q;
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;
        let v3 = x - self.s2;
        let v1 = a1 * self.s1 + a2 * v3;
        let v2 = self.s2 + a2 * self.s1 + a3 * v3;
        self.s1 = 2.0 * v1 - self.s1;
        self.s2 = 2.0 * v2 - self.s2;
        v2
    }
}

// --------------------------------------------------------------------------------- octaver

/// Isolates the fundamental before the flip-flop (the top of a bass is around 400 Hz).
const OCTAVE_TRACK_HZ: f64 = 350.0;
const OCTAVE_FOLLOW_ATTACK_S: f64 = 0.002;
const OCTAVE_FOLLOW_RELEASE_S: f64 = 0.060;
/// The flip-flop turns over only when the fundamental rises this far (relative to its level)
/// above zero: noise around zero does not toggle it.
const OCTAVE_HYSTERESIS: f32 = 0.1;
/// Cutoff that rounds the square wave at tone 0 and 1.
const OCTAVE_TONE_MIN_HZ: f32 = 60.0;
const OCTAVE_TONE_MAX_HZ: f32 = 400.0;
/// A square of amplitude A has a fundamental of 1.27 A; the sub is made equal to the input's.
const OCTAVE_SQUARE_GAIN: f32 = 0.8;

/// Octaver: one octave down for a single note at a time. A flip-flop toggles at every rising
/// zero crossing of the fundamental, which makes a square wave at half the pitch; the
/// envelope of the playing scales it. Added to the input. `a` level of the lower octave, `b`
/// tone. Example: A2 (110 Hz) gets an A1 (55 Hz) beneath it; a chord confuses it.
#[derive(Debug, Clone)]
pub(crate) struct Octaver {
    track: Biquad,
    above: bool,
    flip: f32,
    envelope: f32,
    attack: f32,
    release: f32,
    sub: f32,
    rate: f32,
}

impl Octaver {
    pub(crate) fn new(sample_rate: f64) -> Octaver {
        let mut octaver = Octaver {
            track: Biquad::new(FilterKind::LowPass, sample_rate, OCTAVE_TRACK_HZ),
            above: false,
            flip: 1.0,
            envelope: 0.0,
            attack: 0.0,
            release: 0.0,
            sub: 0.0,
            rate: 0.0,
        };
        octaver.retime(sample_rate);
        octaver
    }

    pub(crate) fn retime(&mut self, sample_rate: f64) {
        self.rate = sample_rate as f32;
        self.track = Biquad::new(FilterKind::LowPass, sample_rate, OCTAVE_TRACK_HZ);
        self.attack = keep(OCTAVE_FOLLOW_ATTACK_S, sample_rate);
        self.release = keep(OCTAVE_FOLLOW_RELEASE_S, sample_rate);
    }

    pub(crate) fn process(&mut self, x: f32, a: f32, b: f32) -> f32 {
        let fundamental = self.track.process(x);
        let level = fundamental.abs();
        let coeff = if level > self.envelope {
            self.attack
        } else {
            self.release
        };
        self.envelope = level + coeff * (self.envelope - level);

        let threshold = OCTAVE_HYSTERESIS * self.envelope;
        if fundamental > threshold && !self.above {
            self.above = true;
            self.flip = -self.flip;
        } else if fundamental < -threshold {
            self.above = false;
        }

        let square = if self.envelope < SILENCE {
            0.0
        } else {
            self.flip * self.envelope * OCTAVE_SQUARE_GAIN
        };
        let hz = OCTAVE_TONE_MIN_HZ + (OCTAVE_TONE_MAX_HZ - OCTAVE_TONE_MIN_HZ) * b;
        self.sub += pole(hz, f64::from(self.rate)) * (square - self.sub);
        x + a * self.sub
    }
}

// ----------------------------------------------------------------------------------- delay

const DELAY_MIN_S: f32 = 0.030;
const DELAY_MAX_S: f32 = 0.800;
/// Feedback at `b` = 1: repeats die away instead of building up.
const DELAY_FEEDBACK_MAX: f32 = 0.85;
/// Each repeat gets darker, like tape or an analog delay.
const DELAY_DAMP_HZ: f32 = 3000.0;

/// Delay: repeats of the input. `a` time (30 to 800 ms), `b` feedback (0 to 85 %). The
/// buffer is made in `new` for the longest time at the highest rate. Wet is the repeats
/// alone: the chain's mix sets how loud they are against the dry bass.
#[derive(Debug, Clone)]
pub(crate) struct Delay {
    line: Line,
    damped: f32,
    rate: f64,
}

impl Delay {
    pub(crate) fn new(sample_rate: f64) -> Delay {
        let size = (MAX_RATE_HZ * f64::from(DELAY_MAX_S)) as usize + 4;
        Delay {
            line: Line::new(size),
            damped: 0.0,
            rate: sample_rate,
        }
    }

    pub(crate) fn retime(&mut self, sample_rate: f64) {
        self.rate = sample_rate;
    }

    pub(crate) fn process(&mut self, x: f32, a: f32, b: f32) -> f32 {
        let seconds = DELAY_MIN_S + (DELAY_MAX_S - DELAY_MIN_S) * a;
        let repeat = self.line.tap(seconds * self.rate as f32);
        self.damped += pole(DELAY_DAMP_HZ, self.rate) * (repeat - self.damped);
        self.line.push(x + DELAY_FEEDBACK_MAX * b * self.damped);
        repeat
    }
}

// ---------------------------------------------------------------------------------- reverb

/// Freeverb's tunings in samples at 44.1 kHz (Jezar, public domain): eight combs, four
/// allpasses. Scaled to the device's rate.
const COMB_TUNING: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
const ALLPASS_TUNING: [usize; 4] = [556, 441, 341, 225];
const TUNING_RATE_HZ: f64 = 44_100.0;
/// The right side's lines are this much longer (at 44.1 kHz), which decorrelates the sides.
const STEREO_SPREAD: usize = 23;
/// Comb feedback at size 0 and 1: a room of about 0.3 s to a hall of several seconds.
const ROOM_MIN: f32 = 0.70;
const ROOM_MAX: f32 = 0.97;
/// Damping at `b` = 1: how much of the highs the combs absorb.
const DAMP_MAX: f32 = 0.8;
/// Brings the eight combs back to about the input's level.
const REVERB_INPUT: f32 = 0.015;
const REVERB_OUTPUT: f32 = 3.0;
const ALLPASS_FEEDBACK: f32 = 0.5;

/// Keeps decaying tails out of the denormal range, where floats are slow.
fn flush(v: f32) -> f32 {
    if v.abs() < 1e-20 {
        0.0
    } else {
        v
    }
}

#[derive(Debug, Clone)]
struct Comb {
    buf: Vec<f32>,
    at: usize,
    len: usize,
    damped: f32,
}

#[derive(Debug, Clone)]
struct Allpass {
    buf: Vec<f32>,
    at: usize,
    len: usize,
}

/// Small-room reverb after Freeverb: eight damped feedback combs in parallel, four allpasses
/// in series. `a` size (decay), `b` damping. Lines are made for the highest rate in `new`.
#[derive(Debug, Clone)]
pub(crate) struct Reverb {
    combs: [Comb; 8],
    allpasses: [Allpass; 4],
    spread: usize,
}

impl Reverb {
    pub(crate) fn new(sample_rate: f64, side: Side) -> Reverb {
        let spread = if side == Side::Right {
            STEREO_SPREAD
        } else {
            0
        };
        let room = |tuning: usize| {
            ((tuning + spread) as f64 * MAX_RATE_HZ / TUNING_RATE_HZ).ceil() as usize + 1
        };
        let mut reverb = Reverb {
            combs: COMB_TUNING.map(|t| Comb {
                buf: vec![0.0; room(t)],
                at: 0,
                len: 1,
                damped: 0.0,
            }),
            allpasses: ALLPASS_TUNING.map(|t| Allpass {
                buf: vec![0.0; room(t)],
                at: 0,
                len: 1,
            }),
            spread,
        };
        reverb.retime(sample_rate);
        reverb
    }

    /// New device rate: the lines get their lengths for it; their content stays.
    pub(crate) fn retime(&mut self, sample_rate: f64) {
        let len = |tuning: usize, room: usize| {
            let scaled = ((tuning + self.spread) as f64 * sample_rate / TUNING_RATE_HZ) as usize;
            scaled.clamp(1, room)
        };
        for (comb, t) in self.combs.iter_mut().zip(COMB_TUNING) {
            comb.len = len(t, comb.buf.len());
            comb.at = 0;
        }
        for (pass, t) in self.allpasses.iter_mut().zip(ALLPASS_TUNING) {
            pass.len = len(t, pass.buf.len());
            pass.at = 0;
        }
    }

    pub(crate) fn process(&mut self, x: f32, a: f32, b: f32) -> f32 {
        let feedback = ROOM_MIN + (ROOM_MAX - ROOM_MIN) * a;
        let damp = DAMP_MAX * b;
        let input = x * REVERB_INPUT;

        let mut sum = 0.0;
        for comb in &mut self.combs {
            let out = comb.buf[comb.at];
            comb.damped = flush(out * (1.0 - damp) + comb.damped * damp);
            comb.buf[comb.at] = input + comb.damped * feedback;
            comb.at = (comb.at + 1) % comb.len;
            sum += out;
        }
        for pass in &mut self.allpasses {
            let buffered = pass.buf[pass.at];
            let out = buffered - sum;
            pass.buf[pass.at] = flush(sum + buffered * ALLPASS_FEEDBACK);
            pass.at = (pass.at + 1) % pass.len;
            sum = out;
        }
        sum * REVERB_OUTPUT
    }
}

// ------------------------------------------------------------------------------------ fuzz

/// Gain at fuzz 0 and 1 (+0 and +40 dB) ahead of the clipper.
const FUZZ_GAIN_MAX: f32 = 100.0;
/// Offset ahead of the clipper: one half clips earlier than the other (even harmonics, the
/// sputter of a worn fuzz).
const FUZZ_BIAS: f32 = 0.3;
/// Level after the clipper: the clipped wave stays below the limiter's ceiling.
const FUZZ_LEVEL: f32 = 0.8;
const FUZZ_TONE_MIN_HZ: f32 = 600.0;
const FUZZ_TONE_MAX_HZ: f32 = 6000.0;
/// Takes the offset the asymmetric clipping leaves.
const FUZZ_DC_HZ: f32 = 20.0;

/// Fuzz: hard, asymmetric clipping of a strongly amplified signal. `a` fuzz (gain up to
/// 40 dB), `b` tone (600 Hz to 6 kHz low-pass).
#[derive(Debug, Clone)]
pub(crate) struct Fuzz {
    tone: f32,
    dc: f32,
    rate: f64,
}

impl Fuzz {
    pub(crate) fn new(sample_rate: f64) -> Fuzz {
        Fuzz {
            tone: 0.0,
            dc: 0.0,
            rate: sample_rate,
        }
    }

    pub(crate) fn retime(&mut self, sample_rate: f64) {
        self.rate = sample_rate;
    }

    pub(crate) fn process(&mut self, x: f32, a: f32, b: f32) -> f32 {
        let gain = 1.0 + (FUZZ_GAIN_MAX - 1.0) * a;
        let clipped = (gain * x + FUZZ_BIAS).tanh() - FUZZ_BIAS.tanh();

        self.dc += pole(FUZZ_DC_HZ, self.rate) * (clipped - self.dc);
        let centred = clipped - self.dc;
        let hz = FUZZ_TONE_MIN_HZ + (FUZZ_TONE_MAX_HZ - FUZZ_TONE_MIN_HZ) * b;
        self.tone += pole(hz, self.rate) * (centred - self.tone);
        self.tone * FUZZ_LEVEL
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::rms_db;
    use std::f64::consts::PI;

    const SR: f64 = 48_000.0;

    fn sine(hz: f64, amplitude: f32, n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| amplitude * (2.0 * PI * hz * i as f64 / SR).sin() as f32)
            .collect()
    }

    /// A pluck: a sine that dies away over a second.
    fn pluck(hz: f64, n: usize) -> Vec<f32> {
        sine(hz, 0.5, n)
            .iter()
            .enumerate()
            .map(|(i, v)| v * (-(i as f32) / SR as f32 * 4.0).exp())
            .collect()
    }

    #[test]
    fn chorus_swings_the_delay() {
        // Depth 0 is a fixed delay; with depth the copy drifts away from it.
        let input = sine(220.0, 0.4, 48_000);
        let run = |depth: f32| {
            let mut chorus = Chorus::new(SR, Side::Mono);
            input
                .iter()
                .map(|&x| chorus.process(x, 0.5, depth))
                .collect::<Vec<f32>>()
        };
        let (fixed, swung) = (run(0.0), run(1.0));
        let apart = fixed[4800..]
            .iter()
            .zip(&swung[4800..])
            .map(|(f, s)| (f - s).abs())
            .fold(0.0, f32::max);
        assert!(apart > 0.1, "{apart}");
        assert!(swung.iter().all(|v| v.is_finite() && v.abs() <= 0.41));
    }

    #[test]
    fn delay_repeats_after_its_time() {
        let mut delay = Delay::new(SR);
        // a = 0: 30 ms = 1440 samples. An impulse comes back there, and again, quieter.
        let mut out = vec![delay.process(1.0, 0.0, 0.5)];
        out.extend((1..4000).map(|_| delay.process(0.0, 0.0, 0.5)));
        assert!((out[1440] - 1.0).abs() < 1e-4, "{}", out[1440]);
        let second = out[2880];
        assert!(second > 0.1 && second < 0.5, "{second}");
    }

    #[test]
    fn reverb_rings_on_and_decays() {
        let mut reverb = Reverb::new(SR, Side::Mono);
        let mut out = vec![reverb.process(1.0, 0.5, 0.5)];
        out.extend((1..96_000).map(|_| reverb.process(0.0, 0.5, 0.5)));
        let early = rms_db(&out[2000..6000]);
        let late = rms_db(&out[80_000..]);
        assert!(early > -60.0, "{early}");
        assert!(late < early - 10.0, "{early} → {late}");
        assert!(out.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn reverb_keeps_a_sensible_level() {
        let mut reverb = Reverb::new(SR, Side::Left);
        let input = sine(110.0, 0.3, 96_000);
        let out: Vec<f32> = input.iter().map(|&x| reverb.process(x, 0.5, 0.5)).collect();
        let change = rms_db(&out[48_000..]) - rms_db(&input[48_000..]);
        assert!(change.abs() < 9.0, "{change} dB");
    }

    #[test]
    fn envelope_filter_opens_on_the_attack() {
        let mut filter = EnvelopeFilter::new(SR);
        // A bright tone (2 kHz): loud at the attack (open filter), dark when it has died away.
        let input = pluck(2000.0, 48_000);
        let out: Vec<f32> = input.iter().map(|&x| filter.process(x, 0.5, 0.4)).collect();
        let gain_db = |from: usize, to: usize| rms_db(&out[from..to]) - rms_db(&input[from..to]);
        assert!(gain_db(480, 2400) > gain_db(36_000, 48_000) + 6.0);
    }

    #[test]
    fn octaver_drops_an_octave() {
        let mut octaver = Octaver::new(SR);
        let input = sine(110.0, 0.4, 48_000);
        // a = 1: input plus sub. Count rising zero crossings of the sub alone.
        let sub: Vec<f32> = input
            .iter()
            .map(|&x| octaver.process(x, 1.0, 0.8) - x)
            .collect();
        let crossings = |v: &[f32]| v.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count();
        let (input_n, sub_n) = (crossings(&input[24_000..]), crossings(&sub[24_000..]));
        assert!(
            sub_n.abs_diff(input_n / 2) <= 2,
            "{sub_n} crossings against {input_n}"
        );
        assert!(rms_db(&sub[24_000..]) > -30.0);
    }

    #[test]
    fn octaver_is_silent_on_silence() {
        let mut octaver = Octaver::new(SR);
        for _ in 0..4800 {
            assert_eq!(octaver.process(0.0, 1.0, 0.5), 0.0);
        }
    }

    #[test]
    fn fuzz_squares_the_wave_and_stays_centred() {
        let mut fuzz = Fuzz::new(SR);
        let input = sine(110.0, 0.3, 48_000);
        let out: Vec<f32> = input.iter().map(|&x| fuzz.process(x, 1.0, 1.0)).collect();
        let tail = &out[24_000..];
        let peak = tail.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        let mean = tail.iter().sum::<f32>() / tail.len() as f32;
        assert!(peak > 0.1 && peak < 1.0, "peak {peak}");
        assert!(mean.abs() < 0.05 * peak, "offset {mean}");
        // Much louder than a sine of the same peak would be: it is a square.
        assert!(
            rms_db(tail) > -10.0 + 20.0 * peak.log10(),
            "{}",
            rms_db(tail)
        );
    }
}
