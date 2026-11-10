//! The CLAP host against the test plugins of `crates/test-clap-gain` (built as examples by
//! `cargo test --workspace`): scan, load, process, parameters through the ring, state, latency,
//! and no allocation on the audio thread.
//!
//! A counting global allocator needs `unsafe impl GlobalAlloc`; it only forwards to the system
//! allocator, so it is sound.
#![allow(unsafe_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kontra_drivers::plugins::{
    probe_file, LoadedPlugin, PluginError, PluginFormat, PluginProcessor, ProcessError,
};
use kontra_drivers::{read_audio, write_wav, AudioBuffer};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

const GAIN_ID: &str = "de.arianw.kontra.test.gain";
const GAIN_MONO_ID: &str = "de.arianw.kontra.test.gain-mono";
const GAIN_PARAM: u32 = 7;
const LATENCY: u32 = 64;
const RATE: f64 = 48_000.0;
const BLOCK: usize = 256;

struct Counting;

thread_local! {
    /// Allocations made by this thread while armed.
    static COUNT: Cell<usize> = const { Cell::new(0) };
    static ARMED: Cell<bool> = const { Cell::new(false) };
}

fn note() {
    // try_with: the thread-local may already be gone at thread exit.
    let _ = ARMED.try_with(|armed| {
        if armed.get() {
            let _ = COUNT.try_with(|c| c.set(c.get() + 1));
        }
    });
}

// SAFETY: every call is forwarded unchanged to `System`; counting touches only const-initialised
// thread-locals, which never allocate.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        note();
        // SAFETY: same contract as the caller's.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: same contract as the caller's.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        note();
        // SAFETY: same contract as the caller's.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        note();
        // SAFETY: same contract as the caller's.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

/// Allocations `work` makes on this thread.
fn allocations(work: impl FnOnce()) -> usize {
    COUNT.with(|c| c.set(0));
    ARMED.with(|a| a.set(true));
    work();
    ARMED.with(|a| a.set(false));
    COUNT.with(Cell::get)
}

