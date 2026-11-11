//! Plugins in the effect chain (LH-EIN-19 to LH-EIN-21) with the test plugins of
//! `crates/test-clap-gain` and `crates/test-vst3-gain` (built as examples by `cargo test
//! --workspace`): the signal goes through, parameters and state apply, a plugin that is gone
//! bypasses its slot, instances close only after their chain is dropped, and the audio side
//! does not allocate.
//!
//! A counting global allocator needs `unsafe impl GlobalAlloc`; it only forwards to the system
//! allocator, so it is sound.
#![allow(unsafe_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kontra_drivers::plugins::{probe_file, PluginFormat, PluginHost, PluginInfo};
use kontra_dsp::Chain;
use kontra_engine::{describe_plugin, ChainBuilder, PluginBank, PluginProblems};
use kontra_model::{ChainSpec, EffectKind, PluginSpec, SlotSpec};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

const RATE: f64 = 48_000.0;
const GAIN_PARAM: u32 = 7;
/// The block the insert collects: how much the audio is late.
const DELAY: usize = 64;
/// What the chain reports: the block plus the latency the test plugin announces (it announces
/// 64 frames but does not delay the audio).
const LATENCY: usize = 64 + 64;
const PLUGIN_ID: u32 = 1;
/// How long a test waits for the plugin thread before it gives up.
const PATIENCE: Duration = Duration::from_secs(10);
const POLL_EVERY: Duration = Duration::from_millis(2);

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
    let mut spec = describe_plugin(&found(format, name), None).expect("describe");
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

/// Polls the builder until the chain of a pending `fit` is ready.
fn await_chain(builder: &mut ChainBuilder) -> Box<Chain> {
    let start = Instant::now();
    loop {
        if let Some(chain) = builder.poll() {
            return chain;
        }
        assert!(start.elapsed() < PATIENCE, "the plugins did not load");
        std::thread::sleep(POLL_EVERY);
    }
}

/// `fit`, then waits for the plugins to load.
fn fit_ready(builder: &mut ChainBuilder, spec: &ChainSpec, bank: &PluginBank) -> Box<Chain> {
    match builder.fit(spec, bank) {
        Some(chain) => chain,
        None => await_chain(builder),
    }
}

/// Runs blocks until `done` is true of a block's output, or gives up: parameters and changes
/// travel through the plugin thread, so they arrive a moment after the call.
fn run_until(chain: &mut Chain, input: &[f32], done: impl Fn(&[f32]) -> bool) -> Vec<f32> {
    let start = Instant::now();
    loop {
        let out = run(chain, input);
        if done(&out) {
            return out;
        }
        assert!(start.elapsed() < PATIENCE, "the plugin did not follow");
        std::thread::sleep(POLL_EVERY);
    }
}

/// Runs blocks until the plugin's own change comes out of the builder.
fn await_changes(
    builder: &mut ChainBuilder,
    chain: &mut Chain,
    input: &[f32],
) -> Vec<(u32, u32, f64)> {
    let start = Instant::now();
    loop {
        run(chain, input);
        let changes = builder.take_param_changes();
        if !changes.is_empty() {
            return changes;
        }
        assert!(start.elapsed() < PATIENCE, "no change reported");
        std::thread::sleep(POLL_EVERY);
    }
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

    fit_ready(&mut builder, &plugin_chain(), &bank);
    let want_ms = LATENCY as f64 / RATE * 1000.0;
    assert!((builder.latency_ms() - want_ms).abs() < 1e-9);

    builder.fit(&ChainSpec::EMPTY, &bank).expect("empty chain");
    assert_eq!(builder.latency_ms(), 0.0, "plugin gone");
}

