#!/usr/bin/env bash
# Builds, signs and publishes one release to GitHub, so every installed copy updates itself.
#
#   tools/publish.sh 0.1.1 "What changed, in a line or two"
#
# Before running: set the version in crates/artosia/Cargo.toml, commit, and be signed in to
# the GitHub CLI (gh auth status). The private signing key is read from the file named by
# ARTOSIA_KEY_FILE, or ~/.artosia-keys/update-signing.key; it is never printed.
#
# With a code-signing certificate on this PC, set ARTOSIA_SIGN_SHA1 to its thumbprint:
# Artosia's own programs and the setup are then signed, so Windows names the publisher.
# The time stamp comes from Certum's server, as its manual gives it, unless
# ARTOSIA_TIMESTAMP_URL names another. Without the thumbprint the release is unsigned,
# as every release before the certificate was.
# The release is uploaded as a draft and published last, so no copy sees half of it.
set -euo pipefail

version="${1:?Give the version, such as 0.1.1}"
# `--finish` in place of the notes takes up a release that stopped as a draft, because the
# session ended or the PC went off, or that went out while the Mac was off: it builds and
# uploads only what the release still lacks.
finish=0
if [ "${2:-}" = "--finish" ]; then finish=1; notes=""; else notes="${2:-}"; fi
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

key="${ARTOSIA_KEY_FILE:-$USERPROFILE/.artosia-keys/update-signing.key}"
[ -f "$key" ] || { echo "No signing key at $key"; exit 1; }
files="https://github.com/kengomatsuo/artosia/releases/download/v$version"
windows="dist/$version/windows.md"
body="dist/$version/release-notes.md"
# The release page: the Windows part, then a line for each other system that built.
write_body() {
    cat "$windows" "dist/$version/linux.md" "dist/$version/macos.md" > "$body" 2>/dev/null || true
}
has_asset() {
    gh release view "v$version" --json assets --jq '.assets[].name' | grep -qx "$1"
}
if [ "$finish" = 1 ]; then
    draft="$(gh release view "v$version" --json isDraft --jq .isDraft 2>/dev/null)" || { echo "No release v$version to finish"; exit 1; }
    # A draft made before windows.md existed kept its Windows part alone in release-notes.md.
    [ -f "$windows" ] || cp "$body" "$windows"
    notes="$(sed '1,/^## What is new$/d' "$windows" | sed '1{/^$/d}')"
    echo "Finishing v$version."
else
    draft=true
    grep -q "^version = \"$version\"" crates/artosia/Cargo.toml || { echo "crates/artosia/Cargo.toml is not at $version"; exit 1; }
    [ -z "$(git status --porcelain)" ] || { echo "Commit first: the release is built from what is committed"; exit 1; }
fi

if [ "$finish" = 0 ]; then
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

    # Signs the given files with the certificate named by ARTOSIA_SIGN_SHA1 and checks each
    # signature; does nothing when no certificate is named. One call for all files, so a card
    # asks for its PIN once (SignTool: Microsoft's page, read 2026-11-12).
    sign() {
        [ -n "${ARTOSIA_SIGN_SHA1:-}" ] || return 0
        local tool
        tool=$(ls "/c/Program Files (x86)/Windows Kits/10/bin/"*/x64/signtool.exe 2>/dev/null | sort -V | tail -1)
        [ -n "$tool" ] || { echo "No signtool.exe: install the Windows SDK's signing tools"; exit 1; }
        "$tool" sign //sha1 "$ARTOSIA_SIGN_SHA1" //fd SHA256 //tr "${ARTOSIA_TIMESTAMP_URL:-http://time.certum.pl}" //td SHA256 \
            //d "Artosia" //du "https://artosia.matsuokengo.com" "$@"
        "$tool" verify //pa "$@"
    }
    if [ -n "${ARTOSIA_SIGN_SHA1:-}" ]; then
        echo "This release is SIGNED."
    else
        echo "This release is UNSIGNED: no certificate is named."
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
    # Our own programs are signed before the feed is built, because the feed records each
    # file's hash. GStreamer's files keep whatever signature they came with.
    sign "$programs/artosia.exe" "$programs/artosia-output.exe" "$programs/artosia-stream.exe" "$programs/artosia-output-plain.exe"

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
    if [ -n "${ARTOSIA_SIGN_SHA1:-}" ]; then
        # A signed setup carries its trust in the certificate, so each release ships a fresh one.
        cp "$built" "$setup"
        sign "$setup"
        echo "This release carries a newly built, signed setup."
    elif [ "${ARTOSIA_NEW_SETUP:-0}" != "1" ] && gh release download --pattern Artosia-setup.exe --dir "dist/$version" 2>/dev/null; then
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
    } > "$windows"

    git tag "v$version"
    git push origin HEAD "v$version"
    write_body
    gh release create "v$version" "$setup" "$zip" --draft --title "Artosia $version" --notes-file "$body"
    # About eighty files: a few at a time, so one slow upload does not fail the whole release.
    find "$feed" -type f -print0 | xargs -0 -n 12 gh release upload "v$version" --clobber
