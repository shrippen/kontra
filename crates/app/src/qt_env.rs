//! Environment variables Qt reads before the app starts (platform theme, controls style).
//!
//! `std::env::set_var` is not enough on Windows: it changes the process environment, but Qt
//! asks the C runtime, which keeps its own copy from the start of the program. The packed
//! Kontra came up in Windows' native controls style although main.rs had asked for Fusion.
//! `qputenv` (cpp/qt_env.cpp) reaches the copy Qt reads, on every system.
//!
//! Part of the cxx-qt bridge: the generated glue uses `unsafe`, so the workspace lint is relaxed
//! for this file as for bridge.rs (agent.md). The C++ side only calls Qt.
#![allow(unsafe_code)]

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("kontra-app/qt_env.h");
        /// Sets `name` to `value` for Qt.
        fn set_qt_env(name: &str, value: &str);
    }
}
