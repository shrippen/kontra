#!/usr/bin/env bash
# Installs a Qt and Kirigami (KF6) that Kontra can build and run against, for
# containers and CI where the distribution's Qt is too old (Ubuntu 24.04: 6.4).
#
#   /opt/qt-tools          Python venv: aqtinstall, cmake >= 3.29, ninja
#   /opt/qt/<ver>/gcc_64   Qt from download.qt.io (aqtinstall), plus qtshadertools
#   /opt/kf6               extra-cmake-modules and Kirigami, built from the KDE
#                          release tarballs (sha256 checked)
#
# Idempotent: every step is skipped when its result is already there.
# Run as root (apt and /opt). Afterwards: ". scripts/qt-env.sh".
# Versions: QT_VERSION (default 6.10.3), KF_VERSION (default 6.30.0).
# KF 6.30 needs Qt >= 6.9.
set -euo pipefail

QT_VERSION="${QT_VERSION:-6.10.3}"
KF_VERSION="${KF_VERSION:-6.30.0}"
QT_BASE="${QT_BASE:-/opt/qt}"
KF6_PREFIX="${KF6_PREFIX:-/opt/kf6}"
QT_TOOLS="${QT_TOOLS:-/opt/qt-tools}"
SRC_DIR="${SRC_DIR:-/opt/src/kf6-${KF_VERSION}}"
JOBS="${JOBS:-$(nproc)}"
QT_ROOT="${QT_BASE}/${QT_VERSION}/gcc_64"
KF_SERIES="${KF_VERSION%.*}"

# Known checksums of the default release; other versions use KDE's .sha256 file.
declare -A KNOWN_SHA256=(
    [extra-cmake-modules-6.30.0.tar.xz]=22c9f7ff930dae7329faebf2942d2424f4a298c43bb3f11e217be6b12f227f6e
    [kirigami-6.30.0.tar.xz]=6de811e559c20dc1086c0cf2c34bb98712dbe4d45f960c62c3c3f887332c95f8
)

log() { printf '\n== %s\n' "$*"; }

# 1. System packages: compiler, GL/xkb headers, xcb runtime bits for the xcb platform plugin.
log "apt packages"
PKGS=(build-essential python3-venv curl xz-utils libgl1-mesa-dev libegl-dev libvulkan-dev
      libxkbcommon-dev libxkbcommon-x11-0 libxcb-cursor0 libxcb-icccm4 libxcb-keysyms1
      libxcb-render-util0 libxcb-image0 libfontconfig1 libdbus-1-3 xvfb xauth)
