#!/usr/bin/env bash
# gf (gather-files) installer
# Usage: curl -LsSf https://gf.bfoos.net/install.sh | bash
#
# This script downloads and installs the gf binary for your platform.

set -euo pipefail

REPO="BrianSigafoos/gather-files"
BINARY_NAME="gf"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

info() {
    echo -e "${BLUE}info:${NC} $1"
}

warn() {
    echo -e "${YELLOW}warn:${NC} $1"
}

error() {
    echo -e "${RED}error:${NC} $1" >&2
    exit 1
}

success() {
    echo -e "${GREEN}success:${NC} $1"
}

# Detect OS
detect_os() {
    case "$(uname -s)" in
        Darwin*)
            echo "darwin"
            ;;
        Linux*)
            echo "linux"
            ;;
        MINGW*|MSYS*|CYGWIN*)
            echo "windows"
            ;;
        *)
            error "Unsupported operating system: $(uname -s)"
            ;;
    esac
}

# Detect architecture
detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)
            echo "x86_64"
            ;;
        arm64|aarch64)
            echo "aarch64"
            ;;
        *)
            error "Unsupported architecture: $(uname -m)"
            ;;
    esac
}

# Get the latest release tag from GitHub
get_latest_version() {
    local latest
    latest=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')
    if [ -z "$latest" ]; then
        error "Could not determine latest version. Check https://github.com/${REPO}/releases"
    fi
    echo "$latest"
}

# Determine install directory
get_install_dir() {
    if [ -d "$HOME/.cargo/bin" ]; then
        echo "$HOME/.cargo/bin"
    elif [ -d "$HOME/.local/bin" ]; then
        echo "$HOME/.local/bin"
    else
        mkdir -p "$HOME/.local/bin"
        echo "$HOME/.local/bin"
    fi
}

TMP_DIR=""

main() {
    echo ""
    echo "  ╭─────────────────────────────────────────╮"
    echo "  │           gf installer                  │"
    echo "  ╰─────────────────────────────────────────╯"
    echo ""

    local os arch version install_dir target download_url

    os=$(detect_os)
    arch=$(detect_arch)

    info "Detected platform: ${arch}-${os}"

    case "$os" in
        darwin)
            target="${arch}-apple-darwin"
            ;;
        linux)
            target="${arch}-unknown-linux-gnu"
            ;;
        *)
            error "Prebuilt binaries not available for ${os}. Please build from source."
            ;;
    esac

    if [[ "$os" == "darwin" ]] && [[ "$arch" != "aarch64" && "$arch" != "x86_64" ]]; then
        error "Unsupported macOS architecture: $arch"
    fi

    version=${GF_VERSION:-$(get_latest_version)}
    info "Installing gf ${version}"

    download_url="https://github.com/${REPO}/releases/download/${version}/${BINARY_NAME}-${target}.tar.gz"
    info "Downloading from: $download_url"

    TMP_DIR=$(mktemp -d)
    trap 'rm -rf "$TMP_DIR"' EXIT

    if ! curl -fsSL "$download_url" -o "$TMP_DIR/gf.tar.gz"; then
        error "Failed to download gf. Check that version ${version} exists at https://github.com/${REPO}/releases"
    fi

    tar -xzf "$TMP_DIR/gf.tar.gz" -C "$TMP_DIR"

    install_dir=$(get_install_dir)
    info "Installing to: $install_dir"

    mv "$TMP_DIR/${BINARY_NAME}" "$install_dir/${BINARY_NAME}"
    chmod +x "$install_dir/${BINARY_NAME}"

    success "gf ${version} installed successfully!"
    echo ""

    if [[ ":$PATH:" != *":$install_dir:"* ]]; then
        warn "$install_dir is not in your PATH"
        echo ""
        echo "Add it to your shell config:"
        echo ""
        echo "  export PATH=\"$install_dir:\$PATH\""
        echo ""
    fi

    echo "Usage:"
    echo "  gf               # gather repo root"
    echo "  gf docs          # gather preset docs"
    echo "  gf ./app         # gather specific path"
    echo ""
}

main "$@"
