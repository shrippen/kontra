//! A gain as VST3 plugin, for the tests of the plugin host (FX3). One parameter `gain` (a factor,
//! 0 to 4, default 1; the controller maps it to 0..1 normalized), the component state is the
//! normalized value as eight bytes, latency is `LATENCY_SAMPLES`. Component and controller are two
//! classes, as in many real plugins.
//!
//! Setting the gain to `GAIN_TRIGGER` makes the controller change it by itself to `GAIN_REPORTED`
//! and report that to the host's component handler with `performEdit`, the way a plugin's own
//! editor does (tests of FX5). The processor is not told: the host has to forward it.
//!
//! The same code builds a stereo and a mono variant (`export_module!(2, …)`, `(1, …)`); the crash
//! and hang modules are in `examples/`.
//!
//! COM needs `unsafe` for every method; this crate is a test fixture, not part of Kontra.
#![allow(unsafe_code)]
#![allow(non_snake_case, non_upper_case_globals, missing_debug_implementations)]

use std::cell::{Cell, RefCell};
use std::ffi::{c_char, c_void, CString};
use std::sync::atomic::{AtomicU64, Ordering};
use std::{ptr, slice};

mod view;

pub use view::{EDITOR_GAIN, EDITOR_MAX_WIDTH, EDITOR_RESIZED, EDITOR_SIZE};
pub use vst3;
use vst3::{uid, Class, ComPtr, ComRef, ComWrapper, Steinberg::Vst::*, Steinberg::*};

pub const LATENCY_SAMPLES: u32 = 64;
pub const GAIN_PARAM_ID: u32 = 7;
pub const GAIN_MAX: f64 = 4.0;
/// Set as the gain, this makes the controller report `GAIN_REPORTED` as its own change.
pub const GAIN_TRIGGER: f64 = 3.5;
pub const GAIN_REPORTED: f64 = 0.5;
const GAIN_DEFAULT_NORMALIZED: f64 = 0.25;
const STATE_BYTES: i32 = 8;
const MONO_CHANNELS: u32 = 1;

fn copy_cstring(src: &str, dst: &mut [c_char]) {
    let c_string = CString::new(src).unwrap_or_default();
    for (src, dst) in c_string.as_bytes_with_nul().iter().zip(dst.iter_mut()) {
        *dst = *src as c_char;
    }
}

fn copy_wstring(src: &str, dst: &mut [TChar]) {
    let mut len = 0;
    for (src, dst) in src.encode_utf16().zip(dst.iter_mut()) {
        *dst = src as TChar;
        len += 1;
    }
    if let Some(end) = dst.get_mut(len) {
        *end = 0;
    }
}

fn component_cid(channels: u32) -> TUID {
    if channels == MONO_CHANNELS {
        uid(0x4B6F6E74, 0x72610003, 0x4D6F6E6F, 0x436F6D70)
    } else {
        uid(0x4B6F6E74, 0x72610002, 0x53746572, 0x436F6D70)
    }
}

fn controller_cid(channels: u32) -> TUID {
    if channels == MONO_CHANNELS {
        uid(0x4B6F6E74, 0x72610003, 0x4D6F6E6F, 0x43747266)
    } else {
        uid(0x4B6F6E74, 0x72610002, 0x53746572, 0x43747266)
    }
}

fn plugin_name(channels: u32) -> &'static str {
    if channels == MONO_CHANNELS {
        "Kontra Test Gain Mono"
    } else {
        "Kontra Test Gain"
    }
}

fn arrangement(channels: u32) -> SpeakerArrangement {
    if channels == MONO_CHANNELS {
        SpeakerArr::kMono
    } else {
        SpeakerArr::kStereo
    }
}

struct Processor {
    channels: u32,
    /// The normalized gain as f64 bits.
    gain: AtomicU64,
}

impl Class for Processor {
    type Interfaces = (IComponent, IAudioProcessor, IProcessContextRequirements);
}

impl IPluginBaseTrait for Processor {
    unsafe fn initialize(&self, _context: *mut FUnknown) -> tresult {
        kResultOk
    }

    unsafe fn terminate(&self) -> tresult {
        kResultOk
    }
}

impl IComponentTrait for Processor {
    unsafe fn getControllerClassId(&self, class_id: *mut TUID) -> tresult {
        *class_id = controller_cid(self.channels);
        kResultOk
    }

    unsafe fn setIoMode(&self, _mode: IoMode) -> tresult {
        kResultOk
    }

