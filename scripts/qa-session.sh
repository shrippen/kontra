#!/usr/bin/env bash
# One isolated test run of Kontra in its own X server (Harmonicon W2). Kontra never sees the
# desktop, the real audio interface, the real data folder or the real Downloads (review #236:
# a file name from there would stand in a QA picture of the library):
#
#   Xvfb :N ─► Kontra (xcb, no Wayland, own XDG_RUNTIME_DIR, own data folder with output
#              "kontra:null", input from a file) ─► [--run CMD with DISPLAY=:N] ─► clean up
#
#   scripts/qa-session.sh --input-file take.wav [--a11y] [--size 1280x720] [--keep]
#                         [--run "xdotool key space"] [-- <kontra arguments>]
#
#   --input-file  required; Kontra would open the owner's interface without it (13.10.)
#   --a11y        own D-Bus session bus for AT-SPI and Orca, started inside the clean environment;
#                 Kontra and the --run command share it (scripts/qactl.py), accessibility always on
#   --size        screen of the X server (default 1920x1080)
#   --keep        keep the work folder (data, log) and print it
#   --run CMD     run CMD (with DISPLAY set) once the window is there, then stop Kontra;
#                 without it Kontra runs until it quits or Ctrl-C
#   --seed DIR    copy DIR's files into Kontra's data folder first (a library.json with linked
#                 folders, progress/ files), for pictures of a library that is not empty; a
#                 settings.json in DIR is ignored (output stays "kontra:null", --settings applies)
#   --downloads DIR  copy DIR's files into Kontra's download folder; "@DOWNLOADS@" in the
#                 seeded library.json becomes that folder (pieces with the origin "Download")
#   --prepare-only  only prepare the data folder, print its path and stop: no X server, no Kontra
#                 (scripts/qa-session.test.sh)
#   --settings J  more settings as a JSON object, e.g. '{"monitor": true}'; the null output stays
#
# Background mode, for QA rounds with a tester agent (skill qa-personas; one session per tester,
# any number in parallel, each on its own display):
#
#   scripts/qa-session.sh start <workdir> --input-file take.wav [--a11y] [--size 1366x800]
#                         [--seed DIR] [--downloads DIR] [--settings J] [-- <kontra arguments>]
#   <workdir>/qa <command>        qactl.py in this session: shot, click, drag, key, type, hover
#                                 and, with --a11y, tree, find, press, text, wait, layout, focus
#   <workdir>/qa stop             end the session, no process stays
#   scripts/qa-session.sh stop <workdir>   the same
#
#   start returns once the window is there. <workdir> then holds session.env (DISPLAY, buses,
#   PIDs, data folder; sourceable), the wrapper qa, data/ downloads/ runtime/ and the logs.
#   shot-<nr>.png (one pixel per click coordinate: no scaling) and actions.log stay after stop.
#   Only the process groups in session.env are ever stopped, each by PID and start time (and
#   boot id), never a process found by name. The session has its own HOME and XDG folders under
#   <workdir>/home (Music, Documents; Downloads is <workdir>/downloads): a dialog starts there.
#
# Kontra binary: $KONTRA_BIN, else $CARGO_TARGET_DIR/debug/kontra, else target/debug/kontra.
set -euo pipefail
# Paths given by the caller (--input-file, KONTRA_BIN, CARGO_TARGET_DIR) and --run hold from the
# caller's folder; only the default binary is looked up in the repository.
caller="$PWD"
repo="$(cd "$(dirname "$0")/.." && pwd)"

# Start time of a process (field 22 of /proc/<pid>/stat, in ticks since boot). With the boot id it
# names one process for good: a PID alone is reused (after a reboot pipewire may hold it).
pstart() {
    awk -F') ' '{ split($2, f, " "); print f[20] }' "/proc/$1/stat" 2> /dev/null
}
# PID in the lock file of an X display (empty if there is none).
lock_pid() {
    { tr -d ' \n' < "/tmp/.X$1-lock"; } 2> /dev/null || true
}
boot_id() {
    cat /proc/sys/kernel/random/boot_id
}

