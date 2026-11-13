#!/usr/bin/env bash
# Builds the Mac edition on a Mac with Apple silicon:
#
#   tools/package-macos.sh <version> [--out <folder>] [--key-file <file> (--notes <text> | --notes-file <file>)]
#
# It writes into <folder> (default dist/<version>/macos):
#   Artosia.app                      the app: the operator, the projector program and the part
#                                    of GStreamer they need, signed ad hoc
#   Artosia-macos-aarch64.dmg        the download: the app, a shortcut to Applications and a
#                                    note on the first open
#   feed/                            with --key-file: the signed update list of this platform
#                                    and its files, to upload beside the Windows ones
#
# GStreamer is the official packages unpacked into a folder of our own (docs/macos.md), named
# by ARTOSIA_GSTREAMER_ROOT or found at ~/artosia-gst/root. Nothing asks for a password and
# nothing is paid for: the app is signed ad hoc, so macOS refuses its first open until the
# person presses Open Anyway (the README in the disk image says how).
set -euo pipefail

version="${1:?Give the version, such as 0.1.23}"
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
[ "$(uname -s)" = "Darwin" ] && [ "$(uname -m)" = "arm64" ] || { echo "Run this on a Mac with Apple silicon."; exit 1; }
grep -q "^version = \"$version\"" crates/artosia/Cargo.toml || { echo "crates/artosia/Cargo.toml is not at $version"; exit 1; }
out="${out:-dist/$version/macos}"
platform="macos-aarch64"

gst="${ARTOSIA_GSTREAMER_ROOT:-$HOME/artosia-gst/root}"
[ -d "$gst/lib/pkgconfig" ] && [ -d "$gst/lib/gstreamer-1.0" ] || { echo "No unpacked GStreamer at $gst (docs/macos.md says how to make it)."; exit 1; }
export ARTOSIA_GSTREAMER_ROOT="$gst"
# The framework's own pkg-config and nothing of Homebrew's: GStreamer's page on building for
# macOS says the two must not be mixed. Its .pc files find the folder they are in
# (`prefix=${pcfiledir}/../..`), so an unpacked copy needs no path set right.
export PKG_CONFIG="$gst/bin/pkg-config"
export PKG_CONFIG_LIBDIR="$gst/lib/pkgconfig"
unset PKG_CONFIG_PATH

# Speech recognition needs CMake and libclang; with both the operator carries it, as on Linux.
# Xcode has libclang, and whisper.cpp links only the system's own frameworks there, so the app
# gains no file.
features="artosia-output/video"
libclang="$(xcode-select -p 2>/dev/null)/Toolchains/XcodeDefault.xctoolchain/usr/lib"
if command -v cmake >/dev/null 2>&1 && [ -f "$libclang/libclang.dylib" ]; then
    export LIBCLANG_PATH="$libclang"
    features="$features,artosia/speech"
    echo "This build is WITH speech recognition."
else
    echo "This build is WITHOUT speech recognition (CMake or Xcode's libclang is missing)."
fi
cargo build --release -p artosia -p artosia-output --features "$features"

app="$out/Artosia.app"
contents="$app/Contents"
rm -rf "$out"
mkdir -p "$contents/MacOS" "$contents/Resources"
# The operator's file is named as the app is: macOS words its menu from the program's name
# ("Quit artosia" stood in the menu of 0.1.24, read from the real window on 2026-11-13).
install -m 755 target/release/artosia "$contents/MacOS/Artosia"
install -m 755 target/release/artosia-output "$contents/MacOS/artosia-output"
strip "$contents/MacOS/Artosia" "$contents/MacOS/artosia-output" 2>/dev/null
# The projector program loads GStreamer from the app's own copy, laid out as GStreamer's
# framework is, so every library in it finds the others by the run paths it was built with.
# The build's own run path, the unpacked folder on this Mac, is taken out: it means nothing on
# another computer.
install_name_tool -add_rpath "@executable_path/../Resources/gstreamer/lib" "$contents/MacOS/artosia-output"
while otool -l "$contents/MacOS/artosia-output" | grep -q "path $gst/lib "; do
    install_name_tool -delete_rpath "$gst/lib" "$contents/MacOS/artosia-output"
done
install -m 644 assets/brand/icons/artosia.icns "$contents/Resources/artosia.icns"
sed "s/{version}/$version/g" tools/macos/Info.plist > "$contents/Info.plist"
python3 tools/macos/info-strings.py "$contents/Resources"
python3 tools/gstreamer-subset.py --video-only --thin arm64 --copy "$contents/Resources/gstreamer"

# Signed inside out: every library, plugin and helper, the projector program, then the app,
# whose signature seals all of them. A file changed after this line breaks the seal.
find "$contents/Resources/gstreamer" -type f \( -name '*.dylib' -o -name 'gst-plugin-scanner' \) -print0 | xargs -0 -n 20 codesign --force --sign - 2>/dev/null
codesign --force --sign - "$contents/MacOS/artosia-output"
codesign --force --sign - "$app"
codesign --verify --deep --strict "$app"
echo "App: $app ($(du -sh "$app" | awk '{print $1}'), $(find "$app" -type f | wc -l | tr -d ' ') files, signature verified)"

image="$out/Artosia-$platform.dmg"
stage="$out/dmg"
mkdir -p "$stage"
cp -R "$app" "$stage/"
ln -s /Applications "$stage/Applications"
install -m 644 tools/macos/README.txt "$stage/Read me first.txt"
hdiutil create -volname "Artosia" -srcfolder "$stage" -ov -format UDZO -quiet "$image"
rm -rf "$stage"
echo "Package: $image ($(( ($(stat -f %z "$image") + 524288) / 1048576 )) MB)"

if [ -n "$key" ]; then
    cargo build --release -p artosia-update --bin artosia-release
    # A release is every file of Contents: the signature's own list is one of them, so a copy
    # that swaps in a release's files is that release, seal and all.
    target/release/artosia-release build --dir "$contents" --version "$version" --platform "$platform" \
        --out "$out/feed" --notes "$notes" --key-file "$key"
    echo "Update list: $out/feed ($(ls "$out/feed" | wc -l | tr -d ' ') files)"
fi
