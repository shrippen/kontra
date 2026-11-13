#!/usr/bin/env bash
# Test of scripts/qa-session.sh: a seed must not undo the isolation. Only the data folder is
# prepared (--prepare-only): no X server, no Kontra, no audio device.
#
#   scripts/qa-session.test.sh
set -euo pipefail
root="$(git rev-parse --show-toplevel)"
base="${TMPDIR:-$HOME/.cache/claude}"
mkdir -p "$base"
work=$(mktemp -d -p "$base" qa-session-test.XXXXXX)
trap 'rm -rf "$work"' EXIT

mkdir "$work/seed"
echo '{"library": "kept"}' > "$work/seed/library.json"
# A seed with settings of its own: the real interface, the real data.
echo '{"output_device": "real-interface"}' > "$work/seed/settings.json"
: > "$work/take.wav"

data=$(KONTRA_BIN=/bin/true KONTRA_QA_TMP="$work/tmp" "$root/scripts/qa-session.sh" --input-file "$work/take.wav" \
    --seed "$work/seed" --prepare-only)

grep -q kept "$data/library.json" || { echo "FAIL: the seed's library.json is missing"; exit 1; }
device=$(python3 -c 'import json, sys; print(json.load(open(sys.argv[1])).get("output_device"))' "$data/settings.json")
if [ "$device" != "kontra:null" ]; then
    echo "FAIL: settings.json has output_device '$device', not 'kontra:null'"
    exit 1
fi
echo "qa-session.test: ok"
