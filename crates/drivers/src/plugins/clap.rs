//! The CLAP backend (`clack-host`). Loading a file is the one `unsafe` step: the file's code runs.

use super::{
    ParamInfo, PluginError, PluginFormat, PluginInfo, PortLayout, ProcessError, CHANNELS,
    PARAM_RING_SLOTS,
};
use clack_extensions::audio_ports::{AudioPortInfoBuffer, PluginAudioPorts};
use clack_extensions::latency::PluginLatency;
use clack_extensions::params::{ParamInfoBuffer, ParamInfoFlags, PluginParams};
use clack_extensions::state::PluginState;
use clack_host::events::event_types::ParamValueEvent;
use clack_host::events::io::{
    EventBuffer, InputEvents, OutputEventBuffer, OutputEvents, TryPushError,
};
use clack_host::events::spaces::CoreEventSpace;
use clack_host::events::{Pckn, UnknownEvent};
use clack_host::prelude::*;
use clack_host::utils::ClapId;
use rtrb::{Consumer, Producer, RingBuffer};
use std::ffi::CString;
use std::path::Path;

const HOST_NAME: &str = "Kontra";
const HOST_VENDOR: &str = "shrippen";

/// A parameter change on its way to the audio thread.
#[derive(Debug, Clone, Copy)]
struct ParamChange {
    id: u32,
    value: f64,
}

/// Takes the parameter value events a plugin writes while it is called, one `report` call each.
/// No allocation of its own: `report` decides where they go (a ring, or a list on the main
/// thread).
struct Sink<F: FnMut(ParamChange)> {
    report: F,
}

impl<F: FnMut(ParamChange)> OutputEventBuffer for Sink<F> {
    fn try_push(&mut self, event: &UnknownEvent) -> Result<(), TryPushError> {
        // Other events (notes, transport requests) are of no use for effects.
        let Some(CoreEventSpace::ParamValue(e)) = event.as_core_event() else {
            return Ok(());
        };
        let Some(id) = e.param_id() else {
            return Ok(());
        };
        (self.report)(ParamChange {
            id: id.get(),
            value: e.value(),
        });
        Ok(())
    }
}

struct Host;
struct HostShared;

impl SharedHandler<'_> for HostShared {
    // The plugin asks for a restart, a process call or a main thread callback: Kontra needs
    // none of them for effects.
    fn request_restart(&self) {}
    fn request_process(&self) {}
    fn request_callback(&self) {}
}

impl HostHandlers for Host {
    type Shared<'a> = HostShared;
    type MainThread<'a> = ();
    type AudioProcessor<'a> = ();
}

fn load_error(path: &Path, message: impl ToString) -> PluginError {
    PluginError::Load {
        path: path.to_path_buf(),
        message: message.to_string(),
    }
}

#[allow(unsafe_code)]
fn load_entry(path: &Path) -> Result<PluginEntry, PluginError> {
    // SAFETY: loading a plugin file runs its code; that is what the user asked for by choosing
    // the plugin. A crash is contained by scanning in a probe process; the host calls follow
    // the CLAP threading rules through clack's typed handles.
    unsafe { PluginEntry::load(path) }.map_err(|e| load_error(path, e))
}

fn host_info() -> Result<HostInfo, PluginError> {
    HostInfo::new(
        HOST_NAME,
        HOST_VENDOR,
        env!("CARGO_PKG_REPOSITORY"),
        env!("CARGO_PKG_VERSION"),
    )
    .map_err(|e| PluginError::Plugin(e.to_string()))
}

