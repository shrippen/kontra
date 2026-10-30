use super::*;

const RATE: f64 = 48_000.0;
/// Stamp jitter as modelled in `sim3.py`: ALSA/USB pointer granularity, ±0.5 ms.
const JITTER_S: f64 = 0.000_5;
/// What the loop may be off once locked (simulation: p95 0.08 ms, max 0.12 ms).
const LOCKED_S: f64 = 0.000_15;

/// A device whose clock runs `ppm` off the nominal rate, stamping blocks with jitter.
struct Device {
    ppm: f64,
    /// Added to every stamp from here on (a stamp step, not a loss of samples).
    offset_s: f64,
    sample: u64,
    /// Xorshift state for the jitter: the same every run.
    state: u64,
    jitter_s: f64,
}

impl Device {
    fn new(ppm: f64, jitter_s: f64) -> Device {
        Device {
            ppm,
            offset_s: 0.0,
            sample: 0,
            state: 0x2910_2026,
            jitter_s,
        }
    }

    /// True device time of `sample`.
    fn true_s(&self, sample: u64) -> f64 {
        self.offset_s + sample as f64 / (RATE * (1.0 + self.ppm * 1e-6))
    }

    fn jitter(&mut self) -> f64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        let unit = (self.state >> 11) as f64 / (1u64 << 53) as f64;
        (unit * 2.0 - 1.0) * self.jitter_s
    }

    /// Stamps blocks of `block` frames for `seconds` into `clock`; returns the loop's error
    /// (smoothed minus true time) at each block.
    fn run(&mut self, clock: &mut DeviceClock, block: u64, seconds: f64) -> Vec<f64> {
        let blocks = (seconds * RATE / block as f64) as usize;
        let mut errors = Vec::with_capacity(blocks);
        for _ in 0..blocks {
            let stamp_s = self.true_s(self.sample) + self.jitter();
            clock.stamp(self.sample, stamp_s);
            errors.push(clock.time_s(self.sample).unwrap() - self.true_s(self.sample));
            self.sample += block;
        }
        errors
    }
}

fn largest(errors: &[f64]) -> f64 {
    errors.iter().fold(0.0, |m, e| m.max(e.abs()))
}

/// Jittered stamps: once locked the loop is off by far less than the jitter, and jitter is
/// never taken for a step.
#[test]
fn converges() {
    let mut clock = DeviceClock::new(RATE);
    let mut device = Device::new(0.0, JITTER_S);
    device.run(&mut clock, 128, 5.0);
    let errors = device.run(&mut clock, 128, 5.0);
    assert!(largest(&errors) < LOCKED_S, "{} s", largest(&errors));
    assert_eq!(clock.skipped_s(), 0.0);
}

/// ±50 ppm (two cheap interfaces): counting samples at the nominal rate is 15 ms off after
/// 5 minutes; the loop follows the real rate.
#[test]
fn follows_drift() {
    for ppm in [-50.0, 50.0] {
        let mut clock = DeviceClock::new(RATE);
        let mut device = Device::new(ppm, JITTER_S);
        device.run(&mut clock, 256, 290.0);
        let errors = device.run(&mut clock, 256, 10.0);
        assert!(
            largest(&errors) < LOCKED_S,
            "{ppm} ppm: {} s",
            largest(&errors)
        );
        assert_eq!(clock.skipped_s(), 0.0, "{ppm} ppm");
    }
}

/// The audio server changes its quantum mid-run (128 → 2048 frames): the loop stays locked.
/// When the stamps step with it (+18.7 ms, as output buffering does), the loop counts the
/// step and locks again at once.
#[test]
fn relocks_after_a_quantum_change() {
    let mut clock = DeviceClock::new(RATE);
    let mut device = Device::new(30.0, JITTER_S);
    device.run(&mut clock, 128, 10.0);
    let errors = device.run(&mut clock, 2048, 5.0);
    // Fewer stamps a second let more jitter through (simulation, quantum 2048: max 0.72 ms
    // for input and output together).
    assert!(largest(&errors) < 2.0 * LOCKED_S, "{} s", largest(&errors));
    assert_eq!(clock.skipped_s(), 0.0);

    let step_s = (1024.0 - 128.0) / RATE;
    device.offset_s = step_s;
    let errors = device.run(&mut clock, 1024, 5.0);
    assert!(
        (clock.skipped_s() - step_s).abs() < 2.0 * JITTER_S,
        "{} s",
        clock.skipped_s()
    );
    assert!(largest(&errors[1..]) < 2.0 * JITTER_S, "{errors:?}");
    let settled = &errors[errors.len() / 2..];
    assert!(largest(settled) < 2.0 * LOCKED_S, "{} s", largest(settled));
}

/// Quantum 2048 (43 ms between stamps), first stamp 0.4 ms off, device 50 ppm fast: the loop
/// settles without ringing. Its gain is capped; the locking bandwidth alone (5 Hz) would give
/// 1.34 per update and swing from side to side.
#[test]
fn settles_without_ringing() {
    let mut clock = DeviceClock::new(RATE);
    let mut device = Device::new(50.0, 0.0);
    device.offset_s = 0.000_4;
    device.run(&mut clock, 2048, 0.05);
    device.offset_s = 0.0;
    let errors = device.run(&mut clock, 2048, 6.0);

    let signs: Vec<bool> = errors
        .iter()
        .filter(|e| e.abs() > 1e-6)
        .map(|e| *e > 0.0)
        .collect();
    let flips = signs.windows(2).filter(|w| w[0] != w[1]).count();
    assert!(flips <= 2, "{flips} sign changes: {errors:?}");
    let settled = &errors[errors.len() / 2..];
    assert!(largest(settled) < 1e-5, "{} s", largest(settled));
}

/// A device that skips 0.8 s: one step of 0.8 s, then locked again.
#[test]
fn counts_a_skip() {
    let mut clock = DeviceClock::new(RATE);
    let mut device = Device::new(0.0, JITTER_S);
    device.run(&mut clock, 256, 3.0);
    device.offset_s = 0.8;
    let errors = device.run(&mut clock, 256, 3.0);
    assert!((clock.skipped_s() - 0.8).abs() < 2.0 * JITTER_S);
    let settled = &errors[errors.len() / 2..];
    assert!(largest(settled) < LOCKED_S, "{} s", largest(settled));
}

/// Stamps need not start at 0 (a device clock since boot): the loop locks with `LOCK_HZ` for
/// `LOCK_S` from its first stamp, not from 0 s.
#[test]
fn locks_from_its_first_stamp() {
    let mut late = DeviceClock::new(RATE);
    let mut device = Device::new(50.0, 0.0);
    device.offset_s = 1000.0 + 0.000_4;
    device.run(&mut late, 2048, 0.05);
    device.offset_s = 1000.0;
    let late_errors = device.run(&mut late, 2048, 1.5);

    let mut early = DeviceClock::new(RATE);
    let mut device = Device::new(50.0, 0.0);
    device.offset_s = 0.000_4;
    device.run(&mut early, 2048, 0.05);
    device.offset_s = 0.0;
    let early_errors = device.run(&mut early, 2048, 1.5);

    let tail = |e: &[f64]| largest(&e[e.len() - 5..]);
    assert!(
        (tail(&late_errors) - tail(&early_errors)).abs() < 1e-6,
        "from 1000 s: {} s, from 0 s: {} s",
        tail(&late_errors),
        tail(&early_errors)
    );
}