fn gain_halves_the_bass(format: PluginFormat, name: &str) {
    let bank = bank(vec![gain_spec(format, name, 0.5)]);
    let mut builder = ChainBuilder::new(RATE);
    let mut chain = fit_ready(&mut builder, &plugin_chain(), &bank);

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
    let mut chain = fit_ready(&mut builder, &plugin_chain(), &bank(vec![spec]));

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
    let mut chain = fit_ready(
        &mut builder,
        &plugin_chain(),
        &bank(vec![gain_spec(PluginFormat::Clap, "kontra_clap_gain", 1.0)]),
    );
    let input = sine(2048);
    assert_gain(&run(&mut chain, &input), &input, 1.0);

    // Same shape, other value: no new chain, the instance is told (through the plugin thread,
    // so the audio follows within a few blocks).
    let louder = bank(vec![gain_spec(PluginFormat::Clap, "kontra_clap_gain", 2.0)]);
    assert!(builder.fit(&plugin_chain(), &louder).is_none());
    run_until(&mut chain, &input, |out| {
        let at = out.len() - 1;
        (out[at] - input[at - DELAY] * 2.0).abs() < 1e-5
    });
    // The change may land within that block (review #271: "frame 65: 0.0072 != 0.0144"); the
    // next one is louder from its first frame.
    assert_gain(&run(&mut chain, &input), &input, 2.0);
}

/// Review #271: while a chain of a new shape loads, the running one follows the sliders.
#[test]
fn the_running_chain_follows_while_another_loads() {
    let mut builder = ChainBuilder::new(RATE);
    let quiet = bank(vec![gain_spec(PluginFormat::Clap, "kontra_clap_gain", 1.0)]);
    let mut chain = fit_ready(&mut builder, &plugin_chain(), &quiet);
    let input = sine(2048);

    // A second plugin joins: the new shape loads it (and is never polled here, so it stays
    // loading).
    let mut second = gain_spec(PluginFormat::Clap, "kontra_clap_gain", 1.0);
    second.id = PLUGIN_ID + 1;
    let bigger = chain_of(&[
        SlotSpec::for_plugin(PLUGIN_ID),
        SlotSpec::for_plugin(PLUGIN_ID + 1),
    ]);
    let both = |gain| {
        bank(vec![
            gain_spec(PluginFormat::Clap, "kontra_clap_gain", gain),
            second.clone(),
        ])
    };
    assert!(builder.fit(&bigger, &both(1.0)).is_none());
    assert!(builder.fit(&bigger, &both(2.0)).is_none());

    run_until(&mut chain, &input, |out| {
        let at = out.len() - 1;
        (out[at] - input[at - DELAY] * 2.0).abs() < 1e-5
    });
    assert_gain(&run(&mut chain, &input), &input, 2.0);
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
    let mut chain = fit_ready(&mut builder, &spec, &bank);

    assert!(problems
        .get(PLUGIN_ID)
        .is_some_and(|r| r.contains("gone.clap")));
    assert_eq!(chain.latency_frames(), 0);
    let (l, r) = chain.process(0.5);
    assert!(l.is_finite() && r.is_finite());

    // A slot that names no plugin at all is bypassed as well.
    let mut builder = ChainBuilder::new(RATE);
    let mut chain = fit_ready(&mut builder, &plugin_chain(), &PluginBank::default());
    assert_eq!(chain.process(0.5), (0.5, 0.5));
}

#[test]
fn instances_close_after_their_chain_is_gone() {
    let bank = bank(vec![gain_spec(PluginFormat::Clap, "kontra_clap_gain", 0.5)]);
    let mut builder = ChainBuilder::new(RATE);
    let first = fit_ready(&mut builder, &plugin_chain(), &bank);
    assert_eq!(builder.retiring(), 0);

    // Switching the plugin off keeps its instance (the slot is still in the settings).
    let second = builder.fit(&ChainSpec::EMPTY, &bank).expect("second");
    assert_eq!(builder.retiring(), 0);

    // Taking the slot out of the settings: its instance waits for the chain that runs it.
    assert!(builder
        .fit(&ChainSpec::EMPTY, &PluginBank::default())
        .is_none());
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
    let mut chain = fit_ready(&mut builder, &plugin_chain(), &bank);
    let input = sine(1000);
    run(&mut chain, &input);

    let mut out = Vec::with_capacity(input.len());
    let count = allocations(|| {
        out.extend(input.iter().map(|&x| chain.process(x).0));
    });
    assert_eq!(count, 0, "the chain allocated while a plugin ran");
}

/// The value the test plugins move their gain to by themselves, and the value that makes them.
const TRIGGER: f64 = 3.5;
const REPORTED: f64 = 0.5;

