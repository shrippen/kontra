//! Settings: devices, instrument, tuning, scoring, mixer, latency and calibration.

use super::{connection_key, instrument_key, q, qobject};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use kontra_appcore::{Bass, InstrumentChoice};

impl qobject::Controller {
    pub(super) fn change_connection(mut self: Pin<&mut Self>, key: &QString) {
        let kind = match key.to_string().as_str() {
            "laptop" => kontra_appcore::InputKind::LaptopJack,
            "midi" => kontra_appcore::InputKind::Midi,
            "mic" => kontra_appcore::InputKind::Microphone,
            _ => kontra_appcore::InputKind::Line,
        };
        let result = self.as_mut().core().retune(|s| s.input_kind = kind);
        self.as_mut().refresh_instrument();
        self.as_mut().refresh_devices();
        self.as_mut().after_retune(result);
    }

    pub(super) fn refresh_devices(mut self: Pin<&mut Self>) {
        self.as_mut().refresh_latency_note();
        let midi = self.as_ref().rust().core.midi_port_name();
        self.as_mut().set_midi_port_name(q(&midi));
        let ports = self.as_ref().rust().core.midi_ports_json();
        self.as_mut().set_midi_ports_json(q(&ports));
        let inputs = self.as_ref().rust().core.devices_json(true);
        let outputs = self.as_ref().rust().core.devices_json(false);
        self.as_mut().set_input_devices_json(q(&inputs));
        self.as_mut().set_output_devices_json(q(&outputs));
    }

    pub(super) fn choose_input(mut self: Pin<&mut Self>, id: &QString) {
        let id = id.to_string();
        let value = (!id.is_empty()).then(|| id.clone());
        self.as_mut().core().reconfigure(|s| s.input_device = value);
        self.as_mut().set_input_device(q(&id));
        self.as_mut().refresh_latency_note();
    }

    pub(super) fn choose_midi(mut self: Pin<&mut Self>, name: &QString) {
        self.as_mut().core().choose_midi(&name.to_string());
        self.as_mut().refresh_devices();
    }

    pub(super) fn choose_output(mut self: Pin<&mut Self>, id: &QString) {
        let id = id.to_string();
        let value = (!id.is_empty()).then(|| id.clone());
        self.as_mut()
            .core()
            .reconfigure(|s| s.output_device = value);
        self.as_mut().set_output_device(q(&id));
        self.as_mut().refresh_latency_note();
    }

    /// Another device forgets the measurements (review #202): the note follows.
    pub(super) fn refresh_latency_note(mut self: Pin<&mut Self>) {
        let note = self.as_ref().rust().core.latency_note();
        self.as_mut().set_latency_note(q(&note));
    }

    pub(super) fn change_instrument(mut self: Pin<&mut Self>, strings: i32, fretless: bool) {
        let result = self.as_mut().core().retune(|s| {
            let max = i32::from(s.instrument.strings_max());
            s.strings = u8::try_from(strings.clamp(4, max)).unwrap_or(4);
            if s.instrument != Bass::Double {
                s.instrument = if fretless {
                    Bass::Fretless
                } else {
                    Bass::Fretted
                };
            }
        });
        self.as_mut().refresh_instrument();
        self.as_mut().after_retune(result);
    }

    pub(super) fn choose_instrument(mut self: Pin<&mut Self>, key: &QString) {
        let choice = match key.to_string().as_str() {
            "double" => InstrumentChoice::Double,
            "midi" => InstrumentChoice::Midi,
            _ => InstrumentChoice::EBass,
        };
        let result = self.as_mut().core().choose_instrument(choice);
        self.as_mut().refresh_instrument();
        self.as_mut().refresh_devices();
        self.as_mut().after_retune(result);
    }

