#!/bin/sh
# nosce installer - https://github.com/julienandreu/nosce
# Usage: curl -fsSL https://raw.githubusercontent.com/julienandreu/nosce/main/install.sh | sh
set -e

REPO="julienandreu/nosce"
BINARY_NAME="nosce"
INSTALL_DIR="${NOSCE_INSTALL_DIR:-$HOME/.local/bin}"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()  { printf "${GREEN}[INFO]${NC} %s\n" "$1"; }
warn()  { printf "${YELLOW}[WARN]${NC} %s\n" "$1"; }
error() { printf "${RED}[ERROR]${NC} %s\n" "$1"; exit 1; }

detect_os() {
    case "$(uname -s)" in
        Linux*)  OS="linux" ;;
        Darwin*) OS="darwin" ;;
        *)       error "Unsupported operating system: $(uname -s)" ;;
    esac
}

detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)  ARCH="x86_64" ;;
        arm64|aarch64) ARCH="aarch64" ;;
        *)             error "Unsupported architecture: $(uname -m)" ;;
    esac
}

get_target() {
    case "$OS" in
        linux)
            case "$ARCH" in
                x86_64)  TARGET="x86_64-unknown-linux-musl" ;;
                aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
            esac
            ;;
        darwin)
            TARGET="${ARCH}-apple-darwin"
            ;;
    esac
}

fetch_manifest() {
    MANIFEST_URL="https://github.com/${REPO}/releases/latest/download/latest.json"
    info "Fetching release manifest..."

    MANIFEST="$(curl -fsSL "$MANIFEST_URL")" || error "Failed to fetch latest.json"

    VERSION="$(echo "$MANIFEST" | grep -o '"version"[[:space:]]*:[[:space:]]*"[^"]*"' | head -1 | sed 's/.*"\([^"]*\)"$/\1/')"
    [ -z "$VERSION" ] && error "Failed to parse version from manifest"

    ASSET_NAME="$(echo "$MANIFEST" | grep -A3 "\"$TARGET\"" | grep '"name"' | head -1 | sed 's/.*"\([^"]*\)"$/\1/')"
    [ -z "$ASSET_NAME" ] && error "No asset found for target: $TARGET"

    EXPECTED_SHA256="$(echo "$MANIFEST" | grep -A3 "\"$TARGET\"" | grep '"sha256"' | head -1 | sed 's/.*"\([^"]*\)"$/\1/')"
}

install() {
    info "Detected: $OS $ARCH"
    info "Target:   $TARGET"
    info "Version:  $VERSION"

    DOWNLOAD_URL="https://github.com/${REPO}/releases/latest/download/${ASSET_NAME}"
    TEMP_DIR="$(mktemp -d)"
    ARCHIVE="${TEMP_DIR}/${BINARY_NAME}.tar.gz"

    info "Downloading ${ASSET_NAME}..."
    curl -fsSL "$DOWNLOAD_URL" -o "$ARCHIVE" || error "Failed to download binary"

    if [ -n "$EXPECTED_SHA256" ]; then
        info "Verifying checksum..."
        if command -v sha256sum >/dev/null 2>&1; then
            ACTUAL_SHA256="$(sha256sum "$ARCHIVE" | awk '{print $1}')"
        elif command -v shasum >/dev/null 2>&1; then
            ACTUAL_SHA256="$(shasum -a 256 "$ARCHIVE" | awk '{print $1}')"
        else
            warn "No sha256 tool found, skipping checksum verification"
            ACTUAL_SHA256="$EXPECTED_SHA256"
        fi

        if [ "$ACTUAL_SHA256" != "$EXPECTED_SHA256" ]; then
            rm -rf "$TEMP_DIR"
            error "Checksum mismatch: expected $EXPECTED_SHA256, got $ACTUAL_SHA256"
        fi
    fi

    info "Extracting..."
    tar -xzf "$ARCHIVE" -C "$TEMP_DIR"

    mkdir -p "$INSTALL_DIR"
    mv "${TEMP_DIR}/${BINARY_NAME}" "${INSTALL_DIR}/"
    chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

    rm -rf "$TEMP_DIR"
    info "Installed ${BINARY_NAME} to ${INSTALL_DIR}/${BINARY_NAME}"
}

verify() {
    if command -v "$BINARY_NAME" >/dev/null 2>&1; then
        info "Verification: $("$BINARY_NAME" --version)"
    else
        warn "Binary installed but not in PATH. Add to your shell profile:"
        warn "  export PATH=\"\$HOME/.local/bin:\$PATH\""
    fi
}

main() {
    info "Installing ${BINARY_NAME}..."
    detect_os
    detect_arch
    get_target
    fetch_manifest
    install
    verify
    echo ""
    info "Installation complete! Run '${BINARY_NAME} --help' to get started."
    info "To update later, run: ${BINARY_NAME} update"
}

main
