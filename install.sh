#!/usr/bin/env bash
set -euo pipefail

# Sage Restic Manager - One-line installer
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/theadeyemiolayinka/sage-restic-manager/main/install.sh | bash
#   curl -fsSL ... | bash -s -- v0.1.1
#
# Environment variables:
#   INSTALL_DIR    Target directory (default: /usr/local/bin)

REPO="theadeyemiolayinka/sage-restic-manager"
CRATE_NAME="sage-restic-manager"

# --- Determine version ---
if [ $# -ge 1 ]; then
    VERSION="$1"
else
    echo "Fetching latest release..."
    VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep -oP '"tag_name": "\K[^"]+') || {
        echo "ERROR: Could not determine latest release version."
        echo "Pass a version explicitly: ./install.sh v0.1.1"
        exit 1
    }
fi

# Normalize version
if [[ "$VERSION" != v* ]]; then
    VERSION="v$VERSION"
fi

echo "Installing ${CRATE_NAME} ${VERSION}"

# --- Detect platform ---
OS=$(uname -s)
ARCH=$(uname -m)

case "$OS" in
    Linux)     OS_TARGET="unknown-linux-gnu" ;;
    Darwin)    OS_TARGET="apple-darwin" ;;
    *)         echo "ERROR: Unsupported operating system: $OS"; exit 1 ;;
esac

case "$ARCH" in
    x86_64)    ARCH_TARGET="x86_64" ;;
    aarch64|arm64) ARCH_TARGET="aarch64" ;;
    *)         echo "ERROR: Unsupported architecture: $ARCH"; exit 1 ;;
esac

TARGET="${ARCH_TARGET}-${OS_TARGET}"
BINARY_NAME="${CRATE_NAME}-${TARGET}"
RELEASE_URL="https://github.com/${REPO}/releases/download/${VERSION}"

echo "Detected platform: ${TARGET}"

# --- Create temp directory ---
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

# --- Download artifacts ---
echo "Downloading artifacts..."
for artifact in "$BINARY_NAME" "${BINARY_NAME}.sha256"; do
    url="${RELEASE_URL}/${artifact}"
    echo "  -> ${artifact}"
    if ! curl -fsSL -o "${TMPDIR}/${artifact}" "$url"; then
        echo "ERROR: Failed to download ${artifact}"
        echo "Verify the release exists at: ${RELEASE_URL}"
        exit 1
    fi
done

# --- Verify SHA256 checksum ---
echo "Verifying SHA256 checksum..."
cd "$TMPDIR"
if ! sha256sum -c "${BINARY_NAME}.sha256"; then
    echo "ERROR: SHA256 checksum verification failed. The downloaded binary may be corrupted or tampered with."
    exit 1
fi
cd - >/dev/null

# --- Determine install directory ---
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
if [ ! -d "$INSTALL_DIR" ] || [ ! -w "$INSTALL_DIR" ]; then
    if [ -d "$HOME/.local/bin" ] && [ -w "$HOME/.local/bin" ]; then
        INSTALL_DIR="$HOME/.local/bin"
    else
        INSTALL_DIR="$HOME/bin"
        mkdir -p "$INSTALL_DIR"
    fi
fi

echo "Installing to: ${INSTALL_DIR}/${CRATE_NAME}"
cp "${TMPDIR}/${BINARY_NAME}" "${INSTALL_DIR}/${CRATE_NAME}"
chmod 755 "${INSTALL_DIR}/${CRATE_NAME}"

# --- Verify installation ---
if command -v "${CRATE_NAME}" >/dev/null 2>&1; then
    INSTALLED_VERSION=$(${CRATE_NAME} --version 2>/dev/null || echo "unknown")
    echo "Installed: ${INSTALLED_VERSION}"
else
    echo "WARNING: ${INSTALL_DIR} is not on your PATH."
    echo "  Add it to your shell profile:"
    echo "    export PATH=\"${INSTALL_DIR}:\$PATH\""
fi

echo ""
echo "Installation complete."
echo "Run '${CRATE_NAME}' to start the TUI or '${CRATE_NAME} --help' for CLI options."
