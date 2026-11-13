// Builds the QML module `io.github.shrippen.kontra` (pages compiled into the binary) and the Rust
// bridge. Kante is loaded from its folder at run time (see main.rs), copied unchanged.
use cxx_qt_build::{CxxQtBuilder, QmlModule};

const PAGES: &[&str] = &[
    "qml/SetupPage.qml",
    "qml/Main.qml",
    "qml/LibraryPage.qml",
    "qml/PracticePage.qml",
    "qml/TunerPage.qml",
    "qml/SettingsPage.qml",
    "qml/CalibrationBeat.qml",
    "qml/SummarySheet.qml",
    "qml/SummaryLayer.qml",
    "qml/Texts.qml",
    "qml/LevelAdvice.qml",
    "qml/ProgressPage.qml",
    "qml/StartPage.qml",
    "qml/Mixer.qml",
    "qml/MixerCard.qml",
    "qml/MixerLayer.qml",
    "qml/CountKpi.qml",
    "qml/DayAge.qml",
    "qml/HintTip.qml",
    "qml/ExercisePage.qml",
    "qml/QuizPage.qml",
    "qml/EarPage.qml",
    "qml/BackingAlign.qml",
    "qml/ConnectionChoice.qml",
    "qml/InstrumentChoice.qml",
    "qml/TuningChoice.qml",
    "qml/SongSources.qml",
    "qml/LearnPage.qml",
    "qml/LessonPage.qml",
    "qml/LessonBlock.qml",
    "qml/LessonVideo.qml",
    "qml/GlossaryPage.qml",
    "qml/ReviewPage.qml",
    "qml/TermTip.qml",
    "qml/GlossaryReturn.qml",
    "qml/NoteBar.qml",
    "qml/InputStatus.qml",
    "qml/NavSeparator.qml",
    "qml/DrawerFocus.qml",
    "qml/CollapseBack.qml",
    "qml/NotationStaff.qml",
    "qml/TabStaff.qml",
    "qml/PracticeFooter.qml",
    "qml/NotesArea.qml",
    "qml/SidebarInset.qml",
    "qml/DensityHint.qml",
    "qml/CourseFolderStatus.qml",
    "qml/UpdateNotice.qml",
    "qml/SongMap.qml",
    "qml/CallResponse.qml",
];

fn main() {
    let mut builder =
        CxxQtBuilder::new_qml_module(QmlModule::new("io.github.shrippen.kontra").qml_files(PAGES))
            .files(["src/bridge.rs", "src/translator.rs", "src/qt_env.rs"])
            .crate_include_root(Some("cpp".into()))
            .cpp_file("cpp/translator.cpp")
            .cpp_file("cpp/qt_env.cpp");
    // QML reload without restart (KONTRA_LIVE_QML) is a development tool: debug builds only.
    if std::env::var_os("CARGO_CFG_DEBUG_ASSERTIONS").is_some() {
        builder = builder
            .files(["src/live_qml.rs"])
            .cpp_file("cpp/live_qml.cpp");
    }
    builder.build();
}
