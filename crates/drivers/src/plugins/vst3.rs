//! The VST3 backend (`vst3` crate: bindings of the MIT SDK, COM helpers). Loading a module runs its
//! code, and everything is COM calls, so this file is `unsafe` throughout; each block says why.
//!
//! ```text
//! Foo.vst3/Contents/x86_64-linux/Foo.so      (Windows: x86_64-win/Foo.vst3)
//!   ModuleEntry(handle) ─► GetPluginFactory() ─► IPluginFactory
//!     "Audio Module Class" ─► IComponent + IAudioProcessor   (audio)
//!     controller class, or the component itself ─► IEditController   (parameters)
//!
//! main thread                                    audio thread
//! set_param ─► controller.setParamNormalized
//!           └► rtrb ring ──► IParameterChanges ─► IAudioProcessor::process(ProcessData)
//! ```
//!
//! The host's own COM objects (host application, component handler, state stream, parameter
//! changes) are classes of this file. The ones the audio thread touches are made once, at
//! activation.

mod editor;
mod host;

use super::{
    ParamInfo, PluginError, PluginFormat, PluginInfo, PortLayout, ProcessError, CHANNELS,
    PARAM_RING_SLOTS,
};
use host::{Changes, Handler, HostApp, MemStream};
use libloading::Library;
use rtrb::{Consumer, Producer, RingBuffer};
use std::ffi::{c_char, c_void};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use vst3::Steinberg::Vst::*;
use vst3::Steinberg::*;
use vst3::{ComPtr, ComWrapper, Interface};

const AUDIO_CLASS: &str = "Audio Module Class";
const TUID_BYTES: usize = 16;
/// Parameters with a change in one block; more wait for the next block.
const MAX_QUEUES: usize = 64;
/// Sample rate of the short activation that delivers changes made while inactive.
const FLUSH_RATE_HZ: f64 = 48_000.0;
// The SDK's enum constants are u32 on Linux and i32 on Windows; the casts are no-ops there.
#[allow(clippy::unnecessary_cast)]
const AUDIO: i32 = MediaTypes_::kAudio as i32;
#[allow(clippy::unnecessary_cast)]
const INPUT: i32 = BusDirections_::kInput as i32;
#[allow(clippy::unnecessary_cast)]
const OUTPUT: i32 = BusDirections_::kOutput as i32;
#[allow(clippy::unnecessary_cast)]
const SAMPLE_32: i32 = SymbolicSampleSizes_::kSample32 as i32;
#[allow(clippy::unnecessary_cast)]
const REALTIME: i32 = ProcessModes_::kRealtime as i32;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const ARCH_DIR: &str = "x86_64-linux";
#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
const ARCH_DIR: &str = "aarch64-linux";
#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
const ARCH_DIR: &str = "x86_64-win";
#[cfg(all(target_os = "windows", target_arch = "aarch64"))]
const ARCH_DIR: &str = "arm64-win";
#[cfg(not(any(
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    ),
    all(
        target_os = "windows",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
)))]
const ARCH_DIR: &str = "";

#[cfg(windows)]
const BINARY_EXTENSION: &str = "vst3";
#[cfg(not(windows))]
const BINARY_EXTENSION: &str = "so";

fn fail(message: impl ToString) -> PluginError {
    PluginError::Plugin(message.to_string())
}

fn load_error(path: &Path, message: impl ToString) -> PluginError {
    PluginError::Load {
        path: path.to_path_buf(),
        message: message.to_string(),
    }
}

fn ok(result: tresult) -> bool {
    result == kResultOk || result == kResultTrue
}

// ---------------------------------------------------------------------------------------------
// The module file
// ---------------------------------------------------------------------------------------------

type EntryFn = unsafe extern "system" fn(*mut c_void) -> bool;
type ExitFn = unsafe extern "system" fn() -> bool;
type FactoryFn = unsafe extern "system" fn() -> *mut IPluginFactory;

/// A loaded module: its library, the exit call to make and the factory.
struct Module {
    /// Released before the exit call and the unload.
    factory: Option<ComPtr<IPluginFactory>>,
    exit: Option<ExitFn>,
    /// Kept open as long as any COM object of the plugin lives.
    _library: Library,
}

impl Drop for Module {
    fn drop(&mut self) {
        self.factory = None;
        if let Some(exit) = self.exit {
            // SAFETY: the exit function belongs to the library that is still loaded; no object of
            // the module is alive any more, the factory was released above.
            unsafe { exit() };
        }
    }
}