# Ends a background session. session.env lists its process group leaders as "pid:start". A
# group is signalled only if its leader is still the process we started (same start time, same
# boot) or is gone: then no other process can hold that PID, and what still runs in the group
# is ours. A PID that now belongs to another process is left alone, whatever it leads.
stop_session() {
    local dir="$1"
    if [ ! -f "$dir/session.env" ]; then
        echo "qa-session: $dir has no session.env" >&2
        return 2
    fi
    [ -e "$dir/stopped" ] && return 0
    (
        # shellcheck disable=SC1091
        . "$dir/session.env"
        ours() {
            [ "$BOOT_ID" = "$(boot_id)" ] || return 1
            [ -d "/proc/${1%%:*}" ] || return 0
            [ "$(pstart "${1%%:*}")" = "${1##*:}" ]
        }
        running() {
            local entry
            for entry in $PROCS; do
                ours "$entry" && kill -0 -- "-${entry%%:*}" 2> /dev/null && return 0
            done
            return 1
        }
        signal() {
            local entry
            for entry in $PROCS; do
                ours "$entry" && kill "-$1" -- "-${entry%%:*}" 2> /dev/null || true
            done
        }
        signal TERM
        for _ in $(seq 50); do
            running || break
            sleep 0.1
        done
        signal KILL
        rm -rf "$dir/runtime"
    )
    date +%FT%T > "$dir/stopped"
}

mode=oneshot workdir=""
case "${1:-}" in
    stop)
        [ $# -eq 2 ] || { echo "qa-session: stop <workdir>" >&2; exit 2; }
        stop_session "$(realpath "$2")"
        exit $?
        ;;
    start)
        [ $# -ge 2 ] || { echo "qa-session: start <workdir> [options]" >&2; exit 2; }
        mode=start
        mkdir -p "$2"
        workdir="$(realpath "$2")"
        shift 2
        if [ -e "$workdir/session.env" ] && [ ! -e "$workdir/stopped" ]; then
            echo "qa-session: $workdir holds a running session" >&2
            exit 2
        fi
        # Not the caller's terminal or pipe: the session outlives this script.
        exec < /dev/null
        ;;
esac

input="" a11y=0 size="1920x1080" keep=0 run="" extra="{}" seed="" downloads="" prepare_only=0
while [ $# -gt 0 ]; do
    case "$1" in
        --input-file) input="$2"; shift 2 ;;
        --a11y) a11y=1; shift ;;
        --size) size="$2"; shift 2 ;;
        --keep) keep=1; shift ;;
        --run) run="$2"; shift 2 ;;
        --settings) extra="$2"; shift 2 ;;
        --seed) seed="$2"; shift 2 ;;
        --downloads) downloads="$2"; shift 2 ;;
        --prepare-only) prepare_only=1; shift ;;
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
if [ "$mode" = start ]; then
    work="$workdir"
    rm -rf "$work/stopped" "$work/data" "$work/runtime" "$work/downloads" "$work/home"
    keep=1
else
    base="${KONTRA_QA_TMP:-$HOME/.cache/claude/kontra-qa}"
    mkdir -p "$base"
    work="$(mktemp -d -p "$base" session.XXXXXX)"
fi
mkdir -p "$work/data" "$work/runtime" "$work/downloads"
chmod 700 "$work/runtime"
# The seed first, then the settings: a seed never undoes "kontra:null" (its settings.json is
# dropped).
if [ -n "$seed" ]; then
    cp -r "$seed"/. "$work/data/"
    rm -f "$work/data/settings.json"
fi
if [ -n "$downloads" ]; then
    cp -r "$downloads"/. "$work/downloads/"
fi
if [ -f "$work/data/library.json" ]; then
    sed -i "s|@DOWNLOADS@|$work/downloads|g" "$work/data/library.json"
fi
python3 -c 'import json, sys; print(json.dumps({**json.loads(sys.argv[1]), "output_device": "kontra:null"}))' \
    "$extra" > "$work/data/settings.json"
if [ "$prepare_only" = 1 ]; then
    echo "$work/data"
    exit 0
fi

