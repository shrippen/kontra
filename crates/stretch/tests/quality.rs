//! Quality tests: pitch, length, level, transients, transparency, glides.

use std::f64::consts::TAU;

use kontra_stretch::{stretch_buffer, Stretcher};
use rustfft::num_complex::Complex;
use rustfft::FftPlanner;

const SR: u32 = 48_000;
const SRF: f64 = SR as f64;
/// Main window at 48 kHz (length tolerance).
const WIN: usize = 4096;

fn sine(freq: f64, amp: f64, secs: f64) -> Vec<f32> {
    let len = (secs * SRF) as usize;
    (0..len)
        .map(|n| (amp * (TAU * freq * n as f64 / SRF).sin()) as f32)
        .collect()
}

/// Bass-like tone: harmonics 1..=6 with amplitude 1/k.
fn bass(freq: f64, amp: f64, secs: f64) -> Vec<f32> {
    let len = (secs * SRF) as usize;
    (0..len)
        .map(|n| {
            let t = n as f64 / SRF;
            let v: f64 = (1..=6)
                .map(|k| (TAU * freq * k as f64 * t).sin() / k as f64)
                .sum();
            (amp * v) as f32
        })
        .collect()
}

/// Strongest frequency in [lo, hi] Hz, Hann window, 4x zero padding,
/// parabolic interpolation on log magnitude.
fn peak_hz(x: &[f32], lo: f64, hi: f64) -> f64 {
    let len = x.len();
    let n = (len * 4).next_power_of_two();
    let mut buf: Vec<Complex<f64>> = vec![Complex::default(); n];
    for (i, &v) in x.iter().enumerate() {
        let w = 0.5 - 0.5 * (TAU * i as f64 / len as f64).cos();
        buf[i] = Complex::new(f64::from(v) * w, 0.0);
    }
    FftPlanner::new().plan_fft_forward(n).process(&mut buf);

    let bin = SRF / n as f64;
    let (a, b) = ((lo / bin) as usize, (hi / bin) as usize);
    let k = (a..=b)
        .max_by(|&i, &j| buf[i].norm().total_cmp(&buf[j].norm()))
        .unwrap_or(a);
    let (l, m, r) = (
        buf[k - 1].norm().ln(),
        buf[k].norm().ln(),
        buf[k + 1].norm().ln(),
    );
    let d = 0.5 * (l - r) / (l - 2.0 * m + r);
    (k as f64 + d) * bin
}

fn cents(f: f64, reference: f64) -> f64 {
    1200.0 * (f / reference).log2()
}

fn rms(x: &[f32]) -> f64 {
    (x.iter().map(|&v| f64::from(v).powi(2)).sum::<f64>() / x.len() as f64).sqrt()
}

fn db(x: f64) -> f64 {
    20.0 * x.log10()
}

/// Middle part of a signal, skipping one window at either end.
fn middle(x: &[f32]) -> &[f32] {
    &x[2 * WIN..x.len() - 2 * WIN]
}

/// Stretches a mono signal in blocks via push/pull, changing speed on the
/// way: `speeds[i]` applies from input sample `i * step`.
fn stream(x: &[f32], speeds: &[f64], step: usize, block: usize) -> Vec<f32> {
    let mut st = Stretcher::new(1, SR, block);
    st.set_speed(speeds[0]);
    let mut out = Vec::new();
    let mut buf = vec![0.0; block];
    let mut fed = 0;
    while fed < x.len() {
        let seg = fed / step;
        st.set_speed(speeds[seg.min(speeds.len() - 1)]);
        let need = st.required_input().clamp(1, block).min(x.len() - fed);
        fed += st.push(&[&x[fed..fed + need]]);
        loop {
            let got = st.pull(&mut [&mut buf[..]]);
            out.extend_from_slice(&buf[..got]);
            if got == 0 {
                break;
            }
        }
    }
    out
}

#[test]
fn pitch_and_length_kept() {
    for &(freq, make) in &[(110.0, sine as fn(f64, f64, f64) -> Vec<f32>), (41.2, bass)] {
        let x = make(freq, 0.5, 3.0);
        for &speed in &[0.25, 0.5, 0.75, 1.0, 1.25, 1.5] {
            let y = stretch_buffer(std::slice::from_ref(&x), SR, speed);
            let y = &y[0];

            let want = x.len() as f64 / speed;
            assert!(
                (y.len() as f64 - want).abs() <= WIN as f64,
                "length {speed}"
            );

            let f = peak_hz(middle(y), freq * 0.9, freq * 1.1);
            let c = cents(f, freq);
            println!("{freq} Hz at {speed}: {c:+.3} cent");
            assert!(c.abs() < 2.0, "{freq} Hz at {speed}: {c} cent");
        }
    }
}

