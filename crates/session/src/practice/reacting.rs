//! The drums answer the bass (LH-WIE-05): each drum hit asks the band whether its bar plays as
//! written, ends with a fill or leaves out the hi-hat (`crate::band`).

use super::Practice;
use crate::band::{Band, BarTime, DECIDE_LEAD_S};
use crate::session::Mode;
use kontra_engine::{Output, Sound};
use kontra_model::{DrumHit, DrumKind, Judging, TICKS_PER_QUARTER};

/// Keys of fill strokes in `Practice::scheduled`, apart from the written drums.
const FILL_KEYS: u64 = 4 << 40;
/// Strokes of a fill: sixteenths through the last beat.
const FILL_STROKES: u32 = 4;

/// What happens to one written drum hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Reaction {
    Play,
    /// Left out: a fill or a thinner bar takes its place.
    Drop,
    /// Not decided yet: asked again on the next tick.
    Wait,
}

impl<O: Output> Practice<O> {
    /// Whether the band listens: switched on, generated drums, free play (a run-through or an
    /// open exercise). Never in Wait mode or a probe; nothing here is scored.
    fn band_listens(&self) -> bool {
        let arrangement = self.session.arrangement();
        self.options.band == Band::Reacting
            && !arrangement.drums.is_empty()
            && (matches!(self.session.mode(), Mode::Play) || arrangement.judging == Judging::Open)
    }

    /// The fate of `hit` in `pass`, at real time `now_s`; `origin` is the real time of position
    /// 0 in this pass. A fill is scheduled with the decision.
    pub(super) fn react(
        &mut self,
        pass: u32,
        hit: DrumHit,
        origin: f64,
        speed: f64,
        now_s: f64,
    ) -> Reaction {
        if !self.band_listens() {
            return Reaction::Play;
        }
        let Some(bar) = self.session.arrangement().bar_at(hit.tick) else {
            return Reaction::Play;
        };
        let Some(time) = self.bar_time(bar, origin, speed) else {
            return Reaction::Play;
        };

        // The last beat: a fill if the beat before was free after a phrase.
        let beat = self.beat_of(bar, hit.tick);
        if beat + 1 >= time.beats {
            match self.band.fill((pass, bar), time, now_s) {
                None => return Reaction::Wait,
                Some(true) => {
                    self.schedule_fill(pass, bar, time);
                    return Reaction::Drop;
                }
                Some(false) => {}
            }
        }

        // After a dense bar the next has no hi-hat.
        let before = bar
            .checked_sub(1)
            .and_then(|b| self.bar_time(b, origin, speed));
        let thin = before.is_some_and(|b| self.band.thin((pass, bar), b, now_s));
        if thin && hit.kind == DrumKind::HiHat {
            return Reaction::Drop;
        }
        Reaction::Play
    }

    /// Snare sixteenths through the last beat of `bar`.
    fn schedule_fill(&mut self, pass: u32, bar: usize, time: BarTime) {
        let last = time.beat_start_s(time.beats - 1);
        for stroke in 0..FILL_STROKES {
            let key = FILL_KEYS | (bar as u64) << 3 | u64::from(stroke);
            if !self.scheduled.insert((pass, key)) {
                continue;
            }
            let at = last + time.beat_s * f64::from(stroke) / f64::from(FILL_STROKES);
            // Never in the past: the decision comes `DECIDE_LEAD_S` before the beat.
            let at = at.max(self.clock.now_s() + DECIDE_LEAD_S / 2.0);
            self.output.schedule(at, Sound::drum(DrumKind::Snare));
        }
    }

    /// Real start and beat length of `bar` in this pass.
    fn bar_time(&self, bar: usize, origin: f64, speed: f64) -> Option<BarTime> {
        let arrangement = self.session.arrangement();
        let b = arrangement.bars.get(bar)?;
        let beat_ticks = beat_ticks(b.signature.denominator);
        let start = arrangement.tempo.seconds_at(b.start_ticks);
        let next = arrangement.tempo.seconds_at(b.start_ticks + beat_ticks);
        Some(BarTime {
            start_s: origin + start / speed,
            beat_s: (next - start) / speed,
            beats: u32::from(b.signature.numerator),
        })
    }

    /// The beat of `tick` in `bar`, 0 = the downbeat.
    fn beat_of(&self, bar: usize, tick: u64) -> u32 {
        let Some(b) = self.session.arrangement().bars.get(bar) else {
            return 0;
        };
        let beat = (tick - b.start_ticks.min(tick)) / beat_ticks(b.signature.denominator);
        u32::try_from(beat).unwrap_or(u32::MAX)
    }
}

/// Ticks of one beat of a signature with `denominator` (4: a quarter).
fn beat_ticks(denominator: u8) -> u64 {
    TICKS_PER_QUARTER * 4 / u64::from(denominator.max(1))
}
