#!/bin/sh
# Installs the comtrya binary from GitHub releases. Served as https://comtrya.dev/install.sh
#
#   curl -fsSL https://comtrya.dev/install.sh | sh
#
# Environment variables:
#   VERSION      release tag to install, e.g. v0.10.0 (default: the latest release)
#   INSTALL_DIR  where to put the binary (default: /usr/local/bin, using sudo if needed)
#   GITHUB_TOKEN optional, avoids GitHub API rate limits (e.g. in CI)
set -eu

REPO="comtrya/comtrya"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

fail() {
    echo "error: $*" >&2
    exit 1
}

command -v curl >/dev/null 2>&1 || fail "curl is required"

case "$(uname -s)" in
Linux)
    case "$(uname -m)" in
    x86_64 | amd64) target="x86_64-unknown-linux-musl" ;;
    aarch64 | arm64) target="aarch64-unknown-linux-gnu" ;;
    *) fail "no prebuilt binary for Linux $(uname -m), see https://comtrya.dev/installation.html" ;;
    esac
    ;;
Darwin)
    case "$(uname -m)" in
    arm64) target="aarch64-apple-darwin" ;;
    *) target="x86_64-apple-darwin" ;;
    esac
    ;;
*)
    fail "unsupported system $(uname -s); on Windows use https://comtrya.dev/install.ps1"
    ;;
esac
asset="comtrya-$target"

if [ -n "${VERSION:-}" ]; then
    api="https://api.github.com/repos/$REPO/releases/tags/$VERSION"
else
    api="https://api.github.com/repos/$REPO/releases/latest"
fi
set -- -H "Accept: application/vnd.github+json"
if [ -n "${GITHUB_TOKEN:-}" ]; then
    set -- "$@" -H "Authorization: Bearer $GITHUB_TOKEN"
fi
release=$(curl -fsSL "$@" "$api") ||
    fail "could not find release ${VERSION:-latest} of $REPO"

version=$(printf '%s\n' "$release" | sed -n 's/^  "tag_name": "\(.*\)",$/\1/p')
[ -n "$version" ] || fail "could not read the release version"

# The SHA-256 GitHub recorded for the asset. Releases before mid-2025 have none
digest=$(printf '%s\n' "$release" | awk -v name="\"$asset\"," '
    $1 == "\"name\":" && $2 == name { found = 1 }
    found && $1 == "\"digest\":" { gsub(/[",]/, "", $2); sub(/^sha256:/, "", $2); print $2; exit }
')

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

url="https://github.com/$REPO/releases/download/$version/$asset"
echo "Downloading comtrya $version ($target)"
curl -fsSL "$url" -o "$tmp/comtrya" || fail "download of $url failed"

if [ -n "$digest" ] && [ "$digest" != "null" ]; then
    if command -v sha256sum >/dev/null 2>&1; then
        actual=$(sha256sum "$tmp/comtrya" | cut -d' ' -f1)
    else
        actual=$(shasum -a 256 "$tmp/comtrya" | cut -d' ' -f1)
    fi
    [ "$actual" = "$digest" ] || fail "checksum mismatch: expected $digest, got $actual"
    echo "Checksum verified"
else
    echo "warning: release $version has no published checksum, skipping verification" >&2
fi

chmod +x "$tmp/comtrya"

sudo=""
if ! mkdir -p "$INSTALL_DIR" 2>/dev/null || [ ! -w "$INSTALL_DIR" ]; then
    command -v sudo >/dev/null 2>&1 ||
        fail "$INSTALL_DIR is not writable; run as root or set INSTALL_DIR"
    echo "Installing to $INSTALL_DIR needs sudo"
    sudo="sudo"
    $sudo mkdir -p "$INSTALL_DIR"
fi
$sudo mv "$tmp/comtrya" "$INSTALL_DIR/comtrya"

echo "Installed $("$INSTALL_DIR/comtrya" --version) to $INSTALL_DIR/comtrya"
case ":$PATH:" in
*":$INSTALL_DIR:"*) ;;
*) echo "Add $INSTALL_DIR to your PATH to run comtrya" ;;
esac
