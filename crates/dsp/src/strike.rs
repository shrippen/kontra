//! How a pitched note was struck (LH-BEW-06): slap, pop, ghost note or a plain pluck, from the
//! attack and what is left of it 60 ms later.
//!
//! ```text
//!   |█▆▄▃▂▂▁▁▁▁▁▁▁▁    ghost:  dies at once                     decay < 0.3, little high band
//!   |██▇▅▃▃▂▂▂▂▂▂▂▂    slap:   bright click, falls fast         high band > 1 %, decay < 0.7
//!   |██▇▅▄▃▃▃▃▃▃▃▃▃    pop:    brighter still                   high band > 4 %
//!   |▅▆▇▇▇▇▇▆▆▆▆▅▅▅    pluck:  soft attack, rings on            the rest
//!    ^peak  ^60–80 ms
//! ```
//!
//! Chosen on the real recordings of IDMT-SMT-Bass-Single-Track (testdata/korpus/QUELLEN.md):
//! slap thumb 83 %, slap pop 70 %, dead notes 93 %, plain plucks 98 % (2 % told as a technique).
//! The first measure (brightness of the first 8 ms from the reported onset) told 0 % of real slaps:
//! synthetic slaps are much brighter than real ones, and a reported onset is not the attack's peak.

use crate::biquad::{Biquad, FilterKind};
use kontra_model::{InputSource, Technique};

/// The attack's peak is looked for from this long before the reported onset …
const PEAK_BEFORE_S: f64 = 0.01;
/// … to this long after it.
const PEAK_AFTER_S: f64 = 0.03;
/// The attack: high band share and level over this span from the peak.
const ATTACK_S: f64 = 0.01;
const ATTACK_LEVEL_S: f64 = 0.02;
/// What is left: the peak level from 60 to 80 ms after the attack's peak.
const LATER_FROM_S: f64 = 0.06;
const LATER_TO_S: f64 = 0.08;
/// The high band starts here; the filter settles on this much audio before the peak.
const HIGH_BAND_HZ: f64 = 1500.0;
const SETTLE_S: f64 = 0.005;
/// Thresholds (IDMT, see the module comment). Below `MUTED_DECAY` a note is a ghost note
/// whatever its attack (slaps keep ≥ 0.28); below `GHOST_DECAY` only with a soft attack.
const MUTED_DECAY: f32 = 0.2;
const GHOST_DECAY: f32 = 0.3;
const STRIKE_HIGH: f32 = 0.01;
const STRIKE_DECAY: f32 = 0.7;
const POP_HIGH: f32 = 0.04;

/// Audio needed after a reported onset before the verdict can be made.
pub(crate) fn verdict_after_s() -> f64 {
    PEAK_AFTER_S + LATER_TO_S
}

/// Measures of one attack.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Strike {
    /// Energy above `HIGH_BAND_HZ` over the energy, first 10 ms from the peak.
    pub(crate) high_share: f32,
    /// Peak 60–80 ms after the attack over the attack's peak.
    pub(crate) decay: f32,
}

