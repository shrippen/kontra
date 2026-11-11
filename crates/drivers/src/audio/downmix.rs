//! Interleaved device frames to the instrument's mono signal.

/// Turns interleaved frames into the instrument's mono signal: the chosen channel, or else the
/// loudest one. An average would halve a bass on one input of a two-input interface and hide its
/// clipping (hardware check 13.10.: Scarlett Solo, peaks at −6 dBFS while clipping).
///
/// ```text
/// in 1 (empty)  ─────────            held peak per channel, decaying;
/// in 2 (bass)   ─╱╲╱╲╱╲──  ──► out   switch only when another is 6 dB louder
/// ```
pub(super) struct Downmix {
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
    pub(super) fn new(channels: usize, pick: Option<usize>) -> Downmix {
        Downmix {
            channels,
            pick,
            held: vec![0.0; channels],
            current: 0,
        }
    }

    /// `out` gets one sample per frame of `frames`.
    pub(super) fn mono(&mut self, frames: &[f32], out: &mut [f32]) {
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
