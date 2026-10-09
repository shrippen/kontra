//! The session clock: maps real time to arrangement position, with speed, pause, loops and
//! holds (Wait mode). Keeps a short history so late events (detected notes arrive 50–150 ms
//! after their onset) are placed where the position was when they happened.

use std::collections::VecDeque;

const HISTORY: usize = 64;
const MIN_SPEED: f64 = 0.1;
const MAX_SPEED: f64 = 2.0;

/// Section to repeat, in arrangement seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoopRange {
    pub start_s: f64,
    pub end_s: f64,
}

/// From `real_s` on, position runs from `position_s` at `rate` (0 = stopped) in loop pass `pass`.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Segment {
    real_s: f64,
    position_s: f64,
    rate: f64,
    pass: u32,
}

#[derive(Debug, Clone)]
pub struct Transport {
    speed: f64,
    running: bool,
    looping: Option<LoopRange>,
    end_s: f64,
    segments: VecDeque<Segment>,
}

impl Transport {
    /// Stopped transport at position 0 for an arrangement of `end_s` seconds.
    pub fn new(end_s: f64) -> Transport {
        let mut segments = VecDeque::with_capacity(HISTORY);
        segments.push_back(Segment {
            real_s: f64::NEG_INFINITY,
            position_s: 0.0,
            rate: 0.0,
            pass: 0,
        });
        Transport {
            speed: 1.0,
            running: false,
            looping: None,
            end_s,
            segments,
        }
    }

