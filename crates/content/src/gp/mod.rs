//! Guitar Pro formats: binary 3-5, GPX container (6) and zip container (7/8), both with GPIF XML.

pub(crate) mod binary;
mod gpif;
pub(crate) mod gpx;

use crate::error::ImportError;
use crate::score::Score;
use crate::zip::Zip;

/// Path of the score inside a Guitar Pro 7/8 archive.
const GP7_SCORE: &str = "Content/score.gpif";
/// Name of the score inside a Guitar Pro 6 container.
const GPX_SCORE: &str = "score.gpif";

/// Guitar Pro 6 (`.gpx`).
pub(crate) fn gpx(data: &[u8]) -> Result<Score, ImportError> {
    gpif::parse(&gpx::file(data, GPX_SCORE)?)
}

/// Guitar Pro 7/8 (`.gp`).
pub(crate) fn gp7(data: &[u8]) -> Result<Score, ImportError> {
    let archive = Zip::open(data)?;
    gpif::parse(&archive.read(GP7_SCORE)?)
}
