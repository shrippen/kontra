//! A VST 3 plugin's own editor (FX5): its `IPlugView` embedded in an `EditorFrame`. The host's
//! `IPlugFrame` is also its `IRunLoop` (Linux): the plugin registers timers and descriptors
//! there, and the plugin thread runs them in `idle`, as for CLAP.
//!
//! ```text
//!  controller.createView ─► view: isPlatformTypeSupported(X11) ─ setFrame(Frame) ─ getSize
//!  make_frame ─► window ─► view.attached(window) ─► window.show
//!  every tick:  timers due ─► onTimer    descriptor readable ─► onFDIsSet
//!               view asks resizeView ─► window.resize, view.onSize
//!               window resized ─► checkSizeConstraint, onSize      closed ─► removed
//! ```

use super::{ok, Vst3Plugin};
use crate::plugins::editor::{EditorFrame, FrameEvent, FrameHandle, FrameMaker};
use crate::plugins::PluginError;
use std::cell::RefCell;
use std::ffi::c_void;
use std::ptr;
use std::time::{Duration, Instant};
use vst3::Steinberg::Linux::*;
use vst3::Steinberg::Vst::IEditControllerTrait;
use vst3::Steinberg::*;
use vst3::{Class, ComPtr, ComRef, ComWrapper};

/// The view the host asks the controller for (VST 3: `ViewType::kEditor`).
const EDITOR_VIEW: &std::ffi::CStr = c"editor";

/// The size an editor gets when the plugin does not say.
const FALLBACK_SIZE: (u32, u32) = (640, 480);

/// Timers faster than this are slowed down, as for CLAP.
const MIN_TIMER_PERIOD: Duration = Duration::from_millis(10);

/// A timer the plugin registered.
struct Timer {
    handler: ComPtr<ITimerHandler>,
    period: Duration,
    next: Instant,
}

/// The host's frame for one view: the size the plugin asks for, and its run loop.
#[derive(Default)]
struct Frame {
    resize: RefCell<Option<(u32, u32)>>,
    timers: RefCell<Vec<Timer>>,
    handlers: RefCell<Vec<(ComPtr<IEventHandler>, FileDescriptor)>>,
}

impl Class for Frame {
    type Interfaces = (IPlugFrame, IRunLoop);
}

/// Width and height of a rect, never negative.
fn size_of(rect: &ViewRect) -> (u32, u32) {
    let side = |a: i32, b: i32| u32::try_from(b.saturating_sub(a)).unwrap_or(0);
    (side(rect.left, rect.right), side(rect.top, rect.bottom))
}

fn rect_of((width, height): (u32, u32)) -> ViewRect {
    let side = |v: u32| i32::try_from(v).unwrap_or(i32::MAX);
    ViewRect {
        left: 0,
        top: 0,
        right: side(width),
        bottom: side(height),
    }
}

impl IPlugFrameTrait for Frame {
    unsafe fn resizeView(&self, _view: *mut IPlugView, new_size: *mut ViewRect) -> tresult {
        // SAFETY: the plugin passes a valid rect for the call, or null.
        let Some(rect) = (unsafe { new_size.as_ref() }) else {
            return kInvalidArgument;
        };
        *self.resize.borrow_mut() = Some(size_of(rect));
        kResultOk
    }
}

impl IRunLoopTrait for Frame {
    unsafe fn registerEventHandler(
        &self,
        handler: *mut IEventHandler,
        fd: FileDescriptor,
    ) -> tresult {
        // SAFETY: the plugin passes a live handler; the frame keeps a reference of its own.
        let Some(handler) = (unsafe { ComRef::from_raw(handler) }) else {
            return kInvalidArgument;
        };
        self.handlers.borrow_mut().push((handler.to_com_ptr(), fd));
        kResultOk
    }

    unsafe fn unregisterEventHandler(&self, handler: *mut IEventHandler) -> tresult {
        self.handlers
            .borrow_mut()
            .retain(|(h, _)| h.as_ptr() != handler);
        kResultOk
    }

    unsafe fn registerTimer(
        &self,
        handler: *mut ITimerHandler,
        milliseconds: TimerInterval,
    ) -> tresult {
        // SAFETY: as in `registerEventHandler`.
        let Some(handler) = (unsafe { ComRef::from_raw(handler) }) else {
            return kInvalidArgument;
        };
        let period = Duration::from_millis(milliseconds).max(MIN_TIMER_PERIOD);
        self.timers.borrow_mut().push(Timer {
            handler: handler.to_com_ptr(),
            period,
            next: Instant::now() + period,
        });
        kResultOk
    }

    unsafe fn unregisterTimer(&self, handler: *mut ITimerHandler) -> tresult {
        self.timers
            .borrow_mut()
            .retain(|t| t.handler.as_ptr() != handler);
        kResultOk
    }
}

impl Frame {
    /// The timers due now, each moved a period on.
    fn due(&self, now: Instant) -> Vec<ComPtr<ITimerHandler>> {
        let mut timers = self.timers.borrow_mut();
        let mut due = Vec::new();
        for t in timers.iter_mut().filter(|t| t.next <= now) {
            due.push(t.handler.clone());
            t.next = now + t.period;
        }
        due
    }
}

/// An open editor: the plugin's view, the host's frame object and the window.
pub(super) struct Vst3Editor {
    view: ComPtr<IPlugView>,
    frame: ComWrapper<Frame>,
    window: Box<dyn EditorFrame>,
}

fn editor_error(message: &str) -> PluginError {
    PluginError::Editor(message.to_string())
}

/// The window type this platform embeds views with.
fn platform() -> FIDString {
    kPlatformTypeX11EmbedWindowID
}

