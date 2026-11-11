#!/bin/sh
# Packages the speed kit for this machine's system (to-do: "A speed
# comparison anyone can run"): every benchmark that has a C++ twin and
# runs alone, built in Kavach (`kavach build`) and in C++ (`g++ -O2`,
# static on Linux so it runs on a fresh machine), with their source, and
# `compare`, which times them. Writes .kavach/dist/kavach-speed-<version>-
# <system>-<arch>.tar.gz and a .sha256. Needs cargo, LLVM's tools for
# Kavach (decision 0018) and a C++ compiler.
set -e
cd "$(dirname "$0")/.."

case "$(uname -s)" in
    Darwin) os=macos; cxx="${CXX:-clang++}"; static="" ;;
    *) os=linux; cxx="${CXX:-g++}"; static="-static" ;;
esac
case "$(uname -m)" in
    arm64 | aarch64) arch=arm64 ;;
    *) arch=x86_64 ;;
esac

target="${CARGO_TARGET_DIR:-target}"
echo "building kavach and compare (release)"
cargo build -q --release -p kavach-cli -p kavach-speedkit
kavach="$target/release/kavach"

version="0.1.0-$(git rev-parse --short HEAD)"
name="kavach-speed-$version-$os-$arch"
kit=".kavach/dist/$name"
rm -rf "$kit"
mkdir -p "$kit/programs/kavach" "$kit/programs/cpp"
cp "$target/release/compare" "$kit/"

{
    echo "# Kavach $("$kavach" --version | sed 's/^kavach //'), built with kavach build"
    echo "# C++: $("$cxx" --version | head -1), $cxx -O2 $static"
} > "$kit/kit.txt"

# Suites whose programs run alone, with no GPU, network or arguments.
for suite in loops everyday chains math lookups bytes channels tasks; do
    mkdir -p "$kit/source/kavach/$suite" "$kit/source/cpp/$suite"
    for src in benchmarks/"$suite"/kavach/*.kav; do
        bench=$(basename "$src" .kav)
        cpp="benchmarks/$suite/cpp/$bench.cpp"
        [ -f "$cpp" ] || continue
        echo "  $suite/$bench"
        "$kavach" build "$src" -o "$kit/programs/kavach/$bench" > /dev/null
        "$cxx" -O2 -std=c++20 -pthread $static "$cpp" -o "$kit/programs/cpp/$bench"
        cp "$src" "$kit/source/kavach/$suite/"
        cp "$cpp" "$kit/source/cpp/$suite/"
        echo "$suite $bench" >> "$kit/kit.txt"
    done
done

cat > "$kit/README.txt" <<EOF
Kavach speed kit $version ($os, $arch)

Run ./compare in a terminal. It checks that each Kavach program prints the
same result as its C++ twin, then runs the two in turns and prints the best
time of each and how many times faster Kavach is. The table is also saved
in results.txt. Options: --runs N (default 5), --only <name>.

Nothing else is needed: the programs are already built. Their source is in
source/kavach and source/cpp; the same files, with the measured results, are
at https://github.com/Mithlesh04/kavach/tree/main/benchmarks

Close other busy programs first: they slow both languages, but not always
alike. kit.txt says what built the programs.
EOF
cp LICENSE-APACHE LICENSE-MIT "$kit/"

tar -czf ".kavach/dist/$name.tar.gz" -C .kavach/dist "$name"
(cd .kavach/dist && sha256sum "$name.tar.gz" > "$name.tar.gz.sha256")
rm -rf "$kit"
echo "wrote .kavach/dist/$name.tar.gz"
