#!/usr/bin/env bash
# Publish workspace crates to https://crates.io in dependency order.
#
# Prerequisite (Cargo requirement)
# ---------------------------------
# You CANNOT publish a crate whose dependencies still use unpublished
# `path = "../some-sibling"` entries. Replace each such dependency with a
# `version = "..."` constraint that matches the crate version you publish
# (typically the `[workspace.package] version` in ./Cargo.toml).
#
# Prerequisites (credentials)
# ---------------------------
# • `cargo login` (or `$CARGO_REGISTRY_TOKEN` depending on Cargo version / CI)
# • `cargo owner` configured as needed on crates.io
#
# Version bumps (optional)
# ------------------------
# This repo uses `version.workspace = true`; the single source of truth is
# `[workspace.package]` in the workspace root `./Cargo.toml`.
#
# Replace that version explicitly (and optionally sync `doido-cli` #[command]):
#   ./scripts/publish-crates-io.sh --set-version 0.1.2 --dry-run
#
# Bump patch / minor / major on the semver core (`1.2.3`; pre-release stays attached):
#   ./scripts/publish-crates-io.sh --bump patch
#
# Only edit versions, skip `cargo publish` (e.g. to commit Cargo.toml yourself):
#   ./scripts/publish-crates-io.sh --set-version 0.1.3 --no-publish
#
# Do not rewrite the `#[command(.., version = "..")]` string in `doido-cli`:
#   SKIP_CLI_COMMAND_VERSION_SYNC=1 ./scripts/publish-crates-io.sh --bump patch
#
# Publish a single crate (must match a package name listed in this script’s PACKAGES array):
#   ./scripts/publish-crates-io.sh --crate doido-core --dry-run
#   ./scripts/publish-crates-io.sh -c doido-router-macros
#
# Skip the sleep between crates:
#   PUBLISH_DELAY_SECONDS=0 ./scripts/publish-crates-io.sh ...
#
set -euo pipefail

usage() {
  sed -n '1,/^$/p' "$0" | tail -n +2
}

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "${ROOT:-}" ]]; then
  SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
fi
readonly ROOT
readonly WORKSPACE_CARGO="${ROOT}/Cargo.toml"
readonly CLI_LIB="${ROOT}/doido-cli/src/lib.rs"

SET_VERSION=""
BUMP_KIND=""
NO_PUBLISH=0
SINGLE_CRATE=""
PUBLISH_ARGS=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h | --help)
      usage
      exit 0
      ;;
    --crate)
      [[ -z "${SINGLE_CRATE:-}" ]] || {
        echo 'error: only one --crate (or -c) may be given' >&2
        exit 1
      }
      [[ -n "${2:-}" ]] || {
        echo "error: --crate requires PACKAGE" >&2
        exit 1
      }
      SINGLE_CRATE="$2"
      shift 2
      ;;
    -c)
      [[ -z "${SINGLE_CRATE:-}" ]] || {
        echo 'error: only one --crate (or -c) may be given' >&2
        exit 1
      }
      [[ -n "${2:-}" ]] || {
        echo "error: -c requires PACKAGE" >&2
        exit 1
      }
      SINGLE_CRATE="$2"
      shift 2
      ;;
    --set-version)
      [[ -n "${2:-}" ]] || {
        echo "error: --set-version requires VERSION" >&2
        exit 1
      }
      SET_VERSION="$2"
      shift 2
      ;;
    --bump)
      [[ -n "${2:-}" ]] || {
        echo "error: --bump requires patch|minor|major" >&2
        exit 1
      }
      BUMP_KIND="$2"
      shift 2
      ;;
    --no-publish)
      NO_PUBLISH=1
      shift
      ;;
    *)
      PUBLISH_ARGS+=("$1")
      shift
      ;;
  esac
done

if [[ -n "$SET_VERSION" && -n "$BUMP_KIND" ]]; then
  echo "error: use only one of --set-version and --bump" >&2
  exit 1
fi

if [[ -n "$BUMP_KIND" && "$BUMP_KIND" != "patch" && "$BUMP_KIND" != "minor" && "$BUMP_KIND" != "major" ]]; then
  echo "error: --bump must be patch, minor, or major" >&2
  exit 1
fi

readonly WORKSPACE_REGEX='^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.+-]*)?$'