    unsafe fn getBusCount(&self, media: MediaType, dir: BusDirection) -> i32 {
        let audio = media as MediaTypes == MediaTypes_::kAudio;
        let known = dir as BusDirections == BusDirections_::kInput
            || dir as BusDirections == BusDirections_::kOutput;
        i32::from(audio && known)
    }

    unsafe fn getBusInfo(
        &self,
        media: MediaType,
        dir: BusDirection,
        index: i32,
        bus: *mut BusInfo,
    ) -> tresult {
        if media as MediaTypes != MediaTypes_::kAudio || index != 0 {
            return kInvalidArgument;
        }
        let bus = &mut *bus;
        bus.mediaType = MediaTypes_::kAudio as MediaType;
        bus.direction = dir;
        bus.channelCount = self.channels as i32;
        let name = if dir as BusDirections == BusDirections_::kInput {
            "Input"
        } else {
            "Output"
        };
        copy_wstring(name, &mut bus.name);
        bus.busType = BusTypes_::kMain as BusType;
        bus.flags = BusInfo_::BusFlags_::kDefaultActive as _;
        kResultOk
    }

    unsafe fn getRoutingInfo(&self, _in: *mut RoutingInfo, _out: *mut RoutingInfo) -> tresult {
        kNotImplemented
    }

    unsafe fn activateBus(&self, _m: MediaType, _d: BusDirection, _i: i32, _s: TBool) -> tresult {
        kResultOk
    }

    unsafe fn setActive(&self, _state: TBool) -> tresult {
        kResultOk
    }

    unsafe fn setState(&self, state: *mut IBStream) -> tresult {
        let Some(stream) = ComRef::from_raw(state) else {
            return kInvalidArgument;
        };
        let mut bytes = [0_u8; STATE_BYTES as usize];
        let mut read = 0;
        let result = stream.read(bytes.as_mut_ptr().cast(), STATE_BYTES, &mut read);
        if result != kResultOk || read != STATE_BYTES {
            return kResultFalse;
        }
        self.gain
            .store(f64::from_le_bytes(bytes).to_bits(), Ordering::Relaxed);
        kResultOk
    }

    unsafe fn getState(&self, state: *mut IBStream) -> tresult {
        let Some(stream) = ComRef::from_raw(state) else {
            return kInvalidArgument;
        };
        let mut bytes = f64::from_bits(self.gain.load(Ordering::Relaxed)).to_le_bytes();
        let mut written = 0;
        stream.write(bytes.as_mut_ptr().cast(), STATE_BYTES, &mut written)
    }
}

impl IAudioProcessorTrait for Processor {
    unsafe fn setBusArrangements(
        &self,
        inputs: *mut SpeakerArrangement,
        num_ins: i32,
        outputs: *mut SpeakerArrangement,
        num_outs: i32,
    ) -> tresult {
        let wanted = arrangement(self.channels);
        let fits = num_ins == 1 && num_outs == 1 && *inputs == wanted && *outputs == wanted;
        if fits {
            kResultTrue
        } else {
            kResultFalse
        }
    }

    unsafe fn getBusArrangement(
        &self,
        _dir: BusDirection,
        index: i32,
        arr: *mut SpeakerArrangement,
    ) -> tresult {
        if index != 0 {
            return kInvalidArgument;
        }
        *arr = arrangement(self.channels);
        kResultOk
    }

    unsafe fn canProcessSampleSize(&self, size: i32) -> tresult {
        if size as SymbolicSampleSizes == SymbolicSampleSizes_::kSample32 {
            kResultOk
        } else {
            kNotImplemented
        }
    }

    unsafe fn getLatencySamples(&self) -> u32 {
        LATENCY_SAMPLES
    }

    unsafe fn setupProcessing(&self, _setup: *mut ProcessSetup) -> tresult {
        kResultOk
    }

    unsafe fn setProcessing(&self, _state: TBool) -> tresult {
        kResultOk
    }

