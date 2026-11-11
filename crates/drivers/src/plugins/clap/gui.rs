//! A CLAP plugin's own editor and what keeps it alive on the plugin thread (FX5): the gui
//! extension (embedded in an `EditorFrame`), timers, file descriptors (Linux), main thread
//! callbacks and parameter flushes the plugin asks for.
//!
//! ```text
//!  plugin ── register_timer / register_fd / request_callback / request_resize ──► Host*
//!  plugin thread, every tick:  ClapPlugin::idle
//!      callback asked  ─► on_main_thread
//!      timer due       ─► on_timer          fd ready ─► on_fd
//!      frame closed    ─► gui.destroy       size asked ─► frame.resize
//! ```

use super::{ClapPlugin, Host};
use crate::plugins::editor::{EditorFrame, FrameEvent, FrameHandle, FrameMaker};
use crate::plugins::PluginError;
use clack_extensions::gui::{
    GuiApiType, GuiConfiguration, GuiSize, HostGuiImpl, PluginGui, Window as GuiWindow,
};
use clack_extensions::params::{
    HostParamsImplMainThread, HostParamsImplShared, ParamClearFlags, ParamRescanFlags,
};
use clack_extensions::timer::{HostTimerImpl, TimerId};
use clack_host::prelude::*;
use clack_host::utils::ClapId;
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Timers faster than this are slowed down; CLAP allows it, and the plugin thread ticks at
/// about this rate anyway.
const MIN_TIMER_PERIOD: Duration = Duration::from_millis(10);

/// The size an editor gets when the plugin does not say.
const FALLBACK_SIZE: GuiSize = GuiSize {
    width: 640,
    height: 480,
};

/// X11 sizes are in physical pixels: the plugin draws 1:1 (`set_scale` is for the APIs with
/// logical sizes).
const X11_SCALE: f64 = 1.0;

/// What the plugin asks for from any thread; the plugin thread acts on it in `idle`.
#[derive(Default)]
pub(in crate::plugins) struct HostShared {
    callback: AtomicBool,
    resize: Mutex<Option<GuiSize>>,
    closed: AtomicBool,
}

impl SharedHandler<'_> for HostShared {
    // A restart or process call is of no use for an effect that always runs.
    fn request_restart(&self) {}
    fn request_process(&self) {}
    fn request_callback(&self) {
        self.callback.store(true, Ordering::Relaxed);
    }
}

impl HostGuiImpl for HostShared {
    fn resize_hints_changed(&self) {}

    fn request_resize(&self, new_size: GuiSize) -> Result<(), HostError> {
        let mut resize = self
            .resize
            .lock()
            .map_err(|_| HostError::Message("poisoned"))?;
        *resize = Some(new_size);
        Ok(())
    }

    // The frame is shown with the editor and hidden by closing it.
    fn request_show(&self) -> Result<(), HostError> {
        Ok(())
    }

    fn request_hide(&self) -> Result<(), HostError> {
        Ok(())
    }

    fn closed(&self, _was_destroyed: bool) {
        self.closed.store(true, Ordering::Relaxed);
    }
}

impl HostParamsImplShared for HostShared {
    // While active the plugin reports its changes in the next block; Kontra's plugins always
    // are active while they run.
    fn request_flush(&self) {}
}

/// A timer the plugin registered.
struct Timer {
    id: TimerId,
    period: Duration,
    next: Instant,
}

/// What the plugin registered on the main thread (the plugin thread).
#[derive(Default)]
pub(in crate::plugins) struct HostMain {
    timers: RefCell<Vec<Timer>>,
    next_timer: RefCell<u32>,
    #[cfg(target_os = "linux")]
    fds: RefCell<Vec<(std::os::fd::RawFd, clack_extensions::posix_fd::FdFlags)>>,
}

impl MainThreadHandler<'_> for HostMain {}

impl HostTimerImpl for HostMain {
    fn register_timer(&self, period_ms: u32) -> Result<TimerId, HostError> {
        let mut next = self.next_timer.borrow_mut();
        let id = TimerId(*next);
        *next = next.wrapping_add(1);
        let period = Duration::from_millis(u64::from(period_ms)).max(MIN_TIMER_PERIOD);
        self.timers.borrow_mut().push(Timer {
            id,
            period,
            next: Instant::now() + period,
        });
        Ok(id)
    }

    fn unregister_timer(&self, timer_id: TimerId) -> Result<(), HostError> {
        self.timers.borrow_mut().retain(|t| t.id != timer_id);
        Ok(())
    }
}

