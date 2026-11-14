//! LH-BEW-09 (Runde 22, `mute`): ends that rang into the rest are observed after the
//! techniques and before the timing; only from enough judged ends, only above a quarter.

use kontra_model::{Arrangement, Instrument, Technique};
use kontra_session::{run_from, MutingTally, Observation, Pocket, Summary, TechniqueTally};

fn run() -> Summary {
    let arrangement = Arrangement::new("Run", Instrument::default_bass());
    let mut s = Summary::from_results(&[], &arrangement);
    s.judged = 10;
    s.hits = 10;
    s.hit_rate = 1.0;
    s
}

fn tally(judged: u32, rang_on: u32) -> MutingTally {
    MutingTally {
        judged,
        rang_on,
        in_time: judged - rang_on,
        ..MutingTally::default()
    }
}

#[test]
fn nothing_muted_is_observed() {
    let mut s = run();
    s.muting = tally(31, 31);
    let expected = Observation::Muting {
        rang_on: 31,
        judged: 31,
    };
    assert_eq!(s.observation(Pocket::Free), Some(expected));
}

#[test]
fn a_quarter_is_not_more_than_a_quarter() {
    let mut s = run();
    s.muting = tally(8, 2);
    assert_eq!(s.observation(Pocket::Free), Some(Observation::Solid));
    s.muting = tally(8, 3);
    assert!(matches!(
        s.observation(Pocket::Free),
        Some(Observation::Muting { .. })
    ));
}

#[test]
fn few_or_no_judged_ends_change_nothing() {
    let mut s = run();
    s.muting = tally(3, 3);
    assert_eq!(s.observation(Pocket::Free), Some(Observation::Solid));

    // Microphone and MIDI judge no ends.
    s.muting = MutingTally {
        untold: 31,
        ..MutingTally::default()
    };
    assert_eq!(s.observation(Pocket::Free), Some(Observation::Solid));
}

#[test]
fn a_weak_technique_ranks_above_muting() {
    let mut s = run();
    s.muting = tally(31, 31);
    s.techniques = vec![TechniqueTally {
        technique: Technique::Slide,
        played: 0,
        judged: 6,
    }];
    assert!(matches!(
        s.observation(Pocket::Free),
        Some(Observation::Technique { .. })
    ));
}

// Review #343: the stored run keeps the share stopped in time, none when nothing was judged.
#[test]
fn a_run_keeps_the_share_stopped_in_time() {
    let mut s = run();
    s.muting = MutingTally {
        judged: 8,
        in_time: 6,
        late: 1,
        rang_on: 1,
        ..MutingTally::default()
    };
    assert_eq!(
        run_from(&s, 60.0, 1.0, "song", None, 1).muted_in_time,
        Some(0.75)
    );

    s.muting = MutingTally::default();
    assert_eq!(run_from(&s, 60.0, 1.0, "song", None, 1).muted_in_time, None);
}