    /// Instrument and input as stored, for setup and settings.
    fn refresh_instrument(mut self: Pin<&mut Self>) {
        let (s, choice) = {
            let this = self.as_ref();
            let core = &this.rust().core;
            (core.settings().clone(), core.instrument_choice())
        };
        self.as_mut().set_strings(i32::from(s.strings));
        self.as_mut().set_fretless(s.instrument == Bass::Fretless);
        self.as_mut().set_instrument(q(instrument_key(choice)));
        self.as_mut()
            .set_laptop_jack(s.input_kind == kontra_appcore::InputKind::LaptopJack);
        self.as_mut()
            .set_connection(q(connection_key(s.input_kind)));
    }

    /// After an instrument change: new strings everywhere, or the error.
    pub(super) fn after_retune(mut self: Pin<&mut Self>, result: Result<(), String>) {
        self.as_mut().set_playing(false);
        self.as_mut().refresh_tuning();
        if self.as_ref().rust().core.arrangement().is_some() {
            self.as_mut().refresh_arrangement();
        }
        if let Err(e) = result {
            self.as_mut().show_error(&e);
        }
    }

    pub(super) fn refresh_tuning(mut self: Pin<&mut Self>) {
        let (tuning, profile, guided, notes) = {
            let this = self.as_ref();
            let core = &this.rust().core;
            (
                core.settings().tuning.clone(),
                core.profile_tuning_json(),
                core.tuning_json(),
                core.note_choices_json(),
            )
        };
        self.as_mut().set_tuning(q(&tuning));
        self.as_mut().set_profile_tuning_json(q(&profile));
        self.as_mut().set_tuning_json(q(&guided));
        self.as_mut().set_note_choices_json(q(&notes));
    }

    pub(super) fn change_tuning(mut self: Pin<&mut Self>, name: &QString) {
        let result = self.as_mut().core().set_tuning(&name.to_string());
        self.as_mut().after_retune(result);
    }

    pub(super) fn change_custom_string(mut self: Pin<&mut Self>, string: i32, midi: i32) {
        let (Ok(string), Ok(midi)) = (usize::try_from(string), u8::try_from(midi)) else {
            return;
        };
        let result = self.as_mut().core().set_custom_string(string, midi);
        self.as_mut().after_retune(result);
    }

    pub(super) fn change_reference(mut self: Pin<&mut Self>, a4_hz: f64) {
        let result = self.as_mut().core().set_reference(a4_hz);
        let kept = self.as_ref().rust().core.settings().a4_hz;
        self.as_mut().set_a4_hz(kept);
        self.as_mut().after_retune(result.map(|_| ()));
    }

    pub(super) fn change_window(mut self: Pin<&mut Self>, window_ms: f64) {
        let kept = self.as_mut().core().set_window(window_ms);
        self.as_mut().set_window_ms(kept);
        self.as_mut().set_strictness(q("custom"));
        self.as_mut().set_playing(false);
    }

    pub(super) fn change_string_colors(mut self: Pin<&mut Self>, preset: &QString) {
        let preset = preset.to_string();
        if !["kante", "mono", "bright"].contains(&preset.as_str()) {
            return;
        }
        let value = q(&preset);
        // Display only: a running take goes on.
        self.as_mut()
            .core()
            .update_settings(|s| s.string_colors = preset);
        self.as_mut().set_string_colors(value);
    }

    pub(super) fn change_left_handed(mut self: Pin<&mut Self>, on: bool) {
        self.as_mut().core().update_settings(|s| s.left_handed = on);
        self.as_mut().set_left_handed(on);
    }

    pub(super) fn change_options(
        mut self: Pin<&mut Self>,
        metronome: bool,
        reference: bool,
        low_string_on_top: bool,
    ) {
        self.as_mut().core().reconfigure(|s| {
            s.metronome = metronome;
            s.reference_tones = reference;
            s.low_string_on_top = low_string_on_top;
        });
        self.as_mut().set_metronome(metronome);
        self.as_mut().set_reference(reference);
        self.as_mut().set_low_string_on_top(low_string_on_top);
        self.as_mut().set_playing(false);
    }

