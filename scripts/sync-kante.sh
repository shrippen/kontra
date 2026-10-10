#!/usr/bin/env bash
# Copies Kante's QML module into the app (GUI rule in agent.md): crates/app/qml/Kante,
# and Kante's web stylesheet for the progress report (LH-FOR-04): crates/appcore/report.
#
# Source: KANTE_DS, default ../Kante (https://git.arianw.de/shrippen/Kante).
# The copy is never edited here; change Kante and sync again.
set -euo pipefail
cd "$(dirname "$0")/.."

DS="${KANTE_DS:-../Kante}"
if [ ! -f "$DS/qml/Kante/qmldir" ]; then
    echo "sync-kante: no Kante design system in $DS (set KANTE_DS)" >&2
    exit 1
fi

rm -rf crates/app/qml/Kante
mkdir -p crates/app/qml
cp -r "$DS/qml/Kante" crates/app/qml/Kante
echo "$(git -C "$DS" describe --always --dirty 2>/dev/null || echo unknown) $(git -C "$DS" rev-parse --abbrev-ref HEAD 2>/dev/null || echo ?)" > crates/app/qml/Kante/.kante-version

cp "$DS/docs/v1/shrippen.css" crates/appcore/report/shrippen.css

echo "sync-kante: $(cat crates/app/qml/Kante/.kante-version) -> crates/app/qml/Kante"
