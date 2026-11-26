#!/usr/bin/env bash
# Test of scripts/qa-session.sh. Part 1: a seed must not undo the isolation; --downloads fills
# the download folder (--prepare-only: no X server, no Kontra, no audio device). Part 2: the
# background mode (start, qa shot, qa stop) with a real Kontra in its own Xvfb: the data
# folders lie in the session, a shot is a PNG, a file dialog opens in the session's X server
# and nowhere else, actions.log is written, stop leaves no process. Part 2 is skipped
# (with a note) where Kontra is not built or Xvfb, xdotool or ImageMagick are missing.
#
#   scripts/qa-session.test.sh
set -euo pipefail
root="$(git rev-parse --show-toplevel)"
base="${TMPDIR:-$HOME/.cache/claude}"
mkdir -p "$base"
work=$(mktemp -d -p "$base" qa-session-test.XXXXXX)
trap 'rm -rf "$work"' EXIT

mkdir "$work/seed"
echo '{"library": "kept", "path": "@DOWNLOADS@/a.gp"}' > "$work/seed/library.json"
mkdir "$work/dl"
: > "$work/dl/a.gp"
# A seed with settings of its own: the real interface, the real data.
echo '{"output_device": "real-interface"}' > "$work/seed/settings.json"
: > "$work/take.wav"

data=$(KONTRA_BIN=/bin/true KONTRA_QA_TMP="$work/tmp" "$root/scripts/qa-session.sh" --input-file "$work/take.wav" \
    --seed "$work/seed" --downloads "$work/dl" --prepare-only)

grep -q kept "$data/library.json" || { echo "FAIL: the seed's library.json is missing"; exit 1; }
device=$(python3 -c 'import json, sys; print(json.load(open(sys.argv[1])).get("output_device"))' "$data/settings.json")
if [ "$device" != "kontra:null" ]; then
    echo "FAIL: settings.json has output_device '$device', not 'kontra:null'"
    exit 1
fi
# --downloads: the files land in the session's download folder, the library points there.
downloads="$(dirname "$data")/downloads"
[ -f "$downloads/a.gp" ] || { echo "FAIL: the download a.gp was not copied"; exit 1; }
grep -q "\"$downloads/a.gp\"" "$data/library.json" || { echo "FAIL: @DOWNLOADS@ was not replaced"; exit 1; }

# Part 2: background mode.
app="${KONTRA_BIN:-${CARGO_TARGET_DIR:-$root/target}/debug/kontra}"
for tool in Xvfb xdotool import python3; do
    if ! command -v "$tool" > /dev/null; then
        echo "qa-session.test: ok (background mode skipped: no $tool)"
        exit 0
    fi
done
if [ ! -x "$app" ]; then
    echo "qa-session.test: ok (background mode skipped: $app not built)"
    exit 0
fi
export KONTRA_BIN="$app"

python3 - "$work/take.wav" <<'PY'
import sys, wave
w = wave.open(sys.argv[1], "wb")
w.setnchannels(1)
w.setsampwidth(2)
w.setframerate(48000)
w.writeframes(b"\0\0" * 48000)
w.close()
PY

fail() {
    echo "FAIL: $1"
    exit 1
}
# The real folders and X server must stay as they are.
touch "$work/marker"
real_x=""
if [ -n "${DISPLAY:-}" ]; then
    real_x=$(xdotool search --name . 2> /dev/null | sort | tr '\n' ' ' || true)
fi

session="$work/s1"
"$root/scripts/qa-session.sh" start "$session" --a11y --size 1280x800 --input-file "$work/take.wav" \
    > "$work/start.log" || { cat "$work/start.log"; fail "start"; }
