#!/bin/sh
# Packages an installable Kavach (decision 0080) for this machine's
# system: a tar.gz with bin/ (kavach, kv), lib/ (the runtime library),
# llvm/ (opt and llc from the Rust toolchain) and the licences, plus a
# .sha256 beside it, in .kavach/dist/. Needs cargo and rustup's
# llvm-tools component.
set -e
cd "$(dirname "$0")/.."

case "$(uname -s)" in
    Darwin) os=macos ;;
    *) os=linux ;;
esac
case "$(uname -m)" in
    arm64 | aarch64) arch=arm64 ;;
    *) arch=x86_64 ;;
esac

target="${CARGO_TARGET_DIR:-target}"
echo "building the compiler (release)"
cargo build -q --release -p kavach-cli

sysroot="$(rustc --print sysroot)"
tools="$(dirname "$(find "$sysroot" -name opt -type f | head -1)")"
if [ ! -f "$tools/opt" ] || [ ! -f "$tools/llc" ]; then
    echo "error: opt and llc not found; run: rustup component add llvm-tools" >&2
    exit 1
fi

version="0.1.0-$(git rev-parse --short HEAD)"
name="kavach-$version-$os-$arch"
dist=".kavach/dist/$name"
rm -rf "$dist"
mkdir -p "$dist/bin" "$dist/lib" "$dist/llvm"

cp "$target/release/kavach" "$dist/bin/kavach"
cp "$target/release/kavach" "$dist/bin/kv"
# The runtime library may sit in deps/ with a hash; the newest is current.
lib="$target/release/libkavach_runtime_native.a"
if [ ! -f "$lib" ]; then
    lib=$(ls -t "$target"/release/deps/libkavach_runtime_native-*.a | head -1)
fi
cp "$lib" "$dist/lib/libkavach_runtime_native.a"
cp "$tools/opt" "$tools/llc" "$dist/llvm/"
# On Linux and macOS the tools link LLVM as a shared library, and their
# run path is `$ORIGIN/../lib`: the install's `lib/` folder.
cp "$tools/../lib/"libLLVM* "$dist/lib/" 2> /dev/null ||
    cp "$tools/../../lib/"libLLVM* "$dist/lib/" 2> /dev/null || true
cp LICENSE-APACHE LICENSE-MIT "$dist/"
cat > "$dist/llvm/NOTICE.txt" <<'EOF'
opt and llc come from the Rust toolchain's llvm-tools component and are
licensed under Apache-2.0 WITH LLVM-exception: https://llvm.org/LICENSE.txt
EOF
cat > "$dist/README.txt" <<EOF
Kavach $version

Add bin/ to your PATH, or run scripts/install-kavach.sh from the
repository. Building programs also needs a C compiler (cc) for linking;
kavach says so if it is missing. Docs: https://mithlesh04.github.io/kavach/
EOF

tar -czf ".kavach/dist/$name.tar.gz" -C .kavach/dist "$name"
(cd .kavach/dist && sha256sum "$name.tar.gz" > "$name.tar.gz.sha256")
rm -rf "$dist"
echo "wrote .kavach/dist/$name.tar.gz"
