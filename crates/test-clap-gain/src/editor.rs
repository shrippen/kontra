//! The gain plugin's editor for the host tests of FX5. It draws nothing; it does what a real
//! editor does to its host, observably:
//!
//! ```text
//!  create      ─► registers a timer and (Linux) a pipe with one byte in it
//!  set_parent  ─► remembers the window
//!  on_fd       ─► reads the byte; with a window: asks for EDITOR_RESIZED
//!  on_timer    ─► "turns the knob": the next block reports EDITOR_GAIN as its own change;
//!                 at gain EDITOR_CLOSE_GAIN it closes itself (`closed`)
//!  set_size    ─► refuses anything wider than EDITOR_MAX_WIDTH
//!  destroy     ─► unregisters both
//! ```

use crate::GainMainThread;
use clack_extensions::gui::{
    GuiApiType, GuiConfiguration, GuiSize, HostGui, PluginGuiImpl, Window,
};
use clack_extensions::timer::{HostTimer, PluginTimerImpl, TimerId};
use clack_plugin::prelude::*;
use std::cell::Cell;

/// The size the editor starts with.
pub const EDITOR_SIZE: (u32, u32) = (300, 200);
/// The size the editor asks for once it sits in a window.
pub const EDITOR_RESIZED: (u32, u32) = (320, 240);
/// The gain the editor sets on its first timer tick.
pub const EDITOR_GAIN: f64 = 2.5;
/// At this gain the editor closes itself, as a plugin's own close button would.
pub const EDITOR_CLOSE_GAIN: f64 = 3.25;
/// Wider sizes are refused: the host gives the window the editor's size back.
pub const EDITOR_MAX_WIDTH: u32 = 400;
const TIMER_MS: u32 = 10;

/// What the editor holds while it exists.
#[derive(Debug, Default)]
pub struct EditorState {
    timer: Option<TimerId>,
    parent: Option<u64>,
    /// The size the host last set (or the first one).
    size: Cell<Option<(u32, u32)>>,
    #[cfg(target_os = "linux")]
    pipe: Option<fds::Pipe>,
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
            fds::close(&self.host, &pipe);
        }
        *editor = EditorState::default();
    }

    fn set_scale(&self, _scale: f64) -> Result<(), PluginError> {
        Ok(())
    }

    fn get_size(&self) -> Option<GuiSize> {
        let (width, height) = self.editor.borrow().size.get().unwrap_or(EDITOR_SIZE);
        Some(GuiSize { width, height })
    }

    fn can_resize(&self) -> bool {
        true
    }

    fn set_size(&self, size: GuiSize) -> Result<(), PluginError> {
        if size.width > EDITOR_MAX_WIDTH {
            return Err(PluginError::Message("too wide"));
        }
        self.editor
            .borrow()
            .size
            .set(Some((size.width, size.height)));
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
        if f64::from(self.shared.gain()) == EDITOR_CLOSE_GAIN {
            if let Some(gui) = self.host.get_extension::<HostGui>() {
                gui.closed(&self.host, true);
            }
            return;
        }
        self.shared.turn_knob(EDITOR_GAIN);
    }
}

impl GainMainThread<'_> {
    /// The editor sits in a window and has news: it asks for a new size.
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
    use std::io::{PipeReader, PipeWriter, Read, Write};
    use std::os::fd::{AsRawFd, RawFd};

    /// Both ends: the reader is registered, the writer keeps the pipe open.
    #[derive(Debug)]
    pub(super) struct Pipe {
        reader: PipeReader,
        _writer: PipeWriter,
    }

    /// A pipe with one byte waiting, registered for reading.
    pub(super) fn open(host: &HostMainThreadHandle) -> Option<Pipe> {
        let (reader, mut writer) = std::io::pipe().ok()?;
        writer.write_all(b"x").ok()?;
        let ext = host.get_extension::<HostPosixFd>()?;
        ext.register_fd(host, reader.as_raw_fd(), FdFlags::READ)
            .ok()?;
        Some(Pipe {
            reader,
            _writer: writer,
        })
    }

    pub(super) fn close(host: &HostMainThreadHandle, pipe: &Pipe) {
        if let Some(ext) = host.get_extension::<HostPosixFd>() {
            let _ = ext.unregister_fd(host, pipe.reader.as_raw_fd());
        }
    }

    impl PluginPosixFdImpl for GainMainThread<'_> {
        fn on_fd(&self, _fd: RawFd, _flags: FdFlags) {
            let mut byte = [0_u8; 1];
            // The host calls this only when the pipe holds a byte, so it does not block.
            let read = {
                let mut editor = self.editor.borrow_mut();
                let Some(pipe) = editor.pipe.as_mut() else {
                    return;
                };
                pipe.reader.read(&mut byte)
            };
            if read.is_ok_and(|n| n == 1) {
                self.grow();
            }
        }
    }
}
