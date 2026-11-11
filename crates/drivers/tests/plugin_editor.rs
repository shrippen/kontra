//! A CLAP plugin's own editor on the plugin thread (FX5), with the test gain plugin and a frame
//! that only records: no window system needed. The plugin's editor registers a timer and (Linux)
//! a descriptor, asks for a size once it sits in a window, and turns its knob on a timer tick
//! (`crates/test-clap-gain/src/editor.rs`).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kontra_drivers::plugins::{
    EditorFrame, FrameEvent, FrameHandle, FrameMaker, PluginError, PluginFormat, PluginHandle,
    PluginHost, PluginLoaded, PluginProcessor, PluginSetup,
};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const GAIN_ID: &str = "de.arianw.kontra.test.gain";
const GAIN_PARAM: u32 = 7;
/// `kontra_clap_gain::EDITOR_*`.
const EDITOR_SIZE: (u32, u32) = (300, 200);
const EDITOR_RESIZED: (u32, u32) = (320, 240);
const EDITOR_GAIN: f64 = 2.5;
const RATE: f64 = 48_000.0;
const BLOCK: usize = 256;
const WINDOW: u32 = 0x4242;
/// How long the plugin thread may take for what the tests wait for.
const PATIENCE: Duration = Duration::from_secs(5);

/// The test plugin `kontra_clap_gain` copied to a folder of its own as `.clap`.
fn gain_file() -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let exe = std::env::current_exe().expect("test binary path");
    let profile = exe.parent().and_then(Path::parent).expect("target folder");
    let built = profile.join("examples").join(format!(
        "{}kontra_clap_gain{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    ));
    assert!(built.exists(), "{} is missing", built.display());
    let dir = profile.join("plugin-tests").join(format!(
        "editor-{}-{}",
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
        sample_rate: RATE,
        max_block: BLOCK as u32,
        crash_marker: None,
    };
    let pending = PluginHost::global().load(setup);
    wait_for(|| pending.poll()).expect("gain loads")
}

/// Polls `check` until it gives something or the patience runs out.
fn wait_for<T>(mut check: impl FnMut() -> Option<T>) -> T {
    let start = Instant::now();
    loop {
        if let Some(found) = check() {
            return found;
        }
        assert!(start.elapsed() < PATIENCE, "gave up waiting");
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// What a recording frame saw.
#[derive(Debug, Default)]
struct Seen {
    made: Option<(String, u32, u32)>,
    sizes: Vec<(u32, u32)>,
    shown: bool,
    /// Set by the test: the player closes the window.
    close: bool,
    dropped: bool,
}

struct Recording(Arc<Mutex<Seen>>);

impl EditorFrame for Recording {
    fn handle(&self) -> FrameHandle {
        FrameHandle::X11(WINDOW)
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.0.lock().unwrap().sizes.push((width, height));
    }

    fn show(&mut self) {
        self.0.lock().unwrap().shown = true;
    }

    fn poll(&mut self) -> Vec<FrameEvent> {
        let mut seen = self.0.lock().unwrap();
        if std::mem::take(&mut seen.close) {
            return vec![FrameEvent::Closed];
        }
        Vec::new()
    }
}

impl Drop for Recording {
    fn drop(&mut self) {
        self.0.lock().unwrap().dropped = true;
    }
}

fn recording(seen: &Arc<Mutex<Seen>>) -> FrameMaker {
    let seen = Arc::clone(seen);
    Box::new(move |title, width, height| {
        seen.lock().unwrap().made = Some((title.to_string(), width, height));
        Ok(Box::new(Recording(seen)) as Box<dyn EditorFrame>)
    })
}

fn run_block(processor: &mut PluginProcessor) {
    let input = vec![0.1_f32; BLOCK];
    let (mut l, mut r) = (vec![0.0; BLOCK], vec![0.0; BLOCK]);
    processor
        .process([&input, &input], [&mut l, &mut r])
        .expect("process");
}

fn closed(handle: &PluginHandle) -> Option<()> {
    (!handle.editor_open()).then_some(())
}

#[test]
fn the_editor_lives_in_its_frame_and_follows_the_plugin() {
    let PluginLoaded {
        mut processor,
        handle,
        ..
    } = load_gain();
    let seen = Arc::new(Mutex::new(Seen::default()));

    handle
        .open_editor("Gain", recording(&seen))
        .expect("editor opens");
    assert!(handle.editor_open());
    {
        let seen = seen.lock().unwrap();
        let made = seen.made.clone().expect("frame made");
        assert_eq!(made, ("Gain".into(), EDITOR_SIZE.0, EDITOR_SIZE.1));
        assert!(seen.shown);
    }

    // Linux: the descriptor wakes the editor, which sits in the window and asks for a size.
    #[cfg(target_os = "linux")]
    wait_for(|| {
        let sizes = seen.lock().unwrap().sizes.clone();
        sizes.contains(&EDITOR_RESIZED).then_some(())
    });

    // The timer turns the knob; the change arrives like any of the plugin's own.
    wait_for(|| {
        run_block(&mut processor);
        let changes = handle.take_changes();
        changes.contains(&(GAIN_PARAM, EDITOR_GAIN)).then_some(())
    });

    // The player closes the window: the editor goes, the frame with it.
    seen.lock().unwrap().close = true;
    wait_for(|| closed(&handle));
    assert!(seen.lock().unwrap().dropped);
    drop(processor);
}

#[test]
fn a_frame_that_cannot_be_made_leaves_no_editor() {
    let PluginLoaded {
        processor, handle, ..
    } = load_gain();
    let failing: FrameMaker = Box::new(|_, _, _| Err(PluginError::Editor("no display".into())));

    let opened = handle.open_editor("Gain", failing);
    assert!(matches!(opened, Err(PluginError::Editor(_))), "{opened:?}");
    assert!(!handle.editor_open());

    // The plugin is fine and opens it the next time.
    let seen = Arc::new(Mutex::new(Seen::default()));
    handle
        .open_editor("Gain", recording(&seen))
        .expect("second try");
    assert!(handle.editor_open());
    handle.close_editor();
    wait_for(|| closed(&handle));
    assert!(seen.lock().unwrap().dropped);
    drop(processor);
}

#[test]
fn closing_the_plugin_closes_its_editor_first() {
    let PluginLoaded {
        processor, handle, ..
    } = load_gain();
    let seen = Arc::new(Mutex::new(Seen::default()));
    handle
        .open_editor("Gain", recording(&seen))
        .expect("editor");

    drop(processor);
    drop(handle);
    wait_for(|| seen.lock().unwrap().dropped.then_some(()));
}

/// The real window (X11). Needs a display, so it runs only on request, under Xvfb:
/// `xvfb-run cargo test -p kontra-drivers --test plugin_editor -- --ignored`.
#[cfg(target_os = "linux")]
#[test]
#[ignore = "needs an X display"]
fn the_editor_opens_in_an_x11_window() {
    let PluginLoaded {
        processor, handle, ..
    } = load_gain();
    handle
        .open_editor("Kontra Test Gain", kontra_drivers::plugins::system_frame())
        .expect("X11 window");
    assert!(handle.editor_open());
    std::thread::sleep(Duration::from_millis(200));
    assert!(handle.editor_open(), "still open after the first ticks");
    handle.close_editor();
    wait_for(|| closed(&handle));
    drop(processor);
}
