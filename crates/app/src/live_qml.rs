//! Development only (LH-LRN-21, `KONTRA_LIVE_QML`): an edited QML file shows without a restart
//! (cpp/live_qml.cpp). Built into debug builds only (build.rs, main.rs).
//!
//! ```text
//! save LessonBlock.qml ──QFileSystemWatcher──▶ 200 ms quiet ──▶ unloadPage() ──▶ cache cleared
//!                                                                 └──▶ reloadPage(): page from disk
//! ```
//!
//! Part of the cxx-qt bridge: the generated glue uses `unsafe`, so the workspace lint is relaxed
//! for this file as for bridge.rs (agent.md). The C++ side only watches files and calls Qt.
#![allow(unsafe_code)]

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qqmlapplicationengine.h");
        type QQmlApplicationEngine = cxx_qt_lib::QQmlApplicationEngine;

        include!("kontra-app/live_qml.h");
        /// Reloads the current page when a QML file in `dir` changes.
        fn watch_live_qml(engine: Pin<&mut QQmlApplicationEngine>, dir: &str);
    }
}
