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

# Speech recognition needs CMake and libclang to build; with both the operator carries it, and
# without them the release is built (and says so) with no recogniser.
# shellcheck source=speech-env.sh
. "$root/tools/speech-env.sh"
features="artosia-output/video"
if [ "${ARTOSIA_SPEECH:-0}" = "1" ]; then
    features="$features,artosia/speech"
    echo "This release is built WITH speech recognition."
else
    echo "This release is built WITHOUT speech recognition."
fi

# The programs, and a second projector built without video for a computer with no GStreamer.
cargo build --release -p artosia -p artosia-output -p artosia-stream --features "$features"
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

# The same programs in one file, for a computer with no internet.
zip="dist/$version/Artosia-windows.zip"
powershell -NoProfile -Command "Compress-Archive -Path '$programs/*' -DestinationPath '$zip' -Force"

# The setup: what a person downloads. Windows trusts an unsigned file by how often that
# exact file was downloaded, so a release carries the last release's setup unchanged and
# the trust carries over (docs/setup.md). ARTOSIA_NEW_SETUP=1 ships a newly built one,
# for a release that changed the setup's own code.
setup="dist/$version/Artosia-setup.exe"
cargo build --profile setup -p artosia-setup
built="target/setup/Artosia-setup.exe"
if [ "${ARTOSIA_NEW_SETUP:-0}" != "1" ] && gh release download --pattern Artosia-setup.exe --dir "dist/$version" 2>/dev/null; then
    if cmp -s "$setup" "$built"; then
        echo "The setup is the last release's file, and a new build gives the same bytes."
    else
        echo "The setup is the last release's file. A new build DIFFERS: if the setup's own code"
        echo "changed, run again with ARTOSIA_NEW_SETUP=1; a new file starts from no trust."
    fi
else
    cp "$built" "$setup"
    echo "This release carries a NEW setup file."
fi

# What a person reads first on the release page: the one file to download. The page lists
# about a hundred files of the self-update around it, and the owner could not tell which one
# to take (2026-11-11: "there's not even a clear downloadable in the release").
megabytes=$(( ($(stat -c %s "$zip") + 524288) / 1048576 ))
setup_megabytes=$(( ($(stat -c %s "$setup") + 1048575) / 1048576 ))
files="https://github.com/kengomatsuo/artosia/releases/download/v$version"
body="dist/$version/release-notes.md"
{
    echo "**Download: [Artosia for Windows]($files/Artosia-setup.exe)** ($setup_megabytes MB)"
    echo
    echo "Open it. It fetches Artosia, adds it to the desktop and the Start menu, and opens it. Windows may warn about an unknown publisher the first time, because the programs are not code-signed. A copy you already have updates itself, so there is nothing to download again."
    echo
    echo "For a computer with no internet: [Artosia-windows.zip]($files/Artosia-windows.zip) (zip, $megabytes MB), to unzip into a folder of your own and open \`artosia.exe\`."
    echo
    echo "Every other file under Assets belongs to the self-update."
    echo
    echo "## What is new"
    echo
    echo "$notes"
} > "$body"

git tag "v$version"
git push origin HEAD "v$version"
gh release create "v$version" "$setup" "$zip" --draft --title "Artosia $version" --notes-file "$body"
# About eighty files: a few at a time, so one slow upload does not fail the whole release.
find "$feed" -type f -print0 | xargs -0 -n 12 gh release upload "v$version" --clobber
gh release edit "v$version" --draft=false --latest
echo "Published v$version."
# The homepage names the newest version and its size.
tools/publish-site.sh
