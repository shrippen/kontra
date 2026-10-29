#!/usr/bin/env bash
# One isolated test run of Kontra in its own X server (Harmonicon W2). Kontra never sees the
# desktop, the real audio interface or the real data folder:
#
#   Xvfb :N ─► Kontra (xcb, no Wayland, own XDG_RUNTIME_DIR, own data folder with output
#              "kontra:null", input from a file) ─► [--run CMD with DISPLAY=:N] ─► clean up
#
#   scripts/qa-session.sh --input-file take.wav [--a11y] [--size 1280x720] [--keep]
#                         [--run "xdotool key space"] [-- <kontra arguments>]
#
#   --input-file  required; Kontra would open the owner's interface without it (13.10.)
#   --a11y        own D-Bus session bus for AT-SPI and Orca, started inside the clean environment
#   --size        screen of the X server (default 1920x1080)
#   --keep        keep the work folder (data, log) and print it
#   --run CMD     run CMD (with DISPLAY set) once the window is there, then stop Kontra;
#                 without it Kontra runs until it quits or Ctrl-C
#
# Kontra binary: $KONTRA_BIN, else $CARGO_TARGET_DIR/debug/kontra, else target/debug/kontra.
set -euo pipefail
# Paths given by the caller (--input-file, KONTRA_BIN, CARGO_TARGET_DIR) and --run hold from the
# caller's folder; only the default binary is looked up in the repository.
caller="$PWD"
repo="$(cd "$(dirname "$0")/.." && pwd)"

input="" a11y=0 size="1920x1080" keep=0 run=""
while [ $# -gt 0 ]; do
    case "$1" in
        --input-file) input="$2"; shift 2 ;;
        --a11y) a11y=1; shift ;;
        --size) size="$2"; shift 2 ;;
        --keep) keep=1; shift ;;
        --run) run="$2"; shift 2 ;;
        --) shift; break ;;
        *) echo "qa-session: unknown option $1 (see the head of this script)" >&2; exit 2 ;;
    esac
done

if [ -n "$input" ] && [ -f "$input" ]; then
    input="$(realpath "$input")"
fi
if [ -z "$input" ] || [ ! -f "$input" ]; then
    echo "qa-session: --input-file with an existing file is required (never the real interface)" >&2
    exit 2
fi
for arg in "$@"; do
    if [ "$arg" = "--input-file" ]; then
        echo "qa-session: give --input-file to the script, not after --" >&2
        exit 2
    fi
done

app="${KONTRA_BIN:-${CARGO_TARGET_DIR:-$repo/target}/debug/kontra}"
app="$(realpath -m "$app")"
if [ ! -x "$app" ]; then
    echo "qa-session: $app not found; cargo build -p kontra-app first" >&2
    exit 2
fi

# Temporary files under ~/.cache/claude (CLAUDE.md), never /tmp.
base="${KONTRA_QA_TMP:-$HOME/.cache/claude/kontra-qa}"
mkdir -p "$base"
work="$(mktemp -d -p "$base" session.XXXXXX)"
mkdir -p "$work/data" "$work/runtime"
chmod 700 "$work/runtime"
echo '{"output_device":"kontra:null"}' > "$work/data/settings.json"

# A free display from :99 up.
display=99
while [ -e "/tmp/.X11-unix/X$display" ] || [ -e "/tmp/.X$display-lock" ]; do
    display=$((display + 1))
done

xvfb_pid="" app_pid=""
cleanup() {
    # Kontra runs in its own process group (with its bus, if any): stop all of it.
    [ -n "$app_pid" ] && kill -- "-$app_pid" 2>/dev/null || true
    [ -n "$xvfb_pid" ] && kill "$xvfb_pid" 2>/dev/null || true
    wait 2>/dev/null || true
    if [ "$keep" = 1 ]; then
        echo "qa-session: kept $work"
    else
        rm -rf "$work"
    fi
}
trap cleanup EXIT INT TERM

Xvfb ":$display" -screen 0 "${size}x24" -nolisten tcp > "$work/xvfb.log" 2>&1 &
xvfb_pid=$!
for _ in $(seq 50); do
    [ -e "/tmp/.X11-unix/X$display" ] && break
    sleep 0.1
done
if [ ! -e "/tmp/.X11-unix/X$display" ]; then
    echo "qa-session: Xvfb :$display did not start (see $work/xvfb.log)" >&2
    keep=1
    exit 1
fi

# The environment is cleaned before anything starts, the session bus too: a bus started with
# the desktop's environment let a portal attach to the real Wayland session (28.10.).
clean=(env -u WAYLAND_DISPLAY -u XDG_SESSION_TYPE -u DBUS_SESSION_BUS_ADDRESS
    QT_QPA_PLATFORM=xcb DISPLAY=":$display" XDG_RUNTIME_DIR="$work/runtime"
    KONTRA_DATA_DIR="$work/data")
bus=()
if [ "$a11y" = 1 ]; then
    bus=(dbus-run-session --)
fi
setsid "${clean[@]}" "${bus[@]}" "$app" --input-file "$input" "$@" > "$work/kontra.log" 2>&1 &
app_pid=$!

# Wait for the window (xdotool), at most 30 s.
window=""
for _ in $(seq 300); do
    # The X server is Kontra's alone: any visible window is its own.
    window="$(DISPLAY=":$display" xdotool search --onlyvisible --name . 2>/dev/null | head -1 || true)"
    [ -n "$window" ] && break
    if ! kill -0 "$app_pid" 2>/dev/null; then
        echo "qa-session: Kontra ended before its window appeared (see $work/kontra.log)" >&2
        keep=1
        exit 1
    fi
    sleep 0.1
done
if [ -z "$window" ]; then
    echo "qa-session: no window after 30 s (see $work/kontra.log)" >&2
    keep=1
    exit 1
fi
echo "qa-session: DISPLAY=:$display window $window data $work/data"

if [ -n "$run" ]; then
    (cd "$caller" && DISPLAY=":$display" KONTRA_QA_WINDOW="$window" bash -c "$run")
    exit $?
fi
wait "$app_pid"
