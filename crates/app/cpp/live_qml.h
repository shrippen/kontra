// Development only (KONTRA_LIVE_QML, LH-LRN-21): an edited QML file shows without a restart.
#pragma once
#include "rust/cxx.h"

#include <QQmlApplicationEngine>

// Watches the *.qml files in `dir`; after a change the current page is dropped, the engine's
// component cache cleared and the page built again from disk (`unloadPage`, `reloadPage` in
// Main.qml).
void watch_live_qml(QQmlApplicationEngine &engine, rust::Str dir);
