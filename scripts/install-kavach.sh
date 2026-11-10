#!/bin/sh
# Installs Kavach on Linux or macOS (decision 0080): downloads the
# newest release archive for this machine (or takes one with
# `--from <file>`), unpacks it to ~/.kavach, and links kavach and kv
# into ~/.local/bin. Then checks for a C compiler, which linking needs.
set -e

REPO=Mithlesh04/kavach
HOME_DIR="${KAVACH_HOME:-$HOME/.kavach}"
BIN_DIR="${KAVACH_BIN:-$HOME/.local/bin}"

case "$(uname -s)" in
    Darwin) os=macos ;;
    *) os=linux ;;
esac
case "$(uname -m)" in
    arm64 | aarch64) arch=arm64 ;;
    *) arch=x86_64 ;;
esac

archive=""
if [ "$1" = "--from" ] && [ -n "$2" ]; then
    archive="$2"
else
    echo "finding the newest release of $REPO for $os-$arch"
    url=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" |
        grep -o "\"browser_download_url\": *\"[^\"]*-$os-$arch\.tar\.gz\"" |
        head -1 | cut -d'"' -f4)
    if [ -z "$url" ]; then
        echo "error: no release archive for $os-$arch yet." >&2
        echo "Build from source instead: https://github.com/$REPO#readme" >&2
        exit 1
    fi
    archive=$(mktemp)
    trap 'rm -f "$archive"' EXIT
    echo "downloading $url"
    curl -fL -o "$archive" "$url"
fi

echo "unpacking into $HOME_DIR"
rm -rf "$HOME_DIR.new"
mkdir -p "$HOME_DIR.new"
tar -xzf "$archive" -C "$HOME_DIR.new" --strip-components=1
rm -rf "$HOME_DIR"
mv "$HOME_DIR.new" "$HOME_DIR"

mkdir -p "$BIN_DIR"
ln -sf "$HOME_DIR/bin/kavach" "$BIN_DIR/kavach"
ln -sf "$HOME_DIR/bin/kavach" "$BIN_DIR/kv"
echo "installed $("$HOME_DIR/bin/kavach" --version)"

case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *) echo "note: add $BIN_DIR to your PATH to run kavach from anywhere" ;;
esac
if ! command -v cc > /dev/null 2>&1; then
    echo "note: building programs needs a C compiler for linking."
    echo "      Debian/Ubuntu: sudo apt install build-essential"
    echo "      Fedora: sudo dnf install gcc    macOS: xcode-select --install"
fi
