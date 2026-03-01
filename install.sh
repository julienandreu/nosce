#!/bin/sh
# install.sh — Download and install the nosce binary.
# Usage: curl -fsSL https://raw.githubusercontent.com/julienandreu/nosce/main/install.sh | sh
set -eu

REPO="julienandreu/nosce"
BINARY="nosce"
INSTALL_DIR="${NOSCE_INSTALL_DIR:-$HOME/.local/bin}"

# -- Colors (disabled if not a terminal) --------------------------------------
if [ -t 1 ]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    CYAN='\033[0;36m'
    BOLD='\033[1m'
    RESET='\033[0m'
else
    RED='' GREEN='' YELLOW='' CYAN='' BOLD='' RESET=''
fi

info()  { printf "%b[info]%b  %s\n" "$CYAN"  "$RESET" "$1"; }
ok()    { printf "%b[ok]%b    %s\n" "$GREEN" "$RESET" "$1"; }
warn()  { printf "%b[warn]%b  %s\n" "$YELLOW" "$RESET" "$1"; }
error() { printf "%b[error]%b %s\n" "$RED"   "$RESET" "$1"; exit 1; }

# -- Detect OS and architecture -----------------------------------------------
detect_platform() {
    OS="$(uname -s)"
    ARCH="$(uname -m)"

    case "$OS" in
        Linux*)  OS="linux" ;;
        Darwin*) OS="darwin" ;;
        *)       error "Unsupported OS: $OS" ;;
    esac

    case "$ARCH" in
        x86_64|amd64)   ARCH="x86_64" ;;
        aarch64|arm64)  ARCH="aarch64" ;;
        *)              error "Unsupported architecture: $ARCH" ;;
    esac
}

# -- Resolve latest release tag -----------------------------------------------
get_latest_version() {
    if command -v curl >/dev/null 2>&1; then
        VERSION="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | sed 's/.*"tag_name": *"//;s/".*//')"
    elif command -v wget >/dev/null 2>&1; then
        VERSION="$(wget -qO- "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | sed 's/.*"tag_name": *"//;s/".*//')"
    else
        error "Neither curl nor wget found. Install one and retry."
    fi

    [ -z "$VERSION" ] && error "Could not determine latest release version."
}

# -- Download and install ------------------------------------------------------
install() {
    ARCHIVE="${BINARY}-${VERSION}-${ARCH}-${OS}.tar.gz"
    URL="https://github.com/$REPO/releases/download/$VERSION/$ARCHIVE"

    info "Downloading $BINARY $VERSION for $OS/$ARCH..."

    TMPDIR="$(mktemp -d)"
    trap 'rm -rf "$TMPDIR"' EXIT

    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$URL" -o "$TMPDIR/$ARCHIVE" || error "Download failed. Check that release $VERSION has a binary for $OS/$ARCH."
    else
        wget -qO "$TMPDIR/$ARCHIVE" "$URL" || error "Download failed. Check that release $VERSION has a binary for $OS/$ARCH."
    fi

    tar -xzf "$TMPDIR/$ARCHIVE" -C "$TMPDIR"

    mkdir -p "$INSTALL_DIR"
    mv "$TMPDIR/$BINARY" "$INSTALL_DIR/$BINARY"
    chmod +x "$INSTALL_DIR/$BINARY"

    ok "Installed $BINARY to $INSTALL_DIR/$BINARY"
}

# -- Post-install checks ------------------------------------------------------
post_install() {
    case ":$PATH:" in
        *":$INSTALL_DIR:"*) ;;
        *)
            warn "$INSTALL_DIR is not in your PATH."
            printf "\n  Add it to your shell profile:\n\n"
            printf "    %bexport PATH=\"%s:\$PATH\"%b\n\n" "$BOLD" "$INSTALL_DIR" "$RESET"
            ;;
    esac

    if command -v "$BINARY" >/dev/null 2>&1; then
        ok "$("$BINARY" --version)"
    else
        warn "Run ${BOLD}export PATH=\"$INSTALL_DIR:\$PATH\"${RESET} then ${BOLD}nosce --version${RESET} to verify."
    fi
}

# -- Main ----------------------------------------------------------------------
main() {
    printf "\n%b%s installer%b\n\n" "$BOLD" "$BINARY" "$RESET"

    detect_platform
    get_latest_version
    install
    post_install

    printf "\n%bDone!%b Run %b%s --help%b to get started.\n\n" "$GREEN" "$RESET" "$BOLD" "$BINARY" "$RESET"
}

main
