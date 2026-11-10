#!/usr/bin/env bash
# Checks a release and tags it locally (W6, release steps in RELEASE.md). Never pushes or
# publishes: it prints the commands for that, the owner confirms a release in words first.
#
#   scripts/release.sh X.Y.Z [--dry-run]   check everything; tag vX.Y.Z unless --dry-run
#
# Checks, in this order (each failure is listed, nothing is tagged):
#   on main, tree clean, tag free
#   version in Cargo.toml and Cargo.lock
#   CHANGELOG.md: headings, section "## [X.Y.Z] — date" with "### English" and "### Deutsch"
#   metainfo: newest <release version="X.Y.Z" date="date">, valid (appstreamcli, if installed)
#   landing page: X.Y.Z in #changes and in the footer
set -uo pipefail
cd "$(git rev-parse --show-toplevel)" || exit 2

METAINFO=packaging/org.shrippen.Kontra.metainfo.xml
LANDING=docs/index.html
CHANGELOG=CHANGELOG.md
MAIN=main
LANGS=("### English" "### Deutsch")

version="${1:-}"
mode="${2:-tag}"
if ! [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || ! [[ "$mode" =~ ^(tag|--dry-run)$ ]]; then
    echo "usage: scripts/release.sh X.Y.Z [--dry-run]" >&2
    exit 2
fi
tag="v$version"

failed=0
fail() {
    echo "release: $*" >&2
    failed=1
}

# Repository state: a release is cut from a clean main, once.
[ "$(git rev-parse --abbrev-ref HEAD)" = "$MAIN" ] || fail "not on $MAIN"
[ -z "$(git status --porcelain)" ] || fail "uncommitted changes"
if git rev-parse -q --verify "refs/tags/$tag" > /dev/null; then
    fail "tag $tag exists"
fi

# Version: the workspace and the lock file agree with the release.
cargo_version=$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"/\1/p' Cargo.toml)
[ "$cargo_version" = "$version" ] || fail "Cargo.toml has version $cargo_version"
lock_version=$(awk '/^name = "kontra-app"$/ { getline; gsub(/version = |"/, ""); print }' Cargo.lock)
[ "$lock_version" = "$version" ] || fail "Cargo.lock has kontra-app $lock_version (cargo update -w)"

# Changelog: the section is the release notes, in every language of the landing page.
date=""
notes=""
if scripts/release-notes.sh --check "$CHANGELOG" > /dev/null; then
    if notes=$(scripts/release-notes.sh "$version" "$CHANGELOG" 2> /dev/null); then
        date=$(grep -m1 "^## \[$version\] — " "$CHANGELOG" | sed 's/.* — //')
        for lang in "${LANGS[@]}"; do
            grep -qxF "$lang" <<< "$notes" || fail "$CHANGELOG $version: no \"$lang\""
        done
    else
        fail "$CHANGELOG: no section \"## [$version] — YYYY-MM-DD\""
    fi
else
    fail "$CHANGELOG headings (scripts/release-notes.sh --check)"
fi

# Metainfo: the newest release entry is this one, with the changelog's date.
newest=$(grep -m1 -o '<release [^>]*>' "$METAINFO" || true)
expected="<release version=\"$version\" date=\"$date\""
[[ "$newest" == "$expected"* ]] || fail "$METAINFO: newest release is '${newest:-none}', want '$expected>'"
if command -v appstreamcli > /dev/null; then
    appstreamcli validate --no-net --explain "$METAINFO" > /dev/null || fail "$METAINFO invalid (appstreamcli validate)"
else
    echo "release: appstreamcli not installed, metainfo not validated" >&2
fi

# Landing page: a release is done when the page shows it.
changes=$(sed -n '/id="changes"/,/<\/section>/p' "$LANDING")
grep -qF "$version" <<< "$changes" || fail "$LANDING: no entry for $version in #changes"
footer=$(sed -n '/<footer/,/<\/footer>/p' "$LANDING")
grep -qF "$version" <<< "$footer" || fail "$LANDING: footer does not show $version"

if [ "$failed" = 1 ]; then
    echo "release: $tag not tagged" >&2
    exit 1
fi
if [ "$mode" = "--dry-run" ]; then
    echo "release: $tag ready (dry run, not tagged)"
    exit 0
fi

git tag -a "$tag" -F - <<< "Kontra $version

$notes" || exit 1
echo "release: tagged $tag. After the owner's go, publish with:"
echo "  git push origin $tag"
echo "  scripts/release-notes.sh $version > notes.md"
echo "  tea release create --repo shrippen/Kontra --tag $tag --title 'Kontra $version' --note-file notes.md"