#[test]
fn streaming_length_follows_speed() {
    let x = sine(110.0, 0.5, 4.0);
    for &speed in &[0.5, 0.75] {
        let y = stream(&x, &[speed], x.len(), 512);
        let lat = Stretcher::new(1, SR, 512).latency_samples();
        let want = (x.len() - lat) as f64 / speed;
        println!("speed {speed}: got {} want {want:.0}", y.len());
        assert!((y.len() as f64 - want).abs() <= WIN as f64 / speed);
    }
}

#[test]
fn level_steady() {
    for &(freq, make) in &[(110.0, sine as fn(f64, f64, f64) -> Vec<f32>), (41.2, bass)] {
        let x = make(freq, 0.5, 3.0);
        let reference = rms(middle(&x));
        for &speed in &[0.25, 0.5, 0.75, 1.5] {
            let y = stretch_buffer(std::slice::from_ref(&x), SR, speed);
            let mid = middle(&y[0]);

            // Every 50 ms block within ±1 dB of the input level.
            let block = (SRF * 0.05) as usize;
            let worst = mid
                .chunks_exact(block)
                .map(|c| db(rms(c) / reference))
                .fold(0.0_f64, |a, d| if d.abs() > a.abs() { d } else { a });
            println!("{freq} Hz at {speed}: worst block {worst:+.3} dB");
            assert!(worst.abs() < 1.0, "{freq} Hz at {speed}: {worst} dB");
        }
    }
}

/// Clicks every 0.5 s (120 BPM), optionally over a bass tone.
fn clicks(count: usize, under: Option<f64>) -> (Vec<f32>, Vec<usize>) {
    let spacing = SR as usize / 2;
    let len = spacing * (count + 1);
    let mut x = match under {
        Some(f) => bass(f, 0.2, len as f64 / SRF),
        None => vec![0.0; len],
    };
    x.resize(len, 0.0);
    let at: Vec<usize> = (0..count).map(|i| spacing / 2 + i * spacing).collect();
    for &p in &at {
        x[p] += 0.8;
    }
    (x, at)
}

/// For each expected click: (peak index, share of energy within ±5 ms).
fn click_stats(y: &[f32], expect: &[usize], half_region: usize) -> Vec<(usize, f64)> {
    let near = (SRF * 0.005) as usize;
    expect
        .iter()
        .map(|&e| {
            let lo = e.saturating_sub(half_region);
            let hi = (e + half_region).min(y.len());
            let region = &y[lo..hi];
            let peak = lo
                + (0..region.len())
                    .max_by(|&i, &j| region[i].abs().total_cmp(&region[j].abs()))
                    .unwrap_or(0);
            let energy =
                |a: usize, b: usize| y[a..b].iter().map(|&v| f64::from(v).powi(2)).sum::<f64>();
            let core = energy(peak - near, peak + near);
            (peak, core / energy(lo, hi))
        })
        .collect()
}

#[test]
fn clicks_stay_sharp() {
    let speed = 0.5;
    let (x, at) = clicks(8, None);
    let y = stretch_buffer(&[x], SR, speed);
    let expect: Vec<usize> = at.iter().map(|&p| (p as f64 / speed) as usize).collect();
    let stats = click_stats(&y[0], &expect, SR as usize / 2);

    for (i, &(peak, share)) in stats.iter().enumerate() {
        let off_ms = (peak as f64 - expect[i] as f64) / SRF * 1000.0;
        println!(
            "click {i}: {:.1} % in 10 ms, offset {off_ms:+.2} ms",
            share * 100.0
        );
        assert!(share >= 0.9, "click {i}: {share}");
        assert!(off_ms.abs() < 5.0, "click {i} offset {off_ms} ms");
    }
    for w in stats.windows(2) {
        let gap = (w[1].0 - w[0].0) as f64;
        let want = SRF; // 0.5 s → 1 s
        assert!((gap - want).abs() < SRF * 0.002, "spacing {gap}");
    }
}

