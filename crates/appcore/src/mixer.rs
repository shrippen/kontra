//! The mixer's settings as the player's gains (LH-WIE-03, LH-EIN-08, LH-EIN-17).

use crate::{instrument_from, standby, Settings};
use kontra_model::ChainSpec;
use kontra_session::{Gains, Limit, Shaping};

/// The mixer's volumes for the player; monitoring off mutes the input.
pub(crate) fn gains(settings: &Settings) -> Gains {
    let volume = |v: f64| v.clamp(0.0, 1.0) as f32;
    Gains {
        click: volume(settings.metronome_volume),
        tone: volume(settings.tone_volume),
        backing: volume(settings.backing_volume),
        drums: volume(settings.drums_volume),
        bass: volume(settings.bass_volume),
        monitor: if settings.monitor {
            volume(settings.monitor_volume)
        } else {
            0.0
        },
        // The shaper clamps the ranges itself.
        shaping: Shaping {
            low_db: settings.monitor_low_db as f32,
            mid_db: settings.monitor_mid_db as f32,
            high_db: settings.monitor_high_db as f32,
            compress: settings.monitor_compress as f32,
            warmth: settings.monitor_warmth as f32,
            chain: settings.monitor_chain,
            limit: if settings.monitor_limit {
                Limit::On
            } else {
                Limit::Off
            },
            bands: instrument_from(settings).tone_bands(),
        },
    }
}

/// Ear training plays its prompts as tones at the "Vorspielen" volume (round 8).
pub(crate) fn ear_gains(settings: &Settings) -> Gains {
    Gains {
        tone: settings.prompt_volume.clamp(0.0, 1.0) as f32,
        ..gains(settings)
    }
}

impl crate::Core {
    /// Changes the mixer (volumes, monitoring) and saves; a running practice hears it at once.
    /// Returns a hint when monitoring is on but cannot work.
    pub fn remix(&mut self, change: impl FnOnce(&mut Settings)) -> Option<String> {
        let was_on = self.settings.monitor;
        self.update_settings(change);
        let gains = gains(&self.settings);
        if let Some(live) = self.live.as_mut() {
            live.practice_mut().set_gains(gains);
        }
        if let Some(live_ear) = self.live_ear.as_mut() {
            live_ear.set_gains(ear_gains(&self.settings));
        }
        self.remix_standby();
        // The tuner's output opens or closes with "Bass hören".
        if self.tuner.is_some() && was_on != self.settings.monitor {
            let _ = self.start_tuner();
        }
        let change = if !was_on && self.settings.monitor {
            // Look for the input at once, not at the next retry.
            self.wake_standby();
            standby::MixerChange::MonitorOn
        } else {
            standby::MixerChange::Other
        };
        self.monitor_hint().or_else(|| self.standby_hint(change))
    }

    /// Monitoring is on, but the running practice has no output or devices at different rates.
    pub(crate) fn monitor_hint(&self) -> Option<String> {
        let live = self.live.as_ref()?;
        if !self.settings.monitor || live.can_monitor() || live.output_is_silent {
            return None;
        }
        Some(self.tr("hint.monitor.rate", &[]))
    }

    /// "Neutral": no shaping and no effects, the limiter on (it protects, it does not colour).
    /// Returns a hint as `remix` does.
    pub fn neutral_shaping(&mut self) -> Option<String> {
        self.remix(|s| {
            s.monitor_low_db = 0.0;
            s.monitor_mid_db = 0.0;
            s.monitor_high_db = 0.0;
            s.monitor_compress = 0.0;
            s.monitor_warmth = 0.0;
            s.monitor_chain = ChainSpec::EMPTY;
            s.monitor_limit = true;
        })
    }
}
