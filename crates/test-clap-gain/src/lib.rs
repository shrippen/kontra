//! A gain as CLAP plugin, for the tests of the plugin host (FX2). One parameter `gain` (a factor,
//! 0 to 4, default 1), state is that value as four bytes, latency is `LATENCY_SAMPLES`.
//!
//! Setting the gain to `GAIN_TRIGGER` makes the plugin change it by itself to `GAIN_REPORTED` and
//! report that as a parameter value event, the way a plugin's own editor does (tests of FX5).
//!
//! The same code builds a stereo and a mono variant (`Gain<2>`, `Gain<1>`); the crash and hang
//! plugins are in `examples/`.

mod editor;

pub use editor::{
    EditorState, EDITOR_CLOSE_GAIN, EDITOR_GAIN, EDITOR_MAX_WIDTH, EDITOR_RESIZED, EDITOR_SIZE,
};

use clack_extensions::audio_ports::{
    AudioPortFlags, AudioPortInfo, AudioPortInfoWriter, PluginAudioPorts, PluginAudioPortsImpl,
};
use clack_extensions::latency::{PluginLatency, PluginLatencyImpl};
use clack_extensions::params::{
    ParamDisplayWriter, ParamInfo, ParamInfoFlags, ParamInfoWriter, PluginAudioProcessorParams,
    PluginMainThreadParams, PluginParams,
};
use clack_extensions::state::{PluginState, PluginStateImpl};
use clack_plugin::events::event_types::ParamValueEvent;
use clack_plugin::events::spaces::CoreEventSpace;
use clack_plugin::events::Pckn;
use clack_plugin::prelude::*;
use clack_plugin::stream::{InputStream, OutputStream};
use std::ffi::CStr;
use std::fmt::Write as _;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU32, Ordering};

pub const PLUGIN_ID: &str = "de.arianw.kontra.test.gain";
pub const PLUGIN_ID_MONO: &str = "de.arianw.kontra.test.gain-mono";
pub const LATENCY_SAMPLES: u32 = 64;
pub const GAIN_PARAM_ID: u32 = 7;
pub const GAIN_MAX: f64 = 4.0;
/// Set as the gain, this makes the plugin report `GAIN_REPORTED` as its own change.
pub const GAIN_TRIGGER: f64 = 3.5;
pub const GAIN_REPORTED: f64 = 0.5;
const GAIN_DEFAULT: f32 = 1.0;
const GAIN_NAME: &[u8] = b"gain";

/// `CHANNELS` is the channel count of the one input and the one output port.
#[derive(Debug)]
pub struct Gain<const CHANNELS: u32>;

/// The gain factor as f32 bits, shared by the three threads.
#[derive(Debug)]
pub struct Shared {
    gain_bits: AtomicU32,
    channels: u32,
    /// A gain the editor set, as f32 bits, for the next block to report; `NO_TURN`: none.
    turned_bits: AtomicU32,
}

/// No knob turned since the last block (a NaN no editor sets).
const NO_TURN: u32 = u32::MAX;

impl Shared {
    fn gain(&self) -> f32 {
        f32::from_bits(self.gain_bits.load(Ordering::Relaxed))
    }

    fn set_gain(&self, value: f32) {
        self.gain_bits.store(value.to_bits(), Ordering::Relaxed);
    }

    /// The editor set the gain: the next block takes it and reports it.
    fn turn_knob(&self, value: f64) {
        self.turned_bits
            .store((value as f32).to_bits(), Ordering::Relaxed);
    }

    /// Reports a gain the editor set, as a real editor's change arrives at the host.
    fn report_turn(&self, output: &mut OutputEvents) {
        let bits = self.turned_bits.swap(NO_TURN, Ordering::Relaxed);
        if bits == NO_TURN {
            return;
        }
        let value = f32::from_bits(bits);
        self.set_gain(value);
        let event = ParamValueEvent::new(
            0,
            ClapId::new(GAIN_PARAM_ID),
            Pckn::match_all(),
            f64::from(value),
        );
        let _ = output.try_push(event);
    }

    /// Applies the parameter events of one block. The trigger value is answered with the plugin's
    /// own change, reported through `output`.
    fn apply(&self, events: &InputEvents, output: &mut OutputEvents) {
        for event in events {
            let Some(CoreEventSpace::ParamValue(e)) = event.as_core_event() else {
                continue;
            };
            if e.param_id().map(|id| id.get()) != Some(GAIN_PARAM_ID) {
                continue;
            }
            if e.value() != GAIN_TRIGGER {
                self.set_gain(e.value() as f32);
                continue;
            }
            self.set_gain(GAIN_REPORTED as f32);
            let reported = ParamValueEvent::new(
                e.header().time(),
                ClapId::new(GAIN_PARAM_ID),
                Pckn::match_all(),
                GAIN_REPORTED,
            );
            let _ = output.try_push(reported);
        }
    }
}

impl PluginShared<'_> for Shared {}

pub struct GainMainThread<'a> {
    shared: &'a Shared,
    host: HostMainThreadHandle<'a>,
    editor: std::cell::RefCell<EditorState>,
}

impl std::fmt::Debug for GainMainThread<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GainMainThread")
            .field("shared", &self.shared)
            .field("editor", &self.editor)
            .finish_non_exhaustive()
    }
}

impl<'a> PluginMainThread<'a, Shared> for GainMainThread<'a> {}

#[derive(Debug)]
pub struct GainProcessor<'a> {
    shared: &'a Shared,
}

impl<const CHANNELS: u32> Plugin for Gain<CHANNELS> {
    type AudioProcessor<'a> = GainProcessor<'a>;
    type Shared<'a> = Shared;
    type MainThread<'a> = GainMainThread<'a>;