impl Strike {
    /// Measures the attack reported at sample `onset`; `sample(i)` reads absolute sample `i`.
    /// `None` without audio from `PEAK_BEFORE_S` before to `verdict_after_s` after the onset.
    pub(crate) fn measure(
        onset: u64,
        sample_rate: f64,
        available: std::ops::Range<u64>,
        sample: impl Fn(u64) -> f32,
    ) -> Option<Strike> {
        let at = |s: f64| (s * sample_rate) as u64;
        let from = onset.checked_sub(at(PEAK_BEFORE_S + SETTLE_S))?;
        if from < available.start || onset + at(verdict_after_s()) > available.end {
            return None;
        }

        // The attack's peak near the reported onset.
        let search = onset - at(PEAK_BEFORE_S)..onset + at(PEAK_AFTER_S);
        let peak = search
            .clone()
            .max_by(|a, b| sample(*a).abs().total_cmp(&sample(*b).abs()))
            .unwrap_or(onset);

        // High band share of the first 10 ms, the filter settled before.
        let mut high_pass = Biquad::new(FilterKind::HighPass, sample_rate, HIGH_BAND_HZ);
        let (mut all, mut high) = (0.0f64, 0.0f64);
        for i in peak - at(SETTLE_S)..peak + at(ATTACK_S) {
            let x = sample(i);
            let y = high_pass.process(x);
            if i >= peak {
                all += f64::from(x) * f64::from(x);
                high += f64::from(y) * f64::from(y);
            }
        }

        let level =
            |range: std::ops::Range<u64>| range.map(|i| sample(i).abs()).fold(0.0f32, f32::max);
        let attack = level(peak..peak + at(ATTACK_LEVEL_S));
        let later = level(peak + at(LATER_FROM_S)..peak + at(LATER_TO_S));
        Some(Strike {
            high_share: (high / all.max(f64::MIN_POSITIVE)) as f32,
            decay: later / attack.max(f32::MIN_POSITIVE),
        })
    }

    /// The technique this attack was played with; at a microphone room noise brightens soft
    /// plucks, so slaps and pops are told only on a cable.
    pub(crate) fn technique(self, source: InputSource) -> Option<Technique> {
        let muted = self.decay < MUTED_DECAY;
        if muted || (self.decay < GHOST_DECAY && self.high_share < STRIKE_HIGH) {
            return Some(Technique::Muted);
        }
        if source == InputSource::Microphone {
            return None;
        }
        if self.high_share > STRIKE_HIGH && self.decay < STRIKE_DECAY {
            return Some(if self.high_share > POP_HIGH {
                Technique::Pop
            } else {
                Technique::Slap
            });
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f64 = 44_100.0;

    /// A hard attack: a few samples, louder than the tone, so it is the attack's peak.
    const CLICK_SAMPLES: usize = 8;

    /// A tone at `hz` from sample `start`, decaying by `per_s` (amplitude factor per second),
    /// with a click of `click` at its start.
    fn note(hz: f64, per_s: f64, click: f32) -> Vec<f32> {
        let start = 4410;
        (0..start + 22050)
            .map(|i| {
                if i < start {
                    return 0.0;
                }
                let t = (i - start) as f64 / SR;
                let tone = (2.0 * std::f64::consts::PI * hz * t).sin() * per_s.powf(t) * 0.5;
                let noise = if i - start < CLICK_SAMPLES {
                    click * if i % 2 == 0 { 1.0 } else { -1.0 }
                } else {
                    0.0
                };
                tone as f32 + noise
            })
            .collect()
    }

    fn strike(samples: &[f32]) -> Strike {
        Strike::measure(4410, SR, 0..samples.len() as u64, |i| samples[i as usize]).unwrap()
    }

    #[test]
    fn tells_strikes_apart() {
        let pluck = strike(&note(55.0, 0.5, 0.0));
        assert_eq!(pluck.technique(InputSource::Line), None, "{pluck:?}");
        let ghost = strike(&note(55.0, 1e-12, 0.0));
        assert_eq!(
            ghost.technique(InputSource::Line),
            Some(Technique::Muted),
            "{ghost:?}"
        );
        let slap = strike(&note(55.0, 0.0001, 0.6));
        assert_eq!(
            slap.technique(InputSource::Line),
            Some(Technique::Slap),
            "{slap:?}"
        );
        assert_eq!(
            slap.technique(InputSource::Microphone),
            None,
            "no slaps at a microphone"
        );
    }

    #[test]
    fn waits_for_enough_audio() {
        let samples = note(55.0, 0.5, 0.0);
        let short = 0..(4410 + 3000) as u64;
        assert!(Strike::measure(4410, SR, short, |i| samples[i as usize]).is_none());
    }
}
