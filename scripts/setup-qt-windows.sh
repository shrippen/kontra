#!/usr/bin/env bash
# Qt and Kirigami for Kontra on Windows (LH-NF-17, Win3), the counterpart of
# scripts/setup-qt-env.sh. Runs in Git Bash (GitHub's windows-latest: Python, CMake, Visual
# Studio and curl are there). CMake takes the newest Visual Studio it finds; Qt's msvc2022
# libraries link with later compilers too.
#
#   C:/Qt/<ver>/msvc2022_64   Qt from download.qt.io (aqtinstall), plus qtshadertools and
#                             qtmultimedia
#   C:/kf6                    extra-cmake-modules and Kirigami, built from the KDE release
#                             tarballs (sha256 checked) with MSVC
#
# Idempotent: every step is skipped when its result is already there.
# Afterwards: ". scripts/qt-env-windows.sh".
set -euo pipefail

QT_VERSION="${QT_VERSION:-6.10.3}"
KF_VERSION="${KF_VERSION:-6.30.0}"
QT_BASE="${QT_BASE:-C:/Qt}"
KF6_PREFIX="${KF6_PREFIX:-C:/kf6}"
SRC_DIR="${SRC_DIR:-C:/src/kf6-${KF_VERSION}}"
QT_ROOT="${QT_BASE}/${QT_VERSION}/msvc2022_64"
KF_SERIES="${KF_VERSION%.*}"

# The same tarballs as on Linux (scripts/setup-qt-env.sh).
declare -A KNOWN_SHA256=(
    [extra-cmake-modules-6.30.0.tar.xz]=22c9f7ff930dae7329faebf2942d2424f4a298c43bb3f11e217be6b12f227f6e
    [kirigami-6.30.0.tar.xz]=6de811e559c20dc1086c0cf2c34bb98712dbe4d45f960c62c3c3f887332c95f8
)

log() { printf '\n== %s\n' "$*"; }

# 1. Qt.
log "Qt ${QT_VERSION} -> ${QT_ROOT}"
if [ ! -x "${QT_ROOT}/bin/qmake.exe" ] || [ ! -d "${QT_ROOT}/lib/cmake/Qt6ShaderTools" ] \
    || [ ! -d "${QT_ROOT}/lib/cmake/Qt6Multimedia" ]; then
    python -m pip install -q 'aqtinstall==3.3.*'
    python -m aqt install-qt windows desktop "${QT_VERSION}" win64_msvc2022_64 \
        -m qtshadertools qtmultimedia -O "${QT_BASE}"
else
    echo "present"
fi

# 2. KDE sources (release tarballs, checksum verified). Windows' own tar reads .xz.
fetch() {
    local name="$1" file="$1-${KF_VERSION}.tar.xz"
    local url="https://download.kde.org/stable/frameworks/${KF_SERIES}/${file}"
    mkdir -p "${SRC_DIR}/dl"
    if [ ! -d "${SRC_DIR}/${name}-${KF_VERSION}" ]; then
        curl -sSfL -o "${SRC_DIR}/dl/${file}" "${url}"
        local want="${KNOWN_SHA256[$file]:-}"
        [ -n "${want}" ] || want=$(curl -sSfL "${url}.sha256" | awk '{print $1}')
        echo "${want}  ${SRC_DIR}/dl/${file}" | sha256sum -c -
        "${SYSTEMROOT:-C:/Windows}/System32/tar.exe" -C "${SRC_DIR}" -xf "${SRC_DIR}/dl/${file}"
    fi
}

# 3. extra-cmake-modules (ECM), then Kirigami. No D-Bus, tests or examples.
log "extra-cmake-modules ${KF_VERSION} -> ${KF6_PREFIX}"
if [ ! -f "${KF6_PREFIX}/share/ECM/cmake/ECMConfigVersion.cmake" ]; then
    fetch extra-cmake-modules
    s="${SRC_DIR}/extra-cmake-modules-${KF_VERSION}"
    cmake -S "$s" -B "$s/build" -A x64 -DCMAKE_INSTALL_PREFIX="${KF6_PREFIX}" \
        -DBUILD_TESTING=OFF -DBUILD_DOC=OFF
    cmake --build "$s/build" --config Release
    cmake --install "$s/build" --config Release
else
    echo "present"
fi

log "Kirigami ${KF_VERSION} -> ${KF6_PREFIX}"
if [ ! -d "${KF6_PREFIX}/lib/cmake/KF6Kirigami" ] \
   || [ "$(cat "${KF6_PREFIX}/.qt-version" 2>/dev/null)" != "${QT_VERSION}" ]; then
    fetch kirigami
    s="${SRC_DIR}/kirigami-${KF_VERSION}"
    rm -rf "$s/build"
    cmake -S "$s" -B "$s/build" -A x64 -Wno-dev \
        -DCMAKE_INSTALL_PREFIX="${KF6_PREFIX}" -DCMAKE_PREFIX_PATH="${QT_ROOT};${KF6_PREFIX}" \
        -DBUILD_TESTING=OFF -DBUILD_EXAMPLES=OFF -DUSE_DBUS=OFF
    # Kirigami packs a file with "tar": Git Bash's reads "C:" as a host name ("Cannot connect
    # to C: resolve failed"), so Windows' own tar goes first for the build.
    PATH="$(cygpath -u "${SYSTEMROOT:-C:/Windows}")/System32:${PATH}" \
        cmake --build "$s/build" --config Release --parallel
    cmake --install "$s/build" --config Release
    echo "${QT_VERSION}" > "${KF6_PREFIX}/.qt-version"   # Kirigami is tied to this Qt build
else
    echo "present"
fi

# 4. Smoke test: Kirigami loads in this Qt, offscreen.
log "smoke test"
here="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=qt-env-windows.sh
QT_VERSION="${QT_VERSION}" QT_ROOT="${QT_ROOT}" KF6_PREFIX="${KF6_PREFIX}" . "${here}/qt-env-windows.sh"
tmp="$(mktemp -d)"
printf 'import QtQuick\nimport org.kde.kirigami as Kirigami\nKirigami.ApplicationWindow { Component.onCompleted: { console.log("kirigami ok, units", Kirigami.Units.gridUnit); Qt.quit() } }\n' > "$tmp/smoke.qml"
timeout 120 "${QT_ROOT}/bin/qml.exe" "$tmp/smoke.qml"
rm -rf "$tmp"
echo "done"
