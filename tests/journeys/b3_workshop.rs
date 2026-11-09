//! J-B3 Refrain sitzt bis Freitag (P2 Jens): the hardest bar becomes the workshop loop; at 60 %
//! with clean passes the tempo climbs through the band up to 100 %. In the run before, notes
//! played on another string than the tab's are named in the summary and stay hits (R7).

use crate::common::{piece, render, run_until, write_midi, Profile, BPM, COUNT_IN_S, RATE};
use kontra_appcore::{Core, UiEvent};
use kontra_model::{midi_to_hz, DetectedNote, InputSource, A4_HZ};
use kontra_session::{LoopRange, Mode, ScoreSettings, Session, SessionEvent, LOG_B_PER_FRET};
use std::time::Duration;

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

/// Riff notes (index) played one string lower than the tab puts them (E A D G: 0 = E).
const MOVED: [usize; 2] = [2, 3];
/// Partials of a synthetic stiff string and its amplitude decay per second (≈ 9 dB/s, not muted).
const PARTIALS: u32 = 30;
const RING: f64 = 1.0;

/// The riff as a stiff string plays it (partial k at `k·f0·√(1 + B·k²)`, falling 1/k), each
/// note with the B of `strings[i]` at its fret, cut at the next note.
fn stiff_take(a: &kontra_model::Arrangement, strings: &[u8]) -> Vec<f32> {
    let quarter_s = 60.0 / BPM;
    let rate = f64::from(RATE);
    let mut out =
        vec![0.0f32; ((COUNT_IN_S + (a.notes.len() as f64 + 1.0) * quarter_s) * rate) as usize];
    for (i, (note, string)) in a.notes.iter().zip(strings).enumerate() {
        let open = a.instrument.tuning.strings[usize::from(*string)].0;
        let fret = note.pitch.0 - open;
        let log_b = a.instrument.string_log_b(*string).unwrap() + f64::from(fret) * LOG_B_PER_FRET;
        let b = 10f64.powf(log_b);
        let hz = midi_to_hz(f64::from(note.pitch.0), A4_HZ);
        let start = ((COUNT_IN_S + i as f64 * quarter_s) * rate) as usize;
        let len = (quarter_s * rate) as usize - 240;
        for (j, sample) in out[start..start + len].iter_mut().enumerate() {
            let t = j as f64 / rate;
            let tone: f64 = (1..=PARTIALS)
                .map(|k| {
                    let k = f64::from(k);
                    let f = k * hz * (1.0 + b * k * k).sqrt();
                    if f > rate / 2.0 {
                        return 0.0;
                    }
                    (std::f64::consts::TAU * f * t).sin() / k
                })
                .sum();
            *sample = (0.3 * tone * (-t * RING).exp()) as f32;
        }
    }
    out
}

/// A song run of the riff from `take`; the summary JSON.
fn song_summary(core: &mut Core, score: &std::path::Path) -> serde_json::Value {
    core.open(score).unwrap();
    core.set_mode("song");
    core.play();
    let events = run_until(core, Duration::from_secs(30), |e| {
        matches!(e, UiEvent::Finished { .. })
    });
    let summary = events
        .iter()
        .find_map(|e| match e {
            UiEvent::Finished { summary_json } => Some(summary_json.clone()),
            _ => None,
        })
        .expect("finished");
    serde_json::from_str(&summary).unwrap()
}

#[test]
fn run_names_notes_on_another_string() {
    // Step 1: the run, two notes one string lower than the tab (same pitch, higher fret).
    let profile = Profile::new("b3-strings");
    let score = profile.dir.join("refrain.mid");
    write_midi(&score, "Refrain", &RIFF);
    let mut core = profile.core(None);
    core.open(&score).unwrap();
    let a = core.arrangement().unwrap().clone();
    let mut strings: Vec<u8> = a.notes.iter().map(|n| n.string.unwrap()).collect();
    for i in MOVED {
        strings[i] -= 1;
    }
    let take = profile.dir.join("saiten.wav");
    kontra_session::write_mono_wav(&take, &stiff_take(&a, &strings), RATE).unwrap();

    let mut core = profile.core(Some(&take));
    let s = song_summary(&mut core, &score);
    assert_eq!(s["hitRate"], 1.0, "the notes stay hits: {s}");
    assert_eq!(
        s["stringsText"], "2 von 7 Noten auf einer anderen Saite als in der Tab",
        "the low E has only one string: {s}"
    );
}

