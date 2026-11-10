//! The device's own stamps: how far capture and playback lie from the callback (R3).

/// A lead longer than this is no lead but a stamp the host got wrong; it counts as none.
const MAX_LEAD_S: f64 = 0.5;

/// Seconds from `earlier` to `later` by the device's stamps (R3). Hardware check 10.11.2026,
/// Scarlett Solo by cable, PipeWire: the stamps of input and output share one clock; with them
/// the round trip stayed at 17.1 to 17.5 ms over three openings, anchored at the first callback
/// it moved between 24.9 and 30.8 ms.
///
/// ```text
/// input   capture ──── lead ────► callback          output  callback ──── lead ────► playback
/// ```
pub(super) fn lead_s(later: cpal::StreamInstant, earlier: cpal::StreamInstant) -> f64 {
    later
        .checked_duration_since(earlier)
        .map(|d| d.as_secs_f64())
        .filter(|s| *s <= MAX_LEAD_S)
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A host's stamps count only where they make sense: forward and under half a second.
    #[test]
    fn lead_takes_only_sane_stamps() {
        let at = |ms: u64| cpal::StreamInstant::from_nanos(1_000_000_000 + ms * 1_000_000);
        assert!((lead_s(at(14), at(10)) - 0.004).abs() < 1e-9);
        assert_eq!(lead_s(at(10), at(14)), 0.0);
        assert_eq!(lead_s(at(900), at(10)), 0.0);
    }
}
