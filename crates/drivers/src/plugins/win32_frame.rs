//! The editor window under Windows: a top-level Win32 window on the plugin thread. The plugin's
//! editor becomes its child; the plugin thread pumps the thread's messages in `poll`, so the
//! editor's own child windows get theirs too.
//!
//! ```text
//!  open ─► RegisterClassW (once) ─► CreateWindowExW ─► state pointer in GWLP_USERDATA
//!  poll ─► PeekMessageW (all of this thread) ─► window_proc: WM_CLOSE ─► Closed, WM_SIZE ─► Resized
//!  drop ─► DestroyWindow ─► state freed
//! ```

use super::editor::{EditorFrame, FrameEvent, FrameHandle};
use super::PluginError;
use std::cell::RefCell;
use std::ptr;
use std::sync::Once;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRectEx, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
    GetWindowLongPtrW, PeekMessageW, RegisterClassW, SetWindowLongPtrW, SetWindowPos, ShowWindow,
    TranslateMessage, CW_USEDEFAULT, GWLP_USERDATA, MSG, PM_REMOVE, SIZE_MINIMIZED, SWP_NOMOVE,
    SWP_NOZORDER, SW_SHOW, WM_CLOSE, WM_SIZE, WNDCLASSW, WS_EX_APPWINDOW, WS_OVERLAPPEDWINDOW,
};

/// The window class of all editor frames.
const CLASS: &str = "KontraPluginEditor";

/// What the window procedure reports to `poll`; it lives on the heap while the window exists.
#[derive(Default)]
struct State {
    events: RefCell<Vec<FrameEvent>>,
}

/// A top-level window for one editor.
pub(super) struct Win32Frame {
    window: HWND,
    state: *mut State,
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn win32_error(what: &str) -> PluginError {
    PluginError::Editor(format!("Win32: {what} failed"))
}

/// The low and high word of a message parameter (WM_SIZE: width, height).
fn words(value: LPARAM) -> (u32, u32) {
    let bits = value as usize;
    ((bits & 0xFFFF) as u32, ((bits >> 16) & 0xFFFF) as u32)
}

/// The window's outer size for an inner size, with the frame and title bar of its style.
fn outer_size(width: u32, height: u32) -> (i32, i32) {
    let side = |v: u32| i32::try_from(v).unwrap_or(i32::MAX);
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: side(width),
        bottom: side(height),
    };
    // SAFETY: `rect` is a valid RECT for the call.
    unsafe { AdjustWindowRectEx(&mut rect, WS_OVERLAPPEDWINDOW, 0, WS_EX_APPWINDOW) };
    (rect.right - rect.left, rect.bottom - rect.top)
}

unsafe extern "system" fn window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: the pointer is the `State` this window was made with, or 0 before it is set and
    // after `Drop` cleared it.
    let state = unsafe { GetWindowLongPtrW(window, GWLP_USERDATA) } as *const State;
    // SAFETY: a non-null pointer points to the live `State` (freed only after DestroyWindow).
    let Some(state) = (unsafe { state.as_ref() }) else {
        // SAFETY: plain call with the arguments the system gave.
        return unsafe { DefWindowProcW(window, message, wparam, lparam) };
    };
    match message {
        // The window stays until the host destroys it; the plugin lets go of it first.
        WM_CLOSE => {
            state.events.borrow_mut().push(FrameEvent::Closed);
            0
        }
        // Minimized is 0×0: no size for the editor.
        WM_SIZE if wparam == SIZE_MINIMIZED as WPARAM => 0,
        WM_SIZE => {
            let (width, height) = words(lparam);
            let resized = FrameEvent::Resized { width, height };
            state.events.borrow_mut().push(resized);
            0
        }
        // SAFETY: plain call with the arguments the system gave.
        _ => unsafe { DefWindowProcW(window, message, wparam, lparam) },
    }
}

