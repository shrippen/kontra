//! Where to sit against the beat (DIDAKTIK I5, LH-BEW-08): an aim for the mean timing offset
//! and whether a run held it.
//!
//! ```text
//!   pushed          on the beat        laid-back
//! ├─────────┤      ├─────┼─────┤      ├─────────┤
//! −30     −10     −5     0     +5    +10       +30 ms (mean offset; negative = early)
//! ```

/// Mean offset in ms that still counts as on the beat.
const ON_BEAT_MS: f64 = 5.0;
/// Laid-back or pushed: the mean sits this far off the beat (micro-timing in grooves is
/// about 10–30 ms; Frühauf et al. 2013).
const OFF_BEAT_MIN_MS: f64 = 10.0;
const OFF_BEAT_MAX_MS: f64 = 30.0;

/// The aim for a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Pocket {
    /// No aim: the summary only says where the run sat.
    #[default]
    Free,
    OnBeat,
    LaidBack,
    Pushed,
}

/// How a run's mean offset relates to its aim.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PocketVerdict {
    Held,
    /// Earlier than the aim allows, by this many ms.
    Early(f64),
    /// Later than the aim allows, by this many ms.
    Late(f64),
}

impl Pocket {
    pub const ALL: [Pocket; 4] = [
        Pocket::Free,
        Pocket::OnBeat,
        Pocket::LaidBack,
        Pocket::Pushed,
    ];

    /// Settings and translation key: "free", "onbeat", "laidback", "pushed".
    pub fn key(self) -> &'static str {
        match self {
            Pocket::Free => "free",
            Pocket::OnBeat => "onbeat",
            Pocket::LaidBack => "laidback",
            Pocket::Pushed => "pushed",
        }
    }

    pub fn from_key(key: &str) -> Option<Pocket> {
        Pocket::ALL.into_iter().find(|p| p.key() == key)
    }

    /// Range of the mean offset in ms that holds this aim; none for `Free`.
    pub fn band_ms(self) -> Option<(f64, f64)> {
        match self {
            Pocket::Free => None,
            Pocket::OnBeat => Some((-ON_BEAT_MS, ON_BEAT_MS)),
            Pocket::LaidBack => Some((OFF_BEAT_MIN_MS, OFF_BEAT_MAX_MS)),
            Pocket::Pushed => Some((-OFF_BEAT_MAX_MS, -OFF_BEAT_MIN_MS)),
        }
    }

    /// Judges a run's mean offset, e.g. laid-back at +4 ms is 6 ms too early.
    pub fn judge(self, mean_offset_ms: f64) -> Option<PocketVerdict> {
        let (low, high) = self.band_ms()?;
        if mean_offset_ms < low {
            return Some(PocketVerdict::Early(low - mean_offset_ms));
        }
        if mean_offset_ms > high {
            return Some(PocketVerdict::Late(mean_offset_ms - high));
        }
        Some(PocketVerdict::Held)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn judges_the_mean_against_the_aim() {
        assert_eq!(Pocket::Free.judge(40.0), None);
        assert_eq!(Pocket::OnBeat.judge(-3.0), Some(PocketVerdict::Held));
        assert_eq!(Pocket::LaidBack.judge(18.0), Some(PocketVerdict::Held));
        assert_eq!(Pocket::LaidBack.judge(4.0), Some(PocketVerdict::Early(6.0)));
        assert_eq!(Pocket::Pushed.judge(-12.0), Some(PocketVerdict::Held));
        assert_eq!(Pocket::Pushed.judge(2.0), Some(PocketVerdict::Late(12.0)));
    }

    #[test]
    fn keys_round_trip() {
        for p in Pocket::ALL {
            assert_eq!(Pocket::from_key(p.key()), Some(p));
        }
        assert_eq!(Pocket::from_key("swing"), None);
    }
}
