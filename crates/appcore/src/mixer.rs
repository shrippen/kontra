//! The mixer's settings as the player's gains (LH-WIE-03, LH-EIN-08, LH-EIN-17).

use crate::{instrument_from, Settings};
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
            drive: settings.monitor_drive as f32,
            width: settings.monitor_width as f32,
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
    /// "Neutral": no shaping and no effects, the limiter on (it protects, it does not colour).
    /// Returns a hint as `remix` does.
    pub fn neutral_shaping(&mut self) -> Option<String> {
        self.remix(|s| {
            s.monitor_low_db = 0.0;
            s.monitor_mid_db = 0.0;
            s.monitor_high_db = 0.0;
            s.monitor_compress = 0.0;
            s.monitor_warmth = 0.0;
            s.monitor_drive = 0.0;
            s.monitor_width = 0.0;
            s.monitor_limit = true;
        })
    }
}
