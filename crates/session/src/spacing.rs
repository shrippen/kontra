//! Practice over days: the streak (LH-FOR-05) and hard passages coming back after a pause
//! (LH-LRN-16, DIDAKTIK I3). Pure functions over what the progress files hold.
//!
//! ```text
//! hardest twice ─▶ due after 1 day ─pass─▶ 2 days ─pass─▶ 4 … 64 days
//!                       ▲                                   │
//!                       └────────────── fail ───────────────┘   (a pass before it is due: nothing)
//! ```

const SECONDS_PER_DAY: u64 = 86_400;

/// A "day" of rest ends in the morning after an evening's practice: one interval day is a
/// night (8 h), each further one a full day.
const NIGHT_SLACK_S: u64 = 16 * 60 * 60;

/// First and longest interval in days.
const FIRST_DAYS: u32 = 1;
const LONGEST_DAYS: u32 = 64;

/// Times a passage must be among the hardest bars before it comes back.
const HARD_TIMES: usize = 2;

/// Gap between two practice days that still keeps the streak: one day missed in between.
const FORGIVEN_GAP_DAYS: i64 = 2;

/// Practice days in the streak ending today: counted back from the latest day with a scored
/// run while each gap misses at most one day. Days are local day numbers in any order; none
/// in the last two days: no streak (0). Example: days 7, 9, 10 on day 11 → 3.
pub fn streak_days(days: &[i64], today: i64) -> u32 {
    let mut days: Vec<i64> = days.iter().copied().filter(|d| *d <= today).collect();
    days.sort_unstable();
    days.dedup();
    let Some(&latest) = days.last() else {
        return 0;
    };
    if today - latest > FORGIVEN_GAP_DAYS {
        return 0;
    }

    let mut count = 1;
    for pair in days.windows(2).rev() {
        if pair[1] - pair[0] > FORGIVEN_GAP_DAYS {
            break;
        }
        count += 1;
    }
    count
}

/// Something that happened to a passage, in the order of time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PassageEvent {
    /// A run had it among its hardest bars.
    Hardest { unix: u64 },
    /// A run without help (LH-LRN-05) played it; `passed` when it reached the bar of a pass
    /// (at least 80 % at the target tempo, the caller decides).
    Probe { unix: u64, passed: bool },
}

impl PassageEvent {
    fn unix(self) -> u64 {
        match self {
            PassageEvent::Hardest { unix } | PassageEvent::Probe { unix, .. } => unix,
        }
    }
}

/// When a passage comes back and the interval that brought it there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spacing {
    pub due_unix: u64,
    pub interval_days: u32,
}

impl Spacing {
    fn after(unix: u64, interval_days: u32) -> Spacing {
        Spacing {
            due_unix: (unix + u64::from(interval_days) * SECONDS_PER_DAY)
                .saturating_sub(NIGHT_SLACK_S),
            interval_days,
        }
    }
}

/// The schedule of a passage from its history, `None` while it was not among the hardest
/// bars twice. Probes before that do not count; a pass before it is due changes nothing.
pub fn spacing(events: &[PassageEvent]) -> Option<Spacing> {
    let mut events = events.to_vec();
    events.sort_by_key(|e| e.unix());

    let mut hard = 0;
    let mut current: Option<Spacing> = None;
    for event in events {
        match (event, current) {
            (PassageEvent::Hardest { unix }, None) => {
                hard += 1;
                if hard >= HARD_TIMES {
                    current = Some(Spacing::after(unix, FIRST_DAYS));
                }
            }
            (PassageEvent::Probe { unix, passed: true }, Some(s)) if unix >= s.due_unix => {
                let longer = (s.interval_days * 2).min(LONGEST_DAYS);
                current = Some(Spacing::after(unix, longer));
            }
            (
                PassageEvent::Probe {
                    unix,
                    passed: false,
                },
                Some(_),
            ) => {
                current = Some(Spacing::after(unix, FIRST_DAYS));
            }
            _ => {}
        }
    }
    current
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: u64 = SECONDS_PER_DAY;

    #[test]
    fn a_missed_day_keeps_the_streak() {
        assert_eq!(streak_days(&[], 10), 0);
        assert_eq!(streak_days(&[10], 10), 1);
        assert_eq!(streak_days(&[9, 10], 10), 2);
        // One day missed (9): kept; also when today is the missed one.
        assert_eq!(streak_days(&[7, 8, 10], 10), 3);
        assert_eq!(streak_days(&[7, 8], 9), 2);
        assert_eq!(streak_days(&[7, 8], 10), 2);
        // Two days missed: over, and an old streak does not count.
        assert_eq!(streak_days(&[7, 8], 11), 0);
        assert_eq!(streak_days(&[3, 4, 5, 9, 10], 10), 2);
        // Several runs a day count once.
        assert_eq!(streak_days(&[10, 10, 9], 10), 2);
    }

    fn hard(day: u64) -> PassageEvent {
        PassageEvent::Hardest { unix: day * DAY }
    }

    fn probe(unix: u64, passed: bool) -> PassageEvent {
        PassageEvent::Probe { unix, passed }
    }

    #[test]
    fn a_passage_comes_back_after_twice_among_the_hardest() {
        assert_eq!(spacing(&[]), None);
        assert_eq!(spacing(&[hard(1)]), None);
        // A probe before does not count.
        assert_eq!(spacing(&[probe(DAY, true), hard(2)]), None);

        let s = spacing(&[hard(1), hard(2)]).unwrap();
        assert_eq!(s.interval_days, 1);
        assert_eq!(s.due_unix, 2 * DAY + DAY - NIGHT_SLACK_S);
    }

    #[test]
    fn passes_double_the_interval_up_to_64_days() {
        let mut events = vec![hard(0), hard(0)];
        let mut expected = Vec::new();
        for _ in 0..8 {
            let due = spacing(&events).unwrap().due_unix;
            events.push(probe(due, true));
            expected.push(spacing(&events).unwrap().interval_days);
        }
        assert_eq!(expected, [2, 4, 8, 16, 32, 64, 64, 64]);
    }

    #[test]
    fn an_early_pass_waits_and_a_fail_starts_over() {
        let entered = spacing(&[hard(0), hard(0)]).unwrap();
        let early = probe(entered.due_unix - 1, true);
        assert_eq!(spacing(&[hard(0), hard(0), early]), Some(entered));

        let on_time = probe(entered.due_unix, true);
        let at_two = spacing(&[hard(0), hard(0), on_time]).unwrap();
        assert_eq!(at_two.interval_days, 2);

        // A fail, even before it is due, sets one day from then.
        let fail = probe(entered.due_unix + DAY, false);
        let s = spacing(&[hard(0), hard(0), on_time, fail]).unwrap();
        assert_eq!(s, Spacing::after(entered.due_unix + DAY, 1));
    }
}