#[test]
fn clicks_over_bass() {
    let speed = 0.5;
    let freq = 55.0;
    let (x, at) = clicks(8, Some(freq));
    let y = stretch_buffer(&[x], SR, speed);
    let y = &y[0];

    // Second difference removes the bass (< -55 dB up to its 6th
    // harmonic), keeps the clicks.
    let hp: Vec<f32> = y.windows(3).map(|w| w[2] - 2.0 * w[1] + w[0]).collect();
    let expect: Vec<usize> = at.iter().map(|&p| (p as f64 / speed) as usize).collect();
    for (i, (peak, share)) in click_stats(&hp, &expect, SR as usize / 2)
        .into_iter()
        .enumerate()
    {
        let off_ms = (peak as f64 - expect[i] as f64) / SRF * 1000.0;
        println!(
            "click {i} over bass: {:.1} % in 10 ms, offset {off_ms:+.2} ms",
            share * 100.0
        );
        assert!(share >= 0.85, "click {i}: {share}");
    }

    let f = peak_hz(middle(y), freq * 0.9, freq * 1.1);
    let c = cents(f, freq);
    println!("bass under clicks: {c:+.3} cent");
    assert!(c.abs() < 2.0);
}

/// Music-like test signal: chord, bass, noise bursts.
fn music(secs: f64) -> Vec<f32> {
    let len = (secs * SRF) as usize;
    let mut seed = 1_u32;
    (0..len)
        .map(|n| {
            let t = n as f64 / SRF;
            let tones: f64 = [55.0, 220.0, 277.2, 329.6, 880.0]
                .iter()
                .map(|f| 0.1 * (TAU * f * t).sin())
                .sum();
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let noise = (f64::from(seed >> 8) / f64::from(1_u32 << 24) - 0.5) * 0.2;
            let beat = (-(t % 0.25) * 40.0).exp();
            (tones + noise * beat) as f32
        })
        .collect()
}

#[test]
fn unity_speed_is_transparent() {
    let x = music(3.0);
    let y = stream(&x, &[1.0], x.len(), 512);
    let n = y.len().min(x.len());
    assert!(n > x.len() / 2);

    let (sig, err) = (WIN..n).fold((0.0, 0.0), |(s, e), i| {
        let d = f64::from(y[i] - x[i]);
        (s + f64::from(x[i]).powi(2), e + d * d)
    });
    let snr = 10.0 * (sig / err).log10();
    println!("speed 1.0 SNR {snr:.1} dB");
    assert!(snr > 30.0);
}

#[test]
fn latency_matches_output() {
    let block = 256;
    let mut st = Stretcher::new(2, SR, block);
    let lat = st.latency_samples();
    let hop = WIN / 4;
    let zeros = vec![0.0; block];
    let mut buf = vec![vec![0.0; block]; 2];
    let (mut fed, mut made) = (0, 0);
    for _ in 0..400 {
        fed += st.push(&[&zeros, &zeros]);
        let [a, b] = &mut buf[..] else { unreachable!() };
        loop {
            let got = st.pull(&mut [&mut a[..], &mut b[..]]);
            made += got;
            if got == 0 {
                break;
            }
        }
        assert!(made <= fed.saturating_sub(lat), "fed {fed} made {made}");
        if fed > lat + hop {
            assert!(made + lat + hop > fed, "fed {fed} made {made}");
        }
    }
    println!(
        "latency {lat} samples ({:.1} ms)",
        lat as f64 / SRF * 1000.0
    );
}

#[test]
fn speed_change_without_clicks() {
    let (freq, amp) = (220.0, 0.5);
    let x = sine(freq, amp, 6.0);
    let step = SR as usize / 2;
    let speeds = [1.0, 0.5, 0.8, 0.3, 1.2, 0.6, 1.0, 0.75, 0.5, 1.5, 0.5, 1.0];
    let y = stream(&x, &speeds, step, 480);

    // Largest step of a clean sine is 2π·f·A/sr; allow 50 % on top.
    let limit = 1.5 * TAU * freq * amp / SRF;
    let jump = y[WIN..]
        .windows(2)
        .map(|w| f64::from((w[1] - w[0]).abs()))
        .fold(0.0, f64::max);
    println!("max step {jump:.5} (limit {limit:.5})");
    assert!(jump < limit);

    let block = (SRF * 0.05) as usize;
    let reference = rms(&x);
    let worst = y[WIN..y.len() - WIN]
        .chunks_exact(block)
        .map(|c| db(rms(c) / reference))
        .fold(0.0_f64, |a, d| if d.abs() > a.abs() { d } else { a });
    println!("worst level while gliding {worst:+.2} dB");
    assert!(worst.abs() < 1.5);
}