/// The binary inside a bundle (or the file itself, an old style single file module).
fn module_binary(path: &Path) -> Result<PathBuf, PluginError> {
    if !path.is_dir() {
        return Ok(path.to_path_buf());
    }
    if ARCH_DIR.is_empty() {
        return Err(load_error(
            path,
            "VST3 bundles are not supported on this system",
        ));
    }
    let stem = path
        .file_stem()
        .ok_or_else(|| load_error(path, "bundle without a name"))?;
    let binary = path
        .join("Contents")
        .join(ARCH_DIR)
        .join(stem)
        .with_extension(BINARY_EXTENSION);
    Ok(binary)
}

#[cfg(unix)]
fn raw_handle(library: Library) -> (Library, *mut c_void) {
    let unix: libloading::os::unix::Library = library.into();
    let handle = unix.into_raw();
    // SAFETY: `handle` comes from `into_raw` of the library just taken apart and is not closed.
    let unix = unsafe { libloading::os::unix::Library::from_raw(handle) };
    (unix.into(), handle)
}

#[cfg(not(unix))]
fn raw_handle(library: Library) -> (Library, *mut c_void) {
    (library, ptr::null_mut())
}

fn load_module(path: &Path) -> Result<Arc<Module>, PluginError> {
    let binary = module_binary(path)?;

    // SAFETY: loading a module runs its code; that is what the user asked for by choosing the
    // plugin. A crash is contained by scanning in a probe process.
    let library = unsafe { Library::new(&binary) }.map_err(|e| load_error(path, e))?;
    let (library, handle) = raw_handle(library);

    // The entry call is `ModuleEntry(handle)` on Linux and `InitDll()` on Windows, optional there.
    // SAFETY: the symbols have the signatures of the VST3 module spec.
    unsafe {
        if cfg!(target_os = "linux") {
            let entry = library
                .get::<EntryFn>(b"ModuleEntry\0")
                .map_err(|e| load_error(path, e))?;
            if !entry(handle) {
                return Err(load_error(path, "ModuleEntry failed"));
            }
        } else if let Ok(entry) = library.get::<EntryFn>(b"InitDll\0") {
            if !entry(ptr::null_mut()) {
                return Err(load_error(path, "InitDll failed"));
            }
        }
    }
    // SAFETY: as above.
    let exit = unsafe {
        let name: &[u8] = if cfg!(target_os = "linux") {
            b"ModuleExit\0"
        } else {
            b"ExitDll\0"
        };
        library.get::<ExitFn>(name).ok().map(|f| *f)
    };

    // SAFETY: as above; the returned pointer is owned by us (one reference).
    let factory = unsafe {
        library
            .get::<FactoryFn>(b"GetPluginFactory\0")
            .ok()
            .and_then(|get| ComPtr::from_raw(get()))
    };
    let Some(factory) = factory else {
        // The entry call succeeded, so the module is ended before the library is unloaded.
        if let Some(exit) = exit {
            // SAFETY: the library is still loaded; no object of it exists.
            unsafe { exit() };
        }
        return Err(load_error(path, "no plugin factory"));
    };
    Ok(Arc::new(Module {
        factory: Some(factory),
        exit,
        _library: library,
    }))
}

impl Module {
    fn factory(&self) -> Result<&ComPtr<IPluginFactory>, PluginError> {
        self.factory
            .as_ref()
            .ok_or_else(|| fail("module without factory"))
    }
}

// ---------------------------------------------------------------------------------------------
// Strings and ids
// ---------------------------------------------------------------------------------------------

