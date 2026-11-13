#!/bin/sh
# Adds Artosia to this person's applications menu. It writes one file under ~/.local/share and
# nothing else; nothing needs an administrator. Run it again after moving the folder.
#   ./install.sh            add the menu entry
#   ./install.sh --remove   take it away again
set -eu
here=$(cd "$(dirname "$0")" && pwd)
entry="${XDG_DATA_HOME:-$HOME/.local/share}/applications/artosia.desktop"
if [ "${1:-}" = "--remove" ]; then
    rm -f "$entry"
    echo "Artosia is no longer in the applications menu. This folder is untouched."
    exit 0
fi
mkdir -p "$(dirname "$entry")"
# The entry names this folder's program and icon by their full paths, so it works wherever
# the folder was unpacked (the Desktop Entry specification: an absolute Icon path is used as given).
sed -e "s|^Exec=.*|Exec=\"$here/artosia\"|" \
    -e "s|^Icon=.*|Icon=$here/share/icons/hicolor/256x256/apps/artosia.png|" \
    "$here/share/applications/artosia.desktop" > "$entry"
echo "Artosia is in the applications menu. To start it now: $here/artosia"