    pub fn speed(&self) -> f64 {
        self.speed
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn loop_range(&self) -> Option<LoopRange> {
        self.looping
    }

    pub fn end_s(&self) -> f64 {
        self.end_s
    }

    /// Current segment's pass number (loop iteration).
    pub fn pass(&self) -> u32 {
        self.last().pass
    }

    pub fn play(&mut self, real_s: f64) {
        self.running = true;
        let (position, pass) = self.position_at(real_s);
        self.push(Segment {
            real_s,
            position_s: position,
            rate: self.speed,
            pass,
        });
    }

    pub fn pause(&mut self, real_s: f64) {
        self.running = false;
        let (position, pass) = self.position_at(real_s);
        self.push(Segment {
            real_s,
            position_s: position,
            rate: 0.0,
            pass,
        });
    }

    /// Jumps to `position_s` (keeps running state).
    pub fn seek(&mut self, real_s: f64, position_s: f64) {
        let rate = if self.running { self.speed } else { 0.0 };
        let pass = self.pass();
        self.push(Segment {
            real_s,
            position_s: position_s.clamp(0.0, self.end_s),
            rate,
            pass,
        });
    }

    /// Changes speed (0.1–2.0) from `real_s` on.
    pub fn set_speed(&mut self, real_s: f64, speed: f64) {
        self.speed = speed.clamp(MIN_SPEED, MAX_SPEED);
        if self.running {
            self.play(real_s);
        }
    }

    /// Sets or clears the loop; a position outside the new loop jumps to its start.
    pub fn set_loop(&mut self, real_s: f64, range: Option<LoopRange>) {
        self.looping = range.filter(|r| r.end_s > r.start_s);
        let Some(range) = self.looping else {
            return;
        };
        let (position, _) = self.position_at(real_s);
        if position < range.start_s || position >= range.end_s {
            self.seek(real_s, range.start_s);
        }
    }

    /// Position and pass at real time `real_s` (also in the past, within the history).
    pub fn position_at(&self, real_s: f64) -> (f64, u32) {
        let segment = self
            .segments
            .iter()
            .rev()
            .find(|s| s.real_s <= real_s)
            .unwrap_or(self.first());
        let elapsed = (real_s - segment.real_s).max(0.0);
        let position = if segment.rate == 0.0 {
            segment.position_s
        } else {
            segment.position_s + elapsed * segment.rate
        };
        (position.min(self.end_s), segment.pass)
    }

    /// Moves the clock to `real_s`: wraps loops, stops at the end, and holds at `hold_s` if given
    /// (Wait mode). Returns the number of loop passes completed.
    pub fn advance(&mut self, real_s: f64, hold_s: Option<f64>) -> u32 {
        let mut wrapped = 0;
        loop {
            let last = *self.last();
            if last.rate == 0.0 {
                return wrapped;
            }
            let limit = self.looping.map_or(self.end_s, |l| l.end_s);
            let hold = hold_s.filter(|h| *h >= last.position_s && *h < limit);
            let stop_at = hold.unwrap_or(limit);
            let reach = last.real_s + (stop_at - last.position_s) / last.rate;
            if reach > real_s {
                return wrapped;
            }
            if hold.is_some() {
                self.push(Segment {
                    real_s: reach,
                    position_s: stop_at,
                    rate: 0.0,
                    pass: last.pass,
                });
                return wrapped;
            }
            match self.looping {
                Some(range) => {
                    wrapped += 1;
                    self.push(Segment {
                        real_s: reach,
                        position_s: range.start_s,
                        rate: last.rate,
                        pass: last.pass + 1,
                    });
                }
                None => {
                    self.running = false;
                    self.push(Segment {
                        real_s: reach,
                        position_s: self.end_s,
                        rate: 0.0,
                        pass: last.pass,
                    });
                    return wrapped;
                }
            }
        }
    }

    /// Releases a hold (Wait mode) at `real_s` without changing the running state.
    pub fn release(&mut self, real_s: f64) {
        if self.running {
            self.play(real_s);
        }
    }

    /// The latest segment: from real time `.0` the position runs from `.1` at rate `.2` (0 =
    /// stopped) in pass `.3`. Unlike `position_at`, this also describes a start in the future.
    pub fn anchor(&self) -> (f64, f64, f64, u32) {
        let last = self.last();
        (last.real_s, last.position_s, last.rate, last.pass)
    }

    /// True when the clock is held although the transport is running.
    pub fn is_held(&self) -> bool {
        self.running && self.last().rate == 0.0
    }

    fn push(&mut self, segment: Segment) {
        if self.segments.len() == HISTORY {
            self.segments.pop_front();
        }
        self.segments.push_back(segment);
    }

    fn last(&self) -> &Segment {
        // The deque is never empty: `new` pushes one segment and `push` keeps at least one.
        self.segments.back().unwrap_or(&FALLBACK)
    }

    fn first(&self) -> &Segment {
        self.segments.front().unwrap_or(&FALLBACK)
    }
}

const FALLBACK: Segment = Segment {
    real_s: f64::NEG_INFINITY,
    position_s: 0.0,
    rate: 0.0,
    pass: 0,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_speed() {
        let mut t = Transport::new(10.0);
        t.set_speed(0.0, 0.5);
        t.play(1.0);
        assert_eq!(t.position_at(3.0).0, 1.0);
    }

    #[test]
    fn loop_wraps_and_remembers_past() {
        let mut t = Transport::new(10.0);
        t.set_loop(
            0.0,
            Some(LoopRange {
                start_s: 2.0,
                end_s: 4.0,
            }),
        );
        t.play(0.0);
        assert_eq!(t.advance(5.0, None), 2);
        let (pos, pass) = t.position_at(5.0);
        assert!((pos - 3.0).abs() < 1e-9);
        assert_eq!(pass, 2);
        // A late event from real time 1.5 belongs to pass 0 at position 3.5.
        assert_eq!(t.position_at(1.5), (3.5, 0));
    }

    #[test]
    fn holds_and_releases() {
        let mut t = Transport::new(10.0);
        t.play(0.0);
        t.advance(5.0, Some(2.0));
        assert!(t.is_held());
        assert_eq!(t.position_at(5.0).0, 2.0);
        t.release(6.0);
        assert_eq!(t.position_at(7.0).0, 3.0);
    }

    #[test]
    fn stops_at_end() {
        let mut t = Transport::new(3.0);
        t.play(0.0);
        t.advance(10.0, None);
        assert!(!t.is_running());
        assert_eq!(t.position_at(10.0).0, 3.0);
    }
}
