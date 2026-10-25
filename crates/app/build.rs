// Builds the QML module `org.shrippen.kontra` (pages compiled into the binary) and the Rust
// bridge. Kante is loaded from its folder at run time (see main.rs), copied unchanged.
use cxx_qt_build::{CxxQtBuilder, QmlModule};

const PAGES: [&str; 25] = [
    "qml/SetupPage.qml",
    "qml/Main.qml",
    "qml/LibraryPage.qml",
    "qml/PracticePage.qml",
    "qml/TunerPage.qml",
    "qml/SettingsPage.qml",
    "qml/CalibrationBeat.qml",
    "qml/SummarySheet.qml",
    "qml/Texts.qml",
    "qml/LevelAdvice.qml",
    "qml/ProgressPage.qml",
    "qml/StartPage.qml",
    "qml/Mixer.qml",
    "qml/ExercisePage.qml",
    "qml/QuizPage.qml",
    "qml/EarPage.qml",
    "qml/BackingAlign.qml",
    "qml/ConnectionChoice.qml",
    "qml/TuningChoice.qml",
    "qml/SongSources.qml",
    "qml/LearnPage.qml",
    "qml/LessonPage.qml",
    "qml/LessonBlock.qml",
    "qml/GlossaryPage.qml",
    "qml/TermTip.qml",
];

fn main() {
    CxxQtBuilder::new_qml_module(QmlModule::new("org.shrippen.kontra").qml_files(PAGES))
        .files(["src/bridge.rs", "src/translator.rs"])
        .crate_include_root(Some("cpp".into()))
        .cpp_file("cpp/translator.cpp")
        .build();
}
