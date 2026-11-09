#!/usr/bin/env bash
# The course video player (LH-LRN-14) with Qt Multimedia: crates/app/qml/tests plays the WebM and
# MP4 clips from testdata/video and expects the subtitle; the other tests there run too (note bar).
# Headless; CI job app and `make check`.
#
#   scripts/check-video.sh
set -euo pipefail
cd "$(dirname "$0")/.."

# Qt 6's qmltestrunner: the CI Qt (scripts/qt-env.sh), else the distribution's; /usr/bin may hold
# the Qt 5 one.
runner=""
for candidate in ${QT_ROOT:+"$QT_ROOT/bin/qmltestrunner"} /usr/lib/qt6/bin/qmltestrunner; do
    if [ -z "$runner" ] && [ -x "$candidate" ]; then
        runner="$candidate"
    fi
done
if [ -z "$runner" ]; then
    echo "check-video: Qt 6 qmltestrunner not found (Qt Declarative)" >&2
    exit 1
fi

# QML_XHR_ALLOW_FILE_READ: tst_learnwidth reads the German catalog.
QML_XHR_ALLOW_FILE_READ=1 QT_QPA_PLATFORM=offscreen "$runner" -import crates/app/qml -input crates/app/qml/tests
