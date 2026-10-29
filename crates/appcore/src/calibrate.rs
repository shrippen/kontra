//! Latency (LH-EIN-05, LH-EIN-14): by cable (loopback) or "play on the click".
//!
//! ```text
//! cable:     output ─► input        loopback_ms = system latency ─► latency (from Loopback)
//! on click:  click ─► player ─► in  click_ms = latency + lean
//!            with a cable value:    lean = click_ms − loopback_ms, latency stays
//!            without:               latency = click_ms (from Click, holds the lean)
//! ```

use crate::store::{LatencyFrom, Settings};
use crate::{Core, UiEvent, CALIBRATION_LEAD_S};
use kontra_session::{
    CalibrationResult, LiveCalibration, LiveLoopback, LiveSetup, LoopbackResult, Pocket, Side,
    Summary,
};

/// Unicode minus: "−14 ms" reads as a number, "-14 ms" as a dash.
const MINUS: &str = "\u{2212}";

impl Core {
    fn calibration_setup(&mut self) -> LiveSetup {
        self.stop_live();
        self.tuner = None;
        self.live_ear = None;
        self.calibration = None;
        self.loopback = None;
        self.level.reset();
        LiveSetup {
            input: self.input_choice(),
            output_device: self.output_device(),
            source: self.source(),
            backing: None,
            original: None,
            backing_cache: None,
        }
    }

    /// Starts "play on the click" (LH-EIN-05): 8 clicks from one second after the input starts.
    pub fn start_calibration(&mut self) -> Result<(), String> {
        let setup = self.calibration_setup();
        let instrument = self.instrument();
        let calibration = LiveCalibration::start(&setup, &instrument, CALIBRATION_LEAD_S)
            .map_err(|e| self.tr("error.audio", &[("reason", e.to_string())]))?;
        self.calibration = Some(calibration);
        Ok(())
    }

    /// Starts the measurement by cable: twelve tones from the output into the input.
    pub fn start_loopback(&mut self) -> Result<(), String> {
        let setup = self.calibration_setup();
        let instrument = self.instrument();
        let loopback = LiveLoopback::start(&setup, &instrument)
            .map_err(|e| self.tr("error.audio", &[("reason", e.to_string())]))?;
        self.loopback = Some(loopback);
        Ok(())
    }

    pub fn is_calibrating(&self) -> bool {
        self.calibration.is_some() || self.loopback.is_some()
    }

    /// The calibration's click last passed: −4..−1 count-in, 0..7 measured.
    pub fn calibration_beat(&self) -> Option<i32> {
        self.calibration.as_ref()?.beat()
    }

    /// The finished measurement, if one ended since the last tick.
    pub(crate) fn poll_calibration(&mut self) -> Option<UiEvent> {
        if let Some(result) = self.calibration.as_mut().and_then(LiveCalibration::poll) {
            self.calibration = None;
            return Some(self.calibrated(result));
        }
        let result = self.loopback.as_mut().and_then(LiveLoopback::poll)?;
        self.loopback = None;
        Some(self.looped(result))
    }

    fn calibrated(&mut self, result: CalibrationResult) -> UiEvent {
        let (latency_ms, spread_ms) = match result {
            CalibrationResult::Measured {
                latency_ms,
                spread_ms,
            } => (latency_ms.round(), spread_ms),
            CalibrationResult::TooFew { matched } => {
                return UiEvent::Calibrated {
                    latency_ms: None,
                    text: self.tr("calibration.few", &[("matched", matched.to_string())]),
                }
            }
            CalibrationResult::Unsteady { .. } => {
                return UiEvent::Calibrated {
                    latency_ms: None,
                    text: self.tr("calibration.unsteady", &[]),
                }
            }
        };

        // With a cable value the clicks measure the lean; the latency stays (R4).
        if self.settings.latency_from == LatencyFrom::Loopback {
            self.reconfigure(|s| s.click_ms = Some(latency_ms));
            return UiEvent::Calibrated {
                latency_ms: None,
                text: self.lean_text(),
            };
        }

        // Without one the clicks are the latency, lean included; a lean ahead of a short
        // latency gives a median before the click, no latency is below zero.
        let click_ms = latency_ms;
        let latency_ms = latency_ms.max(0.0);
        self.reconfigure(|s| {
            s.click_ms = Some(click_ms);
            s.latency_ms = latency_ms;
            s.latency_from = LatencyFrom::Click;
        });
        let text = self.tr(
            "calibration.done",
            &[
                ("ms", format!("{latency_ms:.0}")),
                ("spread", format!("{spread_ms:.0}")),
            ],
        );
        UiEvent::Calibrated {
            latency_ms: Some(latency_ms),
            text,
        }
    }

    /// "Deine Neigung: −14 ms, du spielst vor dem Klick. Die Latenz bleibt 20 ms …"
    fn lean_text(&self) -> String {
        let lean = self.lean_ms().unwrap_or_default().round();
        let key = if lean < 0.0 {
            "calibration.lean.early"
        } else if lean > 0.0 {
            "calibration.lean.late"
        } else {
            "calibration.lean.none"
        };
        let ms = format!("{:.0}", self.settings.latency_ms);
        self.tr(key, &[("lean", signed_ms(lean)), ("ms", ms)])
    }

