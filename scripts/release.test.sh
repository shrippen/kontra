#!/usr/bin/env bash
# Test of scripts/release.sh (W6) in a throwaway clone: an incomplete release is refused and
# not tagged, a complete one is tagged, and nothing is pushed (the clone has no remote).
#
#   scripts/release.test.sh
set -euo pipefail
root="$(git rev-parse --show-toplevel)"
base="${TMPDIR:-$HOME/.cache/claude}"
mkdir -p "$base"
work=$(mktemp -d -p "$base" release-test.XXXXXX)
trap 'rm -rf "$work"' EXIT

# Tags and commits of the test need a name, whoever runs it.
export GIT_AUTHOR_NAME=test GIT_AUTHOR_EMAIL=test@example.org
export GIT_COMMITTER_NAME=test GIT_COMMITTER_EMAIL=test@example.org

git clone -q --no-hardlinks "$root" "$work/repo"
cd "$work/repo"
git checkout -q -B main
git remote remove origin
# What the script reads, as it is now, also when it is not committed yet.
for f in scripts/release.sh scripts/release-notes.sh CHANGELOG.md docs/index.html \
    packaging/org.shrippen.Kontra.metainfo.xml; do
    cp "$root/$f" "$f"
done
git add -A
git commit -qm "Use the release script under test" --allow-empty

V=9.8.7
D=2026-11-09

# Nothing prepared: refused, every missing part named.
if out=$(scripts/release.sh $V 2>&1); then
    echo "FAIL: unprepared release passed"; exit 1
fi
for want in "Cargo.toml" "CHANGELOG.md" "newest release" "#changes" "footer" "not tagged"; do
    grep -qF "$want" <<< "$out" || { echo "FAIL: no '$want' in: $out"; exit 1; }
done
! git rev-parse -q --verify refs/tags/v$V > /dev/null || { echo "FAIL: tagged"; exit 1; }

# Prepared: version, changelog, metainfo, landing page.
sed -i "/^\[workspace.package\]/,/^\[/s/^version = .*/version = \"$V\"/" Cargo.toml
awk -v v="$V" '/^name = "kontra/ { print; getline; print "version = \"" v "\""; next } { print }' \
    Cargo.lock > Cargo.lock.new && mv Cargo.lock.new Cargo.lock
cat >> CHANGELOG.md <<NOTES

## [$V] — $D

### English

A test release.

### Deutsch

Eine Testversion.
NOTES
sed -i "s|<releases>|<releases>\n    <release version=\"$V\" date=\"$D\"/>|" \
    packaging/org.shrippen.Kontra.metainfo.xml
sed -i "s|<ol class=\"changelog\">|<ol class=\"changelog\"><li>$V</li>|" docs/index.html
sed -i "s|MIT · shrippen ·|MIT · shrippen · $V ·|" docs/index.html
git commit -qam "Prepare $V"

scripts/release.sh $V --dry-run
! git rev-parse -q --verify refs/tags/v$V > /dev/null || { echo "FAIL: dry run tagged"; exit 1; }
out=$(scripts/release.sh $V)
grep -qF "git push origin v$V" <<< "$out" || { echo "FAIL: no push command printed"; exit 1; }
git cat-file -p "v$V" | grep -qF "Eine Testversion." || { echo "FAIL: tag without notes"; exit 1; }

# Twice: the tag exists.
if scripts/release.sh $V > /dev/null 2>&1; then
    echo "FAIL: tagged twice"; exit 1
fi
echo "release.test.sh: ok"
