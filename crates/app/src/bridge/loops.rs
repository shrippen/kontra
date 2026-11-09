//! Loops: saved loops, target speed, note density and building blocks.

use super::{q, qobject};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use kontra_appcore::{Blocks, ChainOrder, Density, DensityLock};

impl qobject::Controller {
    pub(super) fn save_loop(mut self: Pin<&mut Self>) {
        if self.as_mut().core().save_loop() {
            self.as_mut().refresh_practice();
        }
    }

    pub(super) fn remove_loop(mut self: Pin<&mut Self>, index: i32) {
        let Ok(index) = usize::try_from(index) else {
            return;
        };
        self.as_mut().core().remove_loop(index);
        self.as_mut().refresh_practice();
    }

    pub(super) fn change_target_speed(mut self: Pin<&mut Self>, speed: f64) {
        self.as_mut().core().set_target_speed(speed);
        self.as_mut().refresh_practice();
    }

    /// Density of the current section; a pending suggestion goes (it was for another one).
    pub(super) fn refresh_density(mut self: Pin<&mut Self>) {
        let (density, lock) = self.as_ref().rust().core.density();
        self.as_mut().set_density(i32::from(density.percent()));
        self.as_mut().set_density_fixed(lock == DensityLock::Fixed);
        self.as_mut().set_density_suggestion(0);
    }

    pub(super) fn refresh_chain(mut self: Pin<&mut Self>) {
        let chain = self.as_ref().rust().core.chain_json();
        self.as_mut().set_chain_json(q(&chain));
    }

    pub(super) fn start_chain(mut self: Pin<&mut Self>, blocks: &QString, order: &QString) {
        let order = match order.to_string().as_str() {
            "backward" => ChainOrder::Backward,
            _ => ChainOrder::Forward,
        };
        // "off", "2", "4" (bars per block) or "phrases".
        let blocks = match blocks.to_string().as_str() {
            "phrases" => Some(Blocks::Phrases),
            size => size.parse().ok().filter(|n| *n > 0).map(Blocks::Bars),
        };
        let changed = match blocks {
            Some(blocks) => self.as_mut().core().start_chain(blocks, order),
            None => self.as_mut().core().stop_chain(),
        };
        if changed {
            self.as_mut().set_playing(false);
            self.as_mut().refresh_arrangement();
        }
        self.as_mut().after_loop();
    }

    /// The loop moved: density, building blocks, pass count and loop times follow.
    pub(super) fn after_loop(mut self: Pin<&mut Self>) {
        self.as_mut().refresh_density();
        self.as_mut().refresh_chain();
        self.as_mut().reset_passes();
        let (start, end) = self.as_ref().rust().core.loop_seconds();
        self.as_mut().set_loop_start(start);
        self.as_mut().set_loop_end(end);
    }

    pub(super) fn change_density(mut self: Pin<&mut Self>, percent: i32, lock: &QString) {
        let density = Density::from_percent(u8::try_from(percent.clamp(0, 100)).unwrap_or(100));
        let lock = if lock.to_string() == "fixed" {
            DensityLock::Fixed
        } else {
            DensityLock::Free
        };
        self.as_mut().core().set_density(density, lock);
        self.as_mut().set_playing(false);
        self.as_mut().refresh_arrangement();
    }

    pub(super) fn decline_density(mut self: Pin<&mut Self>) {
        self.as_mut().set_density_suggestion(0);
    }
}
