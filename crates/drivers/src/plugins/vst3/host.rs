//! The COM objects this host offers to plugins: the host application, the component handler,
//! the state stream and the parameter changes.

use super::write_wide;
use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::ptr;
use vst3::Steinberg::Vst::*;
use vst3::Steinberg::*;
use vst3::{Class, ComWrapper};

const HOST_NAME: &str = "Kontra";

/// What `initialize` gets as context.
pub(super) struct HostApp;

impl Class for HostApp {
    type Interfaces = (IHostApplication,);
}

impl IHostApplicationTrait for HostApp {
    unsafe fn getName(&self, name: *mut String128) -> tresult {
        // SAFETY: the plugin passes a valid `String128` to fill.
        write_wide(HOST_NAME, unsafe { &mut *name });
        kResultOk
    }

    // No IMessage and no IAttributeList: plugins that need them fail their own setup.
    unsafe fn createInstance(
        &self,
        _cid: *mut TUID,
        _iid: *mut TUID,
        _obj: *mut *mut c_void,
    ) -> tresult {
        kNotImplemented
    }
}

/// The controller's handler: Kontra shows no plugin windows (FX5), edits and restarts are ignored.
pub(super) struct Handler;

impl Class for Handler {
    type Interfaces = (IComponentHandler,);
}

impl IComponentHandlerTrait for Handler {
    unsafe fn beginEdit(&self, _id: ParamID) -> tresult {
        kResultOk
    }

    unsafe fn performEdit(&self, _id: ParamID, _value: ParamValue) -> tresult {
        kResultOk
    }

    unsafe fn endEdit(&self, _id: ParamID) -> tresult {
        kResultOk
    }

    unsafe fn restartComponent(&self, _flags: int32) -> tresult {
        kResultOk
    }
}

/// `IBStream` over bytes in memory, for the state.
pub(super) struct MemStream {
    pub(super) data: RefCell<Vec<u8>>,
    pos: Cell<usize>,
}

impl MemStream {
    pub(super) fn new(data: Vec<u8>) -> ComWrapper<MemStream> {
        ComWrapper::new(MemStream {
            data: RefCell::new(data),
            pos: Cell::new(0),
        })
    }
}

impl Class for MemStream {
    type Interfaces = (IBStream,);
}

impl IBStreamTrait for MemStream {
    unsafe fn read(&self, buffer: *mut c_void, num: int32, read: *mut int32) -> tresult {
        let data = self.data.borrow();
        let start = self.pos.get().min(data.len());
        let n = (num.max(0) as usize).min(data.len() - start);
        // SAFETY: the caller's buffer holds `num` bytes, `n <= num`; the source is in range.
        unsafe { ptr::copy_nonoverlapping(data[start..].as_ptr(), buffer.cast::<u8>(), n) };
        self.pos.set(start + n);
        if !read.is_null() {
            // SAFETY: not null, points to an `int32` of the caller.
            unsafe { *read = n as int32 };
        }
        kResultOk
    }

    unsafe fn write(&self, buffer: *mut c_void, num: int32, written: *mut int32) -> tresult {
        let n = num.max(0) as usize;
        // SAFETY: the caller's buffer holds `num` bytes.
        let bytes = unsafe { std::slice::from_raw_parts(buffer.cast::<u8>(), n) };
        let mut data = self.data.borrow_mut();
        let start = self.pos.get().min(data.len());
        let end = start + n;
        if end > data.len() {
            data.resize(end, 0);
        }
        data[start..end].copy_from_slice(bytes);
        self.pos.set(end);
        if !written.is_null() {
            // SAFETY: not null, points to an `int32` of the caller.
            unsafe { *written = n as int32 };
        }
        kResultOk
    }

    unsafe fn seek(&self, pos: int64, mode: int32, result: *mut int64) -> tresult {
        let len = self.data.borrow().len() as int64;
        let base = match mode as IBStream_::IStreamSeekMode {
            IBStream_::IStreamSeekMode_::kIBSeekSet => 0,
            IBStream_::IStreamSeekMode_::kIBSeekCur => self.pos.get() as int64,
            IBStream_::IStreamSeekMode_::kIBSeekEnd => len,
            _ => return kInvalidArgument,
        };
        let target = base.saturating_add(pos);
        if target < 0 {
            return kInvalidArgument;
        }
        self.pos.set(target as usize);
        if !result.is_null() {
            // SAFETY: not null, points to an `int64` of the caller.
            unsafe { *result = target };
        }
        kResultOk
    }

    unsafe fn tell(&self, pos: *mut int64) -> tresult {
        if pos.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: not null, points to an `int64` of the caller.
        unsafe { *pos = self.pos.get() as int64 };
        kResultOk
    }
}

/// One parameter's change in one block: a single point at the start.
pub(super) struct Queue {
    id: Cell<ParamID>,
    value: Cell<f64>,
}

impl Class for Queue {
    type Interfaces = (IParamValueQueue,);
}

impl IParamValueQueueTrait for Queue {
    unsafe fn getParameterId(&self) -> ParamID {
        self.id.get()
    }

    unsafe fn getPointCount(&self) -> int32 {
        1
    }

    unsafe fn getPoint(&self, index: int32, offset: *mut int32, value: *mut ParamValue) -> tresult {
        if index != 0 {
            return kResultFalse;
        }
        // SAFETY: the plugin passes valid pointers.
        unsafe {
            *offset = 0;
            *value = self.value.get();
        }
        kResultTrue
    }

    unsafe fn addPoint(&self, _offset: int32, _value: ParamValue, _index: *mut int32) -> tresult {
        kResultFalse
    }
}

/// The changes of a block, made of preallocated queues. The output list has none and takes none.
pub(super) struct Changes {
    queues: Vec<ComWrapper<Queue>>,
    pub(super) count: Cell<usize>,
}

impl Changes {
    pub(super) fn new(queues: usize) -> ComWrapper<Changes> {
        let queues = (0..queues)
            .map(|_| {
                ComWrapper::new(Queue {
                    id: Cell::new(0),
                    value: Cell::new(0.0),
                })
            })
            .collect();
        ComWrapper::new(Changes {
            queues,
            count: Cell::new(0),
        })
    }

    /// Sets `id` to `value` for this block: a second change of the same parameter replaces the
    /// first. False when all queues are in use.
    pub(super) fn put(&self, id: ParamID, value: f64) -> bool {
        let count = self.count.get();
        let existing = self.queues[..count].iter().find(|q| q.id.get() == id);
        if let Some(queue) = existing {
            queue.value.set(value);
            return true;
        }
        let Some(queue) = self.queues.get(count) else {
            return false;
        };
        queue.id.set(id);
        queue.value.set(value);
        self.count.set(count + 1);
        true
    }
}

impl Class for Changes {
    type Interfaces = (IParameterChanges,);
}

impl IParameterChangesTrait for Changes {
    unsafe fn getParameterCount(&self) -> int32 {
        self.count.get() as int32
    }

    unsafe fn getParameterData(&self, index: int32) -> *mut IParamValueQueue {
        let queue = usize::try_from(index)
            .ok()
            .filter(|i| *i < self.count.get())
            .and_then(|i| self.queues.get(i));
        match queue.and_then(|q| q.as_com_ref::<IParamValueQueue>()) {
            Some(queue) => queue.as_ptr(),
            None => ptr::null_mut(),
        }
    }

    unsafe fn addParameterData(
        &self,
        _id: *const ParamID,
        _index: *mut int32,
    ) -> *mut IParamValueQueue {
        ptr::null_mut()
    }
}
