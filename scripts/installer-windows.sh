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
"$iscc" "//DAppVersion=$VERSION" "//DSourceDir=$(cygpath -aw "$DIST")" \
    "//DOutputDir=$(cygpath -aw "$OUT")" "$(cygpath -aw packaging/windows/Kontra.iss)"
test -f "$OUT/Kontra-$VERSION-windows-setup.exe"

# The portable ZIP: the folder under its own name, so unpacking makes one folder. Windows' own
# bsdtar writes it (by path: Git Bash's tar is GNU tar, which cannot write or read ZIP);
# Compress-Archive wrote backslashes into the entry names.
bsdtar="$(cygpath -u "$SYSTEMROOT")/System32/tar.exe"
zip="$OUT/Kontra-$VERSION-windows-portable.zip"
"$bsdtar" -a -c -f "$(cygpath -w "$PWD/$zip")" -C "$(cygpath -w "$(dirname "$DIST")")" "$(basename "$DIST")"
test -f "$zip"

# Agent files belong in no package (release-changelog); the licences do. Here-strings, not
# pipes: grep -q stops at the first match, and pipefail fails on the writer's SIGPIPE.
listing="$("$bsdtar" -tf "$(cygpath -w "$PWD/$zip")")"
if grep -qiE '(^|/)(CLAUDE|agent|RULES|RELEASE)\.md$' <<<"$listing"; then
    echo "installer-windows: rules files in the ZIP" >&2
    exit 1
fi
if grep -q '\\' <<<"$listing"; then
    echo "installer-windows: backslashes in the ZIP's entry names" >&2
    exit 1
fi
grep -q '/share/licenses/Kontra/GPL-3.0.txt' <<<"$listing"
grep -q '/share/licenses/Kontra/LGPL-3.0.txt' <<<"$listing"
grep -q '/share/licenses/Kontra/LGPL-2.1.txt' <<<"$listing"
ls -l "$OUT"
