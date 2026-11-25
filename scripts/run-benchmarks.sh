#!/bin/sh
# Copyright (c) 2026 Mithlesh and the Kavach contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
# https://kavachlang.com | https://github.com/Mithlesh04/kavach

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
# A benchmark where Kavach uses all cores has twins written for all
# cores, cpp/<name>_cores.cpp (g++ -O2 -fopenmp), and go/ and csharp/
# ones where the folder has those languages. It gets two rows: `1 core`,
# Kavach with KAVACH_CORES=1 against the one-core programs, and
# `all cores`, Kavach as built against the twins, so every row compares
# like with like.
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
# Times one benchmark: the row label to print, its name, the suffix of
# the other languages' programs (`_cores` for the all-core twins), the
# cores Kavach may use (empty for all), then the arguments every
# language's program gets.
time_one() {
    label=$1
    name=$2
    twin=$3
    cores=$4
    shift 4
    kv=999999
    cpp=999999
    go_ms=-
    cs=-
    [ -x "$OUT/go_$name$twin" ] && go_ms=999999
    [ -x "$OUT/cs_$name$twin/bench" ] && cs=999999
    for run in 1 2 3 4 5 6 7 8 9; do
        if [ -n "$cores" ]; then
            export KAVACH_CORES="$cores"
        fi
        t=$(ms "$OUT/kv_$name" "$@")
        unset KAVACH_CORES
        [ "$t" -lt "$kv" ] && kv=$t
        t=$(ms "$OUT/cpp_$name$twin" "$@")
        [ "$t" -lt "$cpp" ] && cpp=$t
        if [ "$go_ms" != - ]; then
            t=$(ms "$OUT/go_$name$twin" "$@")
            [ "$t" -lt "$go_ms" ] && go_ms=$t
        fi
        if [ "$cs" != - ]; then
            t=$(ms "$OUT/cs_$name$twin/bench" "$@")
            [ "$t" -lt "$cs" ] && cs=$t
        fi
    done
    ratio=$(awk "BEGIN { printf \"%.2f\", $kv / ($cpp > 0 ? $cpp : 1) }")
    py=-
    if [ -z "$twin" ] && [ -f "$BENCH/python/$name.py" ] && command -v python3 > /dev/null 2>&1; then
        py=999999
        for run in 1 2 3; do
            t=$(ms python3 "$BENCH/python/$name.py" "$@")
            [ "$t" -lt "$py" ] && py=$t
        done
    fi
    node_ms=-
    if [ -z "$twin" ] && [ -f "$BENCH/js/$name.js" ] && command -v node > /dev/null 2>&1; then
        node_ms=999999
        for run in 1 2 3; do
            t=$(ms node "$BENCH/js/$name.js" "$@")
            [ "$t" -lt "$node_ms" ] && node_ms=$t
        done
    fi
    printf "%-34s %10s %10s %7sx %11s %9s %7s %7s
" "$label" "$kv" "$cpp" "$ratio" "$py" "$node_ms" "$go_ms" "$cs"
}
# Times a benchmark in one row, or in two (1 core, all cores) when it
# has all-core twins.
time_rows() {
    label=$1
    name=$2
    shift 2
    if [ -x "$OUT/cpp_${name}_cores" ]; then
        time_one "$label (1 core)" "$name" "" 1 "$@"
        time_one "$label (all cores)" "$name" _cores "" "$@"
    else
        time_one "$label" "$name" "" "" "$@"
    fi
}
printf "%-34s %10s %10s %8s %11s %9s %7s %7s\n" benchmark "kavach ms" "c++ ms" ratio "python ms" "node ms" "go ms" "c# ms"
for src in "$BENCH"/kavach/*.kav; do
    name=$(basename "$src" .kav)
    if ! "$KAVACH" build "$src" -o "$OUT/kv_$name" > "$OUT/build.txt" 2>&1; then
        printf "%-10s fails to build:\n" "$name"
        cat "$OUT/build.txt"
        continue
    fi
    g++ -O2 -o "$OUT/cpp_$name" "$BENCH/cpp/$name.cpp"
    if [ -f "$BENCH/cpp/${name}_cores.cpp" ]; then
        g++ -O2 -fopenmp -o "$OUT/cpp_${name}_cores" "$BENCH/cpp/${name}_cores.cpp"
    fi
    for twin in "" _cores; do
        prog=$name$twin
        if [ -f "$BENCH/go/$prog.go" ] && command -v "$GO" > /dev/null 2>&1; then
            "$GO" build -o "$OUT/go_$prog" "$BENCH/go/$prog.go"
        fi
        if [ -f "$BENCH/csharp/$prog.cs" ] && command -v "$DOTNET" > /dev/null 2>&1; then
            mkdir -p "$OUT/cs_src_$prog"
            cp "$BENCH/csharp/bench.csproj" "$OUT/cs_src_$prog/"
            cp "$BENCH/csharp/$prog.cs" "$OUT/cs_src_$prog/Program.cs"
            "$DOTNET" publish "$OUT/cs_src_$prog/bench.csproj" -c Release -o "$OUT/cs_$prog" -v q > /dev/null
        fi
    done
    # Every language, on one core or all, must print what C++ prints.
    want=$("$OUT/cpp_$name" 2>&1 | head -5)
    for exe in "$OUT/cpp_${name}_cores" "$OUT/go_$name" "$OUT/cs_$name/bench"         "$OUT/go_${name}_cores" "$OUT/cs_${name}_cores/bench"; do
        if [ -x "$exe" ] && [ "$("$exe" 2>&1 | head -5)" != "$want" ]; then
            echo "$name: $exe prints something other than C++'s result" >&2
        fi
    done
    if [ -f "$BENCH/sizes" ]; then
        grep -v '^#' "$BENCH/sizes" | grep . | while read -r size size_args; do
            # The arguments are meant to split into words.
            # shellcheck disable=SC2086
            time_rows "$name@$size" "$name" $size_args
        done
    else
        time_rows "$name" "$name"
    fi
done
rm -rf "$OUT"
