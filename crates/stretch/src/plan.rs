//! Hop planner: where the next analysis frame starts.
//!
//! The synthesis hop is fixed; the analysis hop follows the speed. Around a
//! transient the hop is 1:1 (a "run"), so every frame holding the transient
//! copies it to the same output spot. The time a run loses is won back by
//! the following frames ("drift"), and before a run the planner lags behind
//! by half the run so the transient lands on its nominal output time.
//!
//! ```text
//! drift   0 ─╮           ╭── run (1:1) ──╮          ╭─ 0
//!            ╰─ -½ run ──╯               ╰─ +½ run ─╯
//! ```

use crate::sizes::Sizes;

/// Speed glide per frame (≈ 0.1 s time constant at 48 kHz).
const GLIDE: f64 = 0.2;

/// Analysis hop limits relative to the nominal hop while steering drift.
const HOP_MIN_RATIO: f64 = 0.125;
const HOP_MAX_RATIO: f64 = 2.0;

/// Whether the current frame sits inside a transient run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Touch {
    Free,
    Run,
}

/// Whether an untouched onset is close enough to steer for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Lead {
    Clear,
    /// Onset at this absolute input sample.
    Onset(i64),
}

#[derive(Debug)]
pub(crate) struct Planner {
    sizes: Sizes,
    target: f64,
    speed: f64,
    /// Nominal input position of the current frame (speed integrated).
    nominal: f64,
    /// Actual input position of the current frame.
    pos: i64,
    /// Hop from the previous frame to the current one (0 before the first).
    hop_a: usize,
    fresh: bool,
}

impl Planner {
    pub(crate) fn new(sizes: Sizes) -> Self {
        Self {
            sizes,
            target: 1.0,
            speed: 1.0,
            nominal: 0.0,
            pos: 0,
            hop_a: 0,
            fresh: true,
        }
    }

    pub(crate) fn reset(&mut self, start: i64) {
        self.pos = start;
        self.nominal = start as f64;
        self.hop_a = 0;
        self.speed = self.target;
        self.fresh = true;
    }

    pub(crate) fn set_speed(&mut self, speed: f64) {
        self.target = speed;
        if self.fresh {
            self.speed = speed;
        }
    }

    pub(crate) fn speed(&self) -> f64 {
        self.speed
    }

    pub(crate) fn pos(&self) -> i64 {
        self.pos
    }

    pub(crate) fn hop_a(&self) -> usize {
        self.hop_a
    }

    /// Input samples the actual position is ahead of the nominal one.
    pub(crate) fn drift(&self) -> f64 {
        self.pos as f64 - self.nominal
    }

    /// Steps to the next frame.
    pub(crate) fn advance(&mut self, touch: Touch, lead: Lead) {
        self.fresh = false;
        self.speed += (self.target - self.speed) * GLIDE;

        let hop_s = self.sizes.hop as f64;
        let base = hop_s * self.speed;
        self.nominal += base;

        let hop = match touch {
            Touch::Run => self.sizes.hop,
            Touch::Free => self.steer(base, lead),
        };
        self.hop_a = hop.max(1);
        self.pos += self.hop_a as i64;
    }

    /// Hop that moves the drift toward its goal.
    ///
    /// Without onset ahead the goal is zero. With onset `c` ahead, the drift
    /// at the first frame `p` of the run must equal `(p + N/2 - c)(1 - s)`:
    /// the run then adds exactly enough drift that `c` comes out at its
    /// nominal time. Solving `p - nominal = (p + N/2 - c)(1 - s)` for `p`
    /// gives the target position.
    fn steer(&self, base: f64, lead: Lead) -> usize {
        let half = self.sizes.win as f64 / 2.0;
        let target = match lead {
            Lead::Clear => self.nominal,
            Lead::Onset(c) => {
                let s = self.speed;
                (self.nominal + (half - c as f64) * (1.0 - s)) / s
            }
        };

        // On the target line the hop is the run hop (1:1), so allow it.
        let run = match lead {
            Lead::Clear => base,
            Lead::Onset(_) => self.sizes.hop as f64,
        };
        let want = target - self.pos as f64;
        let lo = (base * HOP_MIN_RATIO).max(1.0);
        let hi = (base * HOP_MAX_RATIO).min(half).max(base).max(run);
        want.clamp(lo, hi).round() as usize
    }
}