xvfb_pid="" app_pid="" bus_pids=()
cleanup() {
    # Kontra, each bus and the registry run in process groups of their own (with what they
    # started): stop all of it.
    [ -n "$app_pid" ] && kill -- "-$app_pid" 2>/dev/null || true
    for pid in "${bus_pids[@]}"; do
        kill -- "-$pid" 2>/dev/null || true
    done
    [ -n "$xvfb_pid" ] && kill -- "-$xvfb_pid" 2>/dev/null || true
    wait 2>/dev/null || true
    if [ "$keep" = 1 ]; then
        echo "qa-session: kept $work"
    else
        rm -rf "$work"
    fi
}
trap cleanup EXIT INT TERM

# A free display from :99 up. Xvfb takes its lock file atomically; the display is ours only
# when the lock holds our Xvfb's PID (a socket may be another session's). A server that ends
# at once on a lock or socket is a busy display, any other end (a broken --size) is fatal.
display=99
while :; do
    if [ ! -e "/tmp/.X11-unix/X$display" ] && [ ! -e "/tmp/.X$display-lock" ]; then
        setsid Xvfb ":$display" -screen 0 "${size}x24" -nolisten tcp > "$work/xvfb.log" 2>&1 &
        xvfb_pid=$!
        state=wait
        for _ in $(seq 100); do
            if ! kill -0 "$xvfb_pid" 2> /dev/null; then
                state=ended
                break
            fi
            if [ "$(lock_pid "$display")" = "$xvfb_pid" ] \
                && [ -e "/tmp/.X11-unix/X$display" ]; then
                state=up
                break
            fi
            sleep 0.1
        done
        [ "$state" = up ] && break
        if [ "$state" = wait ]; then
            echo "qa-session: Xvfb :$display did not come up (see $work/xvfb.log)" >&2
            keep=1
            exit 1
        fi
        wait "$xvfb_pid" 2> /dev/null || true
        xvfb_pid=""
        if ! grep -qE 'already active|Cannot establish any listening sockets' "$work/xvfb.log"; then
            echo "qa-session: Xvfb did not start (see $work/xvfb.log)" >&2
            keep=1
            exit 1
        fi
    fi
    display=$((display + 1))
    if [ "$display" -gt 199 ]; then
        echo "qa-session: no free display :99 to :199" >&2
        keep=1
        exit 1
    fi
done
procs=("$xvfb_pid:$(pstart "$xvfb_pid")")

# The environment is cleaned before anything starts, the session bus too: a bus started with
# the desktop's environment let a portal attach to the real Wayland session (28.10.).
# The session's own home: Qt's standard folders (Music, Downloads, Documents) and every config,
# cache and data folder lie inside the work folder, never in the caller's home.
home="$work/home"
mkdir -p "$home/.config" "$home/.local/share" "$home/.local/state" "$home/.cache" \
    "$home/Music" "$home/Documents" "$home/Desktop" "$home/Pictures" "$home/Videos"
cat > "$home/.config/user-dirs.dirs" <<DIRS
XDG_DESKTOP_DIR="$home/Desktop"
XDG_DOCUMENTS_DIR="$home/Documents"
XDG_DOWNLOAD_DIR="$work/downloads"
XDG_MUSIC_DIR="$home/Music"
XDG_PICTURES_DIR="$home/Pictures"
XDG_VIDEOS_DIR="$home/Videos"
DIRS
clean=(env -u WAYLAND_DISPLAY -u XDG_SESSION_TYPE -u DBUS_SESSION_BUS_ADDRESS
    QT_QPA_PLATFORM=xcb DISPLAY=":$display" XDG_RUNTIME_DIR="$work/runtime"
    HOME="$home" XDG_CONFIG_HOME="$home/.config" XDG_DATA_HOME="$home/.local/share"
    XDG_CACHE_HOME="$home/.cache" XDG_STATE_HOME="$home/.local/state"
    KONTRA_DATA_DIR="$work/data" KONTRA_DOWNLOAD_DIR="$work/downloads")
if [ "$mode" = start ]; then
    # One image pixel is one click coordinate: no scaling, whatever the caller has set.
    clean+=(QT_SCALE_FACTOR=1 QT_AUTO_SCREEN_SCALE_FACTOR=0 QT_ENABLE_HIGHDPI_SCALING=0)