read_workspace_package_version() {
  awk '
    /^\[workspace\.package\]$/ { p = 1; next }
    p && /^version = / {
      line = $0
      sub(/^version *= *"/, "", line)
      sub(/"$/, "", line)
      print line
      exit
    }
    p && /^\[/ { p = 0 }
  ' "$WORKSPACE_CARGO"
}

apply_workspace_package_version() {
  local nv="$1"
  if [[ ! "$nv" =~ $WORKSPACE_REGEX ]]; then
    echo "error: version '${nv}' does not match expected semver form (see script WORKSPACE_REGEX)" >&2
    exit 1
  fi

  [[ -f "$WORKSPACE_CARGO" ]] || {
    echo "error: workspace Cargo.toml missing: ${WORKSPACE_CARGO}" >&2
    exit 1
  }

  local tmp
  tmp="$(mktemp)"
  awk -v nv="$nv" '
    BEGIN { replaced = 0 }
    /^\[workspace\.package\]$/ { p = 1; print; next }
    p && /^version = "/ { print "version = \"" nv "\""; replaced = 1; next }
    p && /^\[/ { p = 0; print; next }
    { print }
    END {
      if (!replaced) {
        print "never found [workspace.package] version line" > "/dev/stderr"
        exit 1
      }
    }
  ' "$WORKSPACE_CARGO" >"$tmp"
  mv "$tmp" "$WORKSPACE_CARGO"
  printf 'Updated [%s]: version = \"%s\"\n' "$WORKSPACE_CARGO" "$nv"

  if [[ "${SKIP_CLI_COMMAND_VERSION_SYNC:-0}" != "1" ]] && [[ -f "$CLI_LIB" ]]; then
    command -v python3 >/dev/null 2>&1 || {
      echo 'error: syncing `doido-cli` clap version needs python3, or set SKIP_CLI_COMMAND_VERSION_SYNC=1' >&2
      exit 1
    }
    NV="$nv" CLI_LIB_PATH="$CLI_LIB" python3 <<'PY'
import os
import pathlib
import re

nv = os.environ["NV"]
path = pathlib.Path(os.environ["CLI_LIB_PATH"])
lines = path.read_text().splitlines(keepends=True)
out = []
patched = False
for line in lines:
    if (not patched) and line.strip().startswith("#[command(") and "version" in line:
        nline = re.sub(
            r'(version\s*=\s*)"[^"]*"',
            lambda m: m.group(1) + '"' + nv + '"',
            line,
            count=1,
        )
        if nline != line:
            patched = True
            line = nline
    out.append(line)
path.write_text("".join(out))
if patched:
    print("Synced clap crate version attribute in", path)
PY
  fi
}

maybe_update_versions() {
  local target=""
  if [[ -n "$SET_VERSION" ]]; then
    target="$SET_VERSION"
  fi
  if [[ -n "$BUMP_KIND" ]]; then
    command -v python3 >/dev/null 2>&1 || {
      echo 'error: --bump requires python3 in PATH' >&2
      exit 1
    }
    local current
    current="$(read_workspace_package_version)"
    [[ -n "$current" ]] || {
      echo 'error: could not read workspace [workspace.package] version' >&2
      exit 1
    }
    target="$(python3 -c '
import re, sys

cur = sys.argv[1]
kind = sys.argv[2]

m = re.fullmatch(r"(\d+)\.(\d+)\.(\d+)(?:-(.*))?", cur)
if not m:
    print("cannot parse semver for bump: {!r}".format(cur), file=sys.stderr)
    raise SystemExit(1)
ma, mi, pa = int(m.group(1)), int(m.group(2)), int(m.group(3))
pre = m.group(4)
if kind == "patch":
    pa += 1
elif kind == "minor":
    mi += 1
    pa = 0
elif kind == "major":
    ma += 1
    mi = pa = 0
else:
    raise SystemExit(1)
core = "{}.{}.{}".format(ma, mi, pa)
print(core + ("-" + pre if pre else ""))
' "${current}" "${BUMP_KIND}")"
    printf 'Bumping workspace version: %s -> %s (%s)\n' "${current}" "${target}" "${BUMP_KIND}"
  fi

  if [[ -z "$target" ]]; then
    return 0
  fi

  apply_workspace_package_version "$target"
}

## Order matters: dependents must appear after everything they rely on inside this repo.
readonly PACKAGES=(
  doido-core
  doido-config
  doido-controller-macros
  doido-router-macros
  doido-jobs-macros
  doido-mailer-macros
  doido-cable-macros
  doido-kafka-macros
  doido-mcp-macros
  doido-router
  doido-controller
  doido-model
  doido-view
  doido-middleware
  doido-cache
  doido-jobs
  doido-mailer
  doido-cable
  doido-kafka
  doido-mcp
  doido-generators
  doido-cli
  doido
)

validate_single_publish_target() {
  local needle="$1" p
  for p in "${PACKAGES[@]}"; do
    if [[ "$p" == "$needle" ]]; then
      return 0
    fi
  done
  echo "error: unknown package '${needle}' — not managed by this script." >&2
  echo 'Use one of:' >&2
  printf '  %s\n' "${PACKAGES[@]}" >&2
  exit 1
}

cd "$ROOT"
maybe_update_versions

if [[ "$NO_PUBLISH" -eq 1 ]]; then
  echo '--no-publish: stopping after version edits.'
  exit 0
fi

if [[ -n "${SINGLE_CRATE}" ]]; then
  validate_single_publish_target "${SINGLE_CRATE}"
  printf '\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n'
  printf 'Publishing single crate: %s\n' "${SINGLE_CRATE}"
  printf '━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n'
  cargo publish --allow-dirty -p "${SINGLE_CRATE}" "${PUBLISH_ARGS[@]}"
  echo
  printf 'Finished publishing %s.\n' "${SINGLE_CRATE}"
  exit 0
fi

DELAY_RAW="${PUBLISH_DELAY_SECONDS:-45}"
TOTAL="${#PACKAGES[@]}"
LAST_INDEX=$((TOTAL - 1))

for index in "${!PACKAGES[@]}"; do
  pkg="${PACKAGES[$index]}"
  printf '\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n'
  printf '[%s / %s] Publishing %s\n' "$((index + 1))" "${TOTAL}" "$pkg"
  printf '━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n'
  cargo publish --allow-dirty -p "$pkg" "${PUBLISH_ARGS[@]}"

  if [[ "${index}" -lt "${LAST_INDEX}" && "${DELAY_RAW}" =~ ^[0-9]+$ && "${DELAY_RAW}" != "0" ]]; then
    printf '\n(waiting %ss for crates.io; set PUBLISH_DELAY_SECONDS=0 to skip)\n' "${DELAY_RAW}"
    sleep "${DELAY_RAW}"
  fi
done

echo
echo "Finished publishing ${TOTAL} crates."
