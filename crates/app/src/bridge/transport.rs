//! Transport and loops: play/pause, the probe, mode and view, the loop bounds and its passes
//! (moved out of `bridge.rs` to keep it under its line budget, `tests/physical_design.rs`; no
//! behaviour change).

use super::{q, qobject};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use kontra_appcore::LoopEdge;

impl qobject::Controller {
    pub(super) fn toggle_play(mut self: Pin<&mut Self>) {
        if self.as_ref().rust().core.is_playing() {
            self.as_mut().pause();
            return;
        }
        let finished_before = !self.as_ref().summary_json().is_empty();
        if finished_before {
            // A finished run starts again from the beginning.
            self.as_mut().core().stop_live();
            self.as_mut().set_summary_json(QString::default());
            self.as_mut().arrangement_loaded();
        }
        let events = self.as_mut().core().play();
        self.as_mut().apply(events);
        // A start that resolves other devices forgets the measurements (review #202).
        self.as_mut().refresh_latency_note();
        let playing = self.as_ref().rust().core.is_playing();
        self.as_mut().set_playing(playing);
    }

    pub(super) fn pause(mut self: Pin<&mut Self>) {
        self.as_mut().core().pause();
        self.as_mut().set_playing(false);
    }

    pub(super) fn start_probe(mut self: Pin<&mut Self>) {
        self.as_mut().set_summary_json(QString::default());
        self.as_mut().set_holding_index(-1);
        self.as_mut().arrangement_loaded();
        let events = self.as_mut().core().start_probe();
        self.as_mut().apply(events);
        let (playing, probing) = {
            let this = self.as_ref();
            let core = &this.rust().core;
            (core.is_playing(), core.is_probing())
        };
        self.as_mut().set_probing(probing);
        self.as_mut().set_playing(playing);
    }

    pub(super) fn loop_at(mut self: Pin<&mut Self>, start_s: f64, end_s: f64) -> bool {
        let Some((from, to)) = self.as_ref().rust().core.bars_at(start_s, end_s) else {
            return false;
        };
        let from = i32::try_from(from).unwrap_or(0);
        let to = i32::try_from(to).unwrap_or(0);
        self.as_mut().loop_bars(from, to);
        true
    }

    pub(super) fn rewind(mut self: Pin<&mut Self>) {
        let start = self.as_mut().core().rewind();
        self.as_mut().set_playing(false);
        self.as_mut().set_position(start);
        self.as_mut().set_summary_json(QString::default());
        self.as_mut().set_holding_index(-1);
        self.as_mut().arrangement_loaded();
    }

    pub(super) fn change_speed(mut self: Pin<&mut Self>, speed: f64) {
        self.as_mut().core().set_speed(speed);
        let speed = self.as_ref().rust().core.speed();
        self.as_mut().set_speed(speed);
    }

    pub(super) fn change_mode(mut self: Pin<&mut Self>, mode: &QString) {
        let mode = mode.to_string();
        // Leaving the workshop ends the building blocks: back to the bars chosen before them.
        if self.as_mut().core().set_mode(&mode) {
            self.as_mut().refresh_arrangement();
        }
        self.as_mut().after_loop();
        self.as_mut().set_mode(q(&mode));
        self.as_mut().reset_passes();
        self.as_mut().rewind();
    }

    pub(super) fn change_view(mut self: Pin<&mut Self>, view: &QString) {
        let view = view.to_string();
        self.as_mut().core().set_view(&view);
        self.as_mut().set_view(q(&view));
    }

    pub(super) fn loop_bars(mut self: Pin<&mut Self>, from: i32, to: i32) {
        let from = usize::try_from(from).unwrap_or(0);
        let to = usize::try_from(to).unwrap_or(0);
        let changed = self.as_mut().core().set_loop(from, to);
        if changed {
            self.as_mut().set_playing(false);
            self.as_mut().refresh_arrangement();
        }
        self.as_mut().refresh_density();
        self.as_mut().refresh_chain();
        self.as_mut().reset_passes();
        let (start, end) = self.as_ref().rust().core.loop_seconds();
        self.as_mut().set_loop_start(start);
        self.as_mut().set_loop_end(end);
    }

    pub(super) fn loop_score_bar(mut self: Pin<&mut Self>, bar: i32) {
        let printed = u32::try_from(bar).unwrap_or(0);
        let Some(index) = self.as_ref().rust().core.score_bar_index(printed) else {
            return;
        };
        let index = i32::try_from(index).unwrap_or(0);
        self.as_mut().loop_bars(index, index);
    }

    pub(super) fn loop_edge(mut self: Pin<&mut Self>, edge: &QString) {
        let edge = match edge.to_string().as_str() {
            "start" => LoopEdge::Start,
            "end" => LoopEdge::End,
            _ => return,
        };
        let position = *self.as_ref().position();
        if self.as_mut().core().set_loop_edge(edge, position) {
            self.as_mut().set_playing(false);
            self.as_mut().refresh_arrangement();
        }
        self.as_mut().refresh_density();
        self.as_mut().refresh_chain();
        self.as_mut().reset_passes();
        let (start, end) = self.as_ref().rust().core.loop_seconds();
        self.as_mut().set_loop_start(start);
        self.as_mut().set_loop_end(end);
    }

    pub(super) fn reset_passes(mut self: Pin<&mut Self>) {
        self.as_mut().set_passes(0);
        self.as_mut().set_pass_rate(-1.0);
    }

    pub(super) fn clear_loop(mut self: Pin<&mut Self>) {
        self.as_mut().reset_passes();
        if self.as_mut().core().clear_loop() {
            self.as_mut().set_playing(false);
            self.as_mut().refresh_arrangement();
        }
        self.as_mut().refresh_density();
        self.as_mut().refresh_chain();
        self.as_mut().set_loop_start(-1.0);
        self.as_mut().set_loop_end(-1.0);
    }
}
