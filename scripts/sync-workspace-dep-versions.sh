#!/usr/bin/env bash
# Stamp exact `version = "=X.Y.Z"` on first-party `[workspace.dependencies]` path
# entries from `[workspace.package].version`. Member crates use `*.workspace = true`
# only; Cargo still requires an explicit version when publishing.
#
# Usage:
#   scripts/sync-workspace-dep-versions.sh          # rewrite root Cargo.toml
#   scripts/sync-workspace-dep-versions.sh --check  # exit 1 if out of sync
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CARGO="${ROOT}/Cargo.toml"
CHECK=0
if [ "${1:-}" = "--check" ]; then
	CHECK=1
fi

VER="$(sed -nE 's/^version[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/p' "$CARGO" | head -1)"
if [ -z "$VER" ]; then
	echo "error: could not read [workspace.package].version from $CARGO" >&2
	exit 1
fi

python3 - "$CARGO" "$VER" "$CHECK" <<'PY'
import re
import sys
from pathlib import Path

path = Path(sys.argv[1])
ver = sys.argv[2]
check = int(sys.argv[3])

lines = path.read_text().splitlines()
in_ws_deps = False
need = f'version = "={ver}"'
mismatch = False


def stamped(line: str) -> str:
    cleaned = re.sub(r",\s*version\s*=\s*\"[^\"]*\"", "", line)
    mobj = re.match(r"^(\s*doido[\w-]*\s*=\s*\{)(.*)(\}\s*)$", cleaned)
    if not mobj:
        return line
    inner = re.sub(r"\s+", " ", mobj.group(2).strip().rstrip(","))
    return f"{mobj.group(1)} {inner}, {need} {mobj.group(3)}"


out: list[str] = []
for line in lines:
    if line.strip() == "[workspace.dependencies]":
        in_ws_deps = True
        out.append(line)
        continue
    if in_ws_deps and line.startswith("[") and not line.startswith("[workspace.dependencies]"):
        in_ws_deps = False
    if in_ws_deps and re.match(r"^doido[\w-]*\s*=", line) and "path =" in line:
        next_line = stamped(line)
        if check:
            if need not in line:
                mismatch = True
            out.append(line)
        else:
            out.append(next_line)
    else:
        out.append(line)

if check:
    if mismatch:
        print(
            f'error: first-party [workspace.dependencies] version != "={ver}" '
            f"(run scripts/sync-workspace-dep-versions.sh)",
            file=sys.stderr,
        )
        sys.exit(1)
    print(f"==> workspace dependency versions match {ver}")
else:
    path.write_text("\n".join(out) + "\n")
    print(f"==> synced first-party [workspace.dependencies] to ={ver}")
PY
