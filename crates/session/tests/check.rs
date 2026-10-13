//! Hardware checks (round 10) through a real-time input: a loopback take measures how late the
//! planned tones come back, and the take keeps the audio in step with the plan.

use kontra_model::{
    Arrangement, InputSource, Instrument, Note, Pitch, TempoMap, TICKS_PER_QUARTER,
};
use kontra_session::{
    loopback_length_s, loopback_plan, render_performance, round_trip, write_mono_wav, InputChoice,
    LiveSetup, LiveTake, Mistakes, NULL_DEVICE,
};
use std::time::Duration;

const RATE: u32 = 48_000;
/// The plan starts this long after the input's first sample (`check::LEAD_S`).
const LEAD_S: f64 = 1.0;
/// The simulated cable's delay.
const DELAY_S: f64 = 0.015;

/// The loopback tones as a bass would play them: E2 every 0.6 s (100 BPM quarters).
fn tones() -> Arrangement {
    let mut a = Arrangement::new("Loopback", Instrument::default_bass());
    a.tempo = TempoMap::constant(100.0);
    for i in 0..loopback_plan().len() as u64 {
        let mut n = Note::new(i * TICKS_PER_QUARTER, TICKS_PER_QUARTER / 2, Pitch(40));
        n.string = Some(1);
        a.notes.push(n);
    }
    a.normalize();
    a
}

#[test]
fn loopback_take_measures_the_round_trip() {
    let samples = render_performance(&tones(), 1.0, LEAD_S + DELAY_S, Mistakes::default(), RATE);
    let path = std::env::temp_dir().join(format!("kontra-loopback-{}.wav", std::process::id()));
    write_mono_wav(&path, &samples, RATE).unwrap();
    let setup = LiveSetup {
        input: InputChoice::File {
            path: path.clone(),
            laptop_jack: false,
        },
        output_device: Some(NULL_DEVICE.to_string()),
        source: InputSource::Line,
        backing: None,
    };
    let plan = loopback_plan();
    let mut live = LiveTake::start(
        &setup,
        &Instrument::default_bass(),
        plan.clone(),
        loopback_length_s(),
    )
    .unwrap();
    while !live.poll() {
        std::thread::sleep(Duration::from_millis(10));
    }
    let take = live.finish();
    let _ = std::fs::remove_file(&path);

    let trip = round_trip(&plan, &take).unwrap();
    assert_eq!(trip.heard, trip.sent, "{trip:?}");
    assert!((trip.median_ms - DELAY_S * 1000.0).abs() < 8.0, "{trip:?}");

    // The recording starts at the file's start: about 1 s before the plan's zero.
    assert!((take.start_s + LEAD_S).abs() < 0.05, "{}", take.start_s);
    let first = ((LEAD_S + DELAY_S) * f64::from(RATE)) as usize;
    let quiet = take.samples[..first / 2]
        .iter()
        .fold(0.0f32, |m, s| m.max(s.abs()));
    assert!(quiet < 1e-3, "silence before the first tone: {quiet}");
}

/// A bass take played exactly on the beat: its annotation lands on the notes in the recording,
/// so the corpus pair measures detection, not the alignment.
#[test]
fn take_annotation_lands_on_the_notes() {
    let takes = kontra_session::check_takes();
    let take = takes.iter().find(|t| t.name == "fretted").unwrap();
    let mut a = Arrangement::new("Take", Instrument::default_bass());
    a.tempo = TempoMap::constant(kontra_session::CHECK_BPM);
    for n in &take.notes {
        let start = (n.beat * TICKS_PER_QUARTER as f64) as u64;
        let mut note = Note::new(start, TICKS_PER_QUARTER, Pitch(n.midi));
        note.string = Some(n.string);
        note.fret = Some(n.fret);
        a.notes.push(note);
    }
    a.normalize();
    // Count-in of four beats at 80 BPM: 3 s after the plan's zero.
    let lead = LEAD_S + 4.0 * 60.0 / kontra_session::CHECK_BPM;
    let samples = render_performance(&a, 1.0, lead, Mistakes::default(), RATE);
    let path = std::env::temp_dir().join(format!("kontra-take-{}.wav", std::process::id()));
    write_mono_wav(&path, &samples, RATE).unwrap();
    let setup = LiveSetup {
        input: InputChoice::File {
            path: path.clone(),
            laptop_jack: false,
        },
        output_device: Some(NULL_DEVICE.to_string()),
        source: InputSource::Line,
        backing: None,
    };
    let mut live = LiveTake::start(
        &setup,
        &Instrument::default_bass(),
        take.plan(),
        take.length_s(),
    )
    .unwrap();
    while !live.poll() {
        std::thread::sleep(Duration::from_millis(10));
    }
    let heard = live.finish();
    let _ = std::fs::remove_file(&path);

    let notes = take.annotate(&heard, 0.0);
    let matching = kontra_session::Matching {
        window_s: 0.02,
        require_pitch: true,
    };
    let (e, _) = kontra_session::evaluate_korpus_with(
        kontra_session::KorpusSet::Line4,
        &heard.samples,
        heard.sample_rate,
        &notes,
        matching,
    );
    assert_eq!((e.matched, e.pitch_correct), (8, 8), "{e:?}");
}
