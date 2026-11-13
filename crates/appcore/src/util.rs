//! Small, dependency-free helpers shared across modules (moved out of `lib.rs` to keep it
//! under its line budget, `tests/physical_design.rs`; no behaviour change).

use std::path::Path;

/// A seed that differs between rounds (quiz and ear training questions).
pub(crate) fn time_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |d| d.as_nanos() as u64)
}

pub(crate) fn quantum_too_long_ms(frames: u32, rate: u32) -> Option<f64> {
    let ms = f64::from(frames) / f64::from(rate.max(1)) * 1000.0;
    (ms > 512.0 / 48.0 + 0.05).then_some(ms)
}

pub(crate) fn file_name(path: &Path) -> String {
    path.file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into())
}

pub(crate) fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// The name a piece is shown by: its title, or the file name when the title names nothing
/// ("", "001", "1." from Guitar Pro templates; QA 27.10., J4).
pub(crate) fn piece_name(title: &str, path: &Path) -> String {
    if title.chars().any(char::is_alphabetic) {
        return title.to_string();
    }
    path.file_stem()
        .map_or_else(|| title.to_string(), |s| s.to_string_lossy().into())
}
