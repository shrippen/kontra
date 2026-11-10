//! Kirigami's own texts ("Close Sidebar") in Kontra's language (round 12): its strings go
//! through `qsTr`, which translates only with an installed Qt translator (cpp/translator.cpp).
//!
//! Part of the cxx-qt bridge: the generated glue uses `unsafe`, so the workspace lint is relaxed
//! for this file as for bridge.rs (agent.md). The C++ side only reads files and calls Qt.
#![allow(unsafe_code)]

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qqmlapplicationengine.h");
        type QQmlApplicationEngine = cxx_qt_lib::QQmlApplicationEngine;

        include!("kontra-app/translator.h");
        /// Installs Kirigami's Qt translation for `language`; false if none is installed.
        fn install_kirigami_translator(language: &str) -> bool;
        /// The engine a later `install_kirigami_translator` translates again.
        fn translate_engine(engine: Pin<&mut QQmlApplicationEngine>);
    }
}
