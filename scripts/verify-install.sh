#!/usr/bin/env bash
# Harness check: build a local release binary, serve it like a GitHub Release,
# run scripts/install.sh against it, and assert the installed `doido` works.
#
# Skipped automatically on Windows (install.ps1 is validated separately in CI).

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

case "$(uname -s)" in
MINGW* | MSYS* | CYGWIN* | Windows*)
	echo "skip: verify-install.sh (Unix only; Windows covered by release workflow)"
	exit 0
	;;
esac

need_cmd() {
	command -v "$1" >/dev/null 2>&1 || {
		echo "error: required command not found: $1" >&2
		exit 1
	}
}

need_cmd cargo
need_cmd curl
need_cmd bash

HOST_TARGET="$(rustc -vV | sed -n 's/^host: //p')"
[ -n "$HOST_TARGET" ] || {
	echo "error: could not detect rustc host triple" >&2
	exit 1
}

WORK="$(mktemp -d)"
INSTALL_DIR="${WORK}/install/bin"
RELEASE_DIR="${WORK}/release"
PORT=""
SERVER_PID=""

cleanup() {
	if [ -n "$SERVER_PID" ]; then
		kill "$SERVER_PID" 2>/dev/null || true
		wait "$SERVER_PID" 2>/dev/null || true
	fi
	rm -rf "$WORK"
}
trap cleanup EXIT

echo "==> building doido (${HOST_TARGET}, release)"
cargo build --release --bin doido --target "$HOST_TARGET" >/dev/null

mkdir -p "$RELEASE_DIR" "$INSTALL_DIR"
cp "target/${HOST_TARGET}/release/doido" "${RELEASE_DIR}/doido-${HOST_TARGET}"
chmod +x "${RELEASE_DIR}/doido-${HOST_TARGET}"

if command -v python3 >/dev/null 2>&1; then
	PORT="$(python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
)"
	cd "$RELEASE_DIR"
	python3 -m http.server "$PORT" >/dev/null 2>&1 &
	SERVER_PID=$!
	cd "$ROOT"
	sleep 0.5
	DOWNLOAD_BASE="http://127.0.0.1:${PORT}"
else
	DOWNLOAD_BASE="file://${RELEASE_DIR}"
fi

echo "==> running install.sh (DOIDO_VERSION=test DOIDO_DOWNLOAD_BASE=${DOWNLOAD_BASE})"
DOIDO_VERSION=test \
	DOIDO_INSTALL_DIR="$INSTALL_DIR" \
	DOIDO_DOWNLOAD_BASE="$DOWNLOAD_BASE" \
	bash scripts/install.sh

INSTALLED="${INSTALL_DIR}/doido"
[ -x "$INSTALLED" ] || {
	echo "error: installer did not create executable at ${INSTALLED}" >&2
	exit 1
}

echo "==> verifying installed binary"
"$INSTALLED" --help >/dev/null
"$INSTALLED" --version >/dev/null

echo "==> verify-install: OK"
