//! The gain plugin's editor for the host tests of FX5. It draws nothing; it does what a real
//! editor does to its host, observably:
//!
//! ```text
//!  create      ─► registers a timer and (Linux) a pipe with one byte in it
//!  set_parent  ─► remembers the window
//!  on_fd       ─► reads the byte; with a window: asks for EDITOR_RESIZED
//!  on_timer    ─► "turns the knob": the next block reports EDITOR_GAIN as its own change
//!  destroy     ─► unregisters both
//! ```

use crate::GainMainThread;
#[cfg(target_os = "linux")]
use clack_extensions::gui::HostGui;
use clack_extensions::gui::{GuiApiType, GuiConfiguration, GuiSize, PluginGuiImpl, Window};
use clack_extensions::timer::{HostTimer, PluginTimerImpl, TimerId};
use clack_plugin::prelude::*;

/// The size the editor starts with.
pub const EDITOR_SIZE: (u32, u32) = (300, 200);
/// The size the editor asks for once it sits in a window.
pub const EDITOR_RESIZED: (u32, u32) = (320, 240);
/// The gain the editor sets on its first timer tick.
pub const EDITOR_GAIN: f64 = 2.5;
const TIMER_MS: u32 = 10;

/// What the editor holds while it exists.
#[derive(Debug, Default)]
pub struct EditorState {
    timer: Option<TimerId>,
    parent: Option<u64>,
    #[cfg(target_os = "linux")]
    pipe: Option<[std::os::fd::RawFd; 2]>,
}

fn x11_embedded(configuration: GuiConfiguration) -> bool {
    configuration.api_type == GuiApiType::X11 && !configuration.is_floating
}

impl PluginGuiImpl for GainMainThread<'_> {
    fn is_api_supported(&self, configuration: GuiConfiguration) -> bool {
        x11_embedded(configuration)
    }

    fn get_preferred_api(&self) -> Option<GuiConfiguration<'_>> {
        Some(GuiConfiguration {
            api_type: GuiApiType::X11,
            is_floating: false,
        })
    }

    fn create(&self, configuration: GuiConfiguration) -> Result<(), PluginError> {
        if !x11_embedded(configuration) {
            return Err(PluginError::Message("only embedded X11"));
        }
        let mut editor = self.editor.borrow_mut();
        if let Some(timer) = self.host.get_extension::<HostTimer>() {
            editor.timer = timer.register_timer(&self.host, TIMER_MS).ok();
        }
        #[cfg(target_os = "linux")]
        {
            editor.pipe = fds::open(&self.host);
        }
        Ok(())
    }

    fn destroy(&self) {
        let mut editor = self.editor.borrow_mut();
        if let (Some(timer), Some(id)) = (self.host.get_extension::<HostTimer>(), editor.timer) {
            let _ = timer.unregister_timer(&self.host, id);
        }
        #[cfg(target_os = "linux")]
        if let Some(pipe) = editor.pipe.take() {
            fds::close(&self.host, pipe);
        }
        *editor = EditorState::default();
    }

    fn set_scale(&self, _scale: f64) -> Result<(), PluginError> {
        Ok(())
    }

    fn get_size(&self) -> Option<GuiSize> {
        Some(GuiSize {
            width: EDITOR_SIZE.0,
            height: EDITOR_SIZE.1,
        })
    }

    fn can_resize(&self) -> bool {
        true
    }

    fn set_size(&self, _size: GuiSize) -> Result<(), PluginError> {
        Ok(())
    }

    // `c_ulong` is u64 on Linux and u32 on Windows.
    #[allow(clippy::useless_conversion)]
    fn set_parent(&self, window: Window) -> Result<(), PluginError> {
        self.editor.borrow_mut().parent = window.as_x11_handle().map(u64::from);
        Ok(())
    }

    fn set_transient(&self, _window: Window) -> Result<(), PluginError> {
        Ok(())
    }

    fn show(&self) -> Result<(), PluginError> {
        Ok(())
    }

    fn hide(&self) -> Result<(), PluginError> {
        Ok(())
    }
}

impl PluginTimerImpl for GainMainThread<'_> {
    fn on_timer(&self, _timer_id: TimerId) {
        self.shared.turn_knob(EDITOR_GAIN);
    }
}

impl GainMainThread<'_> {
    /// The editor sits in a window and has news: it asks for a new size.
    // Only the descriptor (Linux) wakes it.
    #[cfg(target_os = "linux")]
    fn grow(&self) {
        if self.editor.borrow().parent.is_none() {
            return;
        }
        if let Some(gui) = self.host.get_extension::<HostGui>() {
            let _ = gui.request_resize(&self.host, EDITOR_RESIZED.0, EDITOR_RESIZED.1);
        }
    }
}

#[cfg(target_os = "linux")]
mod fds {
    use crate::GainMainThread;
    use clack_extensions::posix_fd::{FdFlags, HostPosixFd, PluginPosixFdImpl};
    use clack_plugin::prelude::*;
    use std::os::fd::RawFd;

    /// A pipe with one byte waiting, registered for reading.
    pub(super) fn open(host: &HostMainThreadHandle) -> Option<[RawFd; 2]> {
        let mut pipe = [0; 2];
        // SAFETY: `pipe` has room for the two descriptors; `write` reads one byte of a live
        // buffer.
        #[allow(unsafe_code)]
        unsafe {
            if libc::pipe(pipe.as_mut_ptr()) != 0 {
                return None;
            }
            libc::write(pipe[1], b"x".as_ptr().cast(), 1);
        }
        let ext = host.get_extension::<HostPosixFd>()?;
        ext.register_fd(host, pipe[0], FdFlags::READ).ok()?;
        Some(pipe)
    }

    pub(super) fn close(host: &HostMainThreadHandle, pipe: [RawFd; 2]) {
        if let Some(ext) = host.get_extension::<HostPosixFd>() {
            let _ = ext.unregister_fd(host, pipe[0]);
        }
        // SAFETY: both descriptors came from `pipe` and are closed once.
        #[allow(unsafe_code)]
        unsafe {
            libc::close(pipe[0]);
            libc::close(pipe[1]);
        }
    }

    impl PluginPosixFdImpl for GainMainThread<'_> {
        fn on_fd(&self, fd: RawFd, _flags: FdFlags) {
            let mut byte = [0_u8; 1];
            // SAFETY: reads at most one byte into a live buffer; the pipe is non-empty when
            // the host calls this, so it does not block.
            #[allow(unsafe_code)]
            let read = unsafe { libc::read(fd, byte.as_mut_ptr().cast(), 1) };
            if read == 1 {
                self.grow();
            }
        }
    }
}