    pub(super) fn change_scoring(mut self: Pin<&mut Self>, strictness: &QString, pitch_cents: f64) {
        let strictness = strictness.to_string();
        if !["auto", "loose", "normal", "strict", "custom"].contains(&strictness.as_str()) {
            return;
        }
        let cents = pitch_cents.clamp(0.0, 100.0);
        let value = q(&strictness);
        self.as_mut().core().reconfigure(|s| {
            s.strictness = strictness;
            s.pitch_cents = cents;
        });
        self.as_mut().set_strictness(value);
        self.as_mut().set_pitch_cents(cents);
        self.as_mut().set_playing(false);
    }

    pub(super) fn change_pocket(mut self: Pin<&mut Self>, pocket: &QString) {
        let pocket = pocket.to_string();
        if !["free", "onbeat", "laidback", "pushed"].contains(&pocket.as_str()) {
            return;
        }
        let value = q(&pocket);
        // Read when the run ends: a running take keeps going.
        self.as_mut().core().update_settings(|s| s.pocket = pocket);
        self.as_mut().set_pocket(value);
    }

    pub(super) fn change_play_marks(mut self: Pin<&mut Self>, marks: &QString) {
        let marks = marks.to_string();
        if !["quiet", "off", "full"].contains(&marks.as_str()) {
            return;
        }
        let value = q(&marks);
        // Display only: no need to stop a running take.
        self.as_mut()
            .core()
            .update_settings(|s| s.play_marks = marks);
        self.as_mut().set_play_marks(value);
    }

    pub(super) fn change_motion(mut self: Pin<&mut Self>, motion: &QString) {
        let motion = motion.to_string();
        if !["system", "reduce", "full"].contains(&motion.as_str()) {
            return;
        }
        let value = q(&motion);
        self.as_mut()
            .core()
            .update_settings(|s| s.motion = motion.clone());
        self.as_mut().set_motion(value);
        // Less motion means the page-wise tab instead of the scrolling lane.
        if motion == "reduce" && self.as_ref().view().to_string() == "lane" {
            self.as_mut().change_view(&q("tab"));
        }
    }

    pub(super) fn change_count_in(mut self: Pin<&mut Self>, on: bool) {
        self.as_mut()
            .core()
            .reconfigure(|s| s.count_in_bars = u32::from(on));
        self.as_mut().set_count_in(on);
        self.as_mut().set_playing(false);
    }

    pub(super) fn change_latency(mut self: Pin<&mut Self>, latency_ms: f64) {
        let value = latency_ms.clamp(0.0, 500.0);
        self.as_mut().core().reconfigure(|s| {
            s.latency_ms = value;
            s.latency_from = kontra_appcore::LatencyFrom::Manual;
        });
        self.as_mut().set_latency_ms(value);
        self.as_mut().set_latency_note(QString::default());
        self.as_mut().set_playing(false);
    }

    pub(super) fn change_volume(mut self: Pin<&mut Self>, bus: &QString, value: f64) {
        let value = value.clamp(0.0, 1.0);
        let hint = match bus.to_string().as_str() {
            "backing" => {
                self.as_mut().set_backing_volume(value);
                self.as_mut().core().remix(|s| s.backing_volume = value)
            }
            "metronome" => {
                self.as_mut().set_metronome_volume(value);
                self.as_mut().core().remix(|s| s.metronome_volume = value)
            }
            "tone" => {
                self.as_mut().set_tone_volume(value);
                self.as_mut().core().remix(|s| s.tone_volume = value)
            }
            "drums" => {
                self.as_mut().set_drums_volume(value);
                self.as_mut().core().remix(|s| s.drums_volume = value)
            }
            "bass" => {
                self.as_mut().set_bass_volume(value);
                self.as_mut().core().remix(|s| s.bass_volume = value)
            }
            "prompt" => {
                self.as_mut().set_prompt_volume(value);
                self.as_mut().core().remix(|s| s.prompt_volume = value)
            }
            "monitor" => {
                self.as_mut().set_monitor_volume(value);
                self.as_mut().core().remix(|s| s.monitor_volume = value)
            }
            _ => None,
        };
        if let Some(hint) = hint {
            self.as_mut().set_message(q(&hint));
        }
    }

