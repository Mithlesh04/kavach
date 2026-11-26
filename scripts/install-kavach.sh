#!/bin/sh
# Copyright (c) 2026 Mithlesh and the Kavach contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
# https://kavachlang.com | https://github.com/Mithlesh04/kavach

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
if version="$("$HOME_DIR/bin/kavach" --version 2>&1)"; then
    echo "installed $version"
else
    echo "installed Kavach in $HOME_DIR, but it doesn't start yet:"
    echo "  $version"
    case "$version" in
        *libasound*)
            echo "  kavach needs ALSA's sound library, which desktop Linux has."
            echo "  Ubuntu 24.04 and Debian 13: sudo apt install libasound2t64"
            echo "  Older Ubuntu and Debian: sudo apt install libasound2"
            echo "  Fedora: sudo dnf install alsa-lib"
            ;;
    esac
fi

# New shells find kavach: $BIN_DIR goes into the startup file of each shell
# the user has, once. A script run as `curl … | sh` can't change the shell
# that started it, so that one is told what to run.
# KAVACH_NO_MODIFY_PATH=1 leaves the startup files alone.
add_to() {
    file=$1
    line=$2
    if [ -f "$file" ] && grep -qF "$BIN_DIR" "$file"; then
        return
    fi
    mkdir -p "$(dirname "$file")"
    printf '\n# added by the Kavach installer\n%s\n' "$line" >> "$file"
    echo "added $BIN_DIR to the PATH in $file"
}
if [ -z "$KAVACH_NO_MODIFY_PATH" ]; then
    add_to "$HOME/.profile" "export PATH=\"$BIN_DIR:\$PATH\""
    [ -f "$HOME/.bashrc" ] && add_to "$HOME/.bashrc" "export PATH=\"$BIN_DIR:\$PATH\""
    case "${SHELL:-}" in
        *zsh) add_to "$HOME/.zshrc" "export PATH=\"$BIN_DIR:\$PATH\"" ;;
        *) [ -f "$HOME/.zshrc" ] && add_to "$HOME/.zshrc" "export PATH=\"$BIN_DIR:\$PATH\"" ;;
    esac
    if [ -d "$HOME/.config/fish" ]; then
        add_to "$HOME/.config/fish/conf.d/kavach.fish" "fish_add_path $BIN_DIR"
    fi
fi
case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *) echo "open a new terminal to use kavach, or run in this one: export PATH=\"$BIN_DIR:\$PATH\"" ;;
esac
if ! command -v cc > /dev/null 2>&1; then
    echo "note: building programs needs a C compiler for linking."
    echo "      Debian/Ubuntu: sudo apt install build-essential"
    echo "      Fedora: sudo dnf install gcc    macOS: xcode-select --install"
fi
