//! J-B3 Refrain sitzt bis Freitag (P2 Jens): the hardest bar becomes the workshop loop; at 60 %
//! with clean passes the tempo climbs through the band up to 100 %.

use crate::common::{piece, write_midi, Profile};
use kontra_appcore::Core;
use kontra_model::{DetectedNote, InputSource};
use kontra_session::{LoopRange, Mode, ScoreSettings, Session, SessionEvent};

const RIFF: [u8; 8] = [28, 33, 38, 43, 40, 38, 36, 33];
/// Time from playing a note to its detection (onset analysis needs a few blocks).
const REPORT_S: f64 = 0.08;
const STEP_S: f64 = 0.01;

/// A player who plays every note on time at whatever tempo the workshop runs.
fn play_clean(session: &mut Session, range: LoopRange, until_s: f64) -> Vec<SessionEvent> {
    let times: Vec<f64> = {
        let a = session.arrangement();
        a.notes
            .iter()
            .map(|n| a.tempo.seconds_at(n.start_ticks))
            .collect()
    };
    let pitches: Vec<f64> = session
        .arrangement()
        .notes
        .iter()
        .map(|n| f64::from(n.pitch.0))
        .collect();
    let mut events = Vec::new();
    let mut pending: Vec<DetectedNote> = Vec::new();
    // Just before the loop start, so its first note is played too.
    let mut last_pos = range.start_s - STEP_S;
    let mut last_pass = session.transport().position_at(0.0).1;
    let mut t = 0.0;
    while t <= until_s {
        let (pos, pass) = session.transport().position_at(t);
        let from = if pass == last_pass {
            last_pos
        } else {
            range.start_s - STEP_S
        };
        for (time, midi) in times.iter().zip(&pitches) {
            if *time > from && *time <= pos {
                pending.push(DetectedNote {
                    onset_s: t,
                    midi: *midi,
                    confidence: 0.9,
                    level_db: -12.0,
                    source: InputSource::Line,
                    technique: None,
                    over_background: false,
                });
            }
        }
        (last_pos, last_pass) = (pos, pass);
        pending.retain(|n| {
            let due = n.onset_s + REPORT_S <= t;
            if due {
                session.on_note(n);
            }
            !due
        });
        events.extend(session.update(t));
        t += STEP_S;
    }
    events
}

#[test]
fn hardest_bar_loops_and_tempo_climbs_to_target() {
    // Step 2: a click on bar 2 loops exactly that bar (2.0–4.0 s at 120 BPM).
    let profile = Profile::new("b3");
    let score = profile.dir.join("refrain.mid");
    write_midi(&score, "Refrain", &RIFF);
    let mut core = profile.core(None);
    core.open(&score).unwrap();
    core.set_loop(1, 1);
    assert_eq!(core.loop_seconds(), (2.0, 4.0));
    assert_eq!(Core::workshop_band(), (0.8, 0.9));

    // Step 3-4: 60 %, clean passes until 100 %, one step at a time, never slower.
    let range = LoopRange {
        start_s: 2.0,
        end_s: 4.0,
    };
    let mut session = Session::new(
        piece("Refrain", &RIFF),
        Mode::Workshop(Default::default()),
        ScoreSettings::default(),
    );
    session.set_speed(0.0, 0.6);
    session.set_loop(0.0, Some(range));
    session.play(0.0);
    let events = play_clean(&mut session, range, 90.0);
    let speeds: Vec<f64> = events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::SpeedChanged(v) => Some(*v),
            _ => None,
        })
        .collect();
    let rates: Vec<f64> = events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::PassComplete { hit_rate, .. } => Some(*hit_rate),
            _ => None,
        })
        .collect();
    assert!(rates.iter().all(|r| *r == 1.0), "{rates:?}");
    assert!(speeds.windows(2).all(|w| w[1] > w[0]), "{speeds:?}");
    assert!(
        (speeds.last().copied().unwrap_or(0.0) - 1.0).abs() < 1e-9,
        "{speeds:?}"
    );
    assert!((session.transport().speed() - 1.0).abs() < 1e-9);
}
