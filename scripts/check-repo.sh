#!/bin/sh
# Checks the repository rules from AGENTS.md:
#   1. no file we write is longer than 500 lines
#   2. every relative Markdown link points to a file (and heading) that exists
# Usage: sh scripts/check-repo.sh     Twin: scripts/check-repo.ps1 (keep both in sync)
#
# Each check is one pass over all the files, not a program per file:
# starting a program is slow on Windows (Git Bash).

MAX_LINES=500
cd "$(dirname "$0")/.." || exit 2
tmp=$(mktemp -d) || exit 2
trap 'rm -rf "$tmp"' EXIT
fail=0

# The files git keeps or would keep: what .gitignore leaves out (build
# output, examples' audit logs) is not ours to check. Generated files are
# not counted either. Keep this list in sync with the .ps1 twin.
git ls-files --cached --others --exclude-standard \
    | grep -v -e '\.lock$' -e 'tests/found/' -e 'package-lock\.json$' \
        -e 'website/data/benchmarks\.json$' \
    | while read -r f; do [ -f "$f" ] && echo "./$f"; done > "$tmp/files"

# 1. File length: binary files (with -I) and empty ones count 0.
tr '\n' '\0' < "$tmp/files" | xargs -0 grep -cI '' 2>/dev/null \
    | awk -F: -v max="$MAX_LINES" '{ n = $NF; f = substr($0, 1, length($0) - length(n) - 1) }
        n > max { printf "TOO LONG  %s: %d lines (limit %d). Split it by responsibility.\n", f, n, max; bad = 1 }
        END { exit bad }' || fail=1

# 2. Relative links in Markdown, outside code blocks. One awk pass writes
# each Markdown file's headings as GitHub anchors (lowercase, punctuation
# removed, spaces become hyphens) and each link, with its target's path
# made plain; the shell then checks that each target exists.
grep '\.md$' "$tmp/files" > "$tmp/md"
tr '\n' '\0' < "$tmp/md" | xargs -0 awk -v slugs="$tmp/slugs" -v links="$tmp/links" '
    function plain(path,    n, parts, out, i, k) {
        n = split(path, parts, "/"); k = 0
        for (i = 1; i <= n; i++) {
            if (parts[i] == "." || parts[i] == "") continue
            if (parts[i] == ".." && k > 0 && out[k] != "..") { k--; continue }
            out[++k] = parts[i]
        }
        path = "."
        for (i = 1; i <= k; i++) path = path "/" out[i]
        return path
    }
    FNR == 1 { fence = 0; dir = FILENAME; sub(/\/[^\/]*$/, "", dir) }
    /^```/ { fence = !fence; next }
    fence { next }
    /^#+ / {
        h = $0; sub(/^#+ +/, "", h); h = tolower(h)
        gsub(/[][!"#$%&'"'"'()*+,.\/:;<=>?@\\^`{|}~]/, "", h); gsub(/ /, "-", h)
        print plain(FILENAME) "\t" h > slugs
    }
    {
        line = $0
        while (match(line, /\]\([^) ]+\)/)) {
            link = substr(line, RSTART + 2, RLENGTH - 3)
            line = substr(line, RSTART + RLENGTH)
            if (link ~ /^(https?|mailto):/) continue
            path = link; anchor = ""
            if (index(link, "#")) { path = substr(link, 1, index(link, "#") - 1); anchor = substr(link, index(link, "#") + 1) }
            target = (path == "") ? FILENAME : dir "/" path
            print FILENAME "\t" link "\t" plain(target) "\t" anchor > links
        }
    }'
touch "$tmp/slugs" "$tmp/links"

# Targets that don't exist, then anchors with no such heading.
while IFS='	' read -r src link target anchor; do
    if [ ! -e "$target" ]; then
        echo "BROKEN    $src -> $link (no such file)"
        echo x >> "$tmp/broken"
    elif [ -n "$anchor" ] && [ -f "$target" ]; then
        printf '%s\t%s\t%s\t%s\n' "$src" "$link" "$target" "$anchor" >> "$tmp/anchors"
    fi
done < "$tmp/links"
[ -s "$tmp/broken" ] && fail=1
if [ -s "$tmp/anchors" ]; then
    awk -F'\t' 'NR == FNR { have[$1 "\t" $2] = 1; next }
        !(($3 "\t" $4) in have) { printf "BROKEN    %s -> %s (no such heading)\n", $1, $2; bad = 1 }
        END { exit bad }' "$tmp/slugs" "$tmp/anchors" || fail=1
fi

if [ "$fail" -eq 0 ]; then
    echo "check-repo: all good"
fi
exit "$fail"
