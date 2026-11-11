//! The gain plugin's editor view for the host tests of FX5. It draws nothing; it does what a
//! real view does to its host, observably:
//!
//! ```text
//!  attached        ─► registers a timer and (Linux) a pipe with one byte at the host's IRunLoop
//!  onFDIsSet       ─► reads the byte, asks the frame for EDITOR_RESIZED (resizeView)
//!  onTimer         ─► "turns the knob": performEdit(gain, EDITOR_GAIN)
//!  checkSizeConstraint ─► no wider than EDITOR_MAX_WIDTH
//!  removed         ─► unregisters both
//! ```

use crate::{GAIN_MAX, GAIN_PARAM_ID};
use std::cell::{Cell, RefCell};
use std::ffi::{c_char, c_void, CStr};
use std::rc::Rc;
use vst3::Steinberg::Linux::*;
use vst3::Steinberg::Vst::*;
use vst3::Steinberg::*;
use vst3::{Class, ComPtr, ComRef, ComWrapper};

pub const EDITOR_SIZE: (u32, u32) = (300, 200);
pub const EDITOR_RESIZED: (u32, u32) = (320, 240);
pub const EDITOR_GAIN: f64 = 2.5;
pub const EDITOR_MAX_WIDTH: u32 = 400;
const TIMER_MS: u64 = 10;

/// What the view and its helpers share.
struct Shared {
    handler: Option<ComPtr<IComponentHandler>>,
    frame: RefCell<Option<ComPtr<IPlugFrame>>>,
    size: Cell<(u32, u32)>,
    #[cfg(target_os = "linux")]
    pipe: RefCell<Option<(std::io::PipeReader, std::io::PipeWriter)>>,
}

pub struct View {
    shared: Rc<Shared>,
    tick: ComWrapper<Tick>,
    wake: ComWrapper<Wake>,
}

impl View {
    pub fn new(handler: Option<ComPtr<IComponentHandler>>) -> View {
        let shared = Rc::new(Shared {
            handler,
            frame: RefCell::new(None),
            size: Cell::new(EDITOR_SIZE),
            #[cfg(target_os = "linux")]
            pipe: RefCell::new(None),
        });
        View {
            tick: ComWrapper::new(Tick(Rc::clone(&shared))),
            wake: ComWrapper::new(Wake(Rc::clone(&shared))),
            shared,
        }
    }

    fn run_loop(&self) -> Option<ComPtr<IRunLoop>> {
        self.shared.frame.borrow().as_ref()?.cast::<IRunLoop>()
    }
}

fn rect_of((width, height): (u32, u32)) -> ViewRect {
    ViewRect {
        left: 0,
        top: 0,
        right: width as i32,
        bottom: height as i32,
    }
}

impl Class for View {
    type Interfaces = (IPlugView,);
}

impl IPlugViewTrait for View {
    unsafe fn isPlatformTypeSupported(&self, kind: FIDString) -> tresult {
        let known = [kPlatformTypeX11EmbedWindowID, kPlatformTypeHWND];
        let wanted = |k: FIDString| CStr::from_ptr(k) == CStr::from_ptr(kind);
        if !kind.is_null() && known.into_iter().any(wanted) {
            kResultTrue
        } else {
            kResultFalse
        }
    }

    unsafe fn attached(&self, _parent: *mut c_void, _kind: FIDString) -> tresult {
        let Some(run_loop) = self.run_loop() else {
            return kResultFalse;
        };
        if let Some(tick) = self.tick.as_com_ref::<ITimerHandler>() {
            run_loop.registerTimer(tick.as_ptr(), TIMER_MS);
        }
        #[cfg(target_os = "linux")]
        {
            use std::io::Write;
            use std::os::fd::AsRawFd;
            if let Ok((reader, mut writer)) = std::io::pipe() {
                let _ = writer.write_all(b"x");
                if let Some(wake) = self.wake.as_com_ref::<IEventHandler>() {
                    run_loop.registerEventHandler(wake.as_ptr(), reader.as_raw_fd());
                }
                *self.shared.pipe.borrow_mut() = Some((reader, writer));
            }
        }
        kResultOk
    }

