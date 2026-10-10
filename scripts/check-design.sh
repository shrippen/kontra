#!/usr/bin/env bash
# LH-NF-10: Kontra's own QML takes colours, fonts and sizes from Kante (KanteStyle), never
# literally. Fails on hex colours, numeric sizes (other than 0 and 1 for hairlines) and font
# families in crates/app/qml/*.qml; the vendored Kante copy (qml/Kante/) is not checked.
#
#   scripts/check-design.sh [FILE...]
set -euo pipefail
cd "$(dirname "$0")/.."

if [ "$#" -gt 0 ]; then
    files=("$@")
else
    files=(crates/app/qml/*.qml)
fi

# Size properties that must come from KanteStyle (unit(), spacing, font sizes).
SIZES='width|height|implicitWidth|implicitHeight|preferredWidth|preferredHeight|minimumWidth|minimumHeight|maximumWidth|maximumHeight|spacing|rowSpacing|columnSpacing|margins|leftMargin|rightMargin|topMargin|bottomMargin|padding|leftPadding|rightPadding|topPadding|bottomPadding|radius|pixelSize|pointSize'

status=0
report() {
    local what="$1" pattern="$2"
    local hits
    hits=$(grep -nHE "$pattern" "${files[@]}" | grep -vE '^[^:]+:[0-9]+:\s*//' || true)
    if [ -n "$hits" ]; then
        echo "check-design: $what (use KanteStyle):" >&2
        echo "$hits" >&2
        status=1
    fi
}

report "hex colour" '"#[0-9a-fA-F]{3,8}"'
report "px size" '[0-9]+ *px\b'
report "numeric size" "(^|[^A-Za-z.])(${SIZES}) *: *-?([2-9]|[1-9][0-9]+)(\.[0-9]+)? *(;|$|})"
report "font family" '(font\.family|family *:)'

if [ "$status" -eq 0 ]; then
    echo "check-design: ${#files[@]} files clean"
fi
exit "$status"