/// Runs blocks until `want` changes came out of the builder, or gives up.
fn changes_after_trigger(format: PluginFormat, name: &str) -> Vec<(u32, u32, f64)> {
    let mut builder = ChainBuilder::new(RATE);
    let mut chain = fit_ready(
        &mut builder,
        &plugin_chain(),
        &bank(vec![gain_spec(format, name, 1.0)]),
    );
    let input = sine(1024);
    run(&mut chain, &input);
    assert!(
        builder.take_param_changes().is_empty(),
        "nothing changed yet"
    );

    // The plugin answers the trigger with a change of its own.
    let trigger = bank(vec![gain_spec(format, name, TRIGGER)]);
    assert!(builder.fit(&plugin_chain(), &trigger).is_none());
    let changes = await_changes(&mut builder, &mut chain, &input);
    assert!(
        builder.take_param_changes().is_empty(),
        "a change comes out once"
    );
    changes
}

#[test]
fn a_clap_plugin_reports_its_own_change() {
    let changes = changes_after_trigger(PluginFormat::Clap, "kontra_clap_gain");
    assert_eq!(changes, [(PLUGIN_ID, GAIN_PARAM, REPORTED)]);
}

#[test]
fn a_vst3_plugin_reports_its_own_change() {
    let changes = changes_after_trigger(PluginFormat::Vst3, "kontra_vst3_gain");
    assert_eq!(changes.len(), 1, "{changes:?}");
    let (plugin, param, value) = changes[0];
    assert_eq!((plugin, param), (PLUGIN_ID, GAIN_PARAM));
    assert!((value - REPORTED).abs() < 1e-9, "{value}");
}

/// A change the plugin reported is its value now: the same value from the settings is not
/// sent back, and the plugin keeps it (here the processor runs it as the gain).
#[test]
fn a_reported_change_is_not_echoed_and_the_processor_follows() {
    for (format, name) in [
        (PluginFormat::Clap, "kontra_clap_gain"),
        (PluginFormat::Vst3, "kontra_vst3_gain"),
    ] {
        let mut builder = ChainBuilder::new(RATE);
        let mut chain = fit_ready(
            &mut builder,
            &plugin_chain(),
            &bank(vec![gain_spec(format, name, TRIGGER)]),
        );
        let input = sine(2048);
        let changes = await_changes(&mut builder, &mut chain, &input);
        assert_eq!(changes.len(), 1, "{name}: {changes:?}");

        // The settings took the plugin's value: same shape, no new chain, nothing to send.
        let same = bank(vec![gain_spec(format, name, changes[0].2)]);
        assert!(builder.fit(&plugin_chain(), &same).is_none());
        run(&mut chain, &input);
        assert_gain(&run(&mut chain, &input), &input, REPORTED as f32);
        assert!(builder.take_param_changes().is_empty(), "{name}: no echo");
    }
}

/// The plugin reports from inside the callback: that path must not allocate either. The value
/// that makes it report is set while loading, so the first block reports.
#[test]
fn no_allocation_while_a_plugin_reports_changes() {
    let mut builder = ChainBuilder::new(RATE);
    let trigger = bank(vec![gain_spec(
        PluginFormat::Clap,
        "kontra_clap_gain",
        TRIGGER,
    )]);
    let mut chain = fit_ready(&mut builder, &plugin_chain(), &trigger);
    let input = sine(1000);

    let mut out = Vec::with_capacity(input.len());
    let count = allocations(|| {
        out.extend(input.iter().map(|&x| chain.process(x).0));
    });
    assert_eq!(count, 0, "the chain allocated while a plugin reported");
    let changes = await_changes(&mut builder, &mut chain, &input);
    assert_eq!(changes.len(), 1);
}

/// Loading happens on the plugin thread: `fit` returns before the plugin is there, the chain
/// comes with `poll`, and a chain without plugins needs no wait.
#[test]
fn loading_does_not_block_the_control_thread() {
    let bank = bank(vec![gain_spec(PluginFormat::Clap, "kontra_clap_gain", 0.5)]);
    let mut builder = ChainBuilder::new(RATE);

    assert!(
        builder.fit(&plugin_chain(), &bank).is_none(),
        "still loading"
    );
    assert!(builder.is_loading());
    assert_eq!(
        builder.latency_ms(),
        0.0,
        "no chain yet: the bass passes dry"
    );

    let mut chain = await_chain(&mut builder);
    assert!(!builder.is_loading());
    assert!(builder.poll().is_none(), "one chain per shape");
    let input = sine(2048);
    assert_gain(&run(&mut chain, &input), &input, 0.5);

    // Built-in effects only: at once, as before.
    let plain = chain_of(&[SlotSpec::new(EffectKind::Widener)]);
    assert!(builder.fit(&plain, &bank).is_some());
    assert!(!builder.is_loading());
}

