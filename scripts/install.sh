#!/usr/bin/env bash
# Install the `doido` CLI from a GitHub Release binary.
#
# Usage:
#   curl -fsSL https://github.com/doido-rs/doido/releases/latest/download/install.sh | bash
#   curl -fsSL .../install.sh | DOIDO_VERSION=0.0.9 bash
#
# Environment:
#   DOIDO_VERSION       Semver without leading "v" (default: latest release)
#   DOIDO_INSTALL_DIR   Install directory (default: $HOME/.local/bin)
#   DOIDO_GITHUB_REPO   owner/repo override (default: doido-rs/doido)
#   DOIDO_DOWNLOAD_BASE Override download base URL (for local/harness testing)

set -euo pipefail

DOIDO_GITHUB_REPO="${DOIDO_GITHUB_REPO:-doido-rs/doido}"
DOIDO_INSTALL_DIR="${DOIDO_INSTALL_DIR:-${HOME}/.local/bin}"
DOIDO_VERSION="${DOIDO_VERSION:-}"

info() { printf '==> %s\n' "$*"; }
warn() { printf 'warning: %s\n' "$*" >&2; }
die() { printf 'error: %s\n' "$*" >&2; exit 1; }

need_cmd() {
	command -v "$1" >/dev/null 2>&1 || die "required command not found: $1"
}

detect_target() {
	local os arch
	os="$(uname -s)"
	arch="$(uname -m)"

	case "$os" in
	Linux)
		case "$arch" in
		x86_64 | amd64) echo "x86_64-unknown-linux-gnu" ;;
		aarch64 | arm64) echo "aarch64-unknown-linux-gnu" ;;
		*) die "unsupported Linux architecture: $arch" ;;
		esac
		;;
	Darwin)
		case "$arch" in
		x86_64) echo "x86_64-apple-darwin" ;;
		arm64 | aarch64) echo "aarch64-apple-darwin" ;;
		*) die "unsupported macOS architecture: $arch" ;;
		esac
		;;
	*)
		die "unsupported operating system: $os (use scripts/install.ps1 on Windows)"
		;;
	esac
}

fetch_latest_version() {
	need_cmd curl
	curl -fsSL "https://api.github.com/repos/${DOIDO_GITHUB_REPO}/releases/latest" \
		| sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"v\?\([^"]*\)".*/\1/p' \
		| head -1
}

download_url() {
	local target="$1"
	local version="$2"
	local base="${DOIDO_DOWNLOAD_BASE:-https://github.com/${DOIDO_GITHUB_REPO}/releases/download/v${version}}"
	printf '%s/doido-%s' "$base" "$target"
}

install_binary() {
	local url="$1"
	local dest="$2"
	local tmp

	need_cmd curl
	need_cmd install

	tmp="$(mktemp)"
	info "downloading ${url}"
	curl -fsSL "$url" -o "$tmp"
	chmod +x "$tmp"
	mkdir -p "$(dirname "$dest")"
	install -m 755 "$tmp" "$dest"
	rm -f "$tmp"
}

path_hint() {
	local dir="$1"
	case ":${PATH}:" in
	*":${dir}:"*) ;;
	*)
		printf '\nwarning: %s is not on your PATH\n\n' "$dir" >&2
		printf 'Add it to your shell profile:\n\n'
		printf '  export PATH="%s:$PATH"\n\n' "$dir"
		;;
	esac
}

main() {
	local target version url dest

	target="$(detect_target)"
	info "detected target: ${target}"

	if [ -z "$DOIDO_VERSION" ]; then
		if [ -n "${DOIDO_DOWNLOAD_BASE:-}" ]; then
			die "DOIDO_DOWNLOAD_BASE is set; you must also set DOIDO_VERSION"
		fi
		info "resolving latest release"
		DOIDO_VERSION="$(fetch_latest_version)"
		[ -n "$DOIDO_VERSION" ] || die "could not resolve latest release version"
	fi
	version="$DOIDO_VERSION"
	info "installing doido ${version}"

	url="$(download_url "$target" "$version")"
	dest="${DOIDO_INSTALL_DIR%/}/doido"

	install_binary "$url" "$dest"
	info "installed ${dest}"

	if "$dest" --help >/dev/null 2>&1; then
		info "doido is ready ($( "$dest" --version 2>/dev/null || echo "ok" ))"
	else
		warn "installed binary did not respond to --help; check ${dest}"
	fi

	path_hint "$DOIDO_INSTALL_DIR"
}

main "$@"
