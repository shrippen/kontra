#!/usr/bin/env bash
# A folder with Kontra for Windows that runs without Qt installed (LH-NF-17, Win5): what the
# installer will wrap and what goes into the test machine. Runs in Git Bash after
# scripts/setup-qt-windows.sh; CI uploads the folder as an artifact, no release is made here.
#
#   Kontra/
#     bin/kontra.exe            release build
#     bin/*.dll, bin/qml, …     Qt (windeployqt), Kirigami and the Visual C++ runtime
#     bin/data/locale           Kirigami's translations (Qt looks in <exe folder>/data)
#     share/kontra/qml/Kante    Kante, copied unchanged
#     share/kontra/exercises    the bundled exercises
#     share/kontra/demo         the piece of the first start
#     share/licenses/Kontra     LICENSE (MIT), GPL-3.0.txt and ASIO.txt: this package is built
#                               with ASIO (cargo feature `asio` of kontra-drivers), whose SDK
#                               is GPLv3, so the package is GPLv3 (agent.md, owner, round 18)
#
#   scripts/package-windows.sh [OUT]     (default dist/Kontra)
#
# The layout is the one of the Flatpak (bin/ next to share/): Kontra finds its files relative
# to the program on every system.
set -euo pipefail
cd "$(dirname "$0")/.."
# shellcheck source=qt-env-windows.sh
. scripts/qt-env-windows.sh

OUT="${1:-dist/Kontra}"
SHARE="$OUT/share/kontra"

# With ASIO: only this package gets it (see the head). Its build needs libclang (bindgen),
# which the GitHub runner and the test machine have under "C:\Program Files\LLVM".
cargo build --release -p kontra-app -p kontra-drivers --features kontra-drivers/asio

rm -rf "$OUT"
mkdir -p "$OUT/bin" "$SHARE/qml" "$SHARE/demo"
cp target/release/kontra.exe "$OUT/bin/"

# Kontra's own files, as packaging/io.github.shrippen.Kontra.yml installs them.
cp -r crates/app/qml/Kante "$SHARE/qml/Kante"
cp testdata/demo/etude-1.mid "$SHARE/demo/"
cp -r testdata/exercises "$SHARE/exercises"
rm "$SHARE/exercises/README.md"

# Kirigami: libraries next to the program, its QML module where Qt looks (<exe folder>/qml),
# its translations where Qt's GenericDataLocation looks (<exe folder>/data).
cp "$KF6_PREFIX"/bin/*.dll "$OUT/bin/"
mkdir -p "$OUT/bin/qml"
cp -r "$KF6_PREFIX/lib/qml/org" "$OUT/bin/qml/"
for locale in "$KF6_PREFIX/bin/data/locale" "$KF6_PREFIX/share/locale"; do
    if [ -d "$locale" ]; then
        mkdir -p "$OUT/bin/data"
        cp -r "$locale" "$OUT/bin/data/locale"
        break
    fi
done
# Without them a German Kontra says "Close Sidebar".
test -f "$OUT/bin/data/locale/de/LC_MESSAGES/libkirigami6_qt.qm"

# Qt: the libraries, plugins and QML modules the pages and Kante import, and what Kirigami's
# own libraries and QML plugins need (windeployqt follows only the files it is given: without
# them the program stopped at a "DLL not found" dialog).
# Scanned without the pages' tests: they import QtTest, which the program does not need.
scan="$(mktemp -d)"
cp -r crates/app/qml/. "$scan/"
rm -rf "$scan/tests"
mapfile -t kirigami < <(find "$OUT/bin" -name '*.dll')
"$QT_ROOT/bin/windeployqt.exe" --release --no-compiler-runtime \
    --qmldir "$scan" --qmlimport "$KF6_PREFIX/lib/qml" \
    "$OUT/bin/kontra.exe" "${kirigami[@]}"
rm -rf "$scan"
test ! -e "$OUT/bin/qml/QtTest"

# The C++ runtime of this Visual Studio next to the program: a fresh Windows has none, and the
# folder must start without an installer (Microsoft allows shipping these files app-local).
# vswhere always installs here (Bash cannot read the variable "ProgramFiles(x86)").
vswhere="C:/Program Files (x86)/Microsoft Visual Studio/Installer/vswhere.exe"
studio="$("$vswhere" -latest -products '*' -property installationPath | tr -d '\r')"
runtime="$(find "$(cygpath -u "$studio")/VC/Redist/MSVC" -type d -path '*/x64/Microsoft.VC*.CRT' | sort | tail -1)"
test -d "$runtime"
cp "$runtime"/*.dll "$OUT/bin/"

# The Fusion style Kontra sets on Windows (main.rs) comes with windeployqt; without it the
# program would start in the native style again.
test -d "$OUT/bin/qml/QtQuick/Controls/Fusion"

# The licences this package is under, and why (ASIO).
LICENSES="$OUT/share/licenses/Kontra"
mkdir -p "$LICENSES"
cp LICENSE packaging/windows/GPL-3.0.txt packaging/windows/ASIO.txt "$LICENSES/"

echo "package-windows: $OUT ($(du -sh "$OUT" | cut -f1))"