/// A shape that changes while the plugins load drops those loads; their instances close, and
/// the same shape twice loads once. The instance of a slot stays through shape changes.
#[test]
fn quick_shape_changes_do_not_leak_instances() {
    let spec = gain_spec(PluginFormat::Clap, "kontra_clap_gain", 0.5);
    let path = spec.path.clone();
    let bank = bank(vec![spec]);
    let host = PluginHost::global();
    let mut builder = ChainBuilder::new(RATE);

    // The same shape again while loading: the loads go on, no second instance.
    assert!(builder.fit(&plugin_chain(), &bank).is_none());
    assert!(builder.fit(&plugin_chain(), &bank).is_none());
    let chain = await_chain(&mut builder);
    assert_eq!(host.open_of(&path), 1);

    // Shape changes, and changes again before the answer: the one instance is kept, no load.
    let widener = chain_of(&[SlotSpec::new(EffectKind::Widener)]);
    assert!(builder.fit(&ChainSpec::EMPTY, &bank).is_some());
    assert!(builder.fit(&plugin_chain(), &bank).is_none());
    assert!(builder.fit(&plugin_chain(), &bank).is_none());
    assert!(builder.fit(&widener, &bank).is_some(), "built at once");
    assert!(!builder.is_loading());

    // The chains that fade out go back; the slot leaves the settings and its instance closes.
    drop(chain);
    builder.reap();
    assert_eq!(builder.retiring(), 0);
    assert_eq!(host.open_of(&path), 1, "kept while the slot exists");
    assert!(builder.fit(&widener, &PluginBank::default()).is_none());
    builder.reap();
    let start = Instant::now();
    while host.open_of(&path) > 0 {
        assert!(start.elapsed() < PATIENCE, "an instance leaked");
        std::thread::sleep(POLL_EVERY);
    }
}

/// The crash marker is only there while a plugin's code runs: after a describe, a load and a
/// load that fails, none is left (one left behind would switch the plugin off at the next start).
#[test]
fn no_crash_marker_is_left_behind() {
    let dir = plugin_file(PluginFormat::Clap, "kontra_clap_gain");
    let marker = dir.with_file_name("plugin-loading.marker");
    let info = found(PluginFormat::Clap, "kontra_clap_gain");

    describe_plugin(&info, Some(&marker)).expect("describe");
    assert!(!marker.exists(), "after a describe");

    let good = gain_spec(PluginFormat::Clap, "kontra_clap_gain", 0.5);
    let mut gone = good.clone();
    gone.path = "/nonexistent/gone.clap".into();
    for spec in [good, gone] {
        let bank = bank(vec![spec]).with_crash_marker(marker.clone());
        let mut builder = ChainBuilder::new(RATE);
        fit_ready(&mut builder, &plugin_chain(), &bank);
        assert!(!marker.exists(), "after a load");
    }
}

/// Review #271: Kontra ended while a plugin loaded; the marker stayed and the plugin counted as
/// crashed at the next start, and instances were not closed before the program ended.
#[test]
fn settling_finishes_loads_and_closes() {
    let dir = plugin_file(PluginFormat::Clap, "kontra_clap_gain");
    let marker = dir.with_file_name("plugin-settle.marker");
    let spec = gain_spec(PluginFormat::Clap, "kontra_clap_gain", 0.5);
    let path = spec.path.clone();
    let bank = bank(vec![spec]).with_crash_marker(marker.clone());

    // A load starts, then its builder goes, as when Kontra ends.
    let mut builder = ChainBuilder::new(RATE);
    assert!(builder.fit(&plugin_chain(), &bank).is_none());
    drop(builder);

    assert!(kontra_drivers::plugins::settle_plugins(PATIENCE));
    assert!(!marker.exists(), "the load finished");
    assert_eq!(
        PluginHost::global().open_of(&path),
        0,
        "its instance closed"
    );
}

// Instances outlive chains (FX5, step 3): one per plugin slot of the settings, whatever the shape.