impl Win32Frame {
    /// Makes the window (not shown yet) on this thread.
    pub(super) fn open(title: &str, width: u32, height: u32) -> Result<Win32Frame, PluginError> {
        static REGISTER: Once = Once::new();
        let class = wide(CLASS);
        // SAFETY: a null name is this program's module.
        let instance = unsafe { GetModuleHandleW(ptr::null()) };
        REGISTER.call_once(|| {
            let class_info = WNDCLASSW {
                style: 0,
                lpfnWndProc: Some(window_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: instance,
                hIcon: ptr::null_mut(),
                hCursor: ptr::null_mut(),
                hbrBackground: ptr::null_mut(),
                lpszMenuName: ptr::null(),
                lpszClassName: class.as_ptr(),
            };
            // SAFETY: `class_info` and the class name live through the call; the system copies
            // them. A failure shows as a failed CreateWindowExW below.
            unsafe { RegisterClassW(&class_info) };
        });

        let (outer_width, outer_height) = outer_size(width, height);
        let title = wide(title);
        // SAFETY: class and title are NUL-terminated and live through the call.
        let window = unsafe {
            CreateWindowExW(
                WS_EX_APPWINDOW,
                class.as_ptr(),
                title.as_ptr(),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                outer_width,
                outer_height,
                ptr::null_mut(),
                ptr::null_mut(),
                instance,
                ptr::null(),
            )
        };
        if window.is_null() {
            return Err(win32_error("CreateWindowExW"));
        }
        let state = Box::into_raw(Box::new(State::default()));
        // SAFETY: the window is ours; the pointer stays valid until `Drop`.
        unsafe { SetWindowLongPtrW(window, GWLP_USERDATA, state as isize) };
        Ok(Win32Frame { window, state })
    }
}

impl EditorFrame for Win32Frame {
    fn handle(&self) -> FrameHandle {
        FrameHandle::Win32(self.window as isize)
    }

    fn resize(&mut self, width: u32, height: u32) {
        let (outer_width, outer_height) = outer_size(width, height);
        let flags = SWP_NOMOVE | SWP_NOZORDER;
        // SAFETY: the window is alive (destroyed only in `Drop`).
        unsafe {
            SetWindowPos(
                self.window,
                ptr::null_mut(),
                0,
                0,
                outer_width,
                outer_height,
                flags,
            )
        };
    }

    fn show(&mut self) {
        // SAFETY: as in `resize`.
        unsafe { ShowWindow(self.window, SW_SHOW) };
    }

    fn poll(&mut self) -> Vec<FrameEvent> {
        // All messages of this thread: the editor's child windows live here too.
        // SAFETY: `message` is a valid MSG for each call.
        unsafe {
            let mut message: MSG = std::mem::zeroed();
            while PeekMessageW(&mut message, ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        // SAFETY: the state is alive until `Drop`.
        let state = unsafe { &*self.state };
        std::mem::take(&mut *state.events.borrow_mut())
    }
}

impl Drop for Win32Frame {
    fn drop(&mut self) {
        // SAFETY: the window is ours; after it is gone nothing reads the state, which this
        // frame made with `Box::into_raw` and frees once.
        unsafe {
            SetWindowLongPtrW(self.window, GWLP_USERDATA, 0);
            DestroyWindow(self.window);
            drop(Box::from_raw(self.state));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_MINIMIZE;

    /// Review #280: minimizing reports 0×0; the editor must not be sized to that.
    #[test]
    #[ignore = "needs a display"]
    fn minimizing_is_no_resize() {
        let mut frame = Win32Frame::open("Minimize", 300, 200).expect("window");
        frame.show();
        let _ = frame.poll();

        // SAFETY: the frame's window is alive until it drops at the end.
        unsafe { ShowWindow(frame.window, SW_MINIMIZE) };
        let events = frame.poll();
        let zero = FrameEvent::Resized {
            width: 0,
            height: 0,
        };
        assert!(!events.contains(&zero), "{events:?}");
    }
}
