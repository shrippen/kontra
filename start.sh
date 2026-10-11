#!/usr/bin/env bash
# Build and start Kontra. Arguments go to the app, e.g.
#   ./start.sh
#   ./start.sh --open testdata/demo/etude-1.mid
set -euo pipefail
cd "$(dirname "$0")"

# Qt from scripts/setup-qt-env.sh if installed, else the system's Qt.
if [[ -d /opt/qt ]]; then
    platform="${QT_QPA_PLATFORM:-}"
    # shellcheck source=scripts/qt-env.sh
    . scripts/qt-env.sh

    # qt-env.sh defaults to headless; keep the user's display.
    if [[ -z "$platform" ]]; then
        unset QT_QPA_PLATFORM
    fi
fi

exec cargo run -q -p kontra-app --release -- "$@"