/// A widener that passes the signal untouched (mix 0): a second effect to reorder against.
fn quiet_widener() -> SlotSpec {
    SlotSpec {
        mix: 0.0,
        ..SlotSpec::new(EffectKind::Widener)
    }
}

fn plugin_then_widener() -> ChainSpec {
    chain_of(&[SlotSpec::for_plugin(PLUGIN_ID), quiet_widener()])
}

fn widener_then_plugin() -> ChainSpec {
    chain_of(&[quiet_widener(), SlotSpec::for_plugin(PLUGIN_ID)])
}

/// The bank a settings page would hold after the plugin changed its gain by itself: without
/// parameters and state, so only a kept instance still has the gain (a fresh one starts at 1).
fn bare_bank(spec: &PluginSpec) -> PluginBank {
    let mut bare = spec.clone();
    bare.params.clear();
    bare.state.clear();
    bank(vec![bare])
}

/// What the audio thread does with the chains of a shape change: it takes each chain the
/// builder hands over and gives the old one back (here: drops it). `watch` names a plugin file
/// of which exactly one instance must be open at every step. Returns the running chain and how
/// often the running chain allocated while blocks played through it during the change.
fn switch(
    builder: &mut ChainBuilder,
    mut running: Box<Chain>,
    to: (&ChainSpec, &PluginBank),
    watch: Option<&Path>,
) -> (Box<Chain>, usize) {
    let input = sine(256);
    let mut allocated = 0;
    let start = Instant::now();
    let mut next = builder.fit(to.0, to.1);
    loop {
        if let Some(chain) = next.take() {
            drop(std::mem::replace(&mut running, chain));
            builder.reap();
        }
        if let Some(path) = watch {
            assert_eq!(PluginHost::global().open_of(path), 1, "one instance");
        }
        if !builder.is_loading() {
            return (running, allocated);
        }
        allocated += allocations(|| {
            for &x in &input {
                running.process(x);
            }
        });
        assert!(start.elapsed() < PATIENCE, "the chain did not come");
        std::thread::sleep(POLL_EVERY);
        next = builder.poll();
    }
}

/// A plugin that set its own gain (0.5) in a running chain of `first`, and the bank that no
/// longer knows the value.
fn started(
    format: PluginFormat,
    name: &str,
    first: &ChainSpec,
) -> (ChainBuilder, Box<Chain>, PluginSpec, PluginBank) {
    let spec = gain_spec(format, name, TRIGGER);
    let mut builder = ChainBuilder::new(RATE);
    let mut chain = fit_ready(&mut builder, first, &bank(vec![spec.clone()]));
    let changes = await_changes(&mut builder, &mut chain, &sine(2048));
    assert_eq!(changes.len(), 1, "{name}: {changes:?}");
    let bare = bare_bank(&spec);
    (builder, chain, spec, bare)
}

/// Reordering keeps the instance: it is never loaded again (one open at every step) and keeps
/// the gain it set itself, which a fresh instance from the settings would not have.
#[test]
fn reordering_keeps_the_instance() {
    for (format, name) in [
        (PluginFormat::Clap, "kontra_clap_gain"),
        (PluginFormat::Vst3, "kontra_vst3_gain"),
    ] {
        let (mut builder, chain, spec, bare) = started(format, name, &plugin_then_widener());
        let (mut chain, _) = switch(
            &mut builder,
            chain,
            (&widener_then_plugin(), &bare),
            Some(&spec.path),
        );

        let input = sine(2048);
        run(&mut chain, &input);
        assert_gain(&run(&mut chain, &input), &input, REPORTED as f32);
        assert_eq!(PluginHost::global().open_of(&spec.path), 1, "{name}");
    }
}

/// Off keeps the instance, on uses it again.
#[test]
fn off_then_on_keeps_the_instance() {
    let (mut builder, chain, spec, bare) =
        started(PluginFormat::Clap, "kontra_clap_gain", &plugin_chain());
    let (chain, _) = switch(
        &mut builder,
        chain,
        (&ChainSpec::EMPTY, &bare),
        Some(&spec.path),
    );
    assert_eq!(builder.retiring(), 0, "kept, not retired");

    let (mut chain, _) = switch(
        &mut builder,
        chain,
        (&plugin_chain(), &bare),
        Some(&spec.path),
    );
    let input = sine(2048);
    run(&mut chain, &input);
    assert_gain(&run(&mut chain, &input), &input, REPORTED as f32);
}