fn instantiate(
    entry: &PluginEntry,
    id: &std::ffi::CStr,
    path: &Path,
) -> Result<PluginInstance<Host>, PluginError> {
    PluginInstance::<Host>::new(|_| HostShared, |_| (), entry, id, &host_info()?)
        .map_err(|e| load_error(path, e))
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn lossy(s: Option<&std::ffi::CStr>) -> String {
    s.map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn read_ports(instance: &mut PluginInstance<Host>) -> PortLayout {
    let handle = instance.plugin_handle();
    let Some(ports) = handle.get_extension::<PluginAudioPorts>() else {
        return PortLayout::default();
    };
    let mut layout = PortLayout::default();
    for is_input in [true, false] {
        let list = if is_input {
            &mut layout.inputs
        } else {
            &mut layout.outputs
        };
        for index in 0..ports.count(&handle, is_input) {
            let mut buffer = AudioPortInfoBuffer::new();
            if let Some(info) = ports.get(&handle, index, is_input, &mut buffer) {
                list.push(info.channel_count);
            }
        }
    }
    layout
}

/// Lists the plugins of a file, with their ports (needs an instance of each).
pub(super) fn probe(path: &Path) -> Result<Vec<PluginInfo>, PluginError> {
    let entry = load_entry(path)?;
    let factory = entry
        .get_plugin_factory()
        .ok_or_else(|| load_error(path, "no plugin factory"))?;

    let mut found = Vec::new();
    let mut first_error = None;
    for descriptor in factory.plugin_descriptors() {
        let Some(id) = descriptor.id() else {
            continue;
        };
        let mut instance = match instantiate(&entry, id, path) {
            Ok(instance) => instance,
            Err(e) => {
                first_error.get_or_insert(e);
                continue;
            }
        };
        found.push(PluginInfo {
            format: PluginFormat::Clap,
            id: id.to_string_lossy().into_owned(),
            name: lossy(descriptor.name()),
            vendor: lossy(descriptor.vendor()),
            path: path.to_path_buf(),
            ports: read_ports(&mut instance),
        });
    }
    match (found.is_empty(), first_error) {
        (true, Some(e)) => Err(e),
        _ => Ok(found),
    }
}

pub(super) struct ClapPlugin {
    info: PluginInfo,
    instance: PluginInstance<Host>,
    /// Parameters as they were at load: ids, names and ranges.
    params: Vec<ParamInfo>,
    params_ext: Option<PluginParams>,
    state_ext: Option<PluginState>,
    latency_ext: Option<PluginLatency>,
    /// Main thread end of the ring to the audio thread while activated.
    ring: Option<Producer<ParamChange>>,
    /// Main thread end of the ring from the audio thread: changes the plugin made itself.
    reports: Option<Consumer<ParamChange>>,
    /// Changes the plugin made itself while inactive (in a flush).
    flushed: Vec<ParamChange>,
    in_channels: usize,
    out_channels: usize,
}

impl ClapPlugin {
    pub(super) fn open(path: &Path, id: Option<&str>) -> Result<ClapPlugin, PluginError> {
        let entry = load_entry(path)?;
        let factory = entry
            .get_plugin_factory()
            .ok_or_else(|| load_error(path, "no plugin factory"))?;
        let ids: Vec<_> = factory
            .plugin_descriptors()
            .filter_map(|d| d.id())
            .collect();

        let plugin_id = match id {
            Some(wanted) => ids
                .iter()
                .find(|i| i.to_string_lossy() == wanted)
                .ok_or_else(|| PluginError::NotFound {
                    path: path.to_path_buf(),
                    id: wanted.to_string(),
                })?,
            None if ids.len() == 1 => &ids[0],
            None => {
                return Err(PluginError::Ambiguous {
                    path: path.to_path_buf(),
                    ids: ids
                        .iter()
                        .map(|i| i.to_string_lossy())
                        .collect::<Vec<_>>()
                        .join(", "),
                })
            }
        };
        let plugin_id = CString::from(*plugin_id);
        let descriptor = factory
            .plugin_descriptors()
            .find(|d| d.id() == Some(plugin_id.as_c_str()))
            .ok_or_else(|| load_error(path, "plugin vanished"))?;
        let (name, vendor) = (lossy(descriptor.name()), lossy(descriptor.vendor()));

        let mut instance = instantiate(&entry, &plugin_id, path)?;
        let ports = read_ports(&mut instance);
        let (Some(&in_channels), Some(&out_channels)) =
            (ports.inputs.first(), ports.outputs.first())
        else {
            return Err(PluginError::NotAnEffect { name });
        };
        if in_channels as usize > CHANNELS || out_channels as usize > CHANNELS {
            return Err(PluginError::TooManyChannels { name });
        }

        let handle = instance.plugin_handle();
        let params_ext = handle.get_extension::<PluginParams>();
        let state_ext = handle.get_extension::<PluginState>();
        let latency_ext = handle.get_extension::<PluginLatency>();
        let params = params_ext
            .as_ref()
            .map(|ext| list_params(ext, &handle))
            .unwrap_or_default();

        Ok(ClapPlugin {
            info: PluginInfo {
                format: PluginFormat::Clap,
                id: plugin_id.to_string_lossy().into_owned(),
                name,
                vendor,
                path: path.to_path_buf(),
                ports,
            },
            instance,
            params,
            params_ext,
            state_ext,
            latency_ext,
            ring: None,
            reports: None,
            flushed: Vec::new(),
            in_channels: in_channels as usize,
            out_channels: out_channels as usize,
        })
    }

    pub(super) fn info(&self) -> &PluginInfo {
        &self.info
    }

    pub(super) fn params(&mut self) -> Vec<ParamInfo> {
        let Some(ext) = self.params_ext else {
            return Vec::new();
        };
        let handle = self.instance.plugin_handle();
        let mut params = self.params.clone();
        for p in &mut params {
            let current = ClapId::from_raw(p.id).and_then(|id| ext.get_value(&handle, id));
            p.current = current.unwrap_or(p.default);
        }
        params
    }

    pub(super) fn set_param(&mut self, id: u32, value: f64) -> Result<(), PluginError> {
        let (Some(ext), Some(info)) = (self.params_ext, self.params.iter().find(|p| p.id == id))
        else {
            return Err(PluginError::NoParam(id));
        };
        let value = value.clamp(info.min, info.max);
        let clap_id = ClapId::from_raw(id).ok_or(PluginError::NoParam(id))?;

        // Active: the audio thread takes it from the ring. Inactive: flush it into the plugin.
        if let Some(ring) = self.ring.as_mut() {
            return ring
                .push(ParamChange { id, value })
                .map_err(|_| PluginError::QueueFull);
        }
        let mut events = EventBuffer::new();
        events.push(&ParamValueEvent::new(0, clap_id, Pckn::match_all(), value));
        let mut handle = self
            .instance
            .inactive_plugin_handle()
            .ok_or(PluginError::InUse)?;
        let mut sink = Sink {
            report: |change| self.flushed.push(change),
        };
        let mut output = OutputEvents::from_buffer(&mut sink);
        ext.flush(&mut handle, &events.as_input(), &mut output);
        Ok(())
    }

    /// Changes the plugin made itself since the last call, the last value per parameter.
    pub(super) fn take_changes(&mut self) -> Vec<(u32, f64)> {
        let mut all = std::mem::take(&mut self.flushed);
        while let Some(change) = self.reports.as_mut().and_then(|r| r.pop().ok()) {
            all.push(change);
        }
        super::coalesce(all.into_iter().map(|c| (c.id, c.value)))
    }

    pub(super) fn latency_samples(&mut self) -> u32 {
        let Some(ext) = self.latency_ext else {
            return 0;
        };
        ext.get(&self.instance.plugin_handle())
    }

    pub(super) fn save_state(&mut self) -> Result<Vec<u8>, PluginError> {
        let ext = self.state_ext.ok_or(PluginError::NoState)?;
        let mut bytes = Vec::new();
        ext.save(&self.instance.plugin_handle(), &mut bytes)
            .map_err(|e| PluginError::State(e.to_string()))?;
        Ok(bytes)
    }

    pub(super) fn load_state(&mut self, bytes: &[u8]) -> Result<(), PluginError> {
        let ext = self.state_ext.ok_or(PluginError::NoState)?;
        let mut reader = bytes;
        ext.load(&self.instance.plugin_handle(), &mut reader)
            .map_err(|e| PluginError::State(e.to_string()))
    }

    pub(super) fn activate(
        &mut self,
        sample_rate: f64,
        max_block: u32,
    ) -> Result<ClapProcessor, PluginError> {
        if max_block == 0 {
            return Err(PluginError::Block(max_block));
        }
        let config = PluginAudioConfiguration {
            sample_rate,
            min_frames_count: 1,
            max_frames_count: max_block,
        };
        let stopped = self
            .instance
            .activate(|_, _| (), config)
            .map_err(|e| PluginError::Plugin(e.to_string()))?;

        let (tx, rx) = RingBuffer::new(PARAM_RING_SLOTS);
        let (report_tx, report_rx) = RingBuffer::new(PARAM_RING_SLOTS);
        self.ring = Some(tx);
        self.reports = Some(report_rx);
        let frames = max_block as usize;
        Ok(ClapProcessor {
            processor: stopped.into(),
            in_ports: AudioPorts::with_capacity(CHANNELS, 1),
            out_ports: AudioPorts::with_capacity(CHANNELS, 1),
            scratch_in: [vec![0.0; frames], vec![0.0; frames]],
            scratch_out: [vec![0.0; frames], vec![0.0; frames]],
            events: EventBuffer::with_capacity(PARAM_RING_SLOTS),
            changes: rx,
            reports: report_tx,
            in_channels: self.in_channels,
            out_channels: self.out_channels,
            max_block: frames,
            steady_time: 0,
        })
    }

    pub(super) fn deactivate(&mut self) -> Result<(), PluginError> {
        self.instance
            .try_deactivate()
            .map_err(|_| PluginError::InUse)?;
        // Reports still in the ring are kept for the next `take_changes`.
        self.flush_reports();
        self.ring = None;
        self.reports = None;
        Ok(())
    }
}

impl ClapPlugin {
    /// Moves what the ring holds to the list, which outlives the activation.
    fn flush_reports(&mut self) {
        while let Some(change) = self.reports.as_mut().and_then(|r| r.pop().ok()) {
            self.flushed.push(change);
        }
    }
}

fn list_params(ext: &PluginParams, handle: &PluginMainThreadHandle) -> Vec<ParamInfo> {
    let mut params = Vec::new();
    for index in 0..ext.count(handle) {
        let mut buffer = ParamInfoBuffer::new();
        let Some(info) = ext.get_info(handle, index, &mut buffer) else {
            continue;
        };
        if info.flags.contains(ParamInfoFlags::IS_HIDDEN) {
            continue;
        }
        let current = ext.get_value(handle, info.id).unwrap_or(info.default_value);
        params.push(ParamInfo {
            id: info.id.get(),
            name: text(info.name),
            min: info.min_value,
            max: info.max_value,
            default: info.default_value,
            current,
        });
    }
    params
}

pub(super) struct ClapProcessor {
    processor: PluginAudioProcessor<Host>,
    in_ports: AudioPorts,
    out_ports: AudioPorts,
    /// The block as the plugin's ports want it (the plugin may not read our slices in place).
    scratch_in: [Vec<f32>; CHANNELS],
    scratch_out: [Vec<f32>; CHANNELS],
    events: EventBuffer,
    changes: Consumer<ParamChange>,
    /// Parameter changes the plugin makes itself, to the main thread.
    reports: Producer<ParamChange>,
    in_channels: usize,
    out_channels: usize,
    max_block: usize,
    steady_time: u64,
}

impl ClapProcessor {
    pub(super) fn process(
        &mut self,
        input: [&[f32]; CHANNELS],
        output: [&mut [f32]; CHANNELS],
    ) -> Result<(), ProcessError> {
        let [out_l, out_r] = output;
        let frames = input[0]
            .len()
            .min(input[1].len())
            .min(out_l.len())
            .min(out_r.len());

        let mut done = 0;
        while done < frames {
            let n = (frames - done).min(self.max_block);
            let range = done..done + n;
            let result = self.block(
                [&input[0][range.clone()], &input[1][range.clone()]],
                [&mut out_l[range.clone()], &mut out_r[range]],
            );
            if result.is_err() {
                // Dry instead of silence, also for the rest of the call.
                out_l[done..frames].copy_from_slice(&input[0][done..frames]);
                out_r[done..frames].copy_from_slice(&input[1][done..frames]);
                return result;
            }
            done += n;
        }
        Ok(())
    }

    /// One block of at most `max_block` frames.
    fn block(
        &mut self,
        input: [&[f32]; CHANNELS],
        output: [&mut [f32]; CHANNELS],
    ) -> Result<(), ProcessError> {
        let n = input[0].len();

        // A mono port gets the mean of both channels.
        if self.in_channels == 1 {
            let mean = input[0].iter().zip(input[1]).map(|(l, r)| 0.5 * (l + r));
            for (s, m) in self.scratch_in[0].iter_mut().zip(mean) {
                *s = m;
            }
        } else {
            self.scratch_in[0][..n].copy_from_slice(input[0]);
            self.scratch_in[1][..n].copy_from_slice(input[1]);
        }

        // Parameter changes of the main thread, as events at the start of the block.
        self.events.clear();
        while self.events.len() < PARAM_RING_SLOTS as u32 {
            let Ok(change) = self.changes.pop() else {
                break;
            };
            let Some(id) = ClapId::from_raw(change.id) else {
                continue;
            };
            self.events.push(&ParamValueEvent::new(
                0,
                id,
                Pckn::match_all(),
                change.value,
            ));
        }

        let started = self
            .processor
            .ensure_processing_started()
            .map_err(|_| ProcessError::Start)?;

        let [in_l, in_r] = &mut self.scratch_in;
        let [out_l, out_r] = &mut self.scratch_out;
        let inputs = [in_l, in_r];
        let outputs = [out_l, out_r];
        let in_bufs = self.in_ports.with_input_buffers([AudioPortBuffer {
            latency: 0,
            channels: AudioPortBufferType::f32_input_only(
                inputs
                    .into_iter()
                    .take(self.in_channels)
                    .map(|b| InputChannel::variable(&mut b[..n])),
            ),
        }]);
        let mut out_bufs = self.out_ports.with_output_buffers([AudioPortBuffer {
            latency: 0,
            channels: AudioPortBufferType::f32_output_only(
                outputs
                    .into_iter()
                    .take(self.out_channels)
                    .map(|b| &mut b[..n]),
            ),
        }]);

        // A full ring drops the report: the plugin's value is read again at the next `params`.
        let reports = &mut self.reports;
        let mut sink = Sink {
            report: |change| {
                let _ = reports.push(change);
            },
        };
        let status = started.process(
            &in_bufs,
            &mut out_bufs,
            &InputEvents::from_buffer(&self.events),
            &mut OutputEvents::from_buffer(&mut sink),
            Some(self.steady_time),
            None,
        );
        self.steady_time += n as u64;
        status.map_err(|_| ProcessError::Process)?;

        let [o_l, o_r] = output;
        o_l.copy_from_slice(&self.scratch_out[0][..n]);
        // A mono port comes out on both channels.
        let right = if self.out_channels == 1 { 0 } else { 1 };
        o_r.copy_from_slice(&self.scratch_out[right][..n]);
        Ok(())
    }
}

impl Drop for ClapProcessor {
    fn drop(&mut self) {
        // Processing ends on the thread that owns the processor, as CLAP wants it.
        self.processor.ensure_processing_stopped();
    }
}
