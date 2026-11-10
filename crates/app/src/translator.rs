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
        /// `source` in `context` as Qt translates it now (for the test).
        #[allow(dead_code)]
        fn translated(context: &str, source: &str) -> String;
        /// Loads the Kirigami library as the first page does; false if it is not there (for the
        /// test).
        #[allow(dead_code)]
        fn load_kirigami() -> bool;
    }
}

#[cfg(test)]
mod tests {
    use super::ffi::{install_kirigami_translator, load_kirigami, translated};
    use cxx_qt_lib::QGuiApplication;

    const CONTEXT: &str = "GlobalDrawer";
    const SOURCE: &str = "Close Sidebar";

    /// Review #237: Kontra in English on a German system. Kirigami installs its German
    /// translation when its library loads (with the first page, and again later), in front of
    /// whatever was installed before; Kontra's stays in front all the same. Passes without
    /// proof where Kirigami or its German catalog is not installed.
    #[cfg(target_os = "linux")]
    #[test]
    fn english_goes_in_front_of_kirigamis_own_german() {
        // SAFETY of the order: set before Qt reads them, in this test only.
        std::env::set_var("LANGUAGE", "de");
        std::env::set_var("LANG", "de_DE.UTF-8");
        std::env::set_var("QT_QPA_PLATFORM", "offscreen");
        // CI keeps Kirigami outside the system folders (scripts/setup-qt-env.sh).
        if let Some(prefix) = std::env::var_os("KF6_PREFIX") {
            let share = std::path::PathBuf::from(prefix).join("share");
            let dirs = std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/share".into());
            std::env::set_var("XDG_DATA_DIRS", format!("{}:{dirs}", share.display()));
        }
        let _app = QGuiApplication::new();

        // As at the start: Kontra's language first, then the pages bring Kirigami, which puts
        // its German in front. Kontra's must come back to the front by itself.
        install_kirigami_translator("en");
        if !load_kirigami() {
            return;
        }
        assert_eq!(translated(CONTEXT, SOURCE), SOURCE);

        // And German when asked for (skipped without a German catalog).
        if install_kirigami_translator("de") {
            assert_ne!(translated(CONTEXT, SOURCE), SOURCE);
        }
    }
}