#[test]
fn stereo_image_kept() {
    // Right = left at -6 dB, plus a second tone only on the right.
    let l = bass(41.2, 0.4, 2.0);
    let extra = sine(330.0, 0.2, 2.0);
    let r: Vec<f32> = l.iter().zip(&extra).map(|(a, b)| 0.5 * a + b).collect();
    let y = stretch_buffer(&[l, r], SR, 0.6);
    let solo = stretch_buffer(&[extra], SR, 0.6);

    // Residual of right - 0.5·left must be the stretched extra tone only:
    // compare its level and pitch.
    let res: Vec<f32> = y[1].iter().zip(&y[0]).map(|(r, l)| r - 0.5 * l).collect();
    let lvl = db(rms(middle(&res)) / rms(middle(&solo[0])));
    let f = peak_hz(middle(&res), 300.0, 360.0);
    println!("stereo residual level {lvl:+.2} dB, tone {f:.2} Hz");
    assert!(lvl.abs() < 1.0);
    assert!(cents(f, 330.0).abs() < 2.0);

    // The bass must cancel in the residual (shared phase rotation):
    // projection of the residual onto the left channel.
    let (m, lm) = (middle(&res), middle(&y[0]));
    let dot: f64 = m
        .iter()
        .zip(lm)
        .map(|(a, b)| f64::from(*a) * f64::from(*b))
        .sum();
    let ll: f64 = lm.iter().map(|&b| f64::from(b).powi(2)).sum();
    let leak = (dot / ll).abs();
    println!("bass leak into residual {leak:.5}");
    assert!(leak < 0.01);
}

#[test]
fn dense_clicks() {
    // Sixteenth notes at 120 BPM (125 ms) over a bass line.
    let spacing = SR as usize / 8;
    let count = 24;
    // First click after 0.2 s: right after a reset there is no time to
    // steer the drift (see crate docs).
    let len = spacing * (count + 2);
    let mut x = bass(55.0, 0.2, len as f64 / SRF);
    x.resize(len, 0.0);
    let at: Vec<usize> = (1..=count).map(|i| spacing / 2 + i * spacing).collect();
    for &p in &at {
        x[p] += 0.8;
    }

    for &speed in &[0.75, 0.5, 0.25] {
        let y = stretch_buffer(std::slice::from_ref(&x), SR, speed);
        let hp: Vec<f32> = y[0].windows(3).map(|w| w[2] - 2.0 * w[1] + w[0]).collect();
        let expect: Vec<usize> = at.iter().map(|&p| (p as f64 / speed) as usize).collect();
        let half = (spacing as f64 / speed / 2.0) as usize;
        let stats = click_stats(&hp, &expect, half);
        let share = stats.iter().map(|s| s.1).fold(1.0, f64::min);
        let off = stats
            .iter()
            .zip(&expect)
            .map(|(s, &e)| ((s.0 as f64 - e as f64) / SRF * 1000.0).abs())
            .fold(0.0, f64::max);
        let offs: Vec<String> = stats
            .iter()
            .zip(&expect)
            .map(|(s, &e)| {
                format!(
                    "{:.1}/{:.0}",
                    (s.0 as f64 - e as f64) / SRF * 1000.0,
                    s.1 * 100.0
                )
            })
            .collect();
        println!("offsets {}", offs.join(" "));
        println!(
            "dense at {speed}: worst share {:.1} %, worst offset {off:.2} ms",
            share * 100.0
        );
        assert!(share >= 0.9, "share {share}");
        assert!(off < 2.0, "offset {off}");
    }
}

#[test]
fn stereo_delay_kept() {
    // Right = left delayed by 0.3 ms: the stereo position of a source.
    let delay = 14;
    let l = music(3.0);
    let mut r = vec![0.0; delay];
    r.extend_from_slice(&l[..l.len() - delay]);
    let y = stretch_buffer(&[l, r], SR, 0.5);
    let (a, b) = (middle(&y[0]), middle(&y[1]));

    // Lag of the cross-correlation peak.
    let corr = |lag: usize| -> f64 {
        a[..a.len() - lag]
            .iter()
            .zip(&b[lag..])
            .map(|(x, y)| f64::from(*x) * f64::from(*y))
            .sum()
    };
    let best = (0..40)
        .max_by(|&i, &j| corr(i).total_cmp(&corr(j)))
        .unwrap_or(0);
    println!("inter-channel delay {best} samples (input {delay})");
    assert_eq!(best, delay);
}