impl HostParamsImplMainThread for HostMain {
    // Parameters are read at load; a plugin that changes its list needs a reload.
    fn rescan(&self, _flags: ParamRescanFlags) {}
    fn clear(&self, _param_id: ClapId, _flags: ParamClearFlags) {}
}

#[cfg(target_os = "linux")]
mod fds {
    use super::HostMain;
    use clack_extensions::posix_fd::{FdFlags, HostPosixFdImpl};
    use clack_host::prelude::HostError;
    use std::os::fd::RawFd;

    impl HostPosixFdImpl for HostMain {
        fn register_fd(&self, fd: RawFd, flags: FdFlags) -> Result<(), HostError> {
            let mut fds = self.fds.borrow_mut();
            fds.retain(|(f, _)| *f != fd);
            fds.push((fd, flags));
            Ok(())
        }

        fn modify_fd(&self, fd: RawFd, flags: FdFlags) -> Result<(), HostError> {
            self.register_fd(fd, flags)
        }

        fn unregister_fd(&self, fd: RawFd) -> Result<(), HostError> {
            self.fds.borrow_mut().retain(|(f, _)| *f != fd);
            Ok(())
        }
    }

    /// The registered descriptors that are ready now, with what they are ready for. Does not
    /// wait.
    pub(super) fn ready(registered: &[(RawFd, FdFlags)]) -> Vec<(RawFd, FdFlags)> {
        let mut polled: Vec<libc::pollfd> = registered
            .iter()
            .map(|&(fd, flags)| {
                let mut events = 0;
                if flags.contains(FdFlags::READ) {
                    events |= libc::POLLIN;
                }
                if flags.contains(FdFlags::WRITE) {
                    events |= libc::POLLOUT;
                }
                libc::pollfd {
                    fd,
                    events,
                    revents: 0,
                }
            })
            .collect();
        if polled.is_empty() {
            return Vec::new();
        }
        // SAFETY: `polled` is a valid array of `polled.len()` pollfd structs for the call's
        // duration; a timeout of 0 returns at once.
        #[allow(unsafe_code)]
        let count = unsafe { libc::poll(polled.as_mut_ptr(), polled.len() as libc::nfds_t, 0) };
        if count <= 0 {
            return Vec::new();
        }
        polled
            .iter()
            .filter(|p| p.revents != 0)
            .map(|p| {
                let mut flags = FdFlags::empty();
                if p.revents & libc::POLLIN != 0 {
                    flags |= FdFlags::READ;
                }
                if p.revents & libc::POLLOUT != 0 {
                    flags |= FdFlags::WRITE;
                }
                if p.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 {
                    flags |= FdFlags::ERROR;
                }
                (p.fd, flags)
            })
            .collect()
    }
}

fn editor_error(e: impl ToString) -> PluginError {
    PluginError::Editor(e.to_string())
}

/// The window API this platform embeds editors with.
fn embedded() -> GuiConfiguration<'static> {
    GuiConfiguration {
        api_type: GuiApiType::X11,
        is_floating: false,
    }
}

impl ClapPlugin {
    /// Opens the plugin's editor in a frame from `make_frame`. An open editor stays as it is.
    pub(in crate::plugins) fn open_editor(
        &mut self,
        title: &str,
        make_frame: FrameMaker,
    ) -> Result<(), PluginError> {
        if self.editor.is_some() {
            return Ok(());
        }
        let gui = self.gui_ext.ok_or(PluginError::NoEditor)?;
        let handle = self.instance.plugin_handle();
        if !gui.is_api_supported(&handle, embedded()) {
            return Err(PluginError::NoEditor);
        }
        gui.create(&handle, embedded()).map_err(editor_error)?;
        let _ = gui.set_scale(&handle, X11_SCALE);
        let size = gui.get_size(&handle).unwrap_or(FALLBACK_SIZE);

        let frame = match make_frame(title, size.width, size.height) {
            Ok(frame) => frame,
            Err(e) => {
                gui.destroy(&handle);
                return Err(e);
            }
        };
        // A plugin that fails after taking the window still has it: destroy, then the frame.
        let frame = match embed(&gui, &handle, frame) {
            Ok(frame) => frame,
            Err((e, frame)) => {
                gui.destroy(&handle);
                drop(frame);
                return Err(e);
            }
        };
        self.instance
            .access_shared_handler(|s| s.closed.store(false, Ordering::Relaxed));
        self.editor = Some(frame);
        Ok(())
    }

    /// Closes the editor if it is open: the plugin lets go of the frame before it goes.
    pub(in crate::plugins) fn close_editor(&mut self) {
        let Some(frame) = self.editor.take() else {
            return;
        };
        if let Some(gui) = self.gui_ext {
            gui.destroy(&self.instance.plugin_handle());
        }
        drop(frame);
    }

