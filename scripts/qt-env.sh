# shellcheck shell=bash
# Source this file (". scripts/qt-env.sh") to build and run Kontra against the
# Qt and Kirigami that scripts/setup-qt-env.sh installs under /opt.
# Override the locations with QT_VERSION, QT_ROOT, KF6_PREFIX before sourcing.
QT_VERSION="${QT_VERSION:-6.10.3}"
QT_ROOT="${QT_ROOT:-/opt/qt/${QT_VERSION}/gcc_64}"
KF6_PREFIX="${KF6_PREFIX:-/opt/kf6}"
QT_TOOLS="${QT_TOOLS:-/opt/qt-tools}"   # venv with aqtinstall, cmake >= 3.29, ninja
_kf6_lib="${KF6_PREFIX}/lib/x86_64-linux-gnu"

export QT_ROOT KF6_PREFIX
# Qt tools (qmake6, qml, qmltestrunner, qmllint) first, then cmake/ninja from the venv.
export PATH="${QT_ROOT}/bin:${QT_TOOLS}/bin:${PATH}"
# cxx-qt-build asks this qmake for the Qt install (else it takes qmake6 from PATH).
export QMAKE="${QT_ROOT}/bin/qmake6"
export CMAKE_PREFIX_PATH="${QT_ROOT};${KF6_PREFIX}${CMAKE_PREFIX_PATH:+;${CMAKE_PREFIX_PATH}}"
# Runtime: Qt and Kirigami libraries, QML modules and plugins.
export LD_LIBRARY_PATH="${QT_ROOT}/lib:${_kf6_lib}${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}"
export QML_IMPORT_PATH="${_kf6_lib}/qml${QML_IMPORT_PATH:+:${QML_IMPORT_PATH}}"
export QML2_IMPORT_PATH="${QML_IMPORT_PATH}"
export QT_PLUGIN_PATH="${QT_ROOT}/plugins"
# Qt wants a UTF-8 locale; containers often have plain C.
export LC_ALL="${LC_ALL:-C.UTF-8}"
# Headless by default; set QT_QPA_PLATFORM=xcb (with xvfb-run) when needed.
export QT_QPA_PLATFORM="${QT_QPA_PLATFORM:-offscreen}"
# Controls style: Fusion. org.kde.desktop (qqc2-desktop-style) is not built here,
# it pulls in KConfig, KIconThemes, KColorScheme, Sonnet and more.
export QT_QUICK_CONTROLS_STYLE="${QT_QUICK_CONTROLS_STYLE:-Fusion}"
unset _kf6_lib
