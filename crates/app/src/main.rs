//! Kontra desktop app. Arguments (all optional, for QA runs and screenshots, see QA.md):
//!
//! ```text
//! kontra [--open FILE] [--page library|practice|tuner|settings] [--input-file WAV]
//!        [--input-profile laptop-jack] [--autoplay] [--screenshot PNG] [--screenshot-after MS]
//!        [--quit-after MS] [--theme kante|kante-light|system] [--language de|en]
//!        [--mode play|song|wait|workshop] [--view lane|tab|notation|combined|keys] [--calibrate]
//!        [--loop-bars FROM-TO]
//!
//! Window size for QA (e.g. a 1280×720 beamer at 150 %): Qt's own `-qwindowgeometry 853x480`
//! with `QT_SCALE_FACTOR=1.5`.
//! ```

mod args;
mod bridge;
mod sync_check;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};
use std::path::PathBuf;

/// The app id: the desktop file's name without `.desktop`.
const APP_ID: &str = "org.shrippen.Kontra";

const MAIN_QML: &str = "qrc:/qt/qml/org/shrippen/kontra/qml/Main.qml";

fn main() {
    let mut app = QGuiApplication::new();
    let mut engine = QQmlApplicationEngine::new();
    if let Some(mut app) = app.as_mut() {
        app.as_mut().set_application_name(&QString::from("Kontra"));
        app.as_mut()
            .set_organization_domain(&QString::from("shrippen.org"));
    }
    // Wayland compositors find the window icon (packaging/org.shrippen.Kontra.svg) through the
    // desktop file of this name. Only when it is installed: Qt registers the name with the
    // desktop portal, which warns about an unknown one (a run from the source tree).
    if desktop_file_installed() {
        QGuiApplication::set_desktop_file_name(&QString::from(APP_ID));
    }
    if let Some(mut engine) = engine.as_mut() {
        for dir in qml_dirs() {
            engine
                .as_mut()
                .add_import_path(&QString::from(dir.to_string_lossy().as_ref()));
        }
        // Without this a broken UI leaves a process without a window that never exits.
        engine
            .as_mut()
            .on_object_creation_failed(|_, url| {
                eprintln!("kontra: could not load {}", url.to_qstring());
                std::process::exit(1);
            })
            .release();
        engine.load(&QUrl::from(MAIN_QML));
    }
    if let Some(app) = app.as_mut() {
        app.exec();
    }
}

/// Where Kante's QML folder lives: `KONTRA_QML_DIR`, next to the binary (`../share/kontra/qml`)
/// or, in development, in the crate.
/// True if `<APP_ID>.desktop` is in an XDG applications folder (installed, or a Flatpak).
fn desktop_file_installed() -> bool {
    let file = format!("{APP_ID}.desktop");
    let home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")));
    let system =
        std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    home.into_iter()
        .chain(std::env::split_paths(&system))
        .any(|dir| dir.join("applications").join(&file).exists())
}

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