fi
printf '%s' "$notes" > "dist/$version/notes.txt"
# The Linux edition was built in this PC's WSL until 2026-11-14, when the owner put WSL off
# limits (it is suspected of dropping the F drive, where its disk lives). Until another Linux
# builder exists, a release carries no Linux files, the homepage keeps the newest release that
# has them, and installed Linux copies keep the version they have (docs/linux.md).
linux="dist/$version/linux"
if [ -f "dist/$version/linux.md" ] && has_asset Artosia-linux-x86_64.tar.gz; then
    echo "The release already carries the Linux edition."
else
    echo "The Linux edition is NOT built: WSL is off limits and no other Linux builder exists yet."
fi

# The Mac edition: built on the Mac, reached from Windows over Tailscale (`Host mac` in
# ~/.ssh/config), from the release's tag (docs/macos.md). The Mac makes its update folder too,
# where each file keeps its mode, under a throwaway key; only the list comes back to be signed
# here, so the real key never leaves this PC. When the Mac cannot be reached, or its build
# fails, the release carries no Mac files and says so.
mac="dist/$version/macos"
remote="Repositories/artosia/dist/$version/macos"
if [ -f "dist/$version/macos.md" ] && has_asset Artosia-macos-aarch64.dmg; then
    echo "The release already carries the Mac edition."
elif ssh -o BatchMode=yes mac true 2>/dev/null; then
    # Built every time: a tool left from an older release may lack a command used below.
    cargo build --release -q -p artosia-update --bin artosia-release --target-dir target/release-tool
    rm -rf "$mac" && mkdir -p "$mac"
    if scp -q "dist/$version/notes.txt" "mac:Repositories/artosia-notes-$version.txt" \
        && ssh -o BatchMode=yes mac "export PATH=\$HOME/.cargo/bin:\$HOME/.bun/bin:/opt/homebrew/bin:\$PATH
            cd ~/Repositories/artosia || exit 1
            git diff --quiet HEAD || { echo The clone on the Mac has changes that are not committed: a release is built from what is committed; exit 1; }
            git fetch -q origin --tags --force && git checkout -q origin/main -- tools/package-macos.sh && cp tools/package-macos.sh /tmp/artosia-package-macos.sh && git checkout -q HEAD -- tools/package-macos.sh \
                && git checkout -q --detach v$version \
                && nice -n 19 bash /tmp/artosia-package-macos.sh $version --out dist/$version/macos \
                && cargo build --release -q -p artosia-update --bin artosia-release \
                && rm -f /tmp/artosia-throwaway.key && target/release/artosia-release keygen /tmp/artosia-throwaway.key >/dev/null \
                && rm -rf dist/$version/macos/feed \
                && target/release/artosia-release build --dir dist/$version/macos/Artosia.app/Contents --version $version --platform macos-aarch64 --out dist/$version/macos/feed --notes-file ../artosia-notes-$version.txt --key-file /tmp/artosia-throwaway.key
            built=\$?; rm -f /tmp/artosia-throwaway.key; git checkout -q main && git merge -q --ff-only origin/main; exit \$built" \
        && ssh -o BatchMode=yes mac "cd ~/$remote && tar -cf - feed Artosia-macos-aarch64.dmg" | tar -xf - -C "$mac" \
        && target/release-tool/release/artosia-release.exe sign --dir "$mac/feed" --platform macos-aarch64 --key-file "$key"; then
        gh release upload "v$version" "$mac/Artosia-macos-aarch64.dmg" --clobber
        find "$mac/feed" -type f -print0 | xargs -0 -n 12 gh release upload "v$version" --clobber
        mac_megabytes=$(( ($(stat -c %s "$mac/Artosia-macos-aarch64.dmg") + 524288) / 1048576 ))
        {
            echo
            echo "**Mac (Apple silicon):** [Artosia-macos-aarch64.dmg]($files/Artosia-macos-aarch64.dmg) ($mac_megabytes MB). Drag Artosia to Applications. It is signed with a Developer ID and notarized by Apple."
        } > "dist/$version/macos.md"
        write_body
        gh release edit "v$version" --notes-file "$body"
        echo "This release carries the Mac edition."
    else
        echo "The Mac edition did NOT build: this release carries no Mac files."
    fi
else
    echo "The Mac cannot be reached over ssh: this release carries no Mac files."
fi

# A release already out may have newer ones after it, so only a draft is made the latest.
if [ "$draft" = "true" ]; then
    gh release edit "v$version" --draft=false --latest
    echo "Published v$version."
fi
# The homepage names the newest version and its size.
tools/publish-site.sh
