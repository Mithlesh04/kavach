#!/bin/sh
# Copyright (c) 2026 Mithlesh and the Kavach contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
# https://kavachlang.com | https://github.com/Mithlesh04/kavach

# Adds the license header (decision 0133) to every source file that lacks
# one. The files are the ones scripts/check-repo.sh reports, so the scope
# and what is left out live in one place. A file without a header is new,
# so its header gets this year. A header already there is never changed.
# Usage: sh scripts/add-license-headers.sh [file...]
#   With files named, adds the header to just those.
# Twin: scripts/add-license-headers.ps1 (keep both in sync)

year=$(date +%Y)
tmp=$(mktemp) || exit 2
trap 'rm -f "$tmp"' EXIT

# Whether the first 5 lines already carry the header, as check-repo reads it.
has_header() {
    head -n 5 "$1" | grep -q -E 'Copyright \(c\) [0-9]{4} Mithlesh and the Kavach contributors[[:space:]]*$' &&
        head -n 5 "$1" | grep -q -E 'SPDX-License-Identifier: Apache-2\.0 OR MIT[[:space:]]*$'
}

# Writes the header at the top of the file, after a shebang (`#!`, but not
# Rust's `#![`) if it has one, with a blank line before the rest.
add_header() {
    case "$1" in
        *.rs | *.kav | *.mjs | *.ts | *.js) c='//' ;;
        *.sh | *.ps1) c='#' ;;
        *hooks/*) case "${1##*/}" in *.*) c='' ;; *) c='#' ;; esac ;;
        *) c='' ;;
    esac
    if [ -z "$c" ]; then echo "skipped   $1: not a file type that carries the header"; return; fi
    has_header "$1" && return
    first=$(head -n 1 "$1")
    case "$first" in
        '#!['*) start=1 ;;
        '#!'*) start=2 ;;
        *) start=1 ;;
    esac
    {
        [ "$start" -eq 2 ] && printf '%s\n' "$first"
        printf '%s Copyright (c) %s Mithlesh and the Kavach contributors\n' "$c" "$year"
        printf '%s SPDX-License-Identifier: Apache-2.0 OR MIT\n' "$c"
        printf '%s https://kavachlang.com | https://github.com/Mithlesh04/kavach\n' "$c"
        [ -n "$(tail -n +"$start" "$1" | head -n 1)" ] && echo
        tail -n +"$start" "$1"
    } > "$tmp" || exit 1
    # cat keeps the file's mode, so a hook stays executable.
    cat "$tmp" > "$1" || exit 1
    echo "added     $1"
}

if [ "$#" -gt 0 ]; then
    for f in "$@"; do add_header "$f"; done
    exit 0
fi
cd "$(dirname "$0")/.." || exit 2
sh scripts/check-repo.sh |
    sed -n 's|^NO HEADER \./\(.*\): no license header in its first 5 lines\..*|\1|p' |
    while IFS= read -r f; do add_header "$f"; done