fn c_text(chars: &[c_char]) -> String {
    let bytes: Vec<u8> = chars
        .iter()
        .take_while(|c| **c != 0)
        .map(|c| *c as u8)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

fn wide_text(chars: &[TChar]) -> String {
    let units: Vec<u16> = chars.iter().take_while(|c| **c != 0).copied().collect();
    String::from_utf16_lossy(&units)
}

fn write_wide(text: &str, out: &mut [TChar]) {
    let mut len = 0;
    for (unit, slot) in text.encode_utf16().zip(out.iter_mut()) {
        *slot = unit;
        len += 1;
    }
    if let Some(end) = out.get_mut(len) {
        *end = 0;
    }
}

/// A class id as 32 hex digits, the id of the plugin.
fn id_text(cid: &TUID) -> String {
    cid.iter().map(|b| format!("{:02x}", *b as u8)).collect()
}

// ---------------------------------------------------------------------------------------------
// Instances
// ---------------------------------------------------------------------------------------------

/// A parameter change on its way to the audio thread: the normalized value.
#[derive(Debug, Clone, Copy)]
struct ParamChange {
    id: u32,
    value: f64,
}

fn create<I: Interface>(factory: &ComPtr<IPluginFactory>, cid: &TUID) -> Option<ComPtr<I>> {
    let mut object: *mut c_void = ptr::null_mut();
    // SAFETY: `cid` and the interface id are 16 bytes each; `object` receives one reference.
    let result = unsafe {
        factory.createInstance(cid.as_ptr(), I::IID.as_ptr().cast::<c_char>(), &mut object)
    };
    if !ok(result) || object.is_null() {
        return None;
    }
    // SAFETY: `createInstance` returned a reference to an `I` that we now own.
    unsafe { ComPtr::from_raw(object.cast::<I>()) }
}

/// The audio classes of a factory.
fn audio_classes(factory: &ComPtr<IPluginFactory>) -> Vec<(TUID, String)> {
    let mut classes = Vec::new();
    // SAFETY: plain calls on the factory; the info struct is plain data the factory fills in.
    unsafe {
        for index in 0..factory.countClasses() {
            let mut info: PClassInfo = std::mem::zeroed();
            if !ok(factory.getClassInfo(index, &mut info)) {
                continue;
            }
            if c_text(&info.category) == AUDIO_CLASS {
                classes.push((info.cid, c_text(&info.name)));
            }
        }
    }
    classes
}

fn vendor_of(factory: &ComPtr<IPluginFactory>) -> String {
    // SAFETY: the info struct is plain data the factory fills in.
    unsafe {
        let mut info: PFactoryInfo = std::mem::zeroed();
        if ok(factory.getFactoryInfo(&mut info)) {
            return c_text(&info.vendor);
        }
    }
    String::new()
}

/// Channels of the first audio bus in one direction: the speaker arrangement when the plugin
/// tells it, else the bus info.
fn bus_channels(component: &ComPtr<IComponent>, direction: i32) -> Option<Vec<u32>> {
    let mut channels = Vec::new();
    // SAFETY: plain calls on the component; the info struct is plain data it fills in.
    unsafe {
        let count = component.getBusCount(AUDIO, direction);
        for index in 0..count {
            let mut info: BusInfo = std::mem::zeroed();
            if !ok(component.getBusInfo(AUDIO, direction, index, &mut info)) {
                return None;
            }
            channels.push(info.channelCount.max(0) as u32);
        }
    }
    Some(channels)
}

fn arrangement_channels(processor: &ComPtr<IAudioProcessor>, direction: i32) -> Option<u32> {
    let mut arrangement: SpeakerArrangement = 0;
    // SAFETY: plain call; the plugin writes one value.
    let result = unsafe { processor.getBusArrangement(direction, 0, &mut arrangement) };
    (ok(result) && arrangement != 0).then(|| arrangement.count_ones())
}

/// A plugin's COM objects, shared by the plugin and its processor. The last owner ends them:
/// deactivate, disconnect, controller, component; the module goes last (last field).
struct Parts {
    component: ComPtr<IComponent>,
    processor: ComPtr<IAudioProcessor>,
    controller: Option<ComPtr<IEditController>>,
    /// The controller is a class of its own, so this host initialises and ends it.
    separate: bool,
    /// The processor of an activation still exists.
    in_use: Arc<AtomicBool>,
    /// The component is active (set by the plugin, ended in `drop` if still on).
    active: AtomicBool,
    _host: ComWrapper<HostApp>,
    handler: ComWrapper<Handler>,
    module: Arc<Module>,
}

impl Parts {
    fn new(module: &Arc<Module>, cid: &TUID, path: &Path) -> Result<Parts, PluginError> {
        let factory = module.factory()?;
        let host = ComWrapper::new(HostApp);
        let handler = Handler::new();
        let context = host
            .to_com_ptr::<FUnknown>()
            .ok_or_else(|| fail("host object"))?;

        let component =
            create::<IComponent>(factory, cid).ok_or_else(|| load_error(path, "no component"))?;
        // SAFETY: plain COM calls in the order of the VST3 spec; `context` outlives them.
        unsafe {
            if !ok(component.initialize(context.as_ptr())) {
                return Err(load_error(path, "component refused to initialize"));
            }
        }
        let Some(processor) = component.cast::<IAudioProcessor>() else {
            // SAFETY: the component was initialized above.
            unsafe { component.terminate() };
            return Err(load_error(path, "component is no audio processor"));
        };

        let mut parts = Parts {
            component,
            processor,
            controller: None,
            separate: false,
            in_use: Arc::new(AtomicBool::new(false)),
            active: AtomicBool::new(false),
            _host: host,
            handler,
            module: Arc::clone(module),
        };
        parts.find_controller(&context);
        Ok(parts)
    }

    /// The controller is the component itself or a class of its own (then connected to it).
    fn find_controller(&mut self, context: &ComPtr<FUnknown>) {
        if let Some(controller) = self.component.cast::<IEditController>() {
            self.controller = Some(controller);
        } else {
            let mut cid: TUID = [0; TUID_BYTES];
            // SAFETY: plain calls; `cid` receives the class id.
            let known = unsafe { ok(self.component.getControllerClassId(&mut cid)) };
            let created = known
                .then(|| self.module.factory.as_ref())
                .flatten()
                .and_then(|factory| create::<IEditController>(factory, &cid));
            if let Some(controller) = created {
                // SAFETY: plain COM calls; `context` outlives them.
                let started = unsafe { ok(controller.initialize(context.as_ptr())) };
                if started {
                    self.separate = true;
                    self.controller = Some(controller);
                }
            }
        }
        let Some(controller) = &self.controller else {
            return;
        };
        // SAFETY: plain COM calls with live objects; the handler outlives the controller.
        unsafe {
            if let Some(handler) = self.handler.as_com_ref::<IComponentHandler>() {
                controller.setComponentHandler(handler.as_ptr());
            }
            if self.separate {
                let (a, b) = (
                    self.component.cast::<IConnectionPoint>(),
                    controller.cast::<IConnectionPoint>(),
                );
                if let (Some(a), Some(b)) = (a, b) {
                    a.connect(b.as_ptr());
                    b.connect(a.as_ptr());
                }
            }
        }
        // The controller starts from the state of the component, as a host does.
        if let Some(bytes) = self.component_state() {
            self.controller_state(&bytes);
        }
    }

    fn component_state(&self) -> Option<Vec<u8>> {
        let stream = MemStream::new(Vec::new());
        let pointer = stream.as_com_ref::<IBStream>()?.as_ptr();
        // SAFETY: the stream is alive during the call.
        let result = unsafe { self.component.getState(pointer) };
        ok(result).then(|| stream.data.borrow().clone())
    }

    fn controller_state(&self, bytes: &[u8]) {
        let (Some(controller), stream) = (&self.controller, MemStream::new(bytes.to_vec())) else {
            return;
        };
        if let Some(pointer) = stream.as_com_ref::<IBStream>() {
            // SAFETY: the stream is alive during the call. A controller that does not want the
            // component state answers not implemented, which is fine.
            unsafe { controller.setComponentState(pointer.as_ptr()) };
        }
    }
}

impl Drop for Parts {
    fn drop(&mut self) {
        // The last owner ends the plugin: the plugin or its processor, whichever goes last.
        // SAFETY: plain COM calls in the order of the spec: deactivate, connection, controller,
        // component.
        unsafe {
            if self.active.load(Ordering::Acquire) {
                self.component.setActive(0);
            }
            if self.separate {
                if let Some(controller) = &self.controller {
                    let (a, b) = (
                        self.component.cast::<IConnectionPoint>(),
                        controller.cast::<IConnectionPoint>(),
                    );
                    if let (Some(a), Some(b)) = (a, b) {
                        a.disconnect(b.as_ptr());
                        b.disconnect(a.as_ptr());
                    }
                    controller.terminate();
                }
            }
            self.component.terminate();
        }
    }
}

/// Lists the plugins of a module, with their ports (needs an instance of each).
pub(super) fn probe(path: &Path) -> Result<Vec<PluginInfo>, PluginError> {
    let module = load_module(path)?;
    let vendor = vendor_of(module.factory()?);

    let mut found = Vec::new();
    let mut first_error = None;
    for (cid, name) in audio_classes(module.factory()?) {
        let parts = match Parts::new(&module, &cid, path) {
            Ok(parts) => parts,
            Err(e) => {
                first_error.get_or_insert(e);
                continue;
            }
        };
        found.push(PluginInfo {
            format: PluginFormat::Vst3,
            id: id_text(&cid),
            name,
            vendor: vendor.clone(),
            path: path.to_path_buf(),
            ports: PortLayout {
                inputs: bus_channels(&parts.component, INPUT).unwrap_or_default(),
                outputs: bus_channels(&parts.component, OUTPUT).unwrap_or_default(),
            },
        });
    }
    match (found.is_empty(), first_error) {
        (true, Some(e)) => Err(e),
        _ => Ok(found),
    }
}

pub(super) struct Vst3Plugin {
    info: PluginInfo,
    parts: Arc<Parts>,
    /// Parameters as they were at load: ids, names and ranges in plain units.
    params: Vec<ParamInfo>,
    /// Changes made while inactive, handed to the audio thread with the first block.
    pending: Vec<ParamChange>,
    ring: Option<Producer<ParamChange>>,
    active: bool,
    in_channels: usize,
    out_channels: usize,
    /// The plugin's own editor while it is open.
    editor: Option<editor::Vst3Editor>,
}

impl Vst3Plugin {
    pub(super) fn open(path: &Path, id: Option<&str>) -> Result<Vst3Plugin, PluginError> {
        let module = load_module(path)?;
        let classes = audio_classes(module.factory()?);

        let (cid, name) = match id {
            Some(wanted) => classes
                .iter()
                .find(|(cid, _)| id_text(cid) == wanted)
                .cloned()
                .ok_or_else(|| PluginError::NotFound {
                    path: path.to_path_buf(),
                    id: wanted.to_string(),
                })?,
            None if classes.len() == 1 => classes[0].clone(),
            None => {
                return Err(PluginError::Ambiguous {
                    path: path.to_path_buf(),
                    ids: classes
                        .iter()
                        .map(|(cid, _)| id_text(cid))
                        .collect::<Vec<_>>()
                        .join(", "),
                })
            }
        };
        let parts = Arc::new(Parts::new(&module, &cid, path)?);

        let ports = PortLayout {
            inputs: bus_channels(&parts.component, INPUT).unwrap_or_default(),
            outputs: bus_channels(&parts.component, OUTPUT).unwrap_or_default(),
        };
        let (Some(&in_bus), Some(&out_bus)) = (ports.inputs.first(), ports.outputs.first()) else {
            return Err(PluginError::NotAnEffect { name });
        };
        // The speaker arrangement is what counts; the bus info is the fallback.
        let in_channels = arrangement_channels(&parts.processor, INPUT).unwrap_or(in_bus);
        let out_channels = arrangement_channels(&parts.processor, OUTPUT).unwrap_or(out_bus);
        let (in_channels, out_channels) = (in_channels as usize, out_channels as usize);
        let too_wide = [in_channels, out_channels]
            .iter()
            .any(|c| *c > CHANNELS || *c == 0);
        if too_wide {
            return Err(PluginError::TooManyChannels { name });
        }

        let vendor = vendor_of(module.factory()?);
        let params = list_params(&parts);
        Ok(Vst3Plugin {
            info: PluginInfo {
                format: PluginFormat::Vst3,
                id: id_text(&cid),
                name,
                vendor,
                path: path.to_path_buf(),
                ports,
            },
            parts,
            params,
            pending: Vec::new(),
            ring: None,
            active: false,
            in_channels,
            out_channels,
            editor: None,
        })
    }

    pub(super) fn info(&self) -> &PluginInfo {
        &self.info
    }

    pub(super) fn params(&mut self) -> Vec<ParamInfo> {
        let Some(controller) = &self.parts.controller else {
            return Vec::new();
        };
        let mut params = self.params.clone();
        for p in &mut params {
            // SAFETY: plain calls on the live controller.
            p.current = unsafe {
                controller.normalizedParamToPlain(p.id, controller.getParamNormalized(p.id))
            };
        }
        params
    }

    pub(super) fn set_param(&mut self, id: u32, value: f64) -> Result<(), PluginError> {
        // A second owner, so that `deliver` can borrow `self` while the controller is in use.
        let parts = Arc::clone(&self.parts);
        let (Some(controller), Some(info)) =
            (&parts.controller, self.params.iter().find(|p| p.id == id))
        else {
            return Err(PluginError::NoParam(id));
        };
        let plain = value.clamp(info.min, info.max);
        // SAFETY: plain calls on the live controller.
        let normalized = unsafe { controller.plainParamToNormalized(id, plain) }.clamp(0.0, 1.0);
        let change = ParamChange {
            id,
            value: normalized,
        };
        self.deliver(change)?;
        // SAFETY: as above.
        unsafe { controller.setParamNormalized(id, normalized) };
        Ok(())
    }

    /// Hands a change to the audio thread, or keeps it for the activation.
    fn deliver(&mut self, change: ParamChange) -> Result<(), PluginError> {
        match self.ring.as_mut() {
            // Only a value the processor got is shown by `params`.
            Some(ring) => ring.push(change).map_err(|_| PluginError::QueueFull)?,
            // One entry per parameter, the last value wins: the list stays as short as the plugin.
            None => match self.pending.iter_mut().find(|c| c.id == change.id) {
                Some(known) => known.value = change.value,
                None => self.pending.push(change),
            },
        }
        Ok(())
    }

    /// Changes the plugin's controller made itself (`performEdit`) since the last call, in plain
    /// units. The processor is a separate object that does not hear of them: it gets them here.
    pub(super) fn take_changes(&mut self) -> Vec<(u32, f64)> {
        let parts = Arc::clone(&self.parts);
        let edits = parts.handler.take_edits();
        let Some(controller) = &parts.controller else {
            return Vec::new();
        };
        let mut changes = Vec::with_capacity(edits.len());
        for edit in edits {
            // A full ring drops it: the processor catches up with the next change.
            let _ = self.deliver(edit);
            // SAFETY: plain call on the live controller.
            let plain = unsafe { controller.normalizedParamToPlain(edit.id, edit.value) };
            changes.push((edit.id, plain));
        }
        changes
    }

    pub(super) fn latency_samples(&mut self) -> u32 {
        // SAFETY: plain call on the live processor.
        unsafe { self.parts.processor.getLatencySamples() }
    }

    pub(super) fn save_state(&mut self) -> Result<Vec<u8>, PluginError> {
        self.flush_pending()?;
        self.parts.component_state().ok_or(PluginError::NoState)
    }

    pub(super) fn load_state(&mut self, bytes: &[u8]) -> Result<(), PluginError> {
        let stream = MemStream::new(bytes.to_vec());
        let pointer = stream
            .as_com_ref::<IBStream>()
            .ok_or_else(|| fail("state stream"))?;
        // SAFETY: the stream is alive during the call.
        let result = unsafe { self.parts.component.setState(pointer.as_ptr()) };
        if !ok(result) {
            return Err(PluginError::State("the plugin refused the state".into()));
        }
        self.parts.controller_state(bytes);
        // The state is newer than any change made before it.
        self.pending.clear();
        Ok(())
    }

    /// A component takes parameter changes only in `process`. Changes made while inactive are
    /// run through a short activation (a block of no frames), so the state includes them.
    fn flush_pending(&mut self) -> Result<(), PluginError> {
        if self.active || self.pending.is_empty() {
            return Ok(());
        }
        let mut processor = self.activate(FLUSH_RATE_HZ, 1)?;
        let flushed = processor.flush();
        drop(processor);
        self.deactivate()?;
        flushed.map_err(fail)
    }

    pub(super) fn activate(
        &mut self,
        sample_rate: f64,
        max_block: u32,
    ) -> Result<Vst3Processor, PluginError> {
        let block = i32::try_from(max_block)
            .ok()
            .filter(|b| *b > 0)
            .ok_or(PluginError::Block(max_block))?;
        if self.active {
            return Err(PluginError::InUse);
        }
        let component = &self.parts.component;
        let processor = &self.parts.processor;
        let mut buses = [0, 0];

        // SAFETY: plain COM calls in the order of the spec (setup, buses, active) on live objects.
        unsafe {
            if !ok(processor.canProcessSampleSize(SAMPLE_32)) {
                return Err(fail("the plugin cannot process 32 bit samples"));
            }
            let mut setup = ProcessSetup {
                processMode: REALTIME,
                symbolicSampleSize: SAMPLE_32,
                maxSamplesPerBlock: block,
                sampleRate: sample_rate,
            };
            if !ok(processor.setupProcessing(&mut setup)) {
                return Err(fail("the plugin refused the processing setup"));
            }
            // The first audio bus of each direction works, further buses (side chain) stay off.
            for (slot, direction) in [INPUT, OUTPUT].into_iter().enumerate() {
                buses[slot] = component.getBusCount(AUDIO, direction).max(0) as usize;
                for index in 0..buses[slot] {
                    let on = TBool::from(index == 0);
                    component.activateBus(AUDIO, direction, index as i32, on);
                }
            }
            if !ok(component.setActive(1)) {
                return Err(fail("the plugin refused to activate"));
            }
        }
        self.active = true;
        self.parts.active.store(true, Ordering::Release);

        let (mut tx, rx) = RingBuffer::new(PARAM_RING_SLOTS.max(self.pending.len()));
        for change in self.pending.drain(..) {
            let _ = tx.push(change);
        }
        self.ring = Some(tx);
        self.parts.in_use.store(true, Ordering::Release);

        Ok(Vst3Processor::new(
            &self.parts,
            rx,
            sample_rate,
            max_block as usize,
            [self.in_channels, self.out_channels],
            buses,
        ))
    }

    pub(super) fn deactivate(&mut self) -> Result<(), PluginError> {
        if !self.active {
            return Ok(());
        }
        if self.parts.in_use.load(Ordering::Acquire) {
            return Err(PluginError::InUse);
        }
        // SAFETY: plain call on the live component, the audio side is gone.
        unsafe { self.parts.component.setActive(0) };
        self.parts.active.store(false, Ordering::Release);
        self.active = false;
        self.ring = None;
        Ok(())
    }
}

impl Drop for Vst3Plugin {
    fn drop(&mut self) {
        // The editor goes before the plugin; the component ends inactive.
        self.close_editor();
        let _ = self.deactivate();
    }
}

fn list_params(parts: &Parts) -> Vec<ParamInfo> {
    let Some(controller) = &parts.controller else {
        return Vec::new();
    };
    let mut params = Vec::new();
    // SAFETY: plain calls on the live controller; the info struct is plain data it fills in.
    unsafe {
        for index in 0..controller.getParameterCount() {
            let mut info: ParameterInfo = std::mem::zeroed();
            if !ok(controller.getParameterInfo(index, &mut info)) {
                continue;
            }
            if info.flags & ParameterInfo_::ParameterFlags_::kIsHidden != 0 {
                continue;
            }
            let plain = |normalized: f64| controller.normalizedParamToPlain(info.id, normalized);
            params.push(ParamInfo {
                id: info.id,
                name: wide_text(&info.title),
                min: plain(0.0),
                max: plain(1.0),
                default: plain(info.defaultNormalizedValue),
                current: plain(controller.getParamNormalized(info.id)),
            });
        }
    }
    params
}

pub(super) struct Vst3Processor {
    changes: Consumer<ParamChange>,
    input_changes: ComWrapper<Changes>,
    output_changes: ComWrapper<Changes>,
    context: Box<ProcessContext>,
    in_buses: Vec<AudioBusBuffers>,
    out_buses: Vec<AudioBusBuffers>,
    /// The block as the plugin's buses want it; the channel pointers of the bus buffers point in.
    scratch_in: [Vec<f32>; CHANNELS],
    scratch_out: [Vec<f32>; CHANNELS],
    /// Pointer lists behind the bus buffers.
    in_ptrs: Box<[*mut f32; CHANNELS]>,
    out_ptrs: Box<[*mut f32; CHANNELS]>,
    in_channels: usize,
    out_channels: usize,
    max_block: usize,
    processing: bool,
    /// Keeps the plugin (and with it the library) alive as long as the processor lives. Last
    /// field: it drops after everything else of the processor.
    parts: Arc<Parts>,
}

// SAFETY: a VST3 processor may be called from any one thread at a time, here the audio thread
// that owns this value; its audio calls and the raw pointers into the owned buffers are used from
// that thread only. The plugin on the main thread shares `Parts` and calls only the component and
// the controller. The engine drops the processor on its control thread, not in the callback.
unsafe impl Send for Vst3Processor {}

impl Vst3Processor {
    fn new(
        parts: &Arc<Parts>,
        changes: Consumer<ParamChange>,
        sample_rate: f64,
        max_block: usize,
        channels: [usize; 2],
        buses: [usize; 2],
    ) -> Vst3Processor {
        let [in_channels, out_channels] = channels;
        let mut scratch_in = [vec![0.0_f32; max_block], vec![0.0_f32; max_block]];
        let mut scratch_out = [vec![0.0_f32; max_block], vec![0.0_f32; max_block]];
        let mut in_ptrs = Box::new([scratch_in[0].as_mut_ptr(), scratch_in[1].as_mut_ptr()]);
        let mut out_ptrs = Box::new([scratch_out[0].as_mut_ptr(), scratch_out[1].as_mut_ptr()]);

        let bus_list = |count: usize, channels: usize, list: &mut Box<[*mut f32; CHANNELS]>| {
            (0..count)
                .map(|i| {
                    let used = i == 0;
                    AudioBusBuffers {
                        numChannels: if used { channels as i32 } else { 0 },
                        silenceFlags: 0,
                        __field0: AudioBusBuffers__type0 {
                            channelBuffers32: if used {
                                list.as_mut_ptr()
                            } else {
                                ptr::null_mut()
                            },
                        },
                    }
                })
                .collect::<Vec<_>>()
        };
        let in_buses = bus_list(buses[0], in_channels, &mut in_ptrs);
        let out_buses = bus_list(buses[1], out_channels, &mut out_ptrs);

        // SAFETY: `ProcessContext` is plain data; all zero means no transport state.
        let mut context: Box<ProcessContext> = Box::new(unsafe { std::mem::zeroed() });
        context.sampleRate = sample_rate;

        Vst3Processor {
            changes,
            input_changes: Changes::new(MAX_QUEUES),
            output_changes: Changes::new(0),
            context,
            in_buses,
            out_buses,
            scratch_in,
            scratch_out,
            in_ptrs,
            out_ptrs,
            in_channels,
            out_channels,
            max_block,
            processing: false,
            parts: Arc::clone(parts),
        }
    }

    /// A block of no frames: only the parameter changes are delivered.
    pub(super) fn flush(&mut self) -> Result<(), ProcessError> {
        self.block([&[], &[]], [&mut [], &mut []])
    }

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

        // A mono bus gets the mean of both channels.
        if self.in_channels == 1 {
            let mean = input[0].iter().zip(input[1]).map(|(l, r)| 0.5 * (l + r));
            for (s, m) in self.scratch_in[0].iter_mut().zip(mean) {
                *s = m;
            }
        } else {
            self.scratch_in[0][..n].copy_from_slice(input[0]);
            self.scratch_in[1][..n].copy_from_slice(input[1]);
        }

        // Parameter changes of the main thread: one value per parameter and block.
        self.input_changes.count.set(0);
        while let Ok(change) = self.changes.peek() {
            if !self.input_changes.put(change.id, change.value) {
                break;
            }
            let _ = self.changes.pop();
        }

        if !self.processing {
            // SAFETY: plain call on the live processor, on the audio thread as the spec wants.
            // A plugin without the call answers not implemented, which is fine.
            unsafe { self.parts.processor.setProcessing(1) };
            self.processing = true;
        }

        for bus in self.in_buses.iter_mut().chain(self.out_buses.iter_mut()) {
            bus.silenceFlags = 0;
        }
        let (Some(changes_in), Some(changes_out)) = (
            self.input_changes.as_com_ref::<IParameterChanges>(),
            self.output_changes.as_com_ref::<IParameterChanges>(),
        ) else {
            return Err(ProcessError::Process);
        };
        let mut data = ProcessData {
            processMode: REALTIME,
            symbolicSampleSize: SAMPLE_32,
            numSamples: n as i32,
            numInputs: self.in_buses.len() as i32,
            numOutputs: self.out_buses.len() as i32,
            inputs: self.in_buses.as_mut_ptr(),
            outputs: self.out_buses.as_mut_ptr(),
            inputParameterChanges: changes_in.as_ptr(),
            outputParameterChanges: changes_out.as_ptr(),
            inputEvents: ptr::null_mut(),
            outputEvents: ptr::null_mut(),
            processContext: &mut *self.context,
        };
        // SAFETY: `data` points at buffers and COM objects owned by `self` that stay put (boxes
        // and vectors that are not resized) and live past the call; the scratch pointers were set
        // up for `max_block` frames and `n <= max_block`.
        let result = unsafe { self.parts.processor.process(&mut data) };
        self.context.projectTimeSamples += n as i64;
        if !ok(result) {
            return Err(ProcessError::Process);
        }

        let [o_l, o_r] = output;
        o_l.copy_from_slice(&self.scratch_out[0][..n]);
        // A mono bus comes out on both channels.
        let right = if self.out_channels == 1 { 0 } else { 1 };
        o_r.copy_from_slice(&self.scratch_out[right][..n]);
        Ok(())
    }
}

impl Drop for Vst3Processor {
    fn drop(&mut self) {
        if self.processing {
            // SAFETY: plain call on the live processor; processing ends on the thread that owns it.
            unsafe { self.parts.processor.setProcessing(0) };
        }
        let _ = (&self.in_ptrs, &self.out_ptrs);
        self.parts.in_use.store(false, Ordering::Release);
    }
}
