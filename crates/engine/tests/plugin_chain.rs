//! Plugins in the effect chain (LH-EIN-19 to LH-EIN-21) with the test plugins of
//! `crates/test-clap-gain` and `crates/test-vst3-gain` (built as examples by `cargo test
//! --workspace`): the signal goes through, parameters and state apply, a plugin that is gone
//! bypasses its slot, instances close only after their chain is dropped, and the audio side
//! does not allocate.
//!
//! A counting global allocator needs `unsafe impl GlobalAlloc`; it only forwards to the system
//! allocator, so it is sound.
#![allow(unsafe_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kontra_drivers::plugins::{probe_file, PluginFormat, PluginInfo};
use kontra_dsp::Chain;
use kontra_engine::{describe_plugin, ChainBuilder, PluginBank, PluginProblems};
use kontra_model::{ChainSpec, EffectKind, PluginSpec, SlotSpec};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

const RATE: f64 = 48_000.0;
const GAIN_PARAM: u32 = 7;
/// The block the insert collects: how much the audio is late.
const DELAY: usize = 64;
/// What the chain reports: the block plus the latency the test plugin announces (it announces
/// 64 frames but does not delay the audio).
const LATENCY: usize = 64 + 64;
const PLUGIN_ID: u32 = 1;

struct Counting;

