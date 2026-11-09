#!/usr/bin/env bash
# Layout check per page at the screens of P4 and P7 (W3, LH-ANZ-09): every page in an isolated
# session (scripts/qa-session.sh --a11y), window as large as the screen, then
# `scripts/qactl.py layout` lists widgets cut off by the window, overlapping or without a name.
#
#   P4 Thomas, beamer:      1280×720, 150 %  (QT_SCALE_FACTOR=1.5)
#   P7 Sam, magnified:      1920×1080, 200 % (QT_SCALE_FACTOR=2)
#
#   scripts/check-layout.sh [OUT_DIR]   one report per persona and page; exit 1 if any finding
#
# Needs Xvfb, xdotool, dbus-daemon, at-spi2-core, python3-gi and a built app and CLI
# (cargo build -p kontra-app -p kontra-cli). Local only: the CI has no accessibility stack.
set -uo pipefail
cd "$(dirname "$0")/.."
OUT="${1:-${KONTRA_QA_TMP:-$HOME/.cache/claude/kontra-qa}/layout}"
mkdir -p "$OUT"
TARGET="${CARGO_TARGET_DIR:-target}"
CLI="$TARGET/debug/kontra-cli"
DEMO=testdata/demo/etude-1.mid
# Time for a page to settle after the window was resized (layouts, the lane starting).
SETTLE_S=3

take="$OUT/take.wav"
"$CLI" render "$DEMO" "$take" --lead-in 2.4 > /dev/null || exit 2

# name|screen|scale
PERSONAS=("p4|1280x720|1.5" "p7|1920x1080|2")
# name|Kontra arguments
PAGES=(
    "setup|--page setup"
    "home|--page home"
    "library|--page library"
    "practice-lane|--open $DEMO --view lane"
    "practice-tab|--open $DEMO --view tab"
    "practice-notation|--open $DEMO --view notation"
    "practice-combined|--open $DEMO --view combined"
    "practice-present|--open $DEMO --present"
    "exercises|--page exercises"
    "learn|--page learn"
    "quiz|--page quiz"
    "ear|--page ear"
    "progress|--page progress"
    "tuner|--page tuner"
    "settings|--page settings"
)

found=0
for persona in "${PERSONAS[@]}"; do
    IFS='|' read -r who screen scale <<< "$persona"
    width=${screen%x*}
    height=${screen#*x}
    for page in "${PAGES[@]}"; do
        IFS='|' read -r name args <<< "$page"
        report="$OUT/$who-$name.txt"
        # shellcheck disable=SC2086 # the page's arguments are words
        QT_SCALE_FACTOR="$scale" scripts/qa-session.sh --a11y --input-file "$take" --size "$screen" \
            --run "xdotool windowsize \$KONTRA_QA_WINDOW $width $height && sleep $SETTLE_S && \
                   python3 scripts/qactl.py layout > '$report'" \
            -- --language de $args > /dev/null 2>&1
        count=$(grep -c . "$report" 2> /dev/null)
        count=${count:-0}
        echo "$who $name: $count"
        if [ "$count" != 0 ]; then
            found=1
        fi
    done
done
echo "reports: $OUT"
exit "$found"
