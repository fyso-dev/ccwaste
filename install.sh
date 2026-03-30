#!/bin/bash
set -euo pipefail

REPO="fyso-dev/ccwasted"
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
URL="https://github.com/$REPO/releases/download/$LATEST/ccwasted-$TARGET"

INSTALL_DIR="${CARGO_HOME:-$HOME/.cargo}/bin"
mkdir -p "$INSTALL_DIR"

echo "Downloading ccwasted $LATEST for $TARGET..."
curl -fsSL "$URL" -o "$INSTALL_DIR/ccwasted"
chmod +x "$INSTALL_DIR/ccwasted"
echo "Installed ccwasted to $INSTALL_DIR/ccwasted"
echo "Run 'ccwasted' to get started"