/// The built test plugin `name` ("kontra_clap_gain"), copied to a folder of its own as
/// `<name>.clap` (the host recognises plugin files by that extension).
fn plugin_file(name: &str) -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let exe = std::env::current_exe().expect("test binary path");
    let profile = exe.parent().and_then(Path::parent).expect("target folder");
    let examples = profile.join("examples");
    let built = examples.join(format!(
        "{}{name}{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    ));
    assert!(
        built.exists(),
        "{} is missing: `cargo test --workspace` builds the test plugins",
        built.display()
    );

    // Inside the target folder (`cargo clean` removes it), not in the RAM-backed temp folder.
    let dir = profile.join("plugin-tests").join(format!(
        "clap-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("temp folder");
    let file = dir.join(format!("{name}.clap"));
    std::fs::copy(&built, &file).expect("copy plugin");
    file
}

fn gain_plugin() -> LoadedPlugin {
    LoadedPlugin::open(PluginFormat::Clap, &plugin_file("kontra_clap_gain"), None)
        .expect("load gain")
}

fn ramp() -> Vec<f32> {
    (0..BLOCK)
        .map(|i| (i as f32 / BLOCK as f32) - 0.5)
        .collect()
}

/// Runs one block, the same signal on both channels.
fn run(processor: &mut PluginProcessor, input: &[f32]) -> Result<[Vec<f32>; 2], ProcessError> {
    let (mut l, mut r) = (vec![0.0; input.len()], vec![0.0; input.len()]);
    processor.process([input, input], [&mut l, &mut r])?;
    Ok([l, r])
}

fn assert_scaled(out: &[f32], input: &[f32], factor: f32) {
    assert_eq!(out.len(), input.len());
    for (o, i) in out.iter().zip(input) {
        assert!((o - i * factor).abs() < 1e-6, "{o} != {i} * {factor}");
    }
}

#[test]
fn scan_finds_the_plugin() {
    let file = plugin_file("kontra_clap_gain");
    let found = probe_file(PluginFormat::Clap, &file).expect("probe");

    assert_eq!(found.len(), 1);
    let info = &found[0];
    assert_eq!(info.id, GAIN_ID);
    assert_eq!(info.name, "Kontra Test Gain");
    assert_eq!(info.vendor, "Kontra Tests");
    assert_eq!(info.path, file);
    assert_eq!(
        (info.ports.inputs.clone(), info.ports.outputs.clone()),
        (vec![2], vec![2])
    );
}

#[test]
fn loads_and_lists_params() {
    let mut plugin = gain_plugin();
    assert_eq!(plugin.info().id, GAIN_ID);

    let params = plugin.params();
    assert_eq!(params.len(), 1);
    let gain = &params[0];
    assert_eq!((gain.id, gain.name.as_str()), (GAIN_PARAM, "gain"));
    assert_eq!(
        (gain.min, gain.max, gain.default, gain.current),
        (0.0, 4.0, 1.0, 1.0)
    );
}

#[test]
fn wrong_id_and_unknown_param_are_errors() {
    let file = plugin_file("kontra_clap_gain");
    let err = LoadedPlugin::open(PluginFormat::Clap, &file, Some("nope")).unwrap_err();
    assert!(matches!(err, PluginError::NotFound { .. }));

    let mut plugin = LoadedPlugin::load(&probe_file(PluginFormat::Clap, &file).unwrap()[0])
        .expect("load by info");
    assert!(matches!(
        plugin.set_param(99, 1.0),
        Err(PluginError::NoParam(99))
    ));
}

#[test]
fn halves_the_level_at_gain_half() {
    let mut plugin = gain_plugin();
    let mut processor = plugin.activate(RATE, BLOCK as u32).expect("activate");
    let input = ramp();

    // Gain 1 first: the signal passes unchanged.
    let [l, r] = run(&mut processor, &input).expect("process");
    assert_scaled(&l, &input, 1.0);
    assert_scaled(&r, &input, 1.0);

    // The change goes through the ring and is there with the next block.
    plugin.set_param(GAIN_PARAM, 0.5).expect("set");
    let [l, r] = run(&mut processor, &input).expect("process");
    assert_scaled(&l, &input, 0.5);
    assert_scaled(&r, &input, 0.5);

    // Out of range is clamped to the maximum of 4.
    plugin.set_param(GAIN_PARAM, 100.0).expect("set");
    let [l, _] = run(&mut processor, &input).expect("process");
    assert_scaled(&l, &input, 4.0);
}

#[test]
fn param_set_before_activation_holds() {
    let mut plugin = gain_plugin();
    plugin.set_param(GAIN_PARAM, 0.25).expect("set");
    assert_eq!(plugin.params()[0].current, 0.25);

    let mut processor = plugin.activate(RATE, BLOCK as u32).expect("activate");
    let input = ramp();
    let [l, _] = run(&mut processor, &input).expect("process");
    assert_scaled(&l, &input, 0.25);
}

#[test]
fn blocks_beyond_the_maximum_are_cut() {
    let mut plugin = gain_plugin();
    let mut processor = plugin.activate(RATE, 100).expect("activate");
    plugin.set_param(GAIN_PARAM, 0.5).expect("set");

    let input: Vec<f32> = (0..1000).map(|i| i as f32 * 0.001).collect();
    let [l, r] = run(&mut processor, &input).expect("process");
    assert_scaled(&l, &input, 0.5);
    assert_scaled(&r, &input, 0.5);
}

#[test]
fn state_round_trip() {
    let mut plugin = gain_plugin();
    plugin.set_param(GAIN_PARAM, 0.3).expect("set");
    let state = plugin.save_state().expect("save");
    assert_eq!(state, 0.3_f32.to_le_bytes());

    let mut other = gain_plugin();
    assert_eq!(other.params()[0].current, 1.0);
    other.load_state(&state).expect("load");
    assert!((other.params()[0].current - 0.3).abs() < 1e-6);

    // A truncated state is refused, not half applied.
    assert!(matches!(
        other.load_state(&[1, 2]),
        Err(PluginError::State(_))
    ));
}

#[test]
fn reports_latency() {
    assert_eq!(gain_plugin().latency_samples(), LATENCY);
}

#[test]
fn mono_port_gets_the_mean_and_answers_on_both_channels() {
    let file = plugin_file("kontra_clap_gain_mono");
    let found = probe_file(PluginFormat::Clap, &file).expect("probe");
    assert_eq!(found[0].id, GAIN_MONO_ID);
    assert_eq!(found[0].ports.inputs, vec![1]);

    let mut plugin = LoadedPlugin::open(PluginFormat::Clap, &file, None).expect("load");
    let mut processor = plugin.activate(RATE, BLOCK as u32).expect("activate");
    plugin.set_param(GAIN_PARAM, 0.5).expect("set");

    let left = vec![0.8_f32; BLOCK];
    let right = vec![0.2_f32; BLOCK];
    let (mut out_l, mut out_r) = (vec![0.0; BLOCK], vec![0.0; BLOCK]);
    processor
        .process([&left, &right], [&mut out_l, &mut out_r])
        .expect("process");

    // Mean 0.5, halved.
    assert!(out_l.iter().chain(&out_r).all(|s| (s - 0.25).abs() < 1e-6));
}

#[test]
fn deactivate_needs_the_processor_gone() {
    let mut plugin = gain_plugin();
    let processor = plugin.activate(RATE, BLOCK as u32).expect("activate");
    assert!(matches!(plugin.deactivate(), Err(PluginError::InUse)));

    drop(processor);
    plugin.deactivate().expect("deactivate");
    // And it can run again, with another block size.
    let mut processor = plugin.activate(RATE, 64).expect("activate again");
    let input = vec![0.5_f32; 64];
    assert_scaled(
        &run(&mut processor, &input).expect("process")[0],
        &input,
        1.0,
    );
}

#[test]
fn process_does_not_allocate() {
    let mut plugin = gain_plugin();
    let mut processor = plugin.activate(RATE, BLOCK as u32).expect("activate");
    let input = ramp();
    let (mut l, mut r) = (vec![0.0; BLOCK], vec![0.0; BLOCK]);

    // The first block starts the plugin (the plugin may set itself up); then it is steady.
    processor
        .process([&input, &input], [&mut l, &mut r])
        .expect("warm up");
    // The counter itself works.
    assert!(allocations(|| drop(vec![1_u8; 16])) >= 1);

    let blocks = allocations(|| {
        for i in 0..50 {
            // A parameter change every few blocks, through the ring.
            if i % 5 == 0 {
                plugin
                    .set_param(GAIN_PARAM, 0.5 + f64::from(i) * 0.01)
                    .expect("set");
            }
            processor
                .process([&input, &input], [&mut l, &mut r])
                .expect("process");
        }
    });
    assert_eq!(blocks, 0, "process() or set_param() allocated");
}

#[test]
fn renders_a_file() {
    let file = plugin_file("kontra_clap_gain");
    let dir = file.parent().expect("folder");
    let (input, output) = (dir.join("in.wav"), dir.join("out.wav"));
    let samples: Vec<f32> = (0..2000).map(|i| (i as f32 * 0.01).sin() * 0.8).collect();
    write_wav(
        &input,
        &AudioBuffer {
            sample_rate: 44_100,
            channels: vec![samples.clone()],
        },
    )
    .expect("write input");

    let mut plugin = LoadedPlugin::open(PluginFormat::Clap, &file, None).expect("load");
    plugin
        .render_file(&input, &output, &[(GAIN_PARAM, 0.5)])
        .expect("render");

    let out = read_audio(&output).expect("read output");
    assert_eq!(out.sample_rate, 44_100);
    assert_eq!(out.channels.len(), 2);
    assert_scaled(&out.channels[0], &samples, 0.5);
    assert_scaled(&out.channels[1], &samples, 0.5);
}
