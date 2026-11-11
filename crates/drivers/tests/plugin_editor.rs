//! A plugin's own editor on the plugin thread (FX5), CLAP and VST 3, with the test gain plugins and a frame
//! that only records: no window system needed. The plugin's editor registers a timer and (Linux)
//! a descriptor, asks for a size once it sits in a window, and turns its knob on a timer tick
//! (`crates/test-clap-gain/src/editor.rs`).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kontra_drivers::plugins::{
    probe_file, EditorFrame, FrameEvent, FrameHandle, FrameMaker, PluginError, PluginFormat,
    PluginHandle, PluginHost, PluginLoaded, PluginProcessor, PluginSetup,
};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const GAIN_PARAM: u32 = 7;
/// `kontra_clap_gain::EDITOR_*` and `kontra_vst3_gain::EDITOR_*` (the same values).
const EDITOR_SIZE: (u32, u32) = (300, 200);
// Asked for once a descriptor wakes the editor: Linux only.
#[cfg(target_os = "linux")]
const EDITOR_RESIZED: (u32, u32) = (320, 240);
const EDITOR_GAIN: f64 = 2.5;
const EDITOR_CLOSE_GAIN: f64 = 3.25;
const EDITOR_MAX_WIDTH: u32 = 400;
/// `kontra_clap_gain::EDITOR_SYSTEM_SCALE`: the CLAP editor's own scale where the host sets none.
#[cfg(windows)]
const EDITOR_SYSTEM_SCALE: f64 = 1.5;
const RATE: f64 = 48_000.0;
const BLOCK: usize = 256;
const WINDOW: u32 = 0x4242;
/// How long the plugin thread may take for what the tests wait for.
const PATIENCE: Duration = Duration::from_secs(5);
const BOTH: [PluginFormat; 2] = [PluginFormat::Clap, PluginFormat::Vst3];

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const ARCH_DIR: &str = "x86_64-linux";
#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
const ARCH_DIR: &str = "aarch64-linux";
#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
const ARCH_DIR: &str = "x86_64-win";
#[cfg(all(target_os = "windows", target_arch = "aarch64"))]
const ARCH_DIR: &str = "arm64-win";
#[cfg(windows)]
const BINARY_EXTENSION: &str = "vst3";
#[cfg(not(windows))]
const BINARY_EXTENSION: &str = "so";

