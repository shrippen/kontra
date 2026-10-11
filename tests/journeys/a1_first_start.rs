//! J-A1 Anstecken und los (P1 Lea): first start with the bass in the laptop jack. The assistant
//! is due, the level is too quiet until the volume goes up, calibration is skipped, and the
//! first exercise judges its first note well within 60 s. The level advice itself is drawn by
//! `LevelAdvice.qml`; this test checks the levels Core hands it, against the same limits.

use crate::common::{run_until, Profile, COUNT_IN_S, RATE};
use kontra_appcore::{Core, QaOptions, UiEvent};
use std::path::Path;
use std::time::{Duration, Instant};

/// `LevelAdvice.qml`: below −60 dB "no signal", below −36 dB "too quiet", above −6 dB "loud".
const SILENT_DB: f64 = -60.0;
const QUIET_DB: f64 = -36.0;
const LOUD_DB: f64 = -6.0;
/// Volume knob nearly closed: the rendered bass at a fortieth (−32 dB).
const KNOB_DOWN: f32 = 0.025;
const FIRST_NOTE_LIMIT: Duration = Duration::from_secs(60);

fn take(path: &Path, gain: f32) {
    let demo = kontra_session::import_file(&Core::demo_path()).unwrap();
    let a = demo.arrangement(0).unwrap();
    let samples: Vec<f32> =
        kontra_session::render_performance(&a, 1.0, COUNT_IN_S, Default::default(), RATE)
            .into_iter()
            .map(|s| s * gain)
            .collect();
    kontra_session::write_mono_wav(path, &samples, RATE).unwrap();
}

fn laptop(profile: &Profile, take: &Path) -> Core {
    let qa = QaOptions {
        input_file: Some(take.to_path_buf()),
        laptop_jack: true,
    };
    let mut core = Core::new(&profile.dir, qa);
    core.update_settings(|s| s.language = Some("de".into()));
    core
}

fn loudest(events: &[UiEvent]) -> f64 {
    events
        .iter()
        .filter_map(|e| match e {
            UiEvent::Level { peak_db, .. } => Some(*peak_db),
            _ => None,
        })
        .fold(f64::NEG_INFINITY, f64::max)
}

#[test]
fn laptop_jack_to_first_note() {
    let started = Instant::now();
    let profile = Profile::new("a1");
    let quiet = profile.dir.join("leise.wav");
    let normal = profile.dir.join("normal.wav");
    take(&quiet, KNOB_DOWN);
    take(&normal, 1.0);

    // Step 1: first start, the assistant is due.
    let mut core = laptop(&profile, &quiet);
    assert!(!core.settings().setup_done);

    // Step 2: four strings, standard tuning, laptop jack; the level is too quiet.
    core.reconfigure(|s| {
        s.strings = 4;
        s.tuning = "standard".into();
        s.input_kind = kontra_appcore::InputKind::LaptopJack;
    });
    let hint = core.tr("hint.laptop", &[]);
    assert_ne!(hint, "hint.laptop", "the laptop hint has a text");
    core.start_tuner().unwrap();
    let peak = loudest(&run_until(&mut core, Duration::from_secs(4), |_| false));
    assert!(
        peak > SILENT_DB && peak < QUIET_DB,
        "too quiet, not silent: {peak} dB"
    );
    core.stop_tuner();

    // Step 3: volume up: the level is good.
    let mut core = laptop(&profile, &normal);
    core.start_tuner().unwrap();
    let peak = loudest(&run_until(&mut core, Duration::from_secs(4), |_| false));
    assert!((QUIET_DB..LOUD_DB).contains(&peak), "good: {peak} dB");
    core.stop_tuner();

    // Step 4: calibration skipped, the assistant is done.
    core.update_settings(|s| s.setup_done = true);

    // Step 5: the first exercise, first note judged.
    core.open(&Core::demo_path()).unwrap();
    core.set_mode("play");
    core.play();
    let events = run_until(&mut core, Duration::from_secs(10), |e| {
        matches!(e, UiEvent::Judged { .. })
    });
    assert!(
        events.iter().any(|e| matches!(e, UiEvent::Judged { .. })),
        "a note was judged"
    );
    assert!(
        started.elapsed() < FIRST_NOTE_LIMIT,
        "{:?}",
        started.elapsed()
    );
}
