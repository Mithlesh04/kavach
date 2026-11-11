#!/bin/sh
# Builds and times every benchmark in a folder (benchmarks/loops unless
# another is named), in Kavach and in C++ (g++ -O2), and prints the best
# of 9 runs in milliseconds. Runs of the languages alternate, so other
# work on the machine slows them alike. When the folder also has
# python/<name>.py or js/<name>.js, Python and Node are timed too
# (best of 3: they are slower); with go/<name>.go or csharp/<name>.cs,
# Go and C# too (`go build`, `dotnet publish -c Release`; best of 9, in
# turns with the others), when `go` and `dotnet` (or $GO and $DOTNET)
# are found. A folder with a `sizes` file runs every
# benchmark once per line of it, `<label> <arguments>` (for example
# `millions --items 1000000`), passing the arguments to every language.
# Usage: sh scripts/run-benchmarks.sh [path/to/kavach] [benchmarks/chains]
set -e
cd "$(dirname "$0")/.."
KAVACH=${1:-${CARGO_TARGET_DIR:-target}/release/kavach}
BENCH=${2:-benchmarks/loops}
GO=${GO:-go}
DOTNET=${DOTNET:-dotnet}
export DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_NOLOGO=1
if [ ! -x "$KAVACH" ]; then
    cargo build --release -p kavach-cli
fi
OUT=$(mktemp -d)
ms() {
    start=$(date +%s%N)
    "$@" > "$OUT/out.txt"
    end=$(date +%s%N)
    echo $(( (end - start) / 1000000 ))
}
# Times one benchmark: the row label to print, its name, then the
# arguments every language's program gets.
time_one() {
    label=$1
    name=$2
    shift 2
    kv=999999
    cpp=999999
    go_ms=-
    cs=-
    [ -x "$OUT/go_$name" ] && go_ms=999999
    [ -x "$OUT/cs_$name/bench" ] && cs=999999
    for run in 1 2 3 4 5 6 7 8 9; do
        t=$(ms "$OUT/kv_$name" "$@")
        [ "$t" -lt "$kv" ] && kv=$t
        t=$(ms "$OUT/cpp_$name" "$@")
        [ "$t" -lt "$cpp" ] && cpp=$t
        if [ "$go_ms" != - ]; then
            t=$(ms "$OUT/go_$name" "$@")
            [ "$t" -lt "$go_ms" ] && go_ms=$t
        fi
        if [ "$cs" != - ]; then
            t=$(ms "$OUT/cs_$name/bench" "$@")
            [ "$t" -lt "$cs" ] && cs=$t
        fi
    done
    ratio=$(awk "BEGIN { printf \"%.2f\", $kv / ($cpp > 0 ? $cpp : 1) }")
    py=-
    if [ -f "$BENCH/python/$name.py" ] && command -v python3 > /dev/null 2>&1; then
        py=999999
        for run in 1 2 3; do
            t=$(ms python3 "$BENCH/python/$name.py" "$@")
            [ "$t" -lt "$py" ] && py=$t
        done
    fi
    node_ms=-
    if [ -f "$BENCH/js/$name.js" ] && command -v node > /dev/null 2>&1; then
        node_ms=999999
        for run in 1 2 3; do
            t=$(ms node "$BENCH/js/$name.js" "$@")
            [ "$t" -lt "$node_ms" ] && node_ms=$t
        done
    fi
    printf "%-12s %10s %10s %7sx %11s %9s %7s %7s\n" "$label" "$kv" "$cpp" "$ratio" "$py" "$node_ms" "$go_ms" "$cs"
}
printf "%-12s %10s %10s %8s %11s %9s %7s %7s\n" benchmark "kavach ms" "c++ ms" ratio "python ms" "node ms" "go ms" "c# ms"
for src in "$BENCH"/kavach/*.kav; do
    name=$(basename "$src" .kav)
    if ! "$KAVACH" build "$src" -o "$OUT/kv_$name" > "$OUT/build.txt" 2>&1; then
        printf "%-10s fails to build:\n" "$name"
        cat "$OUT/build.txt"
        continue
    fi
    g++ -O2 -o "$OUT/cpp_$name" "$BENCH/cpp/$name.cpp"
    if [ -f "$BENCH/go/$name.go" ] && command -v "$GO" > /dev/null 2>&1; then
        "$GO" build -o "$OUT/go_$name" "$BENCH/go/$name.go"
    fi
    if [ -f "$BENCH/csharp/$name.cs" ] && command -v "$DOTNET" > /dev/null 2>&1; then
        mkdir -p "$OUT/cs_src_$name"
        cp "$BENCH/csharp/bench.csproj" "$OUT/cs_src_$name/"
        cp "$BENCH/csharp/$name.cs" "$OUT/cs_src_$name/Program.cs"
        "$DOTNET" publish "$OUT/cs_src_$name/bench.csproj" -c Release -o "$OUT/cs_$name" -v q > /dev/null
    fi
    # Every language must print what C++ prints.
    want=$("$OUT/cpp_$name" 2>&1 | head -5)
    for exe in "$OUT/go_$name" "$OUT/cs_$name/bench"; do
        if [ -x "$exe" ] && [ "$("$exe" 2>&1 | head -5)" != "$want" ]; then
            echo "$name: $exe prints something other than C++'s result" >&2
        fi
    done
    if [ -f "$BENCH/sizes" ]; then
        grep -v '^#' "$BENCH/sizes" | grep . | while read -r size size_args; do
            # The arguments are meant to split into words.
            # shellcheck disable=SC2086
            time_one "$name@$size" "$name" $size_args
        done
    else
        time_one "$name" "$name"
    fi
done
rm -rf "$OUT"