thread_local! {
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

fn allocations(work: impl FnOnce()) -> usize {
    COUNT.with(|c| c.set(0));
    ARMED.with(|a| a.set(true));
    work();
    ARMED.with(|a| a.set(false));
    COUNT.with(Cell::get)
}

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

/// The built test plugin `name`, copied where the host recognises it: `<name>.clap`, or a
/// `<name>.vst3` bundle.
fn plugin_file(format: PluginFormat, name: &str) -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let exe = std::env::current_exe().expect("test binary path");
    let profile = exe.parent().and_then(Path::parent).expect("target folder");
    let built = profile.join("examples").join(format!(
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
        "engine-{}-{}",
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
    std::fs::copy(&built, arch.join(format!("{name}.{BINARY_EXTENSION}"))).expect("copy plugin");
    bundle
}

fn found(format: PluginFormat, name: &str) -> PluginInfo {
    let file = plugin_file(format, name);
    probe_file(format, &file).expect("probe").remove(0)
}

/// The settings entry of the gain plugin with its gain parameter at `gain`.
fn gain_spec(format: PluginFormat, name: &str, gain: f64) -> PluginSpec {
    let mut spec = describe_plugin(&found(format, name)).expect("describe");
    spec.id = PLUGIN_ID;
    for p in &mut spec.params {
        if p.id == GAIN_PARAM {
            p.value = gain;
        }
    }
    spec
}

fn chain_of(slots: &[SlotSpec]) -> ChainSpec {
    slots.to_vec().into()
}

fn plugin_chain() -> ChainSpec {
    chain_of(&[SlotSpec::for_plugin(PLUGIN_ID)])
}

fn bank(specs: Vec<PluginSpec>) -> PluginBank {
    PluginBank::new(specs, PluginProblems::default())
}

fn sine(n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| 0.5 * (std::f64::consts::TAU * 110.0 * i as f64 / RATE).sin() as f32)
        .collect()
}

fn run(chain: &mut Chain, input: &[f32]) -> Vec<f32> {
    input.iter().map(|&x| chain.process(x).0).collect()
}

/// `out` is `input` times `factor`, `DELAY` frames later.
fn assert_gain(out: &[f32], input: &[f32], factor: f32) {
    for n in DELAY..out.len() {
        let want = input[n - DELAY] * factor;
        assert!(
            (out[n] - want).abs() < 1e-5,
            "frame {n}: {} != {want}",
            out[n]
        );
    }
}

/// The builder knows what the chain it built last delays the bass by, in ms.
#[test]
fn the_builder_reports_the_latency_in_ms() {
    let bank = bank(vec![gain_spec(PluginFormat::Clap, "kontra_clap_gain", 0.5)]);
    let mut builder = ChainBuilder::new(RATE);
    assert_eq!(builder.latency_ms(), 0.0, "nothing built yet");

    builder.fit(&plugin_chain(), &bank).expect("new chain");
    let want_ms = LATENCY as f64 / RATE * 1000.0;
    assert!((builder.latency_ms() - want_ms).abs() < 1e-9);

    builder.fit(&ChainSpec::EMPTY, &bank).expect("empty chain");
    assert_eq!(builder.latency_ms(), 0.0, "plugin gone");
}

fn gain_halves_the_bass(format: PluginFormat, name: &str) {
    let bank = bank(vec![gain_spec(format, name, 0.5)]);
    let mut builder = ChainBuilder::new(RATE);
    let mut chain = builder.fit(&plugin_chain(), &bank).expect("new chain");

    assert_eq!(chain.latency_frames(), LATENCY);
    let input = sine(4096);
    assert_gain(&run(&mut chain, &input), &input, 0.5);
}

#[test]
fn clap_gain_halves_the_bass() {
    gain_halves_the_bass(PluginFormat::Clap, "kontra_clap_gain");
}

#[test]
fn vst3_gain_halves_the_bass() {
    gain_halves_the_bass(PluginFormat::Vst3, "kontra_vst3_gain");
}

#[test]
fn saved_state_is_applied() {
    // The state of the test plugin is its gain as f32; the parameters list is empty.
    let mut spec = gain_spec(PluginFormat::Clap, "kontra_clap_gain", 1.0);
    spec.state = 0.25f32.to_le_bytes().to_vec();
    spec.params.clear();
    let mut builder = ChainBuilder::new(RATE);
    let mut chain = builder
        .fit(&plugin_chain(), &bank(vec![spec]))
        .expect("new chain");

    let input = sine(2048);
    assert_gain(&run(&mut chain, &input), &input, 0.25);
}

#[test]
fn describe_keeps_the_plugins_own_state() {
    let spec = gain_spec(PluginFormat::Clap, "kontra_clap_gain", 1.0);
    assert_eq!(spec.state, 1.0f32.to_le_bytes());
    assert_eq!(spec.format, "clap");
    assert!(spec
        .params
        .iter()
        .any(|p| p.id == GAIN_PARAM && p.max > p.min));
}

#[test]
fn parameter_changes_reach_the_running_plugin() {
    let mut builder = ChainBuilder::new(RATE);
    let mut chain = builder
        .fit(
            &plugin_chain(),
            &bank(vec![gain_spec(PluginFormat::Clap, "kontra_clap_gain", 1.0)]),
        )
        .expect("new chain");
    let input = sine(2048);
    assert_gain(&run(&mut chain, &input), &input, 1.0);

    // Same shape, other value: no new chain, the instance is told.
    let louder = bank(vec![gain_spec(PluginFormat::Clap, "kontra_clap_gain", 2.0)]);
    assert!(builder.fit(&plugin_chain(), &louder).is_none());
    // The change takes effect with a block after the first one that carries it.
    run(&mut chain, &input);
    let out = run(&mut chain, &input);
    assert_gain(&out, &input, 2.0);
}

#[test]
fn a_missing_plugin_bypasses_its_slot() {
    let mut spec = gain_spec(PluginFormat::Clap, "kontra_clap_gain", 0.5);
    spec.path = "/nonexistent/gone.clap".into();
    let problems = PluginProblems::default();
    let bank = PluginBank::new(vec![spec], problems.clone());
    let mut builder = ChainBuilder::new(RATE);
    // The rest of the chain still works: an overdrive at 0 passes, the plugin slot is out.
    let spec = chain_of(&[
        SlotSpec::for_plugin(PLUGIN_ID),
        SlotSpec::new(EffectKind::Widener),
    ]);
    let mut chain = builder.fit(&spec, &bank).expect("new chain");

    assert!(problems
        .get(PLUGIN_ID)
        .is_some_and(|r| r.contains("gone.clap")));
    assert_eq!(chain.latency_frames(), 0);
    let (l, r) = chain.process(0.5);
    assert!(l.is_finite() && r.is_finite());

    // A slot that names no plugin at all is bypassed as well.
    let mut builder = ChainBuilder::new(RATE);
    let mut chain = builder
        .fit(&plugin_chain(), &PluginBank::default())
        .expect("chain");
    assert_eq!(chain.process(0.5), (0.5, 0.5));
}

#[test]
fn instances_close_after_their_chain_is_gone() {
    let bank = bank(vec![gain_spec(PluginFormat::Clap, "kontra_clap_gain", 0.5)]);
    let mut builder = ChainBuilder::new(RATE);
    let first = builder.fit(&plugin_chain(), &bank).expect("first");
    assert_eq!(builder.retiring(), 0);

    // Taking the plugin out: its instance waits for the chain that runs it.
    let second = builder.fit(&ChainSpec::EMPTY, &bank).expect("second");
    assert_eq!(builder.retiring(), 1);
    builder.reap();
    assert_eq!(builder.retiring(), 1);

    drop(first);
    builder.reap();
    assert_eq!(builder.retiring(), 0);
    drop(second);
}

#[test]
fn no_allocation_in_the_callback_with_a_plugin() {
    let bank = bank(vec![gain_spec(PluginFormat::Clap, "kontra_clap_gain", 0.5)]);
    let mut builder = ChainBuilder::new(RATE);
    let mut chain = builder.fit(&plugin_chain(), &bank).expect("new chain");
    let input = sine(1000);
    run(&mut chain, &input);

    let mut out = Vec::with_capacity(input.len());
    let count = allocations(|| {
        out.extend(input.iter().map(|&x| chain.process(x).0));
    });
    assert_eq!(count, 0, "the chain allocated while a plugin ran");
}
