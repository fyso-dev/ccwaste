#!/bin/bash
set -euo pipefail

REPO="fyso-dev/ccwaste"
LATEST=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
  darwin) OS="apple-darwin" ;;
  linux) OS="unknown-linux-gnu" ;;
  *) echo "Unsupported OS: $OS"; exit 1 ;;
esac

case "$ARCH" in
  x86_64|amd64) ARCH="x86_64" ;;
  arm64|aarch64) ARCH="aarch64" ;;
  *) echo "Unsupported arch: $ARCH"; exit 1 ;;
esac

TARGET="${ARCH}-${OS}"
URL="https://github.com/$REPO/releases/download/$LATEST/ccwaste-$TARGET"

INSTALL_DIR="${CARGO_HOME:-$HOME/.cargo}/bin"
mkdir -p "$INSTALL_DIR"

echo "Downloading ccwaste $LATEST for $TARGET..."
curl -fsSL "$URL" -o "$INSTALL_DIR/ccwaste"
chmod +x "$INSTALL_DIR/ccwaste"
echo "Installed ccwaste to $INSTALL_DIR/ccwaste"
echo "Run 'ccwaste' to get started"
