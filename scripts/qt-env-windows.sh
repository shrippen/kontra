# shellcheck shell=bash
# Source this file in Git Bash (". scripts/qt-env-windows.sh") to build and run Kontra against
# the Qt and Kirigami that scripts/setup-qt-windows.sh installs (LH-NF-17, Win3).
# Override the locations with QT_VERSION, QT_ROOT, KF6_PREFIX before sourcing.
QT_VERSION="${QT_VERSION:-6.10.3}"
QT_ROOT="${QT_ROOT:-C:/Qt/${QT_VERSION}/msvc2022_64}"
KF6_PREFIX="${KF6_PREFIX:-C:/kf6}"

export QT_ROOT KF6_PREFIX
# cxx-qt-build asks this qmake for the Qt install.
export QMAKE="${QT_ROOT}/bin/qmake.exe"
export CMAKE_PREFIX_PATH="${QT_ROOT};${KF6_PREFIX}"
# Runtime: Windows finds DLLs through PATH (Qt, Kirigami).
PATH="$(cygpath -u "${QT_ROOT}/bin"):$(cygpath -u "${KF6_PREFIX}/bin"):${PATH}"
export PATH
export QML_IMPORT_PATH="${KF6_PREFIX}/lib/qml"
export QML2_IMPORT_PATH="${QML_IMPORT_PATH}"
export QT_PLUGIN_PATH="${QT_ROOT}/plugins"
# Headless by default.
export QT_QPA_PLATFORM="${QT_QPA_PLATFORM:-offscreen}"
# Controls style: Fusion, as on Linux (qqc2-desktop-style is not built).
export QT_QUICK_CONTROLS_STYLE="${QT_QUICK_CONTROLS_STYLE:-Fusion}"
