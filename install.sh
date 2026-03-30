#!/usr/bin/env bash
set -euo pipefail

REPO="fyso-dev/ccwaste"
BINARY="ccwaste"

# Detect OS
case "$(uname -s)" in
  Darwin) OS="apple-darwin" ;;
  Linux)  OS="unknown-linux-gnu" ;;
  *)      echo "Unsupported OS: $(uname -s)"; exit 1 ;;
esac

# Detect architecture
case "$(uname -m)" in
  x86_64|amd64)  ARCH="x86_64" ;;
  arm64|aarch64) ARCH="aarch64" ;;
  *)             echo "Unsupported architecture: $(uname -m)"; exit 1 ;;
esac

TARGET="${ARCH}-${OS}"
ASSET="${BINARY}-${TARGET}"

echo "Detected platform: ${TARGET}"

# Get latest release download URL
DOWNLOAD_URL="https://github.com/${REPO}/releases/latest/download/${ASSET}"

# Choose install directory
if [ -d "${HOME}/.cargo/bin" ]; then
  INSTALL_DIR="${HOME}/.cargo/bin"
elif [ -w "/usr/local/bin" ]; then
  INSTALL_DIR="/usr/local/bin"
else
  INSTALL_DIR="${HOME}/.local/bin"
  mkdir -p "${INSTALL_DIR}"
fi

echo "Downloading ${ASSET}..."
curl -fsSL "${DOWNLOAD_URL}" -o "${INSTALL_DIR}/${BINARY}"
chmod +x "${INSTALL_DIR}/${BINARY}"

echo "Installed ${BINARY} to ${INSTALL_DIR}/${BINARY}"

# Verify
if command -v "${BINARY}" &>/dev/null; then
  echo "Version: $(${BINARY} --version)"
else
  echo "Note: ${INSTALL_DIR} is not in your PATH. Add it with:"
  echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
fi
