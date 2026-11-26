//! Tone shaping settings of the monitored bass (LH-EIN-08): plain data. `kontra_dsp` runs them.

use crate::fx::ChainSpec;
use crate::instrument::ToneBands;

/// Largest boost or cut of a band, in dB.
pub const SHAPING_RANGE_DB: f32 = 12.0;

/// Whether the limiter catches peaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Limit {
    On,
    Off,
}

impl Limit {
    /// The limiter amount: 1 catches peaks, 0 passes them.
    pub fn amount(self) -> f32 {
        match self {
            Limit::On => 1.0,
            Limit::Off => 0.0,
        }
    }
}

/// Settings of the tone shaper; `FLAT` changes nothing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shaping {
    /// Bands in dB, within ±`SHAPING_RANGE_DB`.
    pub low_db: f32,
    pub mid_db: f32,
    pub high_db: f32,
    /// Compressor amount, 0 (off) to 1 (strong).
    pub compress: f32,
    /// Saturation, 0 (off) to 1 (warm).
    pub warmth: f32,
    /// The effects after the tone shaping (overdrive, widener, ...). The values act at once
    /// (gliding); a different shape (effects, order, on/off) needs `Shaper::swap_chain`.
    pub chain: ChainSpec,
    pub limit: Limit,
    /// Where the bands sit, from the instrument profile; changes at once, not gliding.
    pub bands: ToneBands,
}

impl Shaping {
    pub const FLAT: Shaping = Shaping {
        low_db: 0.0,
        mid_db: 0.0,
        high_db: 0.0,
        compress: 0.0,
        warmth: 0.0,
        chain: ChainSpec::EMPTY,
        limit: Limit::Off,
        bands: ToneBands::BASS,
    };

    /// The settings with every value inside its range.
    pub fn clamped(self) -> Shaping {
        let band = |db: f32| db.clamp(-SHAPING_RANGE_DB, SHAPING_RANGE_DB);
        Shaping {
            low_db: band(self.low_db),
            mid_db: band(self.mid_db),
            high_db: band(self.high_db),
            compress: self.compress.clamp(0.0, 1.0),
            warmth: self.warmth.clamp(0.0, 1.0),
            chain: self.chain.clamped(),
            limit: self.limit,
            bands: self.bands,
        }
    }
}

impl Default for Shaping {
    fn default() -> Self {
        Shaping::FLAT
    }
}
