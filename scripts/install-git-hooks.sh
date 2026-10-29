#!/usr/bin/env bash
# Point this repo at .githooks/ so pre-commit runs fmt + clippy checks.
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
cd "$root"

if [[ ! -d .git ]]; then
  echo "error: not a git repository" >&2
  exit 1
fi

chmod +x .githooks/pre-commit
git config core.hooksPath .githooks

echo "Git hooks installed (core.hooksPath=.githooks)."
echo "Pre-commit runs: cargo fmt --check --all && cargo clippy --workspace -- -D warnings"
echo "Same as: make check"
