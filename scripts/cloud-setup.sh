#!/usr/bin/env bash
# Setup script for the Claude Code cloud environment (environment settings →
# Setup script). Prepares a fresh container for Kontra and Kante work:
#
#   - system libraries (ALSA headers for cpal), Rust stable with clippy/rustfmt,
#     cargo-deny
#   - tea (Gitea CLI) logged in to git.arianw.de, git credentials for it
#   - /home/user/Kante and /home/user/kontra cloned (Kontra needs ../Kante for
#     scripts/sync-kante.sh, shrippen.github.io needs ../Kante for build.sh)
#   - Qt 6.10 and Kirigami 6.30 under /opt via Kontra's scripts/setup-qt-env.sh
#
# Needs GITEA_TOKEN in the environment (a Gitea access token with
# write:repository, write:issue and read:user; both repos are private).
# Without it only the toolchain is installed. Idempotent; runs as root.
set -uo pipefail

HOME_DIR=/home/user
GITEA=git.arianw.de
TEA_VERSION=0.16.0
log() { printf '\n== %s\n' "$*"; }

log "system packages"
DEBIAN_FRONTEND=noninteractive apt-get update -qq \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y -qq --no-install-recommends \
        libasound2-dev pkg-config build-essential curl git xz-utils

log "rust"
if ! command -v rustup >/dev/null 2>&1; then
    curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal -c rustfmt,clippy
fi
export PATH="$HOME/.cargo/bin:$PATH"
rustup update stable -q >/dev/null 2>&1 || true
rustup component add rustfmt clippy >/dev/null 2>&1 || true
command -v cargo-deny >/dev/null 2>&1 || cargo install -q --locked cargo-deny

log "tea ${TEA_VERSION}"
if ! command -v tea >/dev/null 2>&1; then
    curl -sSfL -o /usr/local/bin/tea \
        "https://dl.gitea.com/tea/${TEA_VERSION}/tea-${TEA_VERSION}-linux-amd64" \
        && chmod +x /usr/local/bin/tea
fi

if [ -z "${GITEA_TOKEN:-}" ]; then
    log "GITEA_TOKEN not set: skipping login, clones and Qt"
    exit 0
fi

log "gitea login"
tea login list 2>/dev/null | grep -q "$GITEA" \
    || tea login add --name arianw --url "https://$GITEA" --token "$GITEA_TOKEN" >/dev/null
# git reads the token from the environment at use time; nothing is written to disk.
git config --global "credential.https://$GITEA.helper" \
    '!f() { echo username=shrippen; echo "password=$GITEA_TOKEN"; }; f'
git config --global user.name  >/dev/null || git config --global user.name "Claude"
git config --global user.email >/dev/null || git config --global user.email "noreply@anthropic.com"

log "repositories"
clone() {
    local repo="$1" dir="$2"
    if [ -d "$dir/.git" ]; then
        git -C "$dir" fetch -q origin
    else
        git clone -q "https://$GITEA/shrippen/$repo.git" "$dir"
    fi
}
clone Kante  "$HOME_DIR/Kante"
clone kontra "$HOME_DIR/kontra"

log "Qt and Kirigami"
bash "$HOME_DIR/kontra/scripts/setup-qt-env.sh" || echo "Qt setup failed; rerun scripts/setup-qt-env.sh"

log "done"
