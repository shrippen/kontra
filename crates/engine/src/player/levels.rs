//! The volumes the mixer plays at, moving toward the gains it was given (Runde 20: modes hand
//! the output over while the bass sounds). A jump in gain is a jump in the signal, a click; a
//! ramp of `RAMP_S` is too short to hear as a fade.
//!
//! ```text
//!  gain  1 ─────┐                1 ─────╲
//!               │   (click)              ╲  RAMP_S
//!        0      └─────           0        ╲─────
//! ```

use super::Gains;

/// How long a change from silence to full volume takes.
const RAMP_S: f32 = 0.008;

/// The volume of each part of the mix, as heard right now.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Levels {
    pub(super) click: f32,
    pub(super) tone: f32,
    pub(super) drums: f32,
    pub(super) backing: f32,
    pub(super) bass: f32,
    pub(super) monitor: f32,
}

impl Levels {
    /// The levels `gains` ask for.
    pub(super) fn of(gains: &Gains) -> Levels {
        Levels {
            click: gains.click,
            tone: gains.tone,
            drums: gains.drums,
            backing: gains.backing,
            bass: gains.bass,
            monitor: gains.monitor,
        }
    }

    /// One frame closer to `target`: at most a full scale per `RAMP_S`.
    pub(super) fn follow(&mut self, target: &Levels, sr: f32) {
        let step = 1.0 / (RAMP_S * sr);
        let toward = |now: &mut f32, want: f32| {
            *now += (want - *now).clamp(-step, step);
        };
        toward(&mut self.click, target.click);
        toward(&mut self.tone, target.tone);
        toward(&mut self.drums, target.drums);
        toward(&mut self.backing, target.backing);
        toward(&mut self.bass, target.bass);
        toward(&mut self.monitor, target.monitor);
    }
}