missing=()
for p in "${PKGS[@]}"; do dpkg -s "$p" >/dev/null 2>&1 || missing+=("$p"); done
if [ ${#missing[@]} -gt 0 ]; then
    apt-get update -qq
    DEBIAN_FRONTEND=noninteractive apt-get install -y -qq --no-install-recommends "${missing[@]}"
else
    echo "all present"
fi

# 2. Tools venv (aqtinstall to fetch Qt; Kirigami needs CMake >= 3.29, Ubuntu has 3.28).
log "tools venv ${QT_TOOLS}"
if [ ! -x "${QT_TOOLS}/bin/aqt" ] || [ ! -x "${QT_TOOLS}/bin/cmake" ]; then
    python3 -m venv "${QT_TOOLS}"
    "${QT_TOOLS}/bin/pip" install -q 'aqtinstall==3.3.*' 'cmake>=3.29,<4.2' ninja
else
    echo "present"
fi
export PATH="${QT_TOOLS}/bin:${PATH}"

# 3. Qt (base, declarative, svg, wayland, tools come with the base set) + qtshadertools.
log "Qt ${QT_VERSION} -> ${QT_ROOT}"
if [ ! -x "${QT_ROOT}/bin/qmake6" ] || [ ! -d "${QT_ROOT}/lib/cmake/Qt6ShaderTools" ]; then
    aqt install-qt linux desktop "${QT_VERSION}" linux_gcc_64 -m qtshadertools -O "${QT_BASE}"
else
    echo "present"
fi

# 4. KDE sources (release tarballs, checksum verified).
fetch() {
    local name="$1" file="$1-${KF_VERSION}.tar.xz"
    local url="https://download.kde.org/stable/frameworks/${KF_SERIES}/${file}"
    mkdir -p "${SRC_DIR}/dl"
    if [ ! -d "${SRC_DIR}/${name}-${KF_VERSION}" ]; then
        curl -sSfL -o "${SRC_DIR}/dl/${file}" "${url}"
        local want="${KNOWN_SHA256[$file]:-}"
        [ -n "${want}" ] || want=$(curl -sSfL "${url}.sha256" | awk '{print $1}')
        echo "${want}  ${SRC_DIR}/dl/${file}" | sha256sum -c -
        tar -C "${SRC_DIR}" -xf "${SRC_DIR}/dl/${file}"
    fi
}

# 5. extra-cmake-modules (ECM), then Kirigami. No D-Bus, tests or examples.
log "extra-cmake-modules ${KF_VERSION} -> ${KF6_PREFIX}"
if [ ! -f "${KF6_PREFIX}/share/ECM/cmake/ECMConfigVersion.cmake" ] \
   || ! grep -q "\"${KF_VERSION}\"" "${KF6_PREFIX}/share/ECM/cmake/ECMConfigVersion.cmake"; then
    fetch extra-cmake-modules
    s="${SRC_DIR}/extra-cmake-modules-${KF_VERSION}"
    cmake -S "$s" -B "$s/build" -G Ninja -DCMAKE_INSTALL_PREFIX="${KF6_PREFIX}" \
        -DBUILD_TESTING=OFF -DBUILD_DOC=OFF
    cmake --build "$s/build"
    cmake --install "$s/build"
else
    echo "present"
fi

KIRIGAMI_CFG="${KF6_PREFIX}/lib/x86_64-linux-gnu/cmake/KF6Kirigami/KF6KirigamiConfigVersion.cmake"
log "Kirigami ${KF_VERSION} -> ${KF6_PREFIX}"
if [ ! -f "${KIRIGAMI_CFG}" ] || ! grep -q "\"${KF_VERSION}\"" "${KIRIGAMI_CFG}" \
   || [ "$(cat "${KF6_PREFIX}/.qt-version" 2>/dev/null)" != "${QT_VERSION}" ]; then
    fetch kirigami
    s="${SRC_DIR}/kirigami-${KF_VERSION}"
    rm -rf "$s/build"
    cmake -S "$s" -B "$s/build" -G Ninja -Wno-dev -DCMAKE_BUILD_TYPE=Release \
        -DCMAKE_INSTALL_PREFIX="${KF6_PREFIX}" -DCMAKE_PREFIX_PATH="${QT_ROOT};${KF6_PREFIX}" \
        -DBUILD_TESTING=OFF -DBUILD_EXAMPLES=OFF -DUSE_DBUS=OFF
    cmake --build "$s/build" -j "${JOBS}"
    cmake --install "$s/build"
    echo "${QT_VERSION}" > "${KF6_PREFIX}/.qt-version"   # Kirigami is tied to this Qt build
else
    echo "present"
fi

# 6. Smoke test: Kirigami loads in the new Qt, offscreen.
log "smoke test"
here="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=qt-env.sh
QT_VERSION="${QT_VERSION}" QT_ROOT="${QT_ROOT}" KF6_PREFIX="${KF6_PREFIX}" QT_TOOLS="${QT_TOOLS}" \
    . "${here}/qt-env.sh"
tmp="$(mktemp -d)"
printf 'import QtQuick\nimport org.kde.kirigami as Kirigami\nKirigami.ApplicationWindow { Component.onCompleted: { console.log("kirigami ok, units", Kirigami.Units.gridUnit); Qt.quit() } }\n' > "$tmp/smoke.qml"
timeout 60 qml "$tmp/smoke.qml"
rm -rf "$tmp"

log "done; to use it:"
cat <<EOF
  . ${here}/qt-env.sh
or export:
  PATH=${QT_ROOT}/bin:${QT_TOOLS}/bin:\$PATH
  QMAKE=${QMAKE}
  CMAKE_PREFIX_PATH=${CMAKE_PREFIX_PATH}
  LD_LIBRARY_PATH=${LD_LIBRARY_PATH}
  QML_IMPORT_PATH=${QML_IMPORT_PATH}   (add the Kante qml/ dir)
  QT_PLUGIN_PATH=${QT_PLUGIN_PATH}
  QT_QPA_PLATFORM=${QT_QPA_PLATFORM}
  QT_QUICK_CONTROLS_STYLE=${QT_QUICK_CONTROLS_STYLE}
  LC_ALL=${LC_ALL}
EOF
