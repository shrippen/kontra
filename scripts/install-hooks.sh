#!/bin/sh
# Uses the hooks in scripts/hooks for this clone (W4): commit-msg checks rules 1-6 of agent.md,
# pre-push runs `make check-lite`. Undo with `git config --unset core.hooksPath`.
set -e
cd "$(git rev-parse --show-toplevel)"
git config core.hooksPath scripts/hooks
echo "hooks: $(git config core.hooksPath)"