# Stop whatever happens from here on, by the session's own file.
trap '"$session/qa" stop > /dev/null 2>&1 || true; rm -rf "$work"' EXIT
[ -x "$session/qa" ] || fail "no wrapper qa"
# Read in a subshell: DISPLAY of the session must not become the test's own.
sv() { (. "$1/session.env"; echo "${!2}"); }
DISPLAY_S=$(sv "$session" DISPLAY)
KONTRA_DATA_DIR=$(sv "$session" KONTRA_DATA_DIR)
KONTRA_DOWNLOAD_DIR=$(sv "$session" KONTRA_DOWNLOAD_DIR)
APP_PID=$(sv "$session" APP_PID)
BUS_PIDS=$(sv "$session" BUS_PIDS)
XVFB_PID=$(sv "$session" XVFB_PID)
case "$KONTRA_DATA_DIR" in "$session"/*) ;; *) fail "data folder $KONTRA_DATA_DIR is outside the session" ;; esac
case "$KONTRA_DOWNLOAD_DIR" in "$session"/*) ;; *) fail "download folder is outside the session" ;; esac
env_of_app=$(tr '\0' '\n' < "/proc/$APP_PID/environ")
echo "$env_of_app" | grep -qx "KONTRA_DATA_DIR=$KONTRA_DATA_DIR" || fail "Kontra has another data folder"
echo "$env_of_app" | grep -qx "KONTRA_DOWNLOAD_DIR=$KONTRA_DOWNLOAD_DIR" || fail "Kontra has another download folder"
echo "$env_of_app" | grep -qx "DISPLAY=$DISPLAY_S" || fail "Kontra is not on the session's display"
echo "$env_of_app" | grep -qE '^(WAYLAND_DISPLAY|XDG_SESSION_TYPE)=' && fail "Kontra sees the Wayland desktop"
grep -q kontra:null "$KONTRA_DATA_DIR/settings.json" || fail "output is not kontra:null"

png="$("$session/qa" shot)"
[ "$png" = "$session/shot-001.png" ] || fail "shot printed '$png'"
[ "$(head -c 4 "$png" | tail -c 3)" = "PNG" ] || fail "shot is not a PNG"
[ "$(python3 -c 'import struct,sys; print(struct.unpack(">II", open(sys.argv[1],"rb").read()[16:24]))' "$png")" = "(1280, 800)" ] \
    || fail "shot is not 1280x800"

# A file dialog: Library page, "+ Add folder" (the only list item with a plus). It must be a
# window of the session's X server and nowhere else.
windows() { DISPLAY="$1" xdotool search --onlyvisible --class kontra 2> /dev/null | wc -l; }
before=$(windows "$DISPLAY_S")
"$session/qa" click 60 63
"$session/qa" wait --role "list item" --contains "+" > /dev/null
box=$("$session/qa" find --role "list item" --contains "+" | head -1 | sed -E 's/.*\[([0-9]+),([0-9]+) ([0-9]+)×([0-9]+)\]/\1 \2 \3 \4/')
read -r bx by bw bh <<< "$box"
"$session/qa" click $((bx + bw / 2)) $((by + bh / 2))
opened=0
for _ in $(seq 50); do
    [ "$(windows "$DISPLAY_S")" -gt "$before" ] && { opened=1; break; }
    sleep 0.1
done
[ "$opened" = 1 ] || fail "no dialog window in the session's X server"
if [ -n "$real_x" ]; then
    [ "$(xdotool search --name . 2> /dev/null | sort | tr '\n' ' ' || true)" = "$real_x" ] \
        || fail "a window appeared on the real display"
fi
"$session/qa" shot > /dev/null

# actions.log: time, command, arguments, last shot.
grep -qP '^\S+\tshot\t\tshot-001$' "$session/actions.log" || fail "actions.log lacks the first shot"
grep -qP '^\S+\tclick\t60 63\tshot-001$' "$session/actions.log" || fail "actions.log lacks the click"
[ "$(wc -l < "$session/actions.log")" -ge 5 ] || fail "actions.log is short"

# Nothing outside the session changed.
for dir in "$HOME/.local/share/kontra" "$HOME/Downloads"; do
    [ -d "$dir" ] || continue
    [ -z "$(find "$dir" -newer "$work/marker" 2> /dev/null | head -1)" ] || fail "$dir was touched"
done

# A second session in parallel on its own display, then stop both.
session2="$work/s2"
"$root/scripts/qa-session.sh" start "$session2" --input-file "$work/take.wav" > /dev/null
[ "$(grep ^DISPLAY= "$session2/session.env")" != "DISPLAY=$DISPLAY_S" ] || fail "two sessions share a display"
"$session2/qa" shot > /dev/null
"$session2/qa" stop
"$session/qa" stop
for pid in $APP_PID $BUS_PIDS $XVFB_PID; do
    if kill -0 "$pid" 2> /dev/null; then
        fail "process $pid is still there after stop"
    fi
done
[ -z "$(ps -o pid= -g "$APP_PID" 2> /dev/null)" ] || fail "the process group of Kontra is still there"
[ ! -e "/tmp/.X11-unix/X${DISPLAY_S#:}" ] || fail "the X socket is still there"
[ -f "$session/shot-001.png" ] && [ -f "$session/actions.log" ] || fail "stop removed shots or the log"
"$session/qa" shot > /dev/null 2>&1 && fail "qa works after stop"
echo "qa-session.test: ok"
