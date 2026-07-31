#!/usr/bin/env bash
# Build doido-generators from its packaged tarball (no sibling workspace crates),
# run `doido new`, and assert the generated app uses crates.io version deps.
#
# Expects `cargo package -p doido-generators` to succeed (run after publish-dry-run
# or standalone). Skipped when packaging fails.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

need_cmd() {
	command -v "$1" >/dev/null 2>&1 || {
		echo "error: required command not found: $1" >&2
		exit 1
	}
}

need_cmd cargo
need_cmd tar

WORK="$(mktemp -d)"
cleanup() {
	rm -rf "$WORK"
}
trap cleanup EXIT

PKG_DIR="$ROOT/target/package"
mkdir -p "$PKG_DIR"

echo "==> packaging doido-generators"
cargo package -p doido-generators --allow-dirty --no-verify --quiet

CRATE_VERSION="$(sed -nE 's/^version[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/p' "$ROOT/Cargo.toml" | head -1)"
TARBALL="$PKG_DIR/doido-generators-${CRATE_VERSION}.crate"
test -f "$TARBALL" || {
	echo "error: missing packaged crate at $TARBALL" >&2
	exit 1
}

EXTRACT="$WORK/extracted"
mkdir -p "$EXTRACT"
tar -xf "$TARBALL" -C "$EXTRACT"
SRC="$EXTRACT/doido-generators-${CRATE_VERSION}"

echo "==> building doido-generators from packaged sources"
CARGO_TARGET_DIR="$WORK/target" cargo build --release --manifest-path "$SRC/Cargo.toml" --quiet

BIN="$WORK/target/release/doido-generators"
APP_DIR="$WORK/app-root"
mkdir -p "$APP_DIR"

echo "==> generating smoke app from packaged binary"
(cd "$APP_DIR" && "$BIN" new smoke-app --non-interactive --cache=redis)

CARGO_TOML="$APP_DIR/smoke-app/Cargo.toml"
test -f "$CARGO_TOML" || {
	echo "error: expected generated Cargo.toml at $CARGO_TOML" >&2
	exit 1
}

if grep -q 'path =' "$CARGO_TOML"; then
	echo "error: packaged generator emitted path dependencies:" >&2
	grep 'path =' "$CARGO_TOML" >&2 || true
	exit 1
fi

if ! grep -q "version = \"${CRATE_VERSION}\"" "$CARGO_TOML"; then
	echo "error: generated Cargo.toml missing version = \"${CRATE_VERSION}\"" >&2
	exit 1
fi

if ! grep -q 'cache-redis' "$CARGO_TOML"; then
	echo "error: generated Cargo.toml missing cache-redis feature wiring" >&2
	exit 1
fi

echo "==> packaged generator emits crates.io version dependencies (v${CRATE_VERSION})"
