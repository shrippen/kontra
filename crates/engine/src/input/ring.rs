//! The input ring between the audio callback and the analysis: samples, the places where a full
//! ring dropped some, and when the device captured each block.
//!
//! ```text
//! callback ─► Capture ─┬─ samples ──┬─► Drain ─► analysis
//!                      ├─ gaps ─────┤
//!                      └─ stamps ───┘   skipped_s: device time not delivered as samples
//! ```

/// Marks a gap in the input ring: a NaN with its own payload, so a NaN from a device is not
/// taken for one.
const GAP_MARK: u32 = 0x7fc0_dead;
/// Gaps the analysis may owe at once. Past that, the open gap grows: samples are dropped until
/// the analysis reads one, even if the ring has room again.
const GAPS: usize = 64;
/// Block stamps the analysis may owe. A full stamp ring drops a stamp, never a sample; the
/// next stamp carries on.
const STAMPS: usize = 1024;

/// When the device captured a block: its first sample's index and the device time of it.
#[derive(Debug, Clone, Copy)]
struct Stamp {
    sample: u64,
    captured_s: f64,
}

/// Audio callback side of the input ring. A full ring (analysis behind) drops samples rather
/// than block; their count goes to the analysis as a marker at the place they were lost, so
/// the time of everything after stays right.
///
/// ```text
/// samples  a b c [d e dropped] f     gap ring: 2
/// ring     a b c  MARK          f    analysis: a b c 0 0 f
/// ```
pub(super) struct Capture {
    ring: rtrb::Producer<f32>,
    gaps: rtrb::Producer<u64>,
    stamps: rtrb::Producer<Stamp>,
    /// Samples dropped since the last marker.
    gap: u64,
    /// Samples received since the start, dropped ones included.
    received: u64,
}

impl Capture {
    /// Copies a block the device captured at `captured_s` (device time since its first block)
    /// into the ring without blocking or allocating.
    pub(super) fn push(&mut self, samples: &[f32], captured_s: f64) {
        let _ = self.stamps.push(Stamp {
            sample: self.received,
            captured_s,
        });
        self.received += samples.len() as u64;

        for &s in samples {
            if self.gap > 0 {
                self.close_gap();
            }
            if self.gap > 0 || self.ring.push(s).is_err() {
                self.gap += 1;
            }
        }
    }

    /// Writes the marker once the ring has room again.
    fn close_gap(&mut self) {
        if self.ring.slots() == 0 || self.gaps.slots() == 0 {
            return;
        }
        let _ = self.gaps.push(self.gap);
        let _ = self.ring.push(f32::from_bits(GAP_MARK));
        self.gap = 0;
    }
}

/// Analysis side of the input ring: gives the lost samples back as silence.
pub(super) struct Drain {
    ring: rtrb::Consumer<f32>,
    gaps: rtrb::Consumer<u64>,
    stamps: rtrb::Consumer<Stamp>,
    /// Silent samples still owed for a gap.
    owed: u64,
    /// Samples handed out since the start, silence for gaps included.
    read: u64,
    /// Device time not delivered as samples, as of the newest block read.
    skipped_s: f64,
}

impl Drain {
    /// Fills the front of `out` with the next samples; returns how many.
    pub(super) fn read(&mut self, out: &mut [f32]) -> usize {
        let mut n = 0;
        while n < out.len() {
            if self.owed > 0 {
                out[n] = 0.0;
                self.owed -= 1;
                n += 1;
                continue;
            }
            let Ok(s) = self.ring.pop() else {
                break;
            };
            if s.to_bits() == GAP_MARK {
                self.owed = self.gaps.pop().unwrap_or(0);
                continue;
            }
            out[n] = s;
            n += 1;
        }
        self.read += n as u64;
        n
    }

    /// Device time since its first block that no sample covers, as of what was read: capture
    /// time minus sample time. It grows only where the device skipped (a stall, an overrun);
    /// late delivery, in one piece or many, leaves it as it is.
    ///
    /// ```text
    /// stamps   sample 0 @ 0.0 s   sample 4800 @ 0.1 s   sample 9600 @ 1.0 s   (48 kHz)
    /// skipped  0.0                0.0                   0.8
    /// ```
    pub(super) fn skipped_s(&mut self, rate: f64) -> f64 {
        while let Ok(stamp) = self.stamps.peek().copied() {
            if stamp.sample >= self.read {
                break;
            }
            let _ = self.stamps.pop();
            self.skipped_s = stamp.captured_s - stamp.sample as f64 / rate;
        }
        self.skipped_s
    }
}

