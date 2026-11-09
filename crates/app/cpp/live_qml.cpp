#include "live_qml.h"

#include <QDir>
#include <QFileSystemWatcher>
#include <QPointer>
#include <QTimer>

namespace {
// An editor writes a file in several steps (temp file, rename): wait until it is quiet.
constexpr int SETTLE_MS = 200;
// Time for the dropped page to be deleted (deferred), so its components leave the cache.
constexpr int DROP_MS = 50;

// Watches every QML file in `dir` and the folder itself; saving by rename replaces a file,
// whose watch is then gone, so this runs again after each change.
void watch_files(QFileSystemWatcher &watcher, const QString &dir)
{
    const QDir folder(dir);
    const auto watched = watcher.files();
    for (const auto &name : folder.entryList({QStringLiteral("*.qml")}, QDir::Files)) {
        const auto path = folder.filePath(name);
        if (!watched.contains(path)) {
            watcher.addPath(path);
        }
    }
    if (!watcher.directories().contains(dir)) {
        watcher.addPath(dir);
    }
}

// Drop the page, clear the cache once it is gone, build it again.
void reload(QQmlApplicationEngine &engine)
{
    const auto roots = engine.rootObjects();
    if (roots.isEmpty()) {
        return;
    }
    QPointer<QObject> root = roots.first();
    QMetaObject::invokeMethod(root, "unloadPage");
    QTimer::singleShot(DROP_MS, &engine, [&engine, root] {
        if (!root) {
            return;
        }
        engine.collectGarbage();
        engine.clearComponentCache();
        QMetaObject::invokeMethod(root, "reloadPage");
    });
}
}

void watch_live_qml(QQmlApplicationEngine &engine, rust::Str dir)
{
    const auto path = QString::fromUtf8(dir.data(), static_cast<qsizetype>(dir.size()));
    auto *watcher = new QFileSystemWatcher(&engine);
    auto *settle = new QTimer(&engine);
    settle->setSingleShot(true);
    settle->setInterval(SETTLE_MS);
    watch_files(*watcher, path);

    const auto changed = [settle] { settle->start(); };
    QObject::connect(watcher, &QFileSystemWatcher::fileChanged, settle, changed);
    QObject::connect(watcher, &QFileSystemWatcher::directoryChanged, settle, changed);
    QObject::connect(settle, &QTimer::timeout, &engine, [&engine, watcher, path] {
        watch_files(*watcher, path);
        reload(engine);
    });
}
