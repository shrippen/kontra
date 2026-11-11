//! The editor window under Linux: a top-level X11 window (also under XWayland), through x11rb's
//! own connection (pure Rust, no libxcb). The plugin's editor becomes its child.

use super::editor::{EditorFrame, FrameEvent, FrameHandle};
use super::PluginError;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    AtomEnum, ClientMessageEvent, ConfigureWindowAux, ConnectionExt as _, CreateWindowAux,
    EventMask, PropMode, Window, WindowClass,
};
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;
use x11rb::COPY_DEPTH_FROM_PARENT;

/// A top-level window for one editor; the connection is its own and ends with it.
pub(super) struct X11Frame {
    conn: RustConnection,
    window: Window,
    delete_atom: u32,
    protocols_atom: u32,
    /// The inner size last reported, so a move (same size) is not a resize.
    size: (u32, u32),
}

fn x11_error(e: impl ToString) -> PluginError {
    PluginError::Editor(format!("X11: {}", e.to_string()))
}

impl X11Frame {
    /// Opens a connection to the display and makes the window (not shown yet).
    pub(super) fn open(title: &str, width: u32, height: u32) -> Result<X11Frame, PluginError> {
        let (conn, screen_num) = RustConnection::connect(None).map_err(x11_error)?;
        let screen = &conn.setup().roots[screen_num];
        let window = conn.generate_id().map_err(x11_error)?;
        let aux = CreateWindowAux::new()
            .background_pixel(screen.black_pixel)
            .event_mask(EventMask::STRUCTURE_NOTIFY);
        conn.create_window(
            COPY_DEPTH_FROM_PARENT,
            window,
            screen.root,
            0,
            0,
            clamp_side(width),
            clamp_side(height),
            0,
            WindowClass::INPUT_OUTPUT,
            screen.root_visual,
            &aux,
        )
        .map_err(x11_error)?;

        // Closing through the window manager arrives as a message, not as a dead connection.
        let atom = |name: &[u8]| -> Result<u32, PluginError> {
            let reply = conn.intern_atom(false, name).map_err(x11_error)?;
            Ok(reply.reply().map_err(x11_error)?.atom)
        };
        let protocols_atom = atom(b"WM_PROTOCOLS")?;
        let delete_atom = atom(b"WM_DELETE_WINDOW")?;
        let utf8 = atom(b"UTF8_STRING")?;
        let net_name = atom(b"_NET_WM_NAME")?;
        conn.change_property32(
            PropMode::REPLACE,
            window,
            protocols_atom,
            AtomEnum::ATOM,
            &[delete_atom],
        )
        .map_err(x11_error)?;
        conn.change_property8(
            PropMode::REPLACE,
            window,
            AtomEnum::WM_NAME,
            AtomEnum::STRING,
            title.as_bytes(),
        )
        .map_err(x11_error)?;
        conn.change_property8(PropMode::REPLACE, window, net_name, utf8, title.as_bytes())
            .map_err(x11_error)?;
        conn.flush().map_err(x11_error)?;

        Ok(X11Frame {
            conn,
            window,
            delete_atom,
            protocols_atom,
            size: (width, height),
        })
    }

    fn is_close(&self, message: &ClientMessageEvent) -> bool {
        message.type_ == self.protocols_atom && message.data.as_data32()[0] == self.delete_atom
    }
}

/// X11 sizes are 16 bit and never 0.
fn clamp_side(side: u32) -> u16 {
    side.clamp(1, u32::from(u16::MAX)) as u16
}

impl EditorFrame for X11Frame {
    fn handle(&self) -> FrameHandle {
        FrameHandle::X11(self.window)
    }

    fn resize(&mut self, width: u32, height: u32) {
        let aux = ConfigureWindowAux::new()
            .width(u32::from(clamp_side(width)))
            .height(u32::from(clamp_side(height)));
        let _ = self.conn.configure_window(self.window, &aux);
        let _ = self.conn.flush();
        self.size = (width, height);
    }

    fn show(&mut self) {
        let _ = self.conn.map_window(self.window);
        let _ = self.conn.flush();
    }

    fn poll(&mut self) -> Vec<FrameEvent> {
        let mut events = Vec::new();
        // A broken connection (the X server went away) closes the editor.
        loop {
            let event = match self.conn.poll_for_event() {
                Ok(Some(event)) => event,
                Ok(None) => break,
                Err(_) => {
                    events.push(FrameEvent::Closed);
                    break;
                }
            };
            match event {
                Event::ClientMessage(m) if self.is_close(&m) => events.push(FrameEvent::Closed),
                Event::DestroyNotify(_) => events.push(FrameEvent::Closed),
                Event::ConfigureNotify(c) => {
                    let size = (u32::from(c.width), u32::from(c.height));
                    if size != self.size {
                        self.size = size;
                        events.push(FrameEvent::Resized {
                            width: size.0,
                            height: size.1,
                        });
                    }
                }
                _ => {}
            }
        }
        events
    }
}

impl Drop for X11Frame {
    fn drop(&mut self) {
        let _ = self.conn.destroy_window(self.window);
        let _ = self.conn.flush();
    }
}