    fn looped(&mut self, result: LoopbackResult) -> UiEvent {
        let latency_ms = match result {
            LoopbackResult::Measured { latency_ms, .. } => latency_ms.round(),
            LoopbackResult::Silent { heard } => {
                return UiEvent::Calibrated {
                    latency_ms: None,
                    text: self.tr(
                        "calibration.loopback.silent",
                        &[("heard", heard.to_string())],
                    ),
                }
            }
            LoopbackResult::Uneven { spread_ms } => {
                return UiEvent::Calibrated {
                    latency_ms: None,
                    text: self.tr(
                        "calibration.loopback.uneven",
                        &[("spread", format!("{spread_ms:.0}"))],
                    ),
                }
            }
        };

        // A lean comes only from clicks after this cable value (review #202).
        self.reconfigure(|s| {
            s.click_ms = None;
            s.loopback_ms = Some(latency_ms);
            s.latency_ms = latency_ms;
            s.latency_from = LatencyFrom::Loopback;
        });
        let text = self.tr(
            "calibration.loopback.done",
            &[("ms", format!("{latency_ms:.0}"))],
        );
        UiEvent::Calibrated {
            latency_ms: Some(latency_ms),
            text,
        }
    }

    /// The player's lean at the clicks (click − loopback), when both were measured and the
    /// cable value is in use: −14 = 14 ms before the click.
    pub fn lean_ms(&self) -> Option<f64> {
        let s = &self.settings;
        if s.latency_from != LatencyFrom::Loopback {
            return None;
        }
        Some(s.click_ms? - s.loopback_ms?)
    }

    /// Where the latency comes from, in words, for next to its value; "" when typed in.
    pub fn latency_note(&self) -> String {
        match (self.settings.latency_from, self.lean_ms()) {
            (LatencyFrom::Manual, _) => String::new(),
            (LatencyFrom::Click, _) => self.tr("latency.from.click", &[]),
            (LatencyFrom::Loopback, None) => self.tr("latency.from.loopback", &[]),
            (LatencyFrom::Loopback, Some(lean)) => {
                self.tr("latency.from.loopback.lean", &[("lean", signed_ms(lean))])
            }
        }
    }

    /// The side for "Neu kalibrieren?" (LH-EIN-14). Never with a cable value: that holds no
    /// lean, so a steady lean in the run is the player's (LH-BEW-09), not the latency's.
    pub(crate) fn latency_hint(&self, s: &Summary, pocket: Pocket) -> Option<Side> {
        if self.settings.latency_from == LatencyFrom::Loopback {
            return None;
        }
        s.recalibrate(pocket)
    }
}

/// Another input or output (device or connection) has another latency: its measurements no
/// longer hold. The value stays as a starting point, typed in from now on (review #202).
pub(crate) fn forget_on_switch(before: &Settings, after: &mut Settings) {
    let same = before.input_device == after.input_device
        && before.output_device == after.output_device
        && before.input_kind == after.input_kind;
    if same {
        return;
    }
    after.loopback_ms = None;
    after.click_ms = None;
    after.latency_from = LatencyFrom::Manual;
}

/// Whole ms with sign: "−14 ms", "+6 ms", "0 ms".
fn signed_ms(ms: f64) -> String {
    let ms = ms.round();
    if ms < 0.0 {
        return format!("{MINUS}{:.0}", -ms);
    }
    if ms > 0.0 {
        return format!("+{ms:.0}");
    }
    "0".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::QaOptions;

    /// A core on a fresh data folder with the silent output.
    fn core(name: &str) -> (Core, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("kontra-cal-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut core = Core::new(&dir, QaOptions::default());
        core.update_settings(|s| s.output_device = Some("kontra:null".into()));
        (core, dir)
    }

    fn clicks(latency_ms: f64) -> CalibrationResult {
        CalibrationResult::Measured {
            latency_ms,
            spread_ms: 1.0,
        }
    }

    fn cable(latency_ms: f64) -> LoopbackResult {
        LoopbackResult::Measured {
            latency_ms,
            spread_ms: 0.5,
        }
    }

    // Review #202: a player 30 ms ahead at 20 ms latency is heard 10 ms before the click; the
    // latency is clamped to 0, the click value keeps −10 for a later lean.
    #[test]
    fn keeps_the_click_median_unclamped() {
        let (mut core, dir) = core("unclamped");
        core.calibrated(clicks(-10.0));
        assert_eq!(core.settings.latency_ms, 0.0);
        assert_eq!(core.settings.click_ms, Some(-10.0));
        let _ = std::fs::remove_dir_all(dir);
    }

    // Review #202: a click value from before the cable is no lean; only clicks after it are.
    #[test]
    fn cable_forgets_earlier_clicks() {
        let (mut core, dir) = core("cable-after");
        core.calibrated(clicks(-10.0));
        core.looped(cable(20.0));
        assert_eq!(core.lean_ms(), None);
        assert_eq!(core.latency_note(), core.tr("latency.from.loopback", &[]));
        core.calibrated(clicks(-10.0));
        assert_eq!(core.lean_ms(), Some(-30.0));
        let _ = std::fs::remove_dir_all(dir);
    }

    // Review #202: another device has another latency; its measurements no longer hold.
    #[test]
    fn device_change_forgets_measurements() {
        let (mut core, dir) = core("device");
        core.looped(cable(20.0));
        core.calibrated(clicks(5.0));
        core.update_settings(|s| s.output_device = Some("kontra:null".into()));
        assert_eq!(core.lean_ms(), Some(-15.0), "same device: kept");

        core.reconfigure(|s| s.input_device = Some("other".into()));
        let s = &core.settings;
        assert_eq!((s.loopback_ms, s.click_ms), (None, None));
        assert_eq!(s.latency_from, LatencyFrom::Manual);
        assert_eq!(s.latency_ms, 20.0, "the value stays as a starting point");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn signs_whole_ms() {
        assert_eq!(signed_ms(-14.4), "\u{2212}14");
        assert_eq!(signed_ms(6.0), "+6");
        assert_eq!(signed_ms(0.3), "0");
    }
}