    unsafe fn process(&self, data: *mut ProcessData) -> tresult {
        let data = &*data;

        // The last point of every queue of the parameter wins.
        if let Some(changes) = ComRef::from_raw(data.inputParameterChanges) {
            for i in 0..changes.getParameterCount() {
                let Some(queue) = ComRef::from_raw(changes.getParameterData(i)) else {
                    continue;
                };
                let points = queue.getPointCount();
                if queue.getParameterId() != GAIN_PARAM_ID || points < 1 {
                    continue;
                }
                let (mut offset, mut value) = (0, 0.0);
                if queue.getPoint(points - 1, &mut offset, &mut value) == kResultTrue {
                    self.gain.store(value.to_bits(), Ordering::Relaxed);
                }
            }
        }
        let gain = (f64::from_bits(self.gain.load(Ordering::Relaxed)) * GAIN_MAX) as f32;

        if data.numInputs != 1 || data.numOutputs != 1 {
            return kResultOk;
        }
        let frames = data.numSamples as usize;
        let input = &*data.inputs;
        let output = &*data.outputs;
        let channels = (input.numChannels.min(output.numChannels)) as usize;
        let inputs = slice::from_raw_parts(input.__field0.channelBuffers32, channels);
        let outputs = slice::from_raw_parts(output.__field0.channelBuffers32, channels);
        for (src, dst) in inputs.iter().zip(outputs) {
            let src = slice::from_raw_parts(*src, frames);
            let dst = slice::from_raw_parts_mut(*dst, frames);
            for (s, d) in src.iter().zip(dst) {
                *d = *s * gain;
            }
        }
        kResultOk
    }

    unsafe fn getTailSamples(&self) -> u32 {
        0
    }
}

impl IProcessContextRequirementsTrait for Processor {
    unsafe fn getProcessContextRequirements(&self) -> u32 {
        0
    }
}

struct Controller {
    gain: Cell<f64>,
    handler: RefCell<Option<ComPtr<IComponentHandler>>>,
}

impl Class for Controller {
    type Interfaces = (IEditController,);
}

impl IPluginBaseTrait for Controller {
    unsafe fn initialize(&self, _context: *mut FUnknown) -> tresult {
        kResultOk
    }

    unsafe fn terminate(&self) -> tresult {
        kResultOk
    }
}

impl IEditControllerTrait for Controller {
    unsafe fn setComponentState(&self, state: *mut IBStream) -> tresult {
        let Some(stream) = ComRef::from_raw(state) else {
            return kInvalidArgument;
        };
        let mut bytes = [0_u8; STATE_BYTES as usize];
        let mut read = 0;
        let result = stream.read(bytes.as_mut_ptr().cast(), STATE_BYTES, &mut read);
        if result != kResultOk || read != STATE_BYTES {
            return kResultFalse;
        }
        self.gain.set(f64::from_le_bytes(bytes));
        kResultOk
    }

    unsafe fn setState(&self, _state: *mut IBStream) -> tresult {
        kResultOk
    }

    unsafe fn getState(&self, _state: *mut IBStream) -> tresult {
        kResultOk
    }

    unsafe fn getParameterCount(&self) -> i32 {
        1
    }

    unsafe fn getParameterInfo(&self, index: i32, info: *mut ParameterInfo) -> tresult {
        if index != 0 {
            return kInvalidArgument;
        }
        let info = &mut *info;
        info.id = GAIN_PARAM_ID;
        copy_wstring("gain", &mut info.title);
        copy_wstring("gain", &mut info.shortTitle);
        copy_wstring("", &mut info.units);
        info.stepCount = 0;
        info.defaultNormalizedValue = GAIN_DEFAULT_NORMALIZED;
        info.unitId = 0;
        info.flags = ParameterInfo_::ParameterFlags_::kCanAutomate as _;
        kResultOk
    }

    unsafe fn getParamStringByValue(&self, _id: u32, v: f64, string: *mut String128) -> tresult {
        copy_wstring(&format!("{:.2}", v * GAIN_MAX), &mut *string);
        kResultOk
    }

    unsafe fn getParamValueByString(&self, _id: u32, _s: *mut TChar, _v: *mut f64) -> tresult {
        kNotImplemented
    }

    unsafe fn normalizedParamToPlain(&self, _id: u32, normalized: f64) -> f64 {
        normalized * GAIN_MAX
    }

    unsafe fn plainParamToNormalized(&self, _id: u32, plain: f64) -> f64 {
        plain / GAIN_MAX
    }

    unsafe fn getParamNormalized(&self, id: u32) -> f64 {
        if id == GAIN_PARAM_ID {
            self.gain.get()
        } else {
            0.0
        }
    }

    unsafe fn setParamNormalized(&self, id: u32, value: f64) -> tresult {
        if id != GAIN_PARAM_ID {
            return kInvalidArgument;
        }
        if value != GAIN_TRIGGER / GAIN_MAX {
            self.gain.set(value);
            return kResultOk;
        }
        // The plugin's "editor" moves the gain itself and tells the host.
        let reported = GAIN_REPORTED / GAIN_MAX;
        self.gain.set(reported);
        if let Some(handler) = self.handler.borrow().as_ref() {
            handler.beginEdit(GAIN_PARAM_ID);
            handler.performEdit(GAIN_PARAM_ID, reported);
            handler.endEdit(GAIN_PARAM_ID);
        }
        kResultOk
    }

