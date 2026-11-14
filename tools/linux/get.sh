#!/usr/bin/env bash
# Artosia for Linux in one line:
#
#   curl -fsSL https://artosia.matsuokengo.com/install.sh | bash
#
# tools/build-site.py serves this file as install.sh on the homepage.
# Downloads the newest release that carries the Linux package, unpacks it into
# ~/.local/share/artosia and adds Artosia to the applications menu. Nothing needs an
# administrator, and running it again brings the newest version. The shape follows bun's
# installer (https://bun.sh/install): strict mode, curl that fails on an error page, a home
# folder install, and the whole script inside one function so half a download runs nothing.
set -euo pipefail

fail() {
    echo "artosia: $*" >&2
    exit 1
}

main() {
    local repo="kengomatsuo/artosia" file="Artosia-linux-x86_64.tar.gz"
    local home="${ARTOSIA_HOME:-${XDG_DATA_HOME:-$HOME/.local/share}/artosia}"

    [ "$(uname -s)" = "Linux" ] || fail "this installs the Linux edition; Windows and Mac downloads are at https://artosia.matsuokengo.com"
    [ "$(uname -m)" = "x86_64" ] || fail "the Linux edition is built for x86_64, and this computer is $(uname -m)"
    local tool
    for tool in curl tar; do
        command -v "$tool" >/dev/null 2>&1 || fail "$tool is needed and was not found"
    done
    # A folder that is there and is not an Artosia is somebody's own: it is left alone.
    if [ -e "$home" ] && [ ! -x "$home/artosia" ]; then
        fail "$home exists and holds no Artosia; set ARTOSIA_HOME to another folder"
    fi

    # The newest release that carries the Linux package. A release built while the Linux
    # build machine was off has none, so "latest" alone can point at nothing.
    # Read whole first: a pipe into head cuts curl off and it prints an error.
    local releases url
    releases="$(curl --fail --silent --show-error --location "https://api.github.com/repos/$repo/releases?per_page=30")" \
        || fail "GitHub did not answer; try again in a minute"
    url="$(grep -o "https://github.com/$repo/releases/download/[^\"]*/$file" <<<"$releases" | head -n 1 || true)"
    [ -n "$url" ] || fail "no release carries the Linux package right now"

    local scratch
    scratch="$(mktemp -d)"
    # shellcheck disable=SC2064
    trap "rm -rf '$scratch'" EXIT
    echo "Downloading $url"
    curl --fail --location --progress-bar --output "$scratch/$file" "$url"
    tar -xzf "$scratch/$file" -C "$scratch"
    [ -x "$scratch/Artosia/artosia" ] || fail "the download did not hold Artosia"

    # The new folder takes the old one's place in two renames, so an Artosia that is running
    # keeps the files it has open and the next start is the new version.
    mkdir -p "$(dirname "$home")"
    rm -rf "$home.new" "$home.old"
    mv "$scratch/Artosia" "$home.new"
    if [ -e "$home" ]; then
        mv "$home" "$home.old"
    fi
    mv "$home.new" "$home"
    rm -rf "$home.old"

    "$home/install.sh" >/dev/null
    echo "Artosia is installed in $home and is in the applications menu."
    echo "To start it now: $home/artosia"
    echo "Video backgrounds and streaming use your distribution's GStreamer; $home/README.txt names the packages."
}

main "$@"
