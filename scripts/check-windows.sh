#!/usr/bin/env bash
# The Rust core for Windows, from Linux (LH-NF-17, Win1): cross-compiled for
# x86_64-pc-windows-gnu and its tests run under Wine. Quicker than waiting for the
# windows-latest job in CI, which stays the reference (real Windows, MSVC).
#
#   cargo (rustup, target x86_64-pc-windows-gnu) ─► *.exe ─► wine (own prefix, no audio driver)
#
#   scripts/check-windows.sh [build|test] [cargo arguments, e.g. -p kontra-drivers]
#
# Needs: a rustup toolchain with the target (`rustup target add x86_64-pc-windows-gnu`; the
# distribution's Rust has none), mingw-w64-gcc, and Wine for `test`. A toolchain outside PATH:
# KONTRA_WIN_RUST=<folder with rustup/ and cargo/>.
#
# Wine gets its own prefix without an audio driver, and no display: the tests never see the
# real interface or the desktop (as scripts/qa-session.sh, 13.10.).
set -euo pipefail
cd "$(dirname "$0")/.."

TARGET=x86_64-pc-windows-gnu
WORK="${KONTRA_WIN_TMP:-$HOME/.cache/claude/kontra-win}"
mode="${1:-test}"
[ $# -gt 0 ] && shift

if [ -n "${KONTRA_WIN_RUST:-}" ]; then
    export RUSTUP_HOME="$KONTRA_WIN_RUST/rustup" CARGO_HOME="$KONTRA_WIN_RUST/cargo"
    export PATH="$CARGO_HOME/bin:$PATH"
fi
if ! rustc --print target-libdir --target "$TARGET" >/dev/null 2>&1 \
    || [ ! -d "$(rustc --print target-libdir --target "$TARGET")" ]; then
    echo "check-windows: this Rust has no $TARGET (see the head of this script)" >&2
    exit 2
fi
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$WORK/target}"
scope=(--workspace --exclude kontra-app)
[ $# -gt 0 ] && scope=("$@")

case "$mode" in
    build)
        cargo check --target "$TARGET" --all-targets "${scope[@]}"
        ;;
    test)
        export WINEPREFIX="$WORK/wineprefix" WINEDEBUG=-all
        unset DISPLAY WAYLAND_DISPLAY
        # No audio driver: WASAPI lists no device, nothing opens the owner's interface.
        wine reg add 'HKCU\Software\Wine\Drivers' /v Audio /t REG_SZ /d '' /f >/dev/null
        export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUNNER=wine
        cargo test --target "$TARGET" --no-fail-fast "${scope[@]}"
        ;;
    *)
        echo "check-windows: build or test, not $mode" >&2
        exit 2
        ;;
esac