/// A slot that leaves the settings closes its instance.
#[test]
fn removing_the_slot_closes_the_instance() {
    let (mut builder, chain, spec, bare) =
        started(PluginFormat::Clap, "kontra_clap_gain", &plugin_chain());
    let (chain, _) = switch(
        &mut builder,
        chain,
        (&ChainSpec::EMPTY, &bare),
        Some(&spec.path),
    );

    let gone = bank(Vec::new());
    let (chain, _) = switch(&mut builder, chain, (&ChainSpec::EMPTY, &gone), None);
    builder.reap();
    let start = Instant::now();
    while PluginHost::global().open_of(&spec.path) > 0 {
        assert!(start.elapsed() < PATIENCE, "the instance stayed open");
        std::thread::sleep(POLL_EVERY);
    }
    drop(chain);
}

/// The blocks that play while the chain changes shape do not allocate, and the plugin gain is
/// back after it.
#[test]
fn no_allocation_while_the_instance_moves() {
    let (mut builder, chain, spec, bare) = started(
        PluginFormat::Clap,
        "kontra_clap_gain",
        &plugin_then_widener(),
    );
    let (mut chain, allocated) = switch(
        &mut builder,
        chain,
        (&widener_then_plugin(), &bare),
        Some(&spec.path),
    );
    assert_eq!(allocated, 0, "the callback allocated during the change");

    let input = sine(1000);
    run(&mut chain, &input);
    assert_gain(&run(&mut chain, &input), &input, REPORTED as f32);
}

/// Shapes that change again before the last one is built neither leak an instance nor hand one
/// processor to two chains: at the end one plugin runs, one instance is open.
#[test]
fn quick_changes_keep_one_owner() {
    let (mut builder, mut running, spec, bare) = started(
        PluginFormat::Clap,
        "kontra_clap_gain",
        &plugin_then_widener(),
    );
    let shapes = [
        widener_then_plugin(),
        plugin_chain(),
        ChainSpec::EMPTY,
        plugin_then_widener(),
        widener_then_plugin(),
        plugin_chain(),
    ];
    for shape in &shapes {
        // The audio thread gets to run between two changes, but not always.
        if let Some(chain) = builder.fit(shape, &bare) {
            drop(std::mem::replace(&mut running, chain));
        }
        if let Some(chain) = builder.poll() {
            drop(std::mem::replace(&mut running, chain));
        }
        assert!(PluginHost::global().open_of(&spec.path) <= 1);
    }
    let (mut running, _) = switch(
        &mut builder,
        running,
        (&plugin_chain(), &bare),
        Some(&spec.path),
    );

    let input = sine(2048);
    run(&mut running, &input);
    assert_gain(&run(&mut running, &input), &input, REPORTED as f32);
}

/// More moves while the slot is off than a parameter ring holds.
const MOVES_WHILE_OFF: usize = 300;

/// Parameter moves while the slot is off are not lost: the plugin has the last one when it is
/// on again, however many there were.
#[test]
fn moves_while_off_arrive_when_on() {
    let spec = gain_spec(PluginFormat::Clap, "kontra_clap_gain", 1.0);
    let with_gain = |gain: f64| {
        let mut moved = spec.clone();
        for p in &mut moved.params {
            if p.id == GAIN_PARAM {
                p.value = gain;
            }
        }
        bank(vec![moved])
    };
    let mut builder = ChainBuilder::new(RATE);
    let chain = fit_ready(&mut builder, &plugin_chain(), &with_gain(1.0));
    let (chain, _) = switch(
        &mut builder,
        chain,
        (&ChainSpec::EMPTY, &with_gain(1.0)),
        Some(&spec.path),
    );

    for i in 0..MOVES_WHILE_OFF {
        builder.fit(&ChainSpec::EMPTY, &with_gain(1.0 + i as f64 * 0.001));
    }
    builder.fit(&ChainSpec::EMPTY, &with_gain(0.25));

    let (mut chain, _) = switch(
        &mut builder,
        chain,
        (&plugin_chain(), &with_gain(0.25)),
        Some(&spec.path),
    );
    let input = sine(2048);
    run_until(&mut chain, &input, |out| {
        (DELAY..out.len()).all(|n| (out[n] - input[n - DELAY] * 0.25).abs() < 1e-5)
    });
}
