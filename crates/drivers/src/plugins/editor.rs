//! The window a plugin's own editor lives in (FX5). The plugin draws into a child of a window the
//! host owns; the host moves, sizes and closes that window. `EditorFrame` is that window as the
//! plugin thread sees it, so the plugin code does not care whether X11, Win32 or a test made it.
//!
//! ```text
//!  PluginHandle::open_editor(title, make_frame)
//!      │ OpenEditor
//!      ▼ kontra-plugins
//!  make_frame(title, size) ─► EditorFrame ─ handle ─► plugin: create, set_parent, show
//!  every tick:  frame.poll() ─► Closed  ─► plugin: destroy, frame dropped
//!               plugin asks for a size  ─► frame.resize
//! ```

use super::PluginError;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// The native window a plugin is told to draw into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameHandle {
    /// An X11 window id (Linux, also under XWayland).
    X11(u32),
}

/// What happened to a frame since the last look.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameEvent {
    /// The player closed the window.
    Closed,
    /// The player resized the window to this inner size, in pixels.
    Resized { width: u32, height: u32 },
}

/// A host window for one plugin editor. Lives on the plugin thread.
pub trait EditorFrame {
    fn handle(&self) -> FrameHandle;
    /// Sets the inner size, in pixels.
    fn resize(&mut self, width: u32, height: u32);
    /// Makes the window visible (after the plugin is in it).
    fn show(&mut self);
    /// Events since the last call, oldest first. Never blocks.
    fn poll(&mut self) -> Vec<FrameEvent>;
}

/// Makes the frame for an editor: title and first inner size in pixels. Runs on the plugin
/// thread, so a window system connection stays on the thread that uses it.
pub type FrameMaker =
    Box<dyn FnOnce(&str, u32, u32) -> Result<Box<dyn EditorFrame>, PluginError> + Send>;

/// Whether a plugin's editor is open, readable from any thread.
#[derive(Debug, Clone, Default)]
pub struct EditorShown(Arc<AtomicBool>);

impl EditorShown {
    pub fn get(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }

    pub(super) fn set(&self, open: bool) {
        self.0.store(open, Ordering::Relaxed);
    }
}

/// Frames of the window system Kontra runs on: X11 under Linux (XWayland under Wayland). Other
/// systems have none yet: their editors do not open.
pub fn system_frame() -> FrameMaker {
    Box::new(|title, width, height| {
        #[cfg(target_os = "linux")]
        {
            let frame = super::x11_frame::X11Frame::open(title, width, height)?;
            Ok(Box::new(frame) as Box<dyn EditorFrame>)
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (title, width, height);
            Err(PluginError::NoEditor)
        }
    })
}
