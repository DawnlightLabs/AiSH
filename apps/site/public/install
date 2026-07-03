#!/usr/bin/env bash
set -euo pipefail

REPO="DawnlightLabs/AiSH"
VERSION="${AISH_VERSION:-latest}"
INSTALL_ROOT="${AISH_INSTALL_ROOT:-$HOME/.local/aish}"
BIN_DIR="$INSTALL_ROOT/bin"
BIN_PATH="$BIN_DIR/aish"
HEADLESS="${AISH_HEADLESS:-0}"
SKIP_MODEL="${AISH_SKIP_MODEL:-0}"

detect_os() {
  case "$(uname -s)" in
    Darwin) echo "macos" ;;
    Linux) echo "linux" ;;
    *) echo "Unsupported OS: $(uname -s)" >&2; exit 1 ;;
  esac
}

detect_arch() {
  case "$(uname -m)" in
    arm64|aarch64) echo "arm64" ;;
    x86_64|amd64) echo "x64" ;;
    *) echo "Unsupported architecture: $(uname -m)" >&2; exit 1 ;;
  esac
}

os="$(detect_os)"
arch="$(detect_arch)"
asset="aish-${os}-${arch}.tar.gz"

if [ "$VERSION" = "latest" ]; then
  url="https://github.com/${REPO}/releases/latest/download/${asset}"
else
  url="https://github.com/${REPO}/releases/download/${VERSION}/${asset}"
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

mkdir -p "$BIN_DIR"

echo "Downloading AiSH from $url"
curl -fsSL "$url" -o "$tmp/aish.tar.gz"

checksum_url="${url}.sha256"
if curl -fsSL "$checksum_url" -o "$tmp/aish.tar.gz.sha256"; then
  expected="$(awk '{print $1}' "$tmp/aish.tar.gz.sha256")"
  actual="$(shasum -a 256 "$tmp/aish.tar.gz" | awk '{print $1}')"
  if [ "$expected" != "$actual" ]; then
    echo "Checksum mismatch for $asset" >&2
    exit 1
  fi
  echo "Verified SHA256: $actual"
else
  echo "Warning: checksum unavailable for $asset" >&2
fi

tar -xzf "$tmp/aish.tar.gz" -C "$tmp"

found="$(find "$tmp" -type f -name aish | head -n 1)"
if [ -z "$found" ]; then
  echo "Downloaded archive did not contain aish" >&2
  exit 1
fi

cp "$found" "$BIN_PATH"
chmod +x "$BIN_PATH"

if [ "$HEADLESS" = "1" ]; then
  setup_args=(--install-headless --add-path --set-model-path --editor-profiles)
  if [ "$SKIP_MODEL" = "1" ]; then
    setup_args+=(--skip-model)
  else
    setup_args+=(--model-check)
  fi
  "$BIN_PATH" "${setup_args[@]}"
else
  "$BIN_PATH" --install
fi

echo
echo "AiSH installed at $BIN_PATH"
echo "Open a new terminal and run: aish"
