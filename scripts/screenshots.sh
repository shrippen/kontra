#!/usr/bin/env bash
# Screenshots of every page from the demo material, in German and English, headless.
# Output: docs/shots/<page>-<lang>.png (release logs, landing page, QA evidence).
#
#   scripts/screenshots.sh [OUT_DIR]
#
# Needs the Qt environment (scripts/setup-qt-env.sh) and builds the app and CLI first.
set -euo pipefail
cd "$(dirname "$0")/.."
# shellcheck source=qt-env.sh
. scripts/qt-env.sh

OUT="${1:-docs/shots}"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
mkdir -p "$OUT"

# cxx-qt: an old qmlcachegen output can call stale method indices after bridge changes.
cargo clean -q -p kontra-app
cargo build -q -p kontra-app -p kontra-cli
APP=target/debug/kontra
CLI=target/debug/kontra-cli
DEMO=testdata/demo/etude-1.mid

# A take with a few mistakes: 20 ms late, every 7th note wrong; one count-in bar (2.4 s).
"$CLI" render "$DEMO" "$WORK/take.wav" --lead-in 2.4 --offset-ms 20 --wrong-every 7 > /dev/null

shot() {
    local name="$1" lang="$2" after="$3"
    shift 3
    KONTRA_DATA_DIR="$WORK/data-$lang" timeout 120 "$APP" --language "$lang" \
        --screenshot "$OUT/$name-$lang.png" --screenshot-after "$after" "$@" > /dev/null 2>&1 || true
    [ -f "$OUT/$name-$lang.png" ] && echo "$OUT/$name-$lang.png"
}

for lang in de en; do
    shot library-empty "$lang" 1500 --page library
    shot practice-lane "$lang" 9000 --open "$DEMO" --input-file "$WORK/take.wav" --autoplay
    shot library "$lang" 1500 --page library
    shot settings "$lang" 1500 --page settings
    shot tuner "$lang" 4000 --page tuner --input-file "$WORK/take.wav"
    shot practice-tab "$lang" 9000 --open "$DEMO" --input-file "$WORK/take.wav" --autoplay --view tab
    shot practice-notation "$lang" 9000 --open "$DEMO" --input-file "$WORK/take.wav" --autoplay --view notation
    shot practice-wait "$lang" 5000 --open "$DEMO" --mode wait --view lane --autoplay
    shot summary "$lang" 25000 --open "$DEMO" --input-file "$WORK/take.wav" --autoplay --mode song --view lane
done