    unsafe fn removed(&self) -> tresult {
        if let Some(run_loop) = self.run_loop() {
            if let Some(tick) = self.tick.as_com_ref::<ITimerHandler>() {
                run_loop.unregisterTimer(tick.as_ptr());
            }
            if let Some(wake) = self.wake.as_com_ref::<IEventHandler>() {
                run_loop.unregisterEventHandler(wake.as_ptr());
            }
        }
        #[cfg(target_os = "linux")]
        self.shared.pipe.borrow_mut().take();
        kResultOk
    }

    unsafe fn onWheel(&self, _distance: f32) -> tresult {
        kResultFalse
    }

    unsafe fn onKeyDown(&self, _key: char16, _code: int16, _modifiers: int16) -> tresult {
        kResultFalse
    }

    unsafe fn onKeyUp(&self, _key: char16, _code: int16, _modifiers: int16) -> tresult {
        kResultFalse
    }

    unsafe fn getSize(&self, size: *mut ViewRect) -> tresult {
        *size = rect_of(self.shared.size.get());
        kResultOk
    }

    unsafe fn onSize(&self, size: *mut ViewRect) -> tresult {
        let rect = &*size;
        let width = (rect.right - rect.left).max(0) as u32;
        let height = (rect.bottom - rect.top).max(0) as u32;
        self.shared.size.set((width, height));
        kResultOk
    }

    unsafe fn onFocus(&self, _state: TBool) -> tresult {
        kResultOk
    }

    unsafe fn setFrame(&self, frame: *mut IPlugFrame) -> tresult {
        *self.shared.frame.borrow_mut() = ComRef::from_raw(frame).map(|f| f.to_com_ptr());
        kResultOk
    }

    unsafe fn canResize(&self) -> tresult {
        kResultTrue
    }

    unsafe fn checkSizeConstraint(&self, rect: *mut ViewRect) -> tresult {
        let rect = &mut *rect;
        let widest = rect.left + EDITOR_MAX_WIDTH as i32;
        if rect.right > widest {
            rect.right = widest;
        }
        kResultTrue
    }
}

/// The view's timer: the knob turns.
struct Tick(Rc<Shared>);

impl Class for Tick {
    type Interfaces = (ITimerHandler,);
}

impl ITimerHandlerTrait for Tick {
    unsafe fn onTimer(&self) {
        let Some(handler) = &self.0.handler else {
            return;
        };
        let normalized = EDITOR_GAIN / GAIN_MAX;
        handler.beginEdit(GAIN_PARAM_ID);
        handler.performEdit(GAIN_PARAM_ID, normalized);
        handler.endEdit(GAIN_PARAM_ID);
    }
}

/// The view's descriptor: news, so it asks for a new size.
struct Wake(Rc<Shared>);

impl Class for Wake {
    type Interfaces = (IEventHandler,);
}

impl IEventHandlerTrait for Wake {
    unsafe fn onFDIsSet(&self, _fd: FileDescriptor) {
        #[cfg(target_os = "linux")]
        {
            use std::io::Read;
            let mut byte = [0_u8; 1];
            let read = match self.0.pipe.borrow_mut().as_mut() {
                Some((reader, _)) => reader.read(&mut byte),
                None => return,
            };
            if !read.is_ok_and(|n| n == 1) {
                return;
            }
        }
        let frame = self.0.frame.borrow().clone();
        if let Some(frame) = frame {
            let mut rect = rect_of(EDITOR_RESIZED);
            frame.resizeView(std::ptr::null_mut(), &mut rect);
        }
    }
}

/// The name the host asks for.
pub(crate) fn is_editor(name: *const c_char) -> bool {
    // SAFETY: the host passes a C string that lives through the call, or null (the editor).
    name.is_null() || unsafe { CStr::from_ptr(name) } == c"editor"
}