impl Vst3Plugin {
    /// Opens the plugin's editor in a frame from `make_frame`. An open editor stays as it is.
    pub(in crate::plugins) fn open_editor(
        &mut self,
        title: &str,
        make_frame: FrameMaker,
    ) -> Result<(), PluginError> {
        if self.editor.is_some() {
            return Ok(());
        }
        let controller = self
            .parts
            .controller
            .as_ref()
            .ok_or(PluginError::NoEditor)?;
        // SAFETY: plain call on the live controller; the name lives through it. The returned
        // view carries a reference that `from_raw` takes over.
        let view = unsafe { ComPtr::from_raw(controller.createView(EDITOR_VIEW.as_ptr())) };
        let view = view.ok_or(PluginError::NoEditor)?;
        // SAFETY: plain call on the live view.
        if unsafe { view.isPlatformTypeSupported(platform()) } != kResultTrue {
            return Err(PluginError::NoEditor);
        }

        let frame = ComWrapper::new(Frame::default());
        let frame_ptr = frame
            .as_com_ref::<IPlugFrame>()
            .ok_or_else(|| editor_error("frame object"))?
            .as_ptr();
        // SAFETY: the frame object lives as long as the editor (`Vst3Editor` keeps it), and
        // `close_editor` unsets it before it goes.
        unsafe { view.setFrame(frame_ptr) };
        let mut rect = rect_of(FALLBACK_SIZE);
        // SAFETY: `rect` is valid for the call.
        let sized = unsafe { ok(view.getSize(&mut rect)) };
        let (width, height) = if sized { size_of(&rect) } else { FALLBACK_SIZE };

        let window = match make_frame(title, width, height) {
            Ok(window) => window,
            Err(e) => {
                // SAFETY: as above; the view lets go of the frame before both go.
                unsafe { view.setFrame(ptr::null_mut()) };
                return Err(e);
            }
        };
        let FrameHandle::X11(id) = window.handle();
        // SAFETY: the window exists (the frame owns it) and stays until `removed` in
        // `close_editor`; an X11 id travels as the pointer value, as the VST 3 spec says.
        let attached = unsafe { view.attached(id as usize as *mut c_void, platform()) };
        if !ok(attached) {
            // SAFETY: as above.
            unsafe { view.setFrame(ptr::null_mut()) };
            drop(window);
            return Err(editor_error("the view did not attach to the window"));
        }
        let mut window = window;
        window.show();
        self.editor = Some(Vst3Editor {
            view,
            frame,
            window,
        });
        Ok(())
    }

    /// Closes the editor if it is open: the view lets go of the window before it goes.
    pub(in crate::plugins) fn close_editor(&mut self) {
        let Some(editor) = self.editor.take() else {
            return;
        };
        // SAFETY: the view was attached to the window, which is still there.
        unsafe {
            editor.view.removed();
            editor.view.setFrame(ptr::null_mut());
        }
        drop(editor.view);
        drop(editor.frame);
        drop(editor.window);
    }

    pub(in crate::plugins) fn editor_open(&self) -> bool {
        self.editor.is_some()
    }

    /// One tick: the editor's timers and descriptors, its size wishes and its window.
    pub(in crate::plugins) fn idle(&mut self) {
        let Some(editor) = self.editor.as_mut() else {
            return;
        };
        let frame = &editor.frame;
        for handler in frame.due(Instant::now()) {
            // SAFETY: the handler is alive (the frame holds a reference).
            unsafe { handler.onTimer() };
        }
        #[cfg(target_os = "linux")]
        {
            let registered: Vec<_> = frame.handlers.borrow().clone();
            let fds: Vec<_> = registered.iter().map(|(_, fd)| *fd).collect();
            for fd in readable(&fds) {
                let handler = registered.iter().find(|(_, f)| *f == fd);
                if let Some((handler, _)) = handler {
                    // SAFETY: as for timers.
                    unsafe { handler.onFDIsSet(fd) };
                }
            }
        }
        let wished = frame.resize.borrow_mut().take();
        if let Some(size) = wished {
            editor.window.resize(size.0, size.1);
            let mut rect = rect_of(size);
            // SAFETY: plain call; `rect` is valid for it.
            unsafe { editor.view.onSize(&mut rect) };
        }
        let mut closed = false;
        for event in editor.window.poll() {
            match event {
                FrameEvent::Closed => closed = true,
                FrameEvent::Resized { width, height } => {
                    let mut rect = rect_of((width, height));
                    // SAFETY: plain calls; `rect` is valid for them.
                    unsafe {
                        editor.view.checkSizeConstraint(&mut rect);
                        let fitted = size_of(&rect);
                        if fitted != (width, height) {
                            editor.window.resize(fitted.0, fitted.1);
                        }
                        editor.view.onSize(&mut rect);
                    }
                }
            }
        }
        if closed {
            self.close_editor();
        }
    }
}

/// The descriptors that are readable now. Does not wait.
#[cfg(target_os = "linux")]
fn readable(fds: &[FileDescriptor]) -> Vec<FileDescriptor> {
    let mut polled: Vec<libc::pollfd> = fds
        .iter()
        .map(|&fd| libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        })
        .collect();
    if polled.is_empty() {
        return Vec::new();
    }
    // SAFETY: `polled` is a valid array of `polled.len()` pollfd structs for the call; a
    // timeout of 0 returns at once.
    let count = unsafe { libc::poll(polled.as_mut_ptr(), polled.len() as libc::nfds_t, 0) };
    if count <= 0 {
        return Vec::new();
    }
    polled
        .iter()
        .filter(|p| p.revents != 0)
        .map(|p| p.fd)
        .collect()
}
