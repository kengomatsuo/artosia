#!/usr/bin/env bash
# Builds, signs and publishes one release to GitHub, so every installed copy updates itself.
#
#   tools/publish.sh 0.1.1 "What changed, in a line or two"
#
# Before running: set the version in crates/artosia/Cargo.toml, commit, and be signed in to
# the GitHub CLI (gh auth status). The private signing key is read from the file named by
# ARTOSIA_KEY_FILE, or ~/.artosia-keys/update-signing.key; it is never printed.
# The release is uploaded as a draft and published last, so no copy sees half of it.
set -euo pipefail

version="${1:?Give the version, such as 0.1.1}"
notes="${2:-}"
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

key="${ARTOSIA_KEY_FILE:-$USERPROFILE/.artosia-keys/update-signing.key}"
[ -f "$key" ] || { echo "No signing key at $key"; exit 1; }
grep -q "^version = \"$version\"" crates/artosia/Cargo.toml || { echo "crates/artosia/Cargo.toml is not at $version"; exit 1; }
[ -z "$(git status --porcelain)" ] || { echo "Commit first: the release is built from what is committed"; exit 1; }

gst="${GSTREAMER_1_0_ROOT_MSVC_X86_64:-C:/Program Files/gstreamer/1.0/msvc_x86_64}"
export PKG_CONFIG="$gst/bin/pkg-config.exe" PKG_CONFIG_PATH="$gst/lib/pkgconfig"

# The programs, and a second projector built without video for a computer with no GStreamer.
cargo build --release -p artosia -p artosia-output -p artosia-stream --features artosia-output/video
cargo build --release -p artosia-output --target-dir target/plain
cargo build --release -p artosia-update --bin artosia-release --target-dir target/release-tool

programs="dist/$version/programs"
feed="dist/$version/feed"
rm -rf "dist/$version"
mkdir -p "$programs"
cp target/release/artosia.exe target/release/artosia-output.exe target/release/artosia-stream.exe "$programs/"
cp target/plain/release/artosia-output.exe "$programs/artosia-output-plain.exe"
# The part of GStreamer the programs need, so video plays on a computer that has none installed.
python tools/gstreamer-subset.py --copy "$programs/gstreamer"

target/release-tool/release/artosia-release.exe build --dir "$programs" --version "$version" --out "$feed" --notes "$notes" --key-file "$key"

# The same programs in one file, for a first install by hand.
zip="dist/$version/Artosia-windows.zip"
powershell -NoProfile -Command "Compress-Archive -Path '$programs/*' -DestinationPath '$zip' -Force"

git tag "v$version"
git push origin HEAD "v$version"
gh release create "v$version" "$zip" --draft --title "Artosia $version" --notes "$notes"
# About eighty files: a few at a time, so one slow upload does not fail the whole release.
find "$feed" -type f -print0 | xargs -0 -n 12 gh release upload "v$version" --clobber
gh release edit "v$version" --draft=false --latest
echo "Published v$version."