#[test]
fn fretless_and_five_string_tell_no_string() {
    // A fretless and a five-string profile (its low B has no value yet): the summary says so.
    for (strings, bass) in [
        (4, kontra_appcore::Bass::Fretless),
        (5, kontra_appcore::Bass::Fretted),
    ] {
        let profile = Profile::new(&format!("b3-unsupported-{strings}"));
        let score = profile.dir.join("refrain.mid");
        write_midi(&score, "Refrain", &RIFF);
        let take = profile.dir.join("take.wav");
        render(&take, &piece("Refrain", &RIFF), Default::default());
        let mut core = profile.core(Some(&take));
        core.reconfigure(|s| {
            s.strings = strings;
            s.instrument = bass;
        });
        let s = song_summary(&mut core, &score);
        assert_eq!(s["hitRate"], 1.0, "{strings} strings: {s}");
        assert_eq!(
            s["stringsText"],
            "Saiten erkennt Kontra mit diesem Instrument noch nicht (nur bundierte Bässe in E-A-D-G)",
            "{strings} strings, {bass:?}: {s}"
        );
    }
}

/// LH-TUN-05: Jens's A string is about 15 cents sharp, the E string in tune. The summary asks to
/// retune the A string.
#[test]
fn a_sharp_string_is_named() {
    // E string frets 0, 1, 3 and A string frets 0, 2, 3, three times.
    const LINE: [u8; 18] = [
        28, 33, 29, 35, 31, 36, 28, 33, 29, 35, 31, 36, 28, 33, 29, 35, 31, 36,
    ];
    const A_STRING: u8 = 1;
    const SHARP_CENT: f64 = 15.0;

    let profile = Profile::new("b3-detuned");
    let score = profile.dir.join("Stimmung.mid");
    write_midi(&score, "Stimmung", &LINE);
    let take = profile.dir.join("verstimmt.wav");

    let mut core = profile.core(None);
    core.open(&score).unwrap();
    let a = core.arrangement().unwrap().clone();
    let cents: Vec<f64> = a
        .notes
        .iter()
        .map(|n| match n.string {
            Some(A_STRING) => SHARP_CENT,
            _ => 0.0,
        })
        .collect();
    assert_eq!(
        cents.iter().filter(|c| **c > 0.0).count(),
        LINE.len() / 2,
        "half the notes on the A string"
    );
    let mistakes = kontra_session::Mistakes {
        detune_cycle_cents: Box::leak(cents.into_boxed_slice()),
        ..Default::default()
    };
    let samples = kontra_session::render_performance(&a, 1.0, COUNT_IN_S, mistakes, RATE);
    kontra_session::write_mono_wav(&take, &samples, RATE).unwrap();

    let mut core = profile.core(Some(&take));
    core.open(&score).unwrap();
    core.set_mode("song");
    core.play();
    let events = run_until(&mut core, Duration::from_secs(20), |e| {
        matches!(e, UiEvent::Finished { .. })
    });
    let summary = events
        .iter()
        .find_map(|e| match e {
            UiEvent::Finished { summary_json } => Some(summary_json.clone()),
            _ => None,
        })
        .expect("finished");
    let s: serde_json::Value = serde_json::from_str(&summary).unwrap();
    assert_eq!(
        s["detunedText"],
        "Die A-Saite klingt etwa 15 Cent zu hoch – nachstimmen?"
    );
}

/// Step 7 (LH-FOR-06): after a run the song map shows every bar's hits; dragging over bars 5–8
/// (the QML test `test_song_map_sets_the_loop` drags) sets them as the loop.
#[test]
fn song_map_shows_the_run_and_sets_the_loop() {
    let profile = Profile::new("b3-songmap");
    let score = profile.dir.join("strophe.mid");
    let riff: Vec<u8> = RIFF.iter().copied().cycle().take(32).collect();
    write_midi(&score, "Strophe", &riff);
    let take = profile.dir.join("strophe.wav");
    let mistakes = kontra_session::Mistakes {
        miss_every: 4,
        ..kontra_session::Mistakes::default()
    };
    render(&take, &piece("Strophe", &riff), mistakes);
    let mut core = profile.core(Some(&take));

    // Before the first run there is nothing to show.
    core.open(&score).unwrap();
    let map: serde_json::Value = serde_json::from_str(&core.song_map_json()).unwrap();
    assert_eq!(map["played"], false);
    assert_eq!(map["values"].as_array().unwrap().len(), 8);

    let s = song_summary(&mut core, &score);
    assert!(s["hitRate"].as_f64().unwrap() < 1.0, "{s}");
    let map: serde_json::Value = serde_json::from_str(&core.song_map_json()).unwrap();
    assert_eq!(map["played"], true);
    // One of four notes left out: 75 % in every bar, the tip says it in numbers.
    for value in map["values"].as_array().unwrap() {
        assert_eq!(value.as_f64(), Some(0.75), "{map}");
    }
    assert_eq!(
        map["tips"][4],
        "Takt 5: 75 % im letzten Durchlauf (3 von 4 Noten)"
    );

    core.set_loop(4, 7);
    let map: serde_json::Value = serde_json::from_str(&core.song_map_json()).unwrap();
    assert_eq!(map["loop"], serde_json::json!([4, 7]));
    assert_eq!(core.loop_seconds(), (8.0, 16.0));
}
