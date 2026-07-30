#!/usr/bin/env bash
# Print the workspace version from [workspace.package] in ./Cargo.toml.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
sed -nE 's/^version[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/p' "${ROOT}/Cargo.toml" | head -1
