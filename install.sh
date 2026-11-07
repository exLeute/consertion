#!/bin/sh
# Installs the latest consertion release for Linux or macOS.
#   curl -fsSL https://raw.githubusercontent.com/exLeute/consertion/master/install.sh | sh
# Set CONSERTION_DIR to install somewhere other than ~/.local/bin.
set -eu

repo="exLeute/consertion"
dir="${CONSERTION_DIR:-$HOME/.local/bin}"

case "$(uname -s)-$(uname -m)" in
  Linux-x86_64)               target=x86_64-unknown-linux-musl ;;
  Linux-aarch64|Linux-arm64)  target=aarch64-unknown-linux-musl ;;
  Darwin-arm64)               target=aarch64-apple-darwin ;;
  Darwin-x86_64)              target=x86_64-apple-darwin ;;
  *) echo "No prebuilt binary for $(uname -s) $(uname -m). Build it instead: cargo install --git https://github.com/$repo" >&2; exit 1 ;;
esac

url="https://github.com/$repo/releases/latest/download/consertion-$target.tar.gz"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "Downloading $url"
if command -v curl >/dev/null 2>&1; then
  curl -fsSL "$url" -o "$tmp/c.tar.gz"
else
  wget -qO "$tmp/c.tar.gz" "$url"
fi
tar xzf "$tmp/c.tar.gz" -C "$tmp"
mkdir -p "$dir"
mv "$tmp/consertion" "$dir/consertion"
chmod +x "$dir/consertion"
echo "Installed to $dir/consertion"

case ":$PATH:" in
  *":$dir:"*) echo "Run: consertion" ;;
  *) echo "Add $dir to your PATH, then run: consertion" ;;
esac
