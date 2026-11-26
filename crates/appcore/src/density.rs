//! Note density of a section in the app (LH-MOD-06): thinning the notes and its hint.

use crate::{call, Core, DensityLock, PracticeMode, SectionDensity, UiEvent};
use kontra_session::Density;
use std::path::Path;

impl Core {
    /// Note density of the current section (the loop, else the whole piece) and its lock.
    pub fn density(&self) -> (Density, DensityLock) {
        self.stored_density()
            .map_or((Density::FULL, DensityLock::Free), |d| {
                let lock = if d.fixed {
                    DensityLock::Fixed
                } else {
                    DensityLock::Free
                };
                (Density::from_percent(d.percent), lock)
            })
    }

    /// Sets the note density of the current section (LH-MOD-06); the next run plays it.
    pub fn set_density(&mut self, density: Density, lock: DensityLock) {
        let section = self.section_key();
        let Some(path) = self.loaded_path().map(Path::to_path_buf) else {
            return;
        };
        let Some(entry) = self.library.entry_mut(&path) else {
            return;
        };
        entry.densities.retain(|d| d.section != section);
        entry.densities.push(SectionDensity {
            section,
            percent: density.percent(),
            fixed: lock == DensityLock::Fixed,
        });
        self.save_library();
        self.rethin();
    }

    /// The loop as stored with a section's density.
    fn section_key(&self) -> Option<(f64, f64)> {
        self.loop_range().map(|r| (r.start_s, r.end_s))
    }

    /// The density stored for the current section, else the whole piece's.
    fn stored_density(&self) -> Option<&SectionDensity> {
        let section = self.section_key();
        let entry = self.library.entry(self.loaded_path()?)?;
        let own = entry.densities.iter().find(|d| match (d.section, section) {
            (Some((a, b)), Some((c, e))) => (a - c).abs() < 0.01 && (b - e).abs() < 0.01,
            _ => false,
        });
        own.or_else(|| entry.densities.iter().find(|d| d.section.is_none()))
    }

    /// Thins the loaded notes to the current density; true if they changed, which ends a
    /// running practice (densities change only between runs).
    pub(crate) fn rethin(&mut self) -> bool {
        let (density, _) = self.density();
        let call = self.call_wanted();
        let Some(loaded) = self.loaded.as_mut() else {
            return false;
        };
        let thinned = call::shape(&loaded.full, density, call);
        if thinned.notes == loaded.arrangement.notes {
            return false;
        }
        loaded.arrangement = thinned;
        self.stop_live();
        true
    }

    /// The suggestion after a pass or run with `hit_rate` (LH-MOD-06): song and workshop only,
    /// not for a fixed section, not when switched off, not after a probe (it only judges).
    pub(crate) fn density_hint(&self, hit_rate: f64) -> Option<UiEvent> {
        if !self.settings.adaptive
            || !matches!(
                self.settings.mode,
                PracticeMode::Song | PracticeMode::Workshop
            )
        {
            return None;
        }
        if self.probe.is_some() || self.call_on() {
            return None;
        }
        let (density, lock) = self.density();
        if lock == DensityLock::Fixed {
            return None;
        }
        let next = density.suggest(hit_rate)?;
        let key = if next > density {
            "density.more"
        } else {
            "density.fewer"
        };
        Some(UiEvent::DensitySuggestion {
            percent: next.percent(),
            text: self.tr(key, &[("percent", format!("{:.0}", hit_rate * 100.0))]),
        })
    }
}
