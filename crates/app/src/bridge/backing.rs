//! Backing track: choice, bass removal, beat map and transpose (LH-BIB-06/09).

use super::{local_path, q, qobject};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;

impl qobject::Controller {
    pub(super) fn change_transpose(mut self: Pin<&mut Self>, semitones: i32) {
        if let Err(text) = self.as_mut().core().set_transpose(semitones) {
            self.as_mut().show_error(&text);
        }
        // The run goes on in the new key: what waits and where it is stay as they are.
        let kept = self.as_ref().rust().core.has_run();
        let holding = *self.as_ref().holding_index();
        self.as_mut().set_run_kept(kept);
        self.as_mut().refresh_arrangement();
        self.as_mut().set_run_kept(false);
        if kept {
            let position = self.as_ref().rust().core.position_s();
            self.as_mut().set_position(position);
            // The note waited for is the same, its name and fret are the new key's.
            let text = usize::try_from(holding)
                .ok()
                .and_then(|index| self.as_ref().rust().core.wait_text(index, None));
            if let Some(text) = text {
                self.as_mut().set_holding_index(holding);
                self.as_mut().set_holding_text(q(&text));
            }
        }
    }

    pub(super) fn choose_backing(mut self: Pin<&mut Self>, url: &QString) {
        let url = url.to_string();
        let path = (!url.is_empty()).then(|| local_path(&url));
        self.as_mut().core().set_backing(path, 0.0);
        let name = self.as_ref().rust().core.backing_name();
        self.as_mut().set_backing_name(q(&name));
        self.as_mut().show_stems();
        self.as_mut().set_playing(false);
    }

    pub(super) fn remove_bass(mut self: Pin<&mut Self>, on: bool) {
        self.as_mut().core().remove_bass(on);
        self.as_mut().show_stems();
        self.as_mut().set_playing(false);
    }

    pub(super) fn cancel_stems(mut self: Pin<&mut Self>) {
        self.as_mut().core().cancel_stems();
        self.as_mut().show_stems();
    }

    pub(super) fn forget_stems(mut self: Pin<&mut Self>) {
        self.as_mut().core().forget_stems();
        self.as_mut().show_stems();
        self.as_mut().set_playing(false);
    }

    pub(super) fn refresh_stems(mut self: Pin<&mut Self>) {
        self.as_mut().core().recheck_stems();
        self.as_mut().show_stems();
    }

    pub(super) fn show_stems(mut self: Pin<&mut Self>) {
        let state = self.as_mut().core().stems_json();
        self.as_mut().set_stems_json(q(&state));
    }

    pub(super) fn backing_json(&self) -> QString {
        q(&self.rust().core.backing_json())
    }

    pub(super) fn backing_analyse(mut self: Pin<&mut Self>) {
        let result = self.as_mut().core().backing_analyse();
        self.as_mut().after_retune(result);
    }

    pub(super) fn backing_adjust(
        mut self: Pin<&mut Self>,
        downbeat: i32,
        shift_ms: f64,
        bar: i32,
        bar_ms: f64,
    ) {
        let bar = usize::try_from(bar).ok().map(|b| (b, bar_ms));
        let result = self.as_mut().core().backing_adjust(downbeat, shift_ms, bar);
        self.as_mut().after_retune(result);
    }

    pub(super) fn backing_half_beat(mut self: Pin<&mut Self>) {
        let result = self.as_mut().core().backing_half_beat();
        self.as_mut().after_retune(result);
    }

    pub(super) fn backing_clear(mut self: Pin<&mut Self>) {
        let result = self.as_mut().core().backing_clear();
        self.as_mut().after_retune(result);
    }
}