/// The test gain plugin of `format`, copied where the host recognises it: `<name>.clap`, or a
/// `<name>.vst3` bundle; a copy of its own per call (a loaded file must not be overwritten).
fn gain_file(format: PluginFormat) -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let name = match format {
        PluginFormat::Clap => "kontra_clap_gain",
        PluginFormat::Vst3 => "kontra_vst3_gain",
    };
    let exe = std::env::current_exe().expect("test binary path");
    let profile = exe.parent().and_then(Path::parent).expect("target folder");
    let built = profile.join("examples").join(format!(
        "{}{name}{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    ));
    assert!(built.exists(), "{} is missing", built.display());
    let dir = profile.join("plugin-tests").join(format!(
        "editor-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    if format == PluginFormat::Clap {
        std::fs::create_dir_all(&dir).expect("folder");
        let file = dir.join(format!("{name}.clap"));
        std::fs::copy(&built, &file).expect("copy plugin");
        return file;
    }
    let bundle = dir.join(format!("{name}.vst3"));
    let arch = bundle.join("Contents").join(ARCH_DIR);
    std::fs::create_dir_all(&arch).expect("bundle folder");
    std::fs::copy(&built, arch.join(format!("{name}.{BINARY_EXTENSION}"))).expect("copy");
    bundle
}

fn load_gain(format: PluginFormat) -> PluginLoaded {
    let path = gain_file(format);
    let id = probe_file(format, &path).expect("probe").remove(0).id;
    let setup = PluginSetup {
        format,
        path,
        id,
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
    /// Set by the test: the player drags the window to this size.
    drag: Option<(u32, u32)>,
    dropped: bool,
}

struct Recording(Arc<Mutex<Seen>>);

impl EditorFrame for Recording {
    fn handle(&self) -> FrameHandle {
        #[cfg(windows)]
        return FrameHandle::Win32(WINDOW as isize);
        #[cfg(not(windows))]
        return FrameHandle::X11(WINDOW);
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
        seen.drag
            .take()
            .map(|(width, height)| FrameEvent::Resized { width, height })
            .into_iter()
            .collect()
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

/// The size an editor opens with. Review #280: Win32 sizes follow the system's scale, which a
/// CLAP editor reads itself; Kontra must not override it with 1:1 (a 150 % screen drew a third
/// too small). X11 has no scale to read: there Kontra says 1:1.
fn first_size(format: PluginFormat) -> (u32, u32) {
    #[cfg(windows)]
    if format == PluginFormat::Clap {
        let scaled = |v: u32| (f64::from(v) * EDITOR_SYSTEM_SCALE).round() as u32;
        return (scaled(EDITOR_SIZE.0), scaled(EDITOR_SIZE.1));
    }
    let _ = format;
    EDITOR_SIZE
}

#[test]
fn the_editor_lives_in_its_frame_and_follows_the_plugin() {
    for format in BOTH {
        lives_in_its_frame(format);
    }
}

fn lives_in_its_frame(format: PluginFormat) {
    let PluginLoaded {
        mut processor,
        handle,
        ..
    } = load_gain(format);
    let seen = Arc::new(Mutex::new(Seen::default()));

    handle
        .open_editor("Gain", recording(&seen))
        .expect("editor opens");
    assert!(handle.editor_open());
    {
        let seen = seen.lock().unwrap();
        let made = seen.made.clone().expect("frame made");
        let (width, height) = first_size(format);
        assert_eq!(made, ("Gain".into(), width, height));
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

/// Review #275: the player's size goes to the plugin; one it refuses snaps back to its own.
#[test]
fn the_player_resizes_within_what_the_editor_takes() {
    let PluginLoaded {
        processor, handle, ..
    } = load_gain(PluginFormat::Clap);
    let seen = Arc::new(Mutex::new(Seen::default()));
    handle
        .open_editor("Gain", recording(&seen))
        .expect("editor");

    let taken = (EDITOR_MAX_WIDTH - 50, 260);
    seen.lock().unwrap().drag = Some(taken);
    wait_for(|| seen.lock().unwrap().drag.is_none().then_some(()));
    seen.lock().unwrap().drag = Some((EDITOR_MAX_WIDTH + 100, 300));
    wait_for(|| {
        let sizes = seen.lock().unwrap().sizes.clone();
        sizes.contains(&taken).then_some(())
    });

    handle.close_editor();
    wait_for(|| closed(&handle));
    drop(processor);
}

/// FX4: values with the plugin's own text and unit, for the values the host set and the ones the
/// plugin changed itself ("1.00 x": the test plugins' gain).
#[test]
fn values_come_with_the_plugins_text() {
    for format in BOTH {
        texts_follow(format);
    }
}

fn texts_follow(format: PluginFormat) {
    let PluginLoaded {
        mut processor,
        handle,
        ..
    } = load_gain(format);
    let mut texts = Vec::new();
    let mut seen = |want: (u32, f64, &str), processor: &mut PluginProcessor| {
        wait_for(|| {
            run_block(processor);
            let _ = handle.take_changes();
            texts.extend(handle.take_texts());
            let found = texts
                .iter()
                .any(|(id, value, text)| (*id, *value, text.as_str()) == want);
            found.then_some(())
        });
    };
    seen((GAIN_PARAM, 1.0, "1.00 x"), &mut processor);
    handle.set_param(GAIN_PARAM, 2.0);
    seen((GAIN_PARAM, 2.0, "2.00 x"), &mut processor);
    drop(processor);
}

/// VST 3: the view's size constraint fits the player's size; the window follows.
#[test]
fn a_vst3_view_fits_the_players_size() {
    let PluginLoaded {
        processor, handle, ..
    } = load_gain(PluginFormat::Vst3);
    let seen = Arc::new(Mutex::new(Seen::default()));
    handle
        .open_editor("Gain", recording(&seen))
        .expect("editor");

    seen.lock().unwrap().drag = Some((EDITOR_MAX_WIDTH + 100, 300));
    wait_for(|| {
        let sizes = seen.lock().unwrap().sizes.clone();
        sizes.contains(&(EDITOR_MAX_WIDTH, 300)).then_some(())
    });

    handle.close_editor();
    wait_for(|| closed(&handle));
    drop(processor);
}

/// Review #275: an editor that closes itself (its own close button) is gone for the host too.
#[test]
fn the_editor_may_close_itself() {
    let PluginLoaded {
        mut processor,
        handle,
        ..
    } = load_gain(PluginFormat::Clap);
    let seen = Arc::new(Mutex::new(Seen::default()));
    handle
        .open_editor("Gain", recording(&seen))
        .expect("editor");

    handle.set_param(GAIN_PARAM, EDITOR_CLOSE_GAIN);
    wait_for(|| {
        run_block(&mut processor);
        closed(&handle)
    });
    assert!(seen.lock().unwrap().dropped);
    drop(processor);
}

#[test]
fn a_frame_that_cannot_be_made_leaves_no_editor() {
    for format in BOTH {
        no_frame_no_editor(format);
    }
}

fn no_frame_no_editor(format: PluginFormat) {
    let PluginLoaded {
        processor, handle, ..
    } = load_gain(format);
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
    for format in BOTH {
        closes_with_the_plugin(format);
    }
}

fn closes_with_the_plugin(format: PluginFormat) {
    let PluginLoaded {
        processor, handle, ..
    } = load_gain(format);
    let seen = Arc::new(Mutex::new(Seen::default()));
    handle
        .open_editor("Gain", recording(&seen))
        .expect("editor");

    drop(processor);
    drop(handle);
    wait_for(|| seen.lock().unwrap().dropped.then_some(()));
}

/// The real window of the system (X11, Win32). Needs a display, so it runs only on request:
/// `xvfb-run cargo test -p kontra-drivers --test plugin_editor -- --ignored` (Linux), the same
/// without `xvfb-run` under Windows.
#[cfg(any(target_os = "linux", windows))]
#[test]
#[ignore = "needs a display"]
fn the_editor_opens_in_a_system_window() {
    for format in BOTH {
        opens_in_system_window(format);
    }
}

#[cfg(any(target_os = "linux", windows))]
fn opens_in_system_window(format: PluginFormat) {
    let PluginLoaded {
        processor, handle, ..
    } = load_gain(format);
    handle
        .open_editor("Kontra Test Gain", kontra_drivers::plugins::system_frame())
        .expect("system window");
    assert!(handle.editor_open());
    std::thread::sleep(Duration::from_millis(200));
    assert!(handle.editor_open(), "still open after the first ticks");
    handle.close_editor();
    wait_for(|| closed(&handle));
    drop(processor);
}
