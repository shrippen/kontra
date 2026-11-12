//! One plugin instance for every player (Runde 20, item 4: "Das Wechseln von Player zu Player
//! darf nicht spürbar sein!"): a player that takes over the chain finds the instance the one
//! before it loaded, with its state and its window, and never loads it again.
//!
//! ```text
//!  standby player ─ builder A ─┐
//!                              ├─ same bank ─► one instance per slot (loaded once)
//!  run player ──── builder B ─┘   B takes the processor once A's chain is gone
//! ```
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kontra_drivers::plugins::{probe_file, PluginFormat, PluginHost, PluginInfo};
use kontra_dsp::Chain;
use kontra_engine::{describe_plugin, ChainBuilder, PluginBank, PluginProblems};
use kontra_model::{ChainSpec, PluginSpec, SlotSpec};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

const RATE: f64 = 48_000.0;
const GAIN_PARAM: u32 = 7;
/// The block the insert collects: how much the audio is late.
const DELAY: usize = 64;
const PLUGIN_ID: u32 = 1;
/// How long a test waits for the plugin thread before it gives up.
const PATIENCE: Duration = Duration::from_secs(10);
const POLL_EVERY: Duration = Duration::from_millis(2);

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
/// `<name>.vst3` bundle. Each call gets its own copy, so instance counts do not mix.
fn plugin_file(format: PluginFormat, name: &str) -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let exe = std::env::current_exe().expect("test binary path");
    // The profile folder (`target/debug`) holds `examples/`; the test binary lies below it in
    // `deps/`, or since Cargo 1.100 in `build/<crate>/<hash>/out/`.
    let profile = exe
        .ancestors()
        .find(|dir| dir.join("examples").is_dir())
        .expect("target folder");
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
        "rack-{}-{}",
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

fn plugin_chain() -> ChainSpec {
    [SlotSpec::for_plugin(PLUGIN_ID)].to_vec().into()
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

/// The next player builds its chain at once from the instance the one before loaded.
fn hands_over(format: PluginFormat, name: &str) {
    let spec = gain_spec(format, name, 0.5);
    let path = spec.path.clone();
    let bank = PluginBank::new(vec![spec], PluginProblems::default());

    let mut standby = ChainBuilder::new(RATE);
    let chain = fit_ready(&mut standby, &plugin_chain(), &bank);
    assert_eq!(PluginHost::global().open_of(&path), 1);
    drop(chain);
    drop(standby);

    let mut run_player = ChainBuilder::new(RATE);
    let mut chain = run_player
        .fit(&plugin_chain(), &bank)
        .expect("the chain comes at once: nothing loads");
    assert_eq!(PluginHost::global().open_of(&path), 1, "the same instance");
    let input = sine(2048);
    assert_gain(&run(&mut chain, &input), &input, 0.5);
}

#[test]
fn a_clap_instance_outlives_its_player() {
    hands_over(PluginFormat::Clap, "kontra_clap_gain");
}

#[test]
fn a_vst3_instance_outlives_its_player() {
    hands_over(PluginFormat::Vst3, "kontra_vst3_gain");
}

/// Players overlap for a moment (a reopen starts the new output before the old one goes): the
/// new one waits for the processor and plays dry meanwhile, then takes it.
#[test]
fn the_next_player_waits_for_the_running_chain() {
    let bank = PluginBank::new(
        vec![gain_spec(PluginFormat::Clap, "kontra_clap_gain", 0.5)],
        PluginProblems::default(),
    );
    let mut standby = ChainBuilder::new(RATE);
    let running = fit_ready(&mut standby, &plugin_chain(), &bank);

    let mut run_player = ChainBuilder::new(RATE);
    assert!(
        run_player.fit(&plugin_chain(), &bank).is_none(),
        "still in use"
    );
    assert!(run_player.poll().is_none(), "still in use");

    drop(running);
    let mut chain = run_player.poll().expect("free now");
    let input = sine(2048);
    assert_gain(&run(&mut chain, &input), &input, 0.5);
}

/// A player whose gains carry no plugin list must not close the instances the others share.
#[test]
fn a_player_without_plugins_closes_none() {
    let spec = gain_spec(PluginFormat::Clap, "kontra_clap_gain", 0.5);
    let path = spec.path.clone();
    let bank = PluginBank::new(vec![spec], PluginProblems::default());
    let mut standby = ChainBuilder::new(RATE);
    drop(fit_ready(&mut standby, &plugin_chain(), &bank));
    drop(standby);

    let mut tuner = ChainBuilder::new(RATE);
    let _ = tuner.fit(&ChainSpec::EMPTY, &PluginBank::default());
    drop(tuner);

    let mut run_player = ChainBuilder::new(RATE);
    run_player
        .fit(&plugin_chain(), &bank)
        .expect("the chain comes at once: nothing loads");
    assert_eq!(PluginHost::global().open_of(&path), 1);
}