fi
# Private buses for Kontra and the --run command (qactl talks to Kontra over AT-SPI): a session
# bus, the accessibility bus beside it and the AT-SPI registry on it, all started here. Not by
# activation: at-spi-bus-launcher starts the registry through systemd where dbus-broker is used,
# which the clean environment does not reach (and must not: it is the desktop's), and an
# activated registry would not know the bus. Qt and qactl find it by AT_SPI_BUS_ADDRESS.
A11Y_CONF=/usr/share/defaults/at-spi2/accessibility.conf
REGISTRY=/usr/lib/at-spi2-registryd
bus() {
    local name="$1"
    shift
    setsid "${clean[@]}" dbus-daemon "$@" --nofork --print-address=3 3> "$work/$name" \
        > "$work/$name.log" 2>&1 &
    bus_pids+=("$!")
    procs+=("$!:$(pstart "$!")")
    for _ in $(seq 50); do
        [ -s "$work/$name" ] && return
        sleep 0.1
    done
    echo "qa-session: bus $name did not start (see $work/$name.log)" >&2
    keep=1
    exit 1
}
if [ "$a11y" = 1 ]; then
    bus session --session
    bus a11y --config-file="$A11Y_CONF" --address="unix:dir=$work/runtime"
    clean+=(DBUS_SESSION_BUS_ADDRESS="$(head -1 "$work/session")"
        AT_SPI_BUS_ADDRESS="$(head -1 "$work/a11y")" QT_LINUX_ACCESSIBILITY_ALWAYS_ON=1)
    setsid "${clean[@]}" "$REGISTRY" > "$work/registry.log" 2>&1 &
    bus_pids+=("$!")
    procs+=("$!:$(pstart "$!")")
fi
setsid "${clean[@]}" "$app" --input-file "$input" "$@" > "$work/kontra.log" 2>&1 &
app_pid=$!
procs+=("$app_pid:$(pstart "$app_pid")")

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

if [ "$mode" = start ]; then
    # The window exists before it is painted: the first shot would be black. Wait for a pixel
    # that is not (at most 10 s).
    for _ in $(seq 100); do
        peak="$(DISPLAY=":$display" import -window root -format '%[fx:maxima]' info: 2>/dev/null || true)"
        [ -n "$peak" ] && [ "$peak" != 0 ] && break
        sleep 0.1
    done
    # Session file (sourceable) and the wrapper that runs qactl.py inside the session.
    {
        printf '%s=%q\n' DISPLAY ":$display" XDG_RUNTIME_DIR "$work/runtime" \
            KONTRA_DATA_DIR "$work/data" KONTRA_DOWNLOAD_DIR "$work/downloads" \
            QA_WORKDIR "$work" QA_WINDOW "$window" XVFB_PID "$xvfb_pid" APP_PID "$app_pid" \
            BUS_PIDS "${bus_pids[*]}" PROCS "${procs[*]}" BOOT_ID "$(boot_id)" HOME_DIR "$home"
        if [ "$a11y" = 1 ]; then
            printf '%s=%q\n' DBUS_SESSION_BUS_ADDRESS "$(head -1 "$work/session")" \
                AT_SPI_BUS_ADDRESS "$(head -1 "$work/a11y")" QT_LINUX_ACCESSIBILITY_ALWAYS_ON 1
        fi
    } > "$work/session.env"
    {
        echo '#!/usr/bin/env bash'
        echo '# qactl.py inside this session (written by qa-session.sh start); "stop" ends it.'
        echo 'set -euo pipefail'
        echo 'here="$(cd "$(dirname "$0")" && pwd)"'
        printf 'if [ "${1:-}" = stop ]; then exec %q stop "$here"; fi\n' "$repo/scripts/qa-session.sh"
        echo '[ ! -e "$here/stopped" ] || { echo "qa: session stopped" >&2; exit 1; }'
        echo 'set -a'
        echo '. "$here/session.env"'
        echo 'set +a'
        printf 'exec env -u WAYLAND_DISPLAY -u XDG_SESSION_TYPE QT_QPA_PLATFORM=xcb python3 %q "$@"\n' \
            "$repo/scripts/qactl.py"
    } > "$work/qa"
    chmod +x "$work/qa"
    trap - EXIT INT TERM
    echo "qa-session: started, use $work/qa"
    exit 0
fi

if [ -n "$run" ]; then
    (cd "$caller" && "${clean[@]}" KONTRA_QA_WINDOW="$window" bash -c "$run")
    exit $?
fi
wait "$app_pid"
