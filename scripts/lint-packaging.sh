#!/usr/bin/env bash
# Lints what the Flatpak and software centres read (W6): metainfo, desktop file, manifest.
# A missing tool is skipped with a message, so the script runs anywhere; the CI installs
# appstreamcli and desktop-file-validate, the Flatpak job also flatpak-builder-lint.
#
#   scripts/lint-packaging.sh
set -uo pipefail
cd "$(dirname "$0")/.."
APP=io.github.shrippen.Kontra
# Builder of Flathub, which carries flatpak-builder-lint.
BUILDER=org.flatpak.Builder

status=0
skip() {
    echo "lint-packaging: $1 not installed, skipped" >&2
}

# --no-net: the screenshots live on the landing page, which is not published yet.
if command -v appstreamcli > /dev/null; then
    appstreamcli validate --no-net "packaging/$APP.metainfo.xml" || status=1
else
    skip appstreamcli
fi

if command -v desktop-file-validate > /dev/null; then
    desktop-file-validate "packaging/$APP.desktop" || status=1
else
    skip desktop-file-validate
fi

if command -v flatpak-builder-lint > /dev/null; then
    flatpak-builder-lint manifest "packaging/$APP.yml" || status=1
elif flatpak info "$BUILDER" > /dev/null 2>&1; then
    flatpak run --command=flatpak-builder-lint "$BUILDER" manifest "packaging/$APP.yml" || status=1
else
    skip flatpak-builder-lint
fi
exit "$status"
