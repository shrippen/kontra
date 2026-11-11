//! The plugin thread outlives a panic in what it runs (FX4): before, one panic ended the thread
//! and no plugin loaded until Kontra restarted. Its own test binary: the thread is one per
//! process, and the panic must not reach the other tests.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kontra_drivers::plugins::{FrameMaker, PluginFormat, PluginHost, PluginLoaded, PluginSetup};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

const GAIN_ID: &str = "de.arianw.kontra.test.gain";
const PATIENCE: Duration = Duration::from_secs(5);

/// A copy of its own per load: a loaded plugin file must not be overwritten.
fn gain_file() -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let exe = std::env::current_exe().expect("test binary path");
    let profile = exe.parent().and_then(Path::parent).expect("target folder");
    let built = profile.join("examples").join(format!(
        "{}kontra_clap_gain{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    ));
    let dir = profile.join("plugin-tests").join(format!(
        "panic-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("folder");
    let file = dir.join("kontra_clap_gain.clap");
    std::fs::copy(&built, &file).expect("copy plugin");
    file
}

fn load_gain() -> PluginLoaded {
    let setup = PluginSetup {
        format: PluginFormat::Clap,
        path: gain_file(),
        id: GAIN_ID.into(),
        state: Vec::new(),
        params: Vec::new(),
        sample_rate: 48_000.0,
        max_block: 256,
        crash_marker: None,
    };
    let pending = PluginHost::global().load(setup);
    let start = Instant::now();
    loop {
        if let Some(result) = pending.poll() {
            return result.expect("gain loads");
        }
        assert!(
            start.elapsed() < PATIENCE,
            "no answer from the plugin thread"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn a_panic_on_the_plugin_thread_leaves_it_running() {
    let first = load_gain();
    let panicking: FrameMaker = Box::new(|_, _, _| panic!("a bug in a frame"));
    let opened = first.handle.open_editor("Gain", panicking);
    assert!(opened.is_err(), "{opened:?}");

    // The thread still answers: a new plugin loads.
    let second = load_gain();
    drop((first, second));
}
