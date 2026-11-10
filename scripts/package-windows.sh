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

cargo build --release -p kontra-app

rm -rf "$OUT"
mkdir -p "$OUT/bin" "$SHARE/qml" "$SHARE/demo"
cp target/release/kontra.exe "$OUT/bin/"

# Kontra's own files, as packaging/org.shrippen.Kontra.yml installs them.
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

# Qt: the libraries, plugins and QML modules the pages and Kante import, and what Kirigami's
# own libraries and QML plugins need (windeployqt follows only the files it is given: without
# them the program stopped at a "DLL not found" dialog).
mapfile -t kirigami < <(find "$OUT/bin" -name '*.dll')
"$QT_ROOT/bin/windeployqt.exe" --release --no-compiler-runtime \
    --qmldir crates/app/qml --qmlimport "$KF6_PREFIX/lib/qml" \
    "$OUT/bin/kontra.exe" "${kirigami[@]}"

# The C++ runtime of this Visual Studio next to the program: a fresh Windows has none, and the
# folder must start without an installer (Microsoft allows shipping these files app-local).
vswhere="${PROGRAMFILES_X86:-C:/Program Files (x86)}/Microsoft Visual Studio/Installer/vswhere.exe"
studio="$("$vswhere" -latest -products '*' -property installationPath | tr -d '\r')"
runtime="$(find "$(cygpath -u "$studio")/VC/Redist/MSVC" -type d -path '*/x64/Microsoft.VC*.CRT' | sort | tail -1)"
test -d "$runtime"
cp "$runtime"/*.dll "$OUT/bin/"

# The Fusion style Kontra sets on Windows (main.rs) comes with windeployqt; without it the
# program would start in the native style again.
test -d "$OUT/bin/qml/QtQuick/Controls/Fusion"

echo "package-windows: $OUT ($(du -sh "$OUT" | cut -f1))"