/// The input ring for `capacity` samples.
pub(super) fn input_ring(capacity: usize) -> (Capture, Drain) {
    let (ring, consumer) = rtrb::RingBuffer::<f32>::new(capacity);
    let (gaps, gaps_consumer) = rtrb::RingBuffer::<u64>::new(GAPS);
    let (stamps, stamps_consumer) = rtrb::RingBuffer::<Stamp>::new(STAMPS);
    let capture = Capture {
        ring,
        gaps,
        stamps,
        gap: 0,
        received: 0,
    };
    let drain = Drain {
        ring: consumer,
        gaps: gaps_consumer,
        stamps: stamps_consumer,
        owed: 0,
        read: 0,
        skipped_s: 0.0,
    };
    (capture, drain)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A ring that overflows (analysis behind) keeps the timeline: what was dropped comes out as
    /// silence in its place, so a later sample keeps its position and every note after the gap
    /// its time.
    ///
    /// ```text
    /// pushed   0 0 0 0 0 0 | 1        ring holds 4, drops 2
    /// drained  0 0 0 0 0 0 | 1        1 at index 6, not 4
    /// ```
    #[test]
    fn overflow_keeps_the_timeline() {
        let (mut capture, mut drain) = input_ring(4);
        let mut out = vec![0.0f32; 16];
        let mut drained = Vec::new();

        capture.push(&[0.0; 6], 0.0);
        let n = drain.read(&mut out);
        drained.extend_from_slice(&out[..n]);

        capture.push(&[1.0], 0.0);
        loop {
            let n = drain.read(&mut out);
            if n == 0 {
                break;
            }
            drained.extend_from_slice(&out[..n]);
        }

        assert_eq!(
            drained.iter().position(|s| *s == 1.0),
            Some(6),
            "{drained:?}"
        );
    }

    /// Two overflows with reading in between: each gap comes back in its place.
    #[test]
    fn gaps_stay_in_order() {
        let (mut capture, mut drain) = input_ring(4);
        let mut out = vec![0.0f32; 4];
        let mut drained = Vec::new();
        let mut read_all = |drain: &mut Drain, drained: &mut Vec<f32>| loop {
            let n = drain.read(&mut out);
            if n == 0 {
                break;
            }
            drained.extend_from_slice(&out[..n]);
        };

        capture.push(&[0.1, 0.1, 0.1, 0.1, 0.1], 0.0);
        read_all(&mut drain, &mut drained);
        capture.push(&[0.2, 0.3, 0.3, 0.3, 0.3, 0.3], 0.0);
        read_all(&mut drain, &mut drained);
        capture.push(&[0.4], 0.0);
        read_all(&mut drain, &mut drained);

        // 5 + 6 + 1 samples; the dropped ones (one 0.1, two 0.3) come back as silence.
        assert_eq!(drained.len(), 12, "{drained:?}");
        assert_eq!(
            drained.iter().position(|s| *s == 0.2),
            Some(5),
            "{drained:?}"
        );
        assert_eq!(drained.last(), Some(&0.4), "{drained:?}");
    }

    /// Stamps tell what the device skipped, whatever the pieces it delivers in: 0.1 s, then
    /// 0.8 s skipped, then 0.4 s read in pieces of 0.1 s (rate 1000 for round numbers).
    #[test]
    fn stamps_tell_what_was_skipped() {
        let rate = 1000.0;
        let (mut capture, mut drain) = input_ring(1000);
        let mut out = [0.0f32; 100];

        capture.push(&[0.0; 100], 0.0);
        drain.read(&mut out);
        assert!(drain.skipped_s(rate).abs() < 1e-9);

        for piece in 0..4 {
            capture.push(&[0.0; 100], 0.9 + f64::from(piece) * 0.1);
            drain.read(&mut out);
            let skipped = drain.skipped_s(rate);
            assert!((skipped - 0.8).abs() < 1e-9, "piece {piece}: {skipped}");
        }
    }
}
