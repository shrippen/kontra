#!/usr/bin/env bash
# The two downloads of Kontra for Windows (Win5): the installer (Inno Setup, per user, no
# administrator rights) and the portable ZIP, both made from the folder of package-windows.sh.
# Runs in Git Bash on a Windows machine; GitHub Actions calls it, nobody ships its output by hand.
#
#   scripts/installer-windows.sh [DIST [OUT]]    (defaults dist/Kontra and dist/release)
#     OUT/Kontra-<version>-windows-setup.exe
#     OUT/Kontra-<version>-windows-portable.zip
set -euo pipefail
cd "$(dirname "$0")/.."

DIST="${1:-dist/Kontra}"
OUT="${2:-dist/release}"
test -f "$DIST/bin/kontra.exe"

# The version of the workspace: the installer, the ZIP and the program say the same.
VERSION="$(sed -n '/^\[workspace.package\]/,/^\[/{s/^version = "\(.*\)"/\1/p}' Cargo.toml)"
test -n "$VERSION"

# Inno Setup 6: on the PATH, in its usual place, or from Chocolatey (a runner without it).
iscc="$(command -v iscc || true)"
for dir in "C:/Program Files (x86)/Inno Setup 6" "C:/Program Files/Inno Setup 6"; do
    if [ -z "$iscc" ] && [ -x "$dir/ISCC.exe" ]; then
        iscc="$dir/ISCC.exe"
    fi
done
if [ -z "$iscc" ]; then
    choco install innosetup --no-progress -y
    iscc="C:/Program Files (x86)/Inno Setup 6/ISCC.exe"
fi

rm -rf "$OUT"
mkdir -p "$OUT"
# Windows paths for iscc; MSYS would turn a leading "/D" into a path, hence the doubled slash.
"$iscc" "//DAppVersion=$VERSION" "//DSourceDir=$(cygpath -w "$DIST")" \
    "//DOutputDir=$(cygpath -w "$OUT")" "$(cygpath -w packaging/windows/Kontra.iss)"
test -f "$OUT/Kontra-$VERSION-windows-setup.exe"

# The portable ZIP: the folder under its own name, so unpacking makes one folder.
zip_path="$(cygpath -w "$PWD/$OUT/Kontra-$VERSION-windows-portable.zip")"
powershell -NoProfile -Command \
    "Compress-Archive -Path '$(cygpath -w "$DIST")' -DestinationPath '$zip_path'"
test -f "$OUT/Kontra-$VERSION-windows-portable.zip"

# Agent files belong in no package (release-changelog); the licences do.
listing="$(tar -tf "$OUT/Kontra-$VERSION-windows-portable.zip")"
if echo "$listing" | grep -qiE '(^|/)(CLAUDE|agent|RULES|RELEASE)\.md$'; then
    echo "installer-windows: rules files in the ZIP" >&2
    exit 1
fi
echo "$listing" | grep -q 'share/licenses/Kontra/GPL-3.0.txt'

ls -l "$OUT"