    unsafe fn setComponentHandler(&self, handler: *mut IComponentHandler) -> tresult {
        *self.handler.borrow_mut() = ComRef::from_raw(handler).map(|h| h.to_com_ptr());
        kResultOk
    }

    unsafe fn createView(&self, name: *const c_char) -> *mut IPlugView {
        if !view::is_editor(name) {
            return ptr::null_mut();
        }
        let view = ComWrapper::new(view::View::new(self.handler.borrow().clone()));
        view.to_com_ptr::<IPlugView>()
            .map_or(ptr::null_mut(), ComPtr::into_raw)
    }
}

pub struct Factory {
    channels: u32,
}

impl Factory {
    pub fn new(channels: u32) -> Factory {
        Factory { channels }
    }
}

impl Class for Factory {
    type Interfaces = (IPluginFactory,);
}

impl IPluginFactoryTrait for Factory {
    unsafe fn getFactoryInfo(&self, info: *mut PFactoryInfo) -> tresult {
        let info = &mut *info;
        copy_cstring("Kontra Tests", &mut info.vendor);
        copy_cstring("", &mut info.url);
        copy_cstring("", &mut info.email);
        info.flags = PFactoryInfo_::FactoryFlags_::kUnicode as int32;
        kResultOk
    }

    unsafe fn countClasses(&self) -> i32 {
        2
    }

    unsafe fn getClassInfo(&self, index: i32, info: *mut PClassInfo) -> tresult {
        let (cid, category) = match index {
            0 => (component_cid(self.channels), "Audio Module Class"),
            1 => (controller_cid(self.channels), "Component Controller Class"),
            _ => return kInvalidArgument,
        };
        let info = &mut *info;
        info.cid = cid;
        info.cardinality = PClassInfo_::ClassCardinality_::kManyInstances as int32;
        copy_cstring(category, &mut info.category);
        copy_cstring(plugin_name(self.channels), &mut info.name);
        kResultOk
    }

    unsafe fn createInstance(
        &self,
        cid: FIDString,
        iid: FIDString,
        obj: *mut *mut c_void,
    ) -> tresult {
        let wanted = *(cid as *const TUID);
        let instance = if wanted == component_cid(self.channels) {
            ComWrapper::new(Processor {
                channels: self.channels,
                gain: AtomicU64::new(GAIN_DEFAULT_NORMALIZED.to_bits()),
            })
            .to_com_ptr::<FUnknown>()
        } else if wanted == controller_cid(self.channels) {
            ComWrapper::new(Controller {
                gain: Cell::new(GAIN_DEFAULT_NORMALIZED),
                handler: RefCell::new(None),
            })
            .to_com_ptr::<FUnknown>()
        } else {
            None
        };
        let Some(instance) = instance else {
            return kInvalidArgument;
        };
        let raw = instance.as_ptr();
        ((*(*raw).vtbl).queryInterface)(raw, iid as *mut TUID, obj)
    }
}

/// The factory of the module, as `GetPluginFactory` returns it.
pub fn factory(channels: u32) -> *mut IPluginFactory {
    match ComWrapper::new(Factory::new(channels)).to_com_ptr::<IPluginFactory>() {
        Some(factory) => factory.into_raw(),
        None => ptr::null_mut(),
    }
}

/// The exported functions of a module: `GetPluginFactory`, and the entry and exit calls of the
/// platform. `$on_entry` runs when the module is loaded (the crash and hang modules).
#[macro_export]
macro_rules! export_module {
    ($channels:expr, $on_entry:expr) => {
        #[cfg(target_os = "linux")]
        #[allow(unreachable_code)]
        #[no_mangle]
        extern "system" fn ModuleEntry(_library_handle: *mut ::std::ffi::c_void) -> bool {
            $on_entry;
            true
        }

        #[cfg(target_os = "linux")]
        #[no_mangle]
        extern "system" fn ModuleExit() -> bool {
            true
        }

        #[cfg(target_os = "windows")]
        #[allow(unreachable_code)]
        #[no_mangle]
        extern "system" fn InitDll() -> bool {
            $on_entry;
            true
        }

        #[cfg(target_os = "windows")]
        #[no_mangle]
        extern "system" fn ExitDll() -> bool {
            true
        }

        #[no_mangle]
        extern "system" fn GetPluginFactory() -> *mut $crate::vst3::Steinberg::IPluginFactory {
            $crate::factory($channels)
        }
    };
}