    fn declare_extensions(builder: &mut PluginExtensions<Self>, _shared: Option<&Shared>) {
        builder
            .register::<PluginAudioPorts>()
            .register::<PluginParams>()
            .register::<PluginState>()
            .register::<PluginLatency>()
            .register::<clack_extensions::gui::PluginGui>()
            .register::<clack_extensions::timer::PluginTimer>();
        #[cfg(target_os = "linux")]
        builder.register::<clack_extensions::posix_fd::PluginPosixFd>();
    }
}

impl<const CHANNELS: u32> DefaultPluginFactory for Gain<CHANNELS> {
    fn get_descriptor() -> PluginDescriptor {
        use clack_plugin::plugin::features::{AUDIO_EFFECT, MONO, STEREO};

        let (id, name, layout) = if CHANNELS == 1 {
            (PLUGIN_ID_MONO, "Kontra Test Gain Mono", MONO)
        } else {
            (PLUGIN_ID, "Kontra Test Gain", STEREO)
        };
        PluginDescriptor::new(id, name)
            .with_vendor("Kontra Tests")
            .with_features([AUDIO_EFFECT, layout])
    }

    fn new_shared(_host: HostSharedHandle<'_>) -> Result<Shared, PluginError> {
        Ok(Shared {
            gain_bits: AtomicU32::new(GAIN_DEFAULT.to_bits()),
            channels: CHANNELS,
            turned_bits: AtomicU32::new(NO_TURN),
        })
    }

    fn new_main_thread<'a>(
        host: HostMainThreadHandle<'a>,
        shared: &'a Shared,
    ) -> Result<GainMainThread<'a>, PluginError> {
        Ok(GainMainThread {
            shared,
            host,
            editor: Default::default(),
        })
    }
}

impl<'a> PluginAudioProcessor<'a, Shared, GainMainThread<'a>> for GainProcessor<'a> {
    fn activate(
        _host: HostAudioProcessorHandle<'a>,
        _main_thread: &GainMainThread<'a>,
        shared: &'a Shared,
        _audio_config: PluginAudioConfiguration,
    ) -> Result<Self, PluginError> {
        Ok(Self { shared })
    }

    fn process(
        &mut self,
        _process: Process,
        mut audio: Audio,
        events: Events,
    ) -> Result<ProcessStatus, PluginError> {
        self.shared.apply(events.input, events.output);
        self.shared.report_turn(events.output);
        let gain = self.shared.gain();

        for mut pair in &mut audio {
            let Some(channels) = pair.channels()?.into_f32() else {
                continue;
            };
            for channel in channels {
                match channel {
                    ChannelPair::InputOutput(input, output) => {
                        for (i, o) in input.iter().zip(output) {
                            *o = i * gain;
                        }
                    }
                    ChannelPair::InPlace(buffer) => {
                        for s in buffer {
                            *s *= gain;
                        }
                    }
                    ChannelPair::OutputOnly(buffer) => buffer.fill(0.0),
                    ChannelPair::InputOnly(_) => {}
                }
            }
        }
        Ok(ProcessStatus::Continue)
    }
}

impl PluginAudioProcessorParams for GainProcessor<'_> {
    fn flush(&mut self, input: &InputEvents, output: &mut OutputEvents) {
        self.shared.apply(input, output);
    }
}

impl PluginMainThreadParams for GainMainThread<'_> {
    fn count(&self) -> u32 {
        1
    }

    fn get_info(&self, index: u32, info: &mut ParamInfoWriter) {
        if index != 0 {
            return;
        }
        info.set(&ParamInfo {
            id: ClapId::new(GAIN_PARAM_ID),
            flags: ParamInfoFlags::IS_AUTOMATABLE,
            cookie: Default::default(),
            name: GAIN_NAME,
            module: b"",
            min_value: 0.0,
            max_value: GAIN_MAX,
            default_value: f64::from(GAIN_DEFAULT),
        });
    }

    fn get_value(&self, id: ClapId) -> Option<f64> {
        (id.get() == GAIN_PARAM_ID).then(|| f64::from(self.shared.gain()))
    }

    fn value_to_text(
        &self,
        _id: ClapId,
        value: f64,
        writer: &mut ParamDisplayWriter,
    ) -> std::fmt::Result {
        write!(writer, "{value:.2} x")
    }

    fn text_to_value(&self, _id: ClapId, text: &CStr) -> Option<f64> {
        text.to_str().ok()?.parse().ok()
    }

    fn flush(&self, input: &InputEvents, output: &mut OutputEvents) {
        self.shared.apply(input, output);
    }
}

impl PluginStateImpl for GainMainThread<'_> {
    fn save(&self, output: &mut OutputStream) -> Result<(), PluginError> {
        output.write_all(&self.shared.gain().to_le_bytes())?;
        Ok(())
    }

    fn load(&self, input: &mut InputStream) -> Result<(), PluginError> {
        let mut bytes = [0_u8; 4];
        input.read_exact(&mut bytes)?;
        self.shared.set_gain(f32::from_le_bytes(bytes));
        Ok(())
    }
}

impl PluginLatencyImpl for GainMainThread<'_> {
    fn get(&self) -> u32 {
        LATENCY_SAMPLES
    }
}

impl PluginAudioPortsImpl for GainMainThread<'_> {
    fn count(&self, _is_input: bool) -> u32 {
        1
    }

    fn get(&self, index: u32, _is_input: bool, writer: &mut AudioPortInfoWriter) {
        if index != 0 {
            return;
        }
        writer.set(&AudioPortInfo {
            id: ClapId::new(0),
            name: b"main",
            channel_count: self.shared.channels,
            flags: AudioPortFlags::IS_MAIN,
            port_type: None,
            in_place_pair: None,
        });
    }
}
