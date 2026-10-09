// Builds the QML module `org.shrippen.kontra` (pages compiled into the binary) and the Rust
// bridge. Kante is loaded from its folder at run time (see main.rs), copied unchanged.
use cxx_qt_build::{CxxQtBuilder, QmlModule};

const PAGES: [&str; 7] = [
    "qml/Main.qml",
    "qml/LibraryPage.qml",
    "qml/PracticePage.qml",
    "qml/TunerPage.qml",
    "qml/SettingsPage.qml",
    "qml/SummarySheet.qml",
    "qml/Texts.qml",
];

fn main() {
    CxxQtBuilder::new_qml_module(QmlModule::new("org.shrippen.kontra").qml_files(PAGES))
        .files(["src/bridge.rs"])
        .build();
}
