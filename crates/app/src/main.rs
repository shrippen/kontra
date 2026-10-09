//! Kontra desktop app. Arguments (all optional, for QA runs and screenshots, see QA.md):
//!
//! ```text
//! kontra [--open FILE] [--page library|practice|tuner|settings] [--input-file WAV]
//!        [--input-profile laptop-jack] [--autoplay] [--screenshot PNG] [--screenshot-after MS]
//!        [--quit-after MS] [--style kante|kante-light|system] [--language de|en]
//!        [--mode play|song|wait|workshop] [--view lane|tab|notation]
//! ```

mod args;
mod bridge;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};
use std::path::PathBuf;

const MAIN_QML: &str = "qrc:/qt/qml/org/shrippen/kontra/qml/Main.qml";

fn main() {
    let mut app = QGuiApplication::new();
    let mut engine = QQmlApplicationEngine::new();
    if let Some(mut app) = app.as_mut() {
        app.as_mut().set_application_name(&QString::from("Kontra"));
        app.as_mut()
            .set_organization_domain(&QString::from("shrippen.org"));
    }
    if let Some(mut engine) = engine.as_mut() {
        for dir in qml_dirs() {
            engine
                .as_mut()
                .add_import_path(&QString::from(dir.to_string_lossy().as_ref()));
        }
        engine.load(&QUrl::from(MAIN_QML));
    }
    if let Some(app) = app.as_mut() {
        app.exec();
    }
}

/// Where Kante's QML folder lives: `KONTRA_QML_DIR`, next to the binary (`../share/kontra/qml`)
/// or, in development, in the crate.
fn qml_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(dir) = std::env::var_os("KONTRA_QML_DIR") {
        dirs.push(PathBuf::from(dir));
    }
    if let Some(installed) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|p| p.join("../share/kontra/qml")))
    {
        dirs.push(installed);
    }
    dirs.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("qml"));
    dirs.into_iter()
        .filter(|d| d.join("Kante/qmldir").exists())
        .collect()
}
