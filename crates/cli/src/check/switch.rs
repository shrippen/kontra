//! `check --steps switch`: how long a change of player leaves the output silent (Runde 20: the
//! owner wants a switch between standby, run, ear training and tuner to be unnoticeable).
//!
//! ```text
//!  standby monitor ──drop──► closed ──open──► new monitor ──first block──► running
//!  │◄─ closing ─►│◄──── opening ────►│◄──── first callback ────►│ = silent gap
//! ```
//!
//! A lower bound: monitor to monitor only closes and opens the devices. A real switch adds what
//! the new mode does before its input opens (a run loads its backing), up to one app tick until
//! the standby opens again, and the fade-in of the effect chain; the bass is heard only once the
//! input delivers too. Monitoring stays at 0: the loop cable of the loopback step would feed
//! back.

use super::Report;
use crate::CliResult;
use kontra_model::Instrument;
use kontra_session as session;
use serde_json::json;
use std::time::{Duration, Instant};

/// Switches per run: enough for a median and a worst case.
const SWITCHES: usize = 10;
/// Time between switches, as a short stay in a mode.
const STAY: Duration = Duration::from_secs(2);
/// A device that has not asked for a block by then counts as failed.
const PATIENCE: Duration = Duration::from_secs(5);
const MS_PER_S: f64 = 1000.0;
/// How often the first block is looked for: fine enough for a gap in ms.
const POLL: Duration = Duration::from_millis(1);

/// One switch, each from its start: when the old devices were closed, the new ones opened, the
/// new output ran.
struct Gap {
    closed_ms: f64,
    opened_ms: f64,
    running_ms: f64,
}

pub(super) fn switch(
    report: &mut Report,
    setup: &session::LiveSetup,
    instrument: &Instrument,
) -> CliResult {
    report.section("Player switch");
    let quiet = session::Gains {
        click: 0.0,
        tone: 0.0,
        backing: 0.0,
        drums: 0.0,
        bass: 0.0,
        monitor: 0.0,
        ..session::Gains::default()
    };

    let mut monitor = open(setup, instrument, &quiet)?;
    running(&monitor)?;
    let mut gaps = Vec::new();
    for _ in 0..SWITCHES {
        std::thread::sleep(STAY);
        let start = Instant::now();
        drop(monitor);
        let closed_ms = ms(start);
        monitor = open(setup, instrument, &quiet)?;
        let opened_ms = ms(start);
        running(&monitor)?;
        gaps.push(Gap {
            closed_ms,
            opened_ms,
            running_ms: ms(start),
        });
    }

    let mut silent: Vec<f64> = gaps.iter().map(|g| g.running_ms).collect();
    silent.sort_by(f64::total_cmp);
    let median = median(&silent);
    let worst = silent[silent.len() - 1];
    report.line(format!(
        "- silent gap per switch, at least: median {median:.0} ms, worst {worst:.0} ms ({SWITCHES} switches)"
    ));
    for g in &gaps {
        report.line(format!(
            "  - close {:.0} ms, open {:.0} ms, first block {:.0} ms, silent {:.0} ms",
            g.closed_ms,
            g.opened_ms - g.closed_ms,
            g.running_ms - g.opened_ms,
            g.running_ms
        ));
    }
    report.json.insert(
        "switch".into(),
        json!({
            "medianMs": median,
            "worstMs": worst,
            "gaps": gaps.iter().map(|g| json!({
                "closedMs": g.closed_ms,
                "openedMs": g.opened_ms,
                "runningMs": g.running_ms,
            })).collect::<Vec<_>>(),
        }),
    );
    Ok(())
}

fn open(
    setup: &session::LiveSetup,
    instrument: &Instrument,
    gains: &session::Gains,
) -> Result<session::LiveMonitor, String> {
    let started = session::LiveMonitor::start(setup, instrument, gains.clone());
    let started = started.ok_or("the switch needs an audio input device")?;
    started.map_err(|e| e.to_string())
}

/// Waits until the output asked for its first block.
fn running(monitor: &session::LiveMonitor) -> CliResult {
    let start = Instant::now();
    while !monitor.is_running() {
        if start.elapsed() > PATIENCE {
            return Err("the output did not start".into());
        }
        std::thread::sleep(POLL);
    }
    Ok(())
}

/// The middle of sorted values; with an even count the mean of the two middle ones.
fn median(sorted: &[f64]) -> f64 {
    let half = sorted.len() / 2;
    if sorted.len() % 2 == 1 {
        return sorted[half];
    }
    (sorted[half - 1] + sorted[half]) / 2.0
}

fn ms(since: Instant) -> f64 {
    since.elapsed().as_secs_f64() * MS_PER_S
}

#[cfg(test)]
mod tests {
    use super::median;

    /// Review #284: with ten switches the upper of the two middle values was called the median.
    #[test]
    fn the_median_of_an_even_count_is_the_mean_of_the_middle() {
        assert_eq!(median(&[1.0, 2.0, 3.0, 4.0]), 2.5);
        assert_eq!(median(&[1.0, 2.0, 3.0]), 2.0);
    }
}
