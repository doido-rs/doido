#!/usr/bin/env bash
# Tar every Cargo.toml in the repo (excluding target/) for CI artifact handoff.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

OUT="${1:-cargo-manifests.tar.gz}"
find . -name Cargo.toml ! -path './target/*' -print0 | tar czf "${OUT}" --null -T -
echo "==> packed $(tar tzf "${OUT}" | wc -l) manifest(s) into ${OUT}"