    pub(in crate::plugins) fn editor_open(&self) -> bool {
        self.editor.is_some()
    }

    /// One tick of the plugin thread for this plugin: what it asked for, its timers and
    /// descriptors, and its editor's window.
    pub(in crate::plugins) fn idle(&mut self) {
        let asked = self
            .instance
            .access_shared_handler(|s| s.callback.swap(false, Ordering::Relaxed));
        if asked {
            self.instance.call_on_main_thread_callback();
        }
        self.run_timers();
        #[cfg(target_os = "linux")]
        self.run_fds();
        self.tend_editor();
    }

    fn run_timers(&mut self) {
        let Some(timer) = self.timer_ext else {
            return;
        };
        let now = Instant::now();
        let due: Vec<TimerId> = self.instance.access_handler(|h| {
            let mut timers = h.timers.borrow_mut();
            let mut due = Vec::new();
            for t in timers.iter_mut().filter(|t| t.next <= now) {
                due.push(t.id);
                // A late tick is not caught up: the next one is a period from now.
                t.next = now + t.period;
            }
            due
        });
        for id in due {
            timer.on_timer(&self.instance.plugin_handle(), id);
        }
    }

    #[cfg(target_os = "linux")]
    fn run_fds(&mut self) {
        let Some(posix) = self.fd_ext else {
            return;
        };
        let registered = self.instance.access_handler(|h| h.fds.borrow().clone());
        for (fd, flags) in fds::ready(&registered) {
            posix.on_fd(&self.instance.plugin_handle(), fd, flags);
        }
    }

    /// Window events to the plugin, the plugin's size wishes to the window.
    fn tend_editor(&mut self) {
        let Some(gui) = self.gui_ext else {
            return;
        };
        let wished = self
            .instance
            .access_shared_handler(|s| s.resize.lock().ok().and_then(|mut r| r.take()));
        let gone = self
            .instance
            .access_shared_handler(|s| s.closed.swap(false, Ordering::Relaxed));
        let Some(frame) = self.editor.as_mut() else {
            return;
        };
        if let Some(size) = wished {
            frame.resize(size.width, size.height);
        }
        let mut closed = gone;
        for event in frame.poll() {
            match event {
                FrameEvent::Closed => closed = true,
                FrameEvent::Resized { width, height } => {
                    let handle = self.instance.plugin_handle();
                    let size = GuiSize { width, height };
                    let fitted = gui.adjust_size(&handle, size).unwrap_or(size);
                    if gui.can_resize(&handle) && gui.set_size(&handle, fitted).is_ok() {
                        continue;
                    }
                    // A fixed size: the window goes back to the editor's.
                    if let Some(own) = gui.get_size(&handle) {
                        frame.resize(own.width, own.height);
                    }
                }
            }
        }
        if closed {
            self.close_editor();
        }
    }
}

/// Puts the editor into `frame` and shows both. The frame must outlive the editor: on failure
/// it comes back with the error, for the caller to drop after `gui.destroy`.
// `c_ulong` is u64 on Linux and u32 on Windows: the conversion is real only on Linux.
#[allow(clippy::useless_conversion)]
fn embed(
    gui: &PluginGui,
    handle: &PluginMainThreadHandle,
    mut frame: Box<dyn EditorFrame>,
) -> Result<Box<dyn EditorFrame>, (PluginError, Box<dyn EditorFrame>)> {
    let FrameHandle::X11(window) = frame.handle();
    let parent = GuiWindow::from_x11_handle(window.into());
    // SAFETY: the window exists (the frame owns it) and stays until `gui.destroy`: in
    // `close_editor`, in `Drop for ClapPlugin`, or on failure here through the caller.
    #[allow(unsafe_code)]
    let set = unsafe { gui.set_parent(handle, parent) };
    if let Err(e) = set {
        return Err((editor_error(e), frame));
    }
    let _ = gui.show(handle);
    frame.show();
    Ok(frame)
}

impl Drop for ClapPlugin {
    // The editor goes before the instance, whoever drops the plugin.
    fn drop(&mut self) {
        self.close_editor();
    }
}

impl Host {
    /// The extensions this host offers plugins.
    pub(super) fn declare(builder: &mut HostExtensions<Self>) {
        use clack_extensions::gui::HostGui;
        use clack_extensions::params::HostParams;
        use clack_extensions::timer::HostTimer;
        builder
            .register::<HostGui>()
            .register::<HostTimer>()
            .register::<HostParams>();
        #[cfg(target_os = "linux")]
        builder.register::<clack_extensions::posix_fd::HostPosixFd>();
    }
}
