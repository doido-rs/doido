#!/usr/bin/env bash
# Restore Cargo.toml files produced by scripts/pack-cargo-manifests.sh.
set -euo pipefail

ARCHIVE="${1:?archive path required}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

tar xzf "${ARCHIVE}"
echo "==> restored manifests from ${ARCHIVE}"