    pub(super) fn change_monitor(mut self: Pin<&mut Self>, on: bool) {
        self.as_mut().set_monitor(on);
        if let Some(hint) = self.as_mut().core().remix(|s| s.monitor = on) {
            self.as_mut().set_message(q(&hint));
        }
    }

    pub(super) fn change_shaping(mut self: Pin<&mut Self>, band: &QString, value: f64) {
        let range = kontra_appcore::Core::shaping_range_db();
        let db = value.clamp(-range, range);
        let hint = match band.to_string().as_str() {
            "low" => {
                self.as_mut().set_monitor_low_db(db);
                self.as_mut().core().remix(|s| s.monitor_low_db = db)
            }
            "mid" => {
                self.as_mut().set_monitor_mid_db(db);
                self.as_mut().core().remix(|s| s.monitor_mid_db = db)
            }
            "high" => {
                self.as_mut().set_monitor_high_db(db);
                self.as_mut().core().remix(|s| s.monitor_high_db = db)
            }
            "compress" => {
                let amount = value.clamp(0.0, 1.0);
                self.as_mut().set_monitor_compress(amount);
                self.as_mut().core().remix(|s| s.monitor_compress = amount)
            }
            "warmth" => {
                let amount = value.clamp(0.0, 1.0);
                self.as_mut().set_monitor_warmth(amount);
                self.as_mut().core().remix(|s| s.monitor_warmth = amount)
            }
            _ => None,
        };
        if let Some(hint) = hint {
            self.as_mut().set_message(q(&hint));
        }
    }

    pub(super) fn reset_shaping(mut self: Pin<&mut Self>) {
        self.as_mut().set_monitor_low_db(0.0);
        self.as_mut().set_monitor_mid_db(0.0);
        self.as_mut().set_monitor_high_db(0.0);
        self.as_mut().set_monitor_compress(0.0);
        self.as_mut().set_monitor_warmth(0.0);
        let hint = self.as_mut().core().remix(|s| {
            s.monitor_low_db = 0.0;
            s.monitor_mid_db = 0.0;
            s.monitor_high_db = 0.0;
            s.monitor_compress = 0.0;
            s.monitor_warmth = 0.0;
        });
        if let Some(hint) = hint {
            self.as_mut().set_message(q(&hint));
        }
    }

    pub(super) fn start_calibration(mut self: Pin<&mut Self>) {
        self.as_mut().set_playing(false);
        self.as_mut().set_tuner_active(false);
        match self.as_mut().core().start_calibration() {
            Ok(()) => {
                let text = self.as_ref().rust().core.tr("calibration.running", &[]);
                self.as_mut().set_calibration_text(q(&text));
                self.as_mut().set_calibrating(true);
            }
            Err(e) => self.as_mut().show_error(&e),
        }
        // A start on other devices forgets the measurements (review #202), also when it fails.
        self.as_mut().refresh_latency_note();
    }

    pub(super) fn start_loopback(mut self: Pin<&mut Self>) {
        self.as_mut().set_playing(false);
        self.as_mut().set_tuner_active(false);
        match self.as_mut().core().start_loopback() {
            Ok(()) => {
                let text = self
                    .as_ref()
                    .rust()
                    .core
                    .tr("calibration.loopback.running", &[]);
                self.as_mut().set_calibration_text(q(&text));
                self.as_mut().set_cable_measuring(true);
                self.as_mut().set_calibrating(true);
            }
            Err(e) => self.as_mut().show_error(&e),
        }
        // A start on other devices forgets the measurements (review #202), also when it fails.
        self.as_mut().refresh_latency_note();
    }
}
