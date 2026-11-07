#!/bin/sh
# Installs consertion on Linux or macOS so you can run it from anywhere.
#
#   From a clone:   ./install.sh      (builds or reuses your local binary, no download)
#   Without one:    curl -fsSL https://raw.githubusercontent.com/exLeute/consertion/master/install.sh | sh
#
# It asks a couple of questions. With no terminal to ask on, it takes the defaults.
# CONSERTION_DIR=/some/bin skips the "where to" question.
set -eu

repo="exLeute/consertion"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# Questions go through /dev/tty, because with `curl | sh` stdin is the script itself.
if (: </dev/tty) 2>/dev/null; then tty=1; else tty=; fi

# ask "question" default -> answer in $ans
ask() {
  ans=
  if [ -n "$tty" ]; then
    printf '%s [%s] ' "$1" "$2" >/dev/tty
    read -r ans </dev/tty || ans=
  fi
  [ -n "$ans" ] || ans="$2"
}

say() { printf '%s\n' "$*"; }
die() { say "$*" >&2; exit 1; }

download() {
  case "$(uname -s)-$(uname -m)" in
    Linux-x86_64)               target=x86_64-unknown-linux-musl ;;
    Linux-aarch64|Linux-arm64)  target=aarch64-unknown-linux-musl ;;
    Darwin-arm64)               target=aarch64-apple-darwin ;;
    Darwin-x86_64)              target=x86_64-apple-darwin ;;
    *) die "No prebuilt binary for $(uname -s) $(uname -m). Build it instead: cargo install --git https://github.com/$repo" ;;
  esac
  url="https://github.com/$repo/releases/latest/download/consertion-$target.tar.gz"
  say "Downloading $url"
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$url" -o "$tmp/c.tar.gz"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO "$tmp/c.tar.gz" "$url"
  else
    die "Need curl or wget to download."
  fi
  tar xzf "$tmp/c.tar.gz" -C "$tmp"
  bin="$tmp/consertion"
}

build() {
  command -v cargo >/dev/null 2>&1 || die "cargo not found. Get Rust from https://rustup.rs, or pick the download option."
  say "Building (takes a minute or two the first time)..."
  (cd "$src" && cargo build --release --locked)
  bin="$src/target/release/consertion"
}

# 1. Get a binary. Inside a clone, use what's already here.
src=
case "$0" in */*) d="$(cd "$(dirname "$0")" && pwd)" ;; *) d= ;; esac
if [ -n "$d" ] && grep -qs '^name = "consertion"' "$d/Cargo.toml"; then src="$d"; fi

if [ -n "$src" ]; then
  built="$src/target/release/consertion"
  say "Found the consertion source in $src"
  if [ -x "$built" ]; then
    say "  1) use the binary you already built ($built)"
    say "  2) rebuild from source"
    say "  3) download the latest release instead"
    ask "Pick one:" 1
  else
    say "  1) build from source"
    say "  2) download the latest release instead"
    ask "Pick one:" 1
    case "$ans" in 1) ans=2 ;; 2) ans=3 ;; esac
  fi
  case "$ans" in
    1) bin="$built" ;;
    2) build ;;
    3) download ;;
    *) die "Unknown choice: $ans" ;;
  esac
else
  download
fi

# 2. Where to put it.
if [ -n "${CONSERTION_DIR:-}" ]; then
  dir="$CONSERTION_DIR"
else
  say "Install to:"
  say "  1) $HOME/.local/bin   (just you, no sudo)"
  say "  2) /usr/local/bin     (everyone, needs sudo)"
  say "  3) somewhere else"
  ask "Pick one:" 1
  case "$ans" in
    1) dir="$HOME/.local/bin" ;;
    2) dir=/usr/local/bin ;;
    3) ask "Folder:" "$HOME/bin"; dir="$ans" ;;
    *) die "Unknown choice: $ans" ;;
  esac
fi
case "$dir" in "~"*) dir="$HOME${dir#\~}" ;; esac

sudo=
if ! mkdir -p "$dir" 2>/dev/null || [ ! -w "$dir" ]; then
  command -v sudo >/dev/null 2>&1 || die "Can't write to $dir and sudo isn't available."
  sudo=sudo
  $sudo mkdir -p "$dir"
fi
# Copy then rename, so a running copy of the old binary isn't clobbered mid-write.
$sudo cp "$bin" "$dir/.consertion.new"
$sudo chmod 755 "$dir/.consertion.new"
$sudo mv -f "$dir/.consertion.new" "$dir/consertion"
say "Installed $dir/consertion"

# 3. Make sure the shell can find it.
case ":$PATH:" in
  *":$dir:"*) say "Done. Run: consertion" ; exit 0 ;;
esac

shell="$(basename "${SHELL:-sh}")"
case "$shell" in
  fish) rc="$HOME/.config/fish/conf.d/consertion.fish"; line="fish_add_path -g '$dir'" ;;
  zsh)  rc="$HOME/.zshrc";   line="export PATH=\"$dir:\$PATH\"" ;;
  bash) rc="$HOME/.bashrc";  line="export PATH=\"$dir:\$PATH\"" ;;
  *)    rc="$HOME/.profile"; line="export PATH=\"$dir:\$PATH\"" ;;
esac

say "$dir isn't on your PATH yet."
ask "Add it to $rc? (y/n)" y
case "$ans" in
  y|Y|yes)
    mkdir -p "$(dirname "$rc")"
    grep -qsF "$line" "$rc" || printf '\n# consertion\n%s\n' "$line" >>"$rc"
    say "Done. Open a new terminal, then run: consertion"
    ;;
  *)
    say "Done. Add this to your shell config to run it from anywhere:"
    say "  $line"
    ;;
esac
