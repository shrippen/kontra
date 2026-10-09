#!/usr/bin/env bash
# Builds the Flatpak (decision round 5: in CI), exports kontra.flatpak and starts it once headless
# with a screenshot as a smoke test.
#
#   scripts/flatpak.sh [OUT_DIR]      (default: flatpak-out)
#
# Needs flatpak and flatpak-builder; installs org.kde.Sdk//6.10 and the Rust extension from
# Flathub for the current user if missing.
set -euo pipefail
cd "$(dirname "$0")/.."
OUT="${1:-flatpak-out}"
APP=org.shrippen.Kontra
mkdir -p "$OUT"

flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak install --user -y --noninteractive flathub \
    org.kde.Sdk//6.10 org.kde.Platform//6.10 org.freedesktop.Sdk.Extension.rust-stable//25.08

# The build has no network: vendor all crates and point cargo at them.
cargo vendor --locked --versioned-dirs vendor > packaging/cargo-vendor.toml

flatpak-builder --user --force-clean --disable-rofiles-fuse --repo=flatpak-repo \
    flatpak-build packaging/$APP.yml
flatpak build-bundle flatpak-repo "$OUT/kontra.flatpak" $APP
flatpak install --user -y --noninteractive --reinstall "$OUT/kontra.flatpak"

# Smoke test: start headless, open the bundled demo, screenshot, quit.
data="$(mktemp -d)"
QT_QPA_PLATFORM=offscreen timeout 120 flatpak run --user --env=QT_QPA_PLATFORM=offscreen \
    --env=KONTRA_DATA_DIR=/tmp/kontra-smoke --filesystem="$PWD/$OUT" $APP \
    --page library --screenshot "$PWD/$OUT/flatpak-library.png" --screenshot-after 2500 --quit-after 4000 \
    > "$OUT/smoke.log" 2>&1 || true
rm -rf "$data"
if [ ! -s "$OUT/flatpak-library.png" ]; then
    echo "flatpak smoke test: no screenshot" >&2
    tail -20 "$OUT/smoke.log" >&2
    exit 1
fi
echo "flatpak: $OUT/kontra.flatpak, smoke screenshot $OUT/flatpak-library.png"
