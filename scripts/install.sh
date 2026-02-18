#!/usr/bin/env bash
# Copyright 2026 Cortex Contributors
# SPDX-License-Identifier: Apache-2.0
#
# Cortex universal installer.
#
# Usage:
#   curl -fsSL https://cortex.dev/install | bash
#   curl -fsSL https://cortex.dev/install | bash -s -- --full   # with Chromium
#
# Environment variables:
#   CORTEX_INSTALL_DIR  — Override install directory (default: /usr/local/bin)
#   CORTEX_VERSION      — Pin a specific version (default: latest)

set -euo pipefail

REPO="cortex-ai/cortex"
INSTALL_DIR="${CORTEX_INSTALL_DIR:-/usr/local/bin}"
VERSION="${CORTEX_VERSION:-latest}"
FULL=false

for arg in "$@"; do
    case "$arg" in
        --full) FULL=true ;;
        --version=*) VERSION="${arg#*=}" ;;
        --dir=*) INSTALL_DIR="${arg#*=}" ;;
        --help|-h)
            echo "Usage: install.sh [--full] [--version=X.Y.Z] [--dir=/path]"
            exit 0
            ;;
    esac
done

# ── Detect platform ───────────────────────────────────────────────
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux)   PLATFORM="linux" ;;
    Darwin)  PLATFORM="darwin" ;;
    *)       echo "Unsupported OS: $OS" >&2; exit 1 ;;
esac

case "$ARCH" in
    x86_64|amd64)   ARCH="x86_64" ;;
    aarch64|arm64)   ARCH="aarch64" ;;
    *)               echo "Unsupported architecture: $ARCH" >&2; exit 1 ;;
esac

TRIPLE="${PLATFORM}-${ARCH}"
echo "Detected platform: ${TRIPLE}"

# ── Resolve version ──────────────────────────────────────────────
if [ "$VERSION" = "latest" ]; then
    VERSION="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
        | grep '"tag_name"' | head -1 | sed 's/.*"v\?\([^"]*\)".*/\1/')"
fi
echo "Installing Cortex v${VERSION}"

# ── Download binary ──────────────────────────────────────────────
ASSET="cortex-${VERSION}-${TRIPLE}.tar.gz"
URL="https://github.com/${REPO}/releases/download/v${VERSION}/${ASSET}"

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

echo "Downloading ${URL}..."
curl -fsSL "$URL" -o "${TMPDIR}/${ASSET}"
tar xzf "${TMPDIR}/${ASSET}" -C "$TMPDIR"

# ── Install ──────────────────────────────────────────────────────
mkdir -p "$INSTALL_DIR"
cp "${TMPDIR}/cortex" "${INSTALL_DIR}/cortex"
chmod +x "${INSTALL_DIR}/cortex"

echo "Installed cortex to ${INSTALL_DIR}/cortex"

# ── Optional: install Chromium ───────────────────────────────────
if [ "$FULL" = true ]; then
    echo "Installing Chromium for full Cortex capabilities..."
    "${INSTALL_DIR}/cortex" doctor --install-chrome
fi

# ── Verify ───────────────────────────────────────────────────────
"${INSTALL_DIR}/cortex" --version

echo ""
echo "Cortex installed successfully!"
echo ""
echo "Quick start:"
echo "  cortex start          # Start the Cortex daemon"
echo "  cortex map example.com  # Map a website"
echo "  cortex plug           # Auto-configure AI agents"
echo ""
if [ "$FULL" = false ]; then
    echo "Tip: Run with --full to also install Chromium for browser fallback."
fi
