#!/usr/bin/env bash
# Builds the Linux edition on a Linux machine (the WSL of the release PC):
#
#   tools/package-linux.sh <version> [--out <folder>] [--key-file <file> (--notes <text> | --notes-file <file>)]
#
# It writes into <folder> (default dist/<version>/linux):
#   Artosia/                         the folder a person unpacks: the programs, a menu entry,
#                                    the icons, install.sh and README.txt
#   Artosia-linux-x86_64.tar.gz      that folder, the download
#   feed/                            with --key-file: the signed update list of this platform
#                                    and its files, to upload beside the Windows ones
#
# The programs use the distribution's GStreamer (docs/linux.md), so nothing of it is packed.
# A second projector built without video is packed too: the operator starts it on a computer
# whose GStreamer is missing, so songs and verses still show.
set -euo pipefail

version="${1:?Give the version, such as 0.1.21}"
shift
out=""; key=""; notes=""
while [ $# -gt 0 ]; do
    case "$1" in
        --out) out="$2"; shift 2 ;;
        --key-file) key="$2"; shift 2 ;;
        --notes) notes="$2"; shift 2 ;;
        --notes-file) notes="$(cat "$2")"; shift 2 ;;
        *) echo "unknown argument: $1" >&2; exit 64 ;;
    esac
done
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
[ "$(uname -s)" = "Linux" ] || { echo "Run this on Linux."; exit 1; }
grep -q "^version = \"$version\"" crates/artosia/Cargo.toml || { echo "crates/artosia/Cargo.toml is not at $version"; exit 1; }
out="${out:-dist/$version/linux}"
platform="linux-$(uname -m)"

# Speech recognition needs CMake and libclang; with both the operator carries it.
features="artosia-output/video"
if command -v cmake >/dev/null 2>&1 && ls /usr/lib/llvm-*/lib/libclang.so* >/dev/null 2>&1; then
    features="$features,artosia/speech"
    echo "This build is WITH speech recognition."
else
    echo "This build is WITHOUT speech recognition (CMake or libclang is missing)."
fi

cargo build --release -p artosia -p artosia-output -p artosia-stream --features "$features"
cargo build --release -p artosia-output --target-dir target/plain

app="$out/Artosia"
rm -rf "$out"
mkdir -p "$app/share/applications" "$app/share/icons"
for program in artosia artosia-output artosia-stream; do
    install -m 755 "target/release/$program" "$app/$program"
done
install -m 755 target/plain/release/artosia-output "$app/artosia-output-plain"
strip "$app/artosia" "$app/artosia-output" "$app/artosia-stream" "$app/artosia-output-plain"
cp -r assets/brand/icons/linux/hicolor "$app/share/icons/hicolor"
install -m 644 tools/linux/artosia.desktop "$app/share/applications/artosia.desktop"
install -m 755 tools/linux/install.sh "$app/install.sh"
install -m 644 tools/linux/README.txt "$app/README.txt"

tar -C "$out" -czf "$out/Artosia-$platform.tar.gz" Artosia
echo "Package: $out/Artosia-$platform.tar.gz ($(( ($(stat -c %s "$out/Artosia-$platform.tar.gz") + 524288) / 1048576 )) MB)"

if [ -n "$key" ]; then
    cargo build --release -p artosia-update --bin artosia-release
    target/release/artosia-release build --dir "$app" --version "$version" --platform "$platform" \
        --out "$out/feed" --notes "$notes" --key-file "$key"
    echo "Update list: $out/feed ($(ls "$out/feed" | wc -l) files)"
fi
